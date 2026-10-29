/* Hodegos — application : arborescence, onglets, éditeur, enregistrement, modifications externes,
   interlocuteurs de la section courante, remarques et état des passes. */
import { connect } from './connection.js';
import { createTree } from './tree.js';
import { createEditor, LANGUAGE_NAMES, locate } from './editor.js';
import { icon } from './icons.js';
import { createPanel } from './panel.js';
import { renderRemarks } from './remarks.js';

const $ = (id) => document.getElementById(id);
const ui = {
  path: $('chemin'),
  theme: $('theme'),
  tree: $('arborescence'),
  tabs: $('onglets'),
  banner: $('bandeau'),
  empty: $('vide'),
  editor: $('editeur'),
  position: $('position'),
  language: $('langage'),
  saveState: $('etat'),
  message: $('message'),
  connection: $('connexion'),
  titular: $('titulaire'),
  mentorErrors: $('mentors-erreurs'),
  mentors: $('interlocuteurs'),
  remarks: $('remarques'),
  pass: $('passe'),
  budget: $('budget'),
  provider: $('fournisseur'),
};

/** Délai de regroupement des déplacements du curseur avant envoi au serveur. */
const CURSOR_DELAY_MS = 150;

/**
 * Onglets ouverts, par chemin. Pour chacun : `version` connue du disque, `state` CodeMirror,
 * `savedDoc` (document au dernier enregistrement), `banner` éventuel :
 * `{ kind: 'changed' | 'conflict' | 'deleted', version }`.
 */
const tabs = new Map();
let currentPath = null;
let wasConnected = false;
/** Dernier `casting` reçu, par chemin. */
const castings = new Map();
/** Remarques affichées (ouvertes, répondues, détachées), par chemin. */
const remarksByPath = new Map();
/** Remarques qui attendent le mentor (palier suivant ou relance). */
const pendingRemarks = new Set();
/** Remarques dont la relance suit une réponse envoyée : elle arrive en `remark_update` spontané. */
const awaitingRelaunch = new Set();
/** Note affichée sous une remarque (« rien ajouté », panne…), par id. */
const remarkNotes = new Map();
/** Remarques dont le champ de réponse est ouvert, et texte saisi par remarque. */
const composing = new Set();
const drafts = new Map();
/** Remarque sélectionnée (carte et passage mis en avant). */
let activeRemark = null;
/** Noms des mentors connus, par `id`, pour signer les remarques. */
const mentorNames = new Map();
/** Dernier message d'indisponibilité du fournisseur affiché. */
let shownProviderMessage = null;
/** Dernier curseur envoyé : `{ path, line }`. */
let sentCursor = null;
let cursorTimer = null;

const server = connect({ onPush, onState });
const editor = createEditor(ui.editor, {
  onChange: () => renderTabs(),
  onCursor: ({ line, col }) => {
    ui.position.textContent = `l.${line}, col. ${col}`;
    scheduleCursor(line, col);
  },
  onAnchor: (id) => selectRemark(id, false),
});
const panel = createPanel(ui.mentors, { onForce: forceMentor });
const tree = createTree(ui.tree, {
  list: (path) => server.request('list_dir', { path }).then((reply) => {
    for (const entry of reply.payload.entries) rememberMentors(entry.mentors);
    return reply.payload.entries;
  }),
  onOpen: openFile,
  onError: showError,
});

/* ---------- Connexion ---------- */

function onState(state) {
  if (state === 'open') {
    setBadge('Connecté', 'success');
    tree.load();
    if (wasConnected) resync();
    wasConnected = true;
  } else {
    setBadge('Déconnecté', 'danger');
  }
}

function setBadge(text, variant) {
  ui.connection.textContent = text;
  ui.connection.className = `hd-badge hd-badge--${variant}`;
}

function onPush(message) {
  if (message.type === 'casting') {
    applyCasting(message.payload);
    return;
  }
  if (message.type === 'mentors_changed') {
    showMentorWarnings(message.payload.warnings);
    return;
  }
  if (message.type === 'pass_started') {
    rememberMentors(message.payload.mentors);
    const names = message.payload.mentors.map((m) => m.name).join(', ');
    ui.pass.textContent = `Passe en cours : ${names}…`;
    return;
  }
  if (message.type === 'remarks') {
    const { path, items } = message.payload;
    remarksByPath.set(path, [...(remarksByPath.get(path) ?? []), ...items]);
    if (path === currentPath) renderRemarksPanel();
    return;
  }
  if (message.type === 'remark_update') {
    const remark = message.payload;
    if (awaitingRelaunch.delete(remark.id)) {
      pendingRemarks.delete(remark.id);
      if (remark.thread.at(-1)?.author === 'user') {
        remarkNotes.set(remark.id, "Le mentor n'a rien ajouté pour l'instant.");
      }
    }
    applyRemark(remark);
    return;
  }
  if (message.type === 'status') {
    applyStatus(message.payload);
    return;
  }
  if (message.type !== 'file_changed') return;
  const { path, version } = message.payload;
  const tab = tabs.get(path);
  if (!tab || version === tab.version) return;
  tab.banner = version === null ? { kind: 'deleted' } : { kind: 'changed', version };
  render();
}

/* ---------- Mentors ---------- */

/** Envoie la ligne du curseur quand elle change, après un court délai. */
function scheduleCursor(line, col) {
  clearTimeout(cursorTimer);
  cursorTimer = setTimeout(() => sendCursor(line, col), CURSOR_DELAY_MS);
}

function sendCursor(line, col = 1) {
  if (!currentPath) return;
  if (sentCursor && sentCursor.path === currentPath && sentCursor.line === line) return;
  sentCursor = { path: currentPath, line };
  server.send('cursor', { path: currentPath, line, col });
}

/** Le serveur replace son curseur en ligne 1 à chaque `open_file` : on lui renvoie le nôtre. */
function resendCursor() {
  sentCursor = null;
  if (!currentPath) return;
  const head = editor.state.selection.main.head;
  sendCursor(editor.state.doc.lineAt(head).number);
}

function rememberMentors(mentors) {
  for (const mentor of mentors ?? []) mentorNames.set(mentor.id, mentor.name);
}

function applyCasting(casting) {
  rememberMentors([casting.titular, ...casting.consultants, ...casting.available].filter(Boolean));
  castings.set(casting.path, casting);
  if (casting.path === currentPath) renderMentors();
}

async function forceMentor(mentorId) {
  const casting = castings.get(currentPath);
  if (!casting) return;
  try {
    const reply = await server.request('cast_override', {
      path: currentPath,
      section_id: casting.section_id,
      mentor_id: mentorId,
    });
    applyCasting(reply.payload);
  } catch (error) {
    showError(error);
  }
}

function renderMentors() {
  const casting = castings.get(currentPath) ?? null;
  panel.render(casting);
  ui.titular.replaceChildren();
  if (!casting) return;
  if (!casting.titular) {
    ui.titular.textContent = 'Aucun mentor pour cette section';
    return;
  }
  const name = document.createElement('strong');
  name.className = 'hd-mentor__name';
  name.textContent = casting.titular.name;
  ui.titular.append('Titulaire : ', name);
  if (casting.consultants.length) {
    ui.titular.append(` · consultant : ${casting.consultants.map((m) => m.name).join(', ')}`);
  }
}

function showMentorWarnings(warnings) {
  ui.mentorErrors.hidden = warnings.length === 0;
  if (!warnings.length) {
    showMessage('Mentors rechargés.');
    return;
  }
  ui.mentorErrors.className = 'hd-badge hd-badge--warning';
  ui.mentorErrors.textContent = warnings.length === 1 ? '1 mentor ignoré' : `${warnings.length} mentors ignorés`;
  ui.mentorErrors.title = warnings.map((w) => `${w.file} : ${w.message}`).join('\n');
  const file = warnings[0].file.split('/').pop();
  showMessage(`Mentor ignoré : ${file} : ${warnings[0].message}`);
}

/* ---------- Remarques ---------- */

const VISIBLE = new Set(['open', 'answered', 'detached']);

/** Remarques affichées d'un fichier, numérotées dans l'ordre du texte, avec leur position. */
function remarkItems(path) {
  const tab = tabs.get(path);
  if (!tab) return [];
  const doc = path === currentPath ? editor.state.doc : tab.state.doc;
  return (remarksByPath.get(path) ?? [])
    .filter((remark) => VISIBLE.has(remark.status))
    .sort((a, b) => a.anchor.line_hint - b.anchor.line_hint)
    .map((remark, index) => ({
      remark,
      number: index + 1,
      // Une remarque détachée n'a plus de passage, même si la citation existe ailleurs.
      position: remark.status === 'detached' ? null : locate(doc, remark.anchor.quote, remark.anchor.line_hint),
    }));
}

/** Remplace une remarque par son nouvel état (palier, fil, statut). */
function applyRemark(remark) {
  const list = remarksByPath.get(remark.path);
  if (!list) return;
  const index = list.findIndex((r) => r.id === remark.id);
  if (index >= 0) list[index] = remark;
  else list.push(remark);
  if (!VISIBLE.has(remark.status)) {
    composing.delete(remark.id);
    drafts.delete(remark.id);
    if (activeRemark === remark.id) activeRemark = null;
  }
  if (remark.path === currentPath) renderRemarksPanel();
}

function confirmOverBudget() {
  return window.confirm(
    "Le budget du mois est atteint. Envoyer quand même cette demande ? Son coût s'ajoutera au mois en cours.",
  );
}

function remarkError(remark, error) {
  if (error.code === 'provider_error') {
    remarkNotes.set(remark.id, `OpenRouter indisponible : ${error.message}`);
  } else {
    showError(error);
  }
}

async function hintNext(remark, confirm = false) {
  remarkNotes.delete(remark.id);
  pendingRemarks.add(remark.id);
  renderRemarksPanel();
  let retry = false;
  try {
    const reply = await server.request('hint_next', { remark_id: remark.id, confirm_over_budget: confirm });
    if (reply.payload.levels_given.length === remark.levels_given.length) {
      remarkNotes.set(remark.id, "Le mentor n'a pas pu formuler le palier suivant ; réessayez plus tard.");
    }
    pendingRemarks.delete(remark.id);
    applyRemark(reply.payload);
  } catch (error) {
    if (error.code === 'budget_exceeded' && !confirm) retry = confirmOverBudget();
    else remarkError(remark, error);
  } finally {
    pendingRemarks.delete(remark.id);
    renderRemarksPanel();
  }
  if (retry) await hintNext(remark, true);
}

async function sendReply(remark, text, confirm = false) {
  if (!text.trim()) return;
  remarkNotes.delete(remark.id);
  pendingRemarks.add(remark.id);
  renderRemarksPanel();
  let retry = false;
  try {
    const reply = await server.request('remark_reply', { remark_id: remark.id, text, confirm_over_budget: confirm });
    // Le mentor répond dans un second `remark_update`, spontané.
    awaitingRelaunch.add(remark.id);
    composing.delete(remark.id);
    drafts.delete(remark.id);
    applyRemark(reply.payload);
  } catch (error) {
    pendingRemarks.delete(remark.id);
    if (error.code === 'budget_exceeded' && !confirm) retry = confirmOverBudget();
    else remarkError(remark, error);
    renderRemarksPanel();
  }
  if (retry) await sendReply(remark, text, true);
}

async function dismissRemark(remark, reason) {
  try {
    const reply = await server.request('remark_dismiss', { remark_id: remark.id, reason });
    remarkNotes.delete(remark.id);
    applyRemark(reply.payload);
  } catch (error) {
    showError(error);
  }
}

function renderRemarksPanel() {
  const items = currentPath ? remarkItems(currentPath) : [];
  renderRemarks(ui.remarks, items.map(({ remark, number, position }) => ({ remark, number, located: Boolean(position) })), {
    activeId: activeRemark,
    mentorName: (id) => mentorNames.get(id) ?? id,
    onSelect: (remark) => selectRemark(remark.id, true),
    pending: pendingRemarks,
    notes: remarkNotes,
    composing,
    drafts,
    onHint: (remark) => hintNext(remark),
    onCompose: (remark, open) => {
      if (open) composing.add(remark.id);
      else composing.delete(remark.id);
      renderRemarksPanel();
      if (open) document.getElementById(`reponse-${remark.id}`)?.focus();
    },
    onDraft: (id, text) => drafts.set(id, text),
    onReply: (remark, text) => sendReply(remark, text),
    onDismiss: (remark, reason) => dismissRemark(remark, reason),
  });
  if (!currentPath) return;
  editor.showAnchors(items
    .filter((item) => item.position)
    .map(({ remark, number, position }) => ({
      id: remark.id,
      number,
      from: position.from,
      to: position.to,
      active: remark.id === activeRemark,
    })));
}

/** Met en avant une remarque ; `reveal` amène aussi son passage à l'écran. */
function selectRemark(id, reveal) {
  activeRemark = id;
  renderRemarksPanel();
  if (!reveal) return;
  const item = remarkItems(currentPath).find((i) => i.remark.id === id);
  if (item?.position) editor.reveal(item.position.from);
}

/* ---------- État du fournisseur, du budget et des passes ---------- */

const OUTCOMES = {
  no_change: 'aucun changement',
  no_mentor: 'aucun mentor pour les sections modifiées',
  excluded: 'fichier sensible, rien n\'est envoyé',
  budget_exceeded: 'budget du mois atteint, pas de passe',
  provider_error: 'fournisseur indisponible, passe abandonnée',
  invalid_response: 'réponse du mentor invalide, passe abandonnée',
};

const money = new Intl.NumberFormat('fr-FR', { style: 'currency', currency: 'USD' });

function applyStatus(status) {
  const last = status.last_pass;
  if (last) {
    let text = OUTCOMES[last.outcome];
    if (last.outcome === 'ok') {
      const count = last.remarks === 0 ? 'aucune remarque' : last.remarks === 1 ? '1 remarque' : `${last.remarks} remarques`;
      text = `${count} en ${(last.duration_ms / 1000).toLocaleString('fr-FR', { maximumFractionDigits: 1 })} s`;
    }
    ui.pass.textContent = `Dernière passe : ${text}`;
  }

  ui.provider.replaceChildren();
  if (status.provider_ok) {
    const dot = document.createElement('span');
    dot.className = 'hd-dot';
    dot.setAttribute('aria-hidden', 'true');
    ui.provider.className = 'app-provider';
    ui.provider.append(dot, 'OpenRouter prêt');
    ui.provider.title = '';
    shownProviderMessage = null;
  } else {
    ui.provider.className = 'hd-badge hd-badge--danger';
    ui.provider.textContent = 'OpenRouter indisponible';
    ui.provider.title = status.provider_message ?? '';
    if (status.provider_message && status.provider_message !== shownProviderMessage) {
      shownProviderMessage = status.provider_message;
      showMessage(`OpenRouter : ${status.provider_message}`);
    }
  }

  const used = `${money.format(status.budget_used)} sur ${money.format(status.budget_cap)}`;
  ui.budget.hidden = status.budget_level === 'ok';
  ui.budget.title = `Budget du mois : ${used}`;
  if (status.budget_level === 'warning') {
    ui.budget.className = 'hd-badge hd-badge--warning';
    ui.budget.textContent = `Budget : ${used}`;
  } else if (status.budget_level === 'exceeded') {
    ui.budget.className = 'hd-badge hd-badge--danger';
    ui.budget.textContent = 'Budget du mois atteint';
  }
}

/** Après une reconnexion, le serveur ne suit plus les fichiers : on les rouvre. */
async function resync() {
  for (const tab of tabs.values()) {
    try {
      const reply = await server.request('open_file', { path: tab.path });
      remarksByPath.set(tab.path, reply.payload.remarks);
      if (reply.payload.version !== tab.version) {
        tab.banner = { kind: 'changed', version: reply.payload.version };
      }
    } catch (error) {
      if (error.code === 'not_found') tab.banner = { kind: 'deleted' };
    }
  }
  render();
  resendCursor();
}

/* ---------- Fichiers ---------- */

async function openFile(path) {
  if (tabs.has(path)) {
    activate(path);
    return;
  }
  try {
    const { payload } = await server.request('open_file', { path });
    remarksByPath.set(path, payload.remarks);
    const state = editor.createState(payload.content, payload.language, path);
    tabs.set(path, {
      path,
      language: payload.language,
      version: payload.version,
      state,
      savedDoc: state.doc,
      banner: null,
      saving: false,
    });
    activate(path);
  } catch (error) {
    showError(error);
  }
}

async function reload(tab) {
  try {
    const { payload } = await server.request('open_file', { path: tab.path });
    remarksByPath.set(tab.path, payload.remarks);
    tab.state = editor.createState(payload.content, payload.language, tab.path);
    tab.savedDoc = tab.state.doc;
    tab.version = payload.version;
    tab.banner = null;
    if (tab.path === currentPath) editor.show(tab.state);
    render();
    resendCursor();
  } catch (error) {
    if (error.code === 'not_found') {
      tab.banner = { kind: 'deleted' };
      render();
    } else {
      showError(error);
    }
  }
}

/** Enregistre l'onglet courant ; `baseVersion` force l'écrasement après un conflit. */
async function save(baseVersion) {
  const tab = tabs.get(currentPath);
  if (!tab) return;
  if (tab.banner?.kind === 'deleted') {
    showMessage('Ce fichier a été supprimé ; il ne peut plus être enregistré.');
    return;
  }
  if (tab.saving) return;
  tab.saving = true;
  const doc = editor.state.doc;
  try {
    const { payload } = await server.request('save', {
      path: tab.path,
      content: doc.toString(),
      base_version: baseVersion ?? tab.version,
    });
    tab.version = payload.version;
    tab.savedDoc = doc;
    tab.banner = null;
    showMessage('');
  } catch (error) {
    if (error.code === 'version_conflict') {
      tab.banner = { kind: 'conflict', version: error.version };
    } else {
      showError(error);
    }
  } finally {
    tab.saving = false;
  }
  render();
}

function isDirty(tab) {
  const state = tab.path === currentPath ? editor.state : tab.state;
  return !state.doc.eq(tab.savedDoc);
}

/* ---------- Onglets ---------- */

function activate(path) {
  const previous = tabs.get(currentPath);
  if (previous) previous.state = editor.state;
  currentPath = path;
  const tab = tabs.get(path);
  if (tab) {
    editor.show(tab.state);
    editor.focus();
    clearTimeout(cursorTimer);
    const head = tab.state.selection.main.head;
    sendCursor(tab.state.doc.lineAt(head).number);
  }
  render();
}

function closeTab(path) {
  const tab = tabs.get(path);
  if (!tab) return;
  if (isDirty(tab) && !window.confirm(`Fermer ${tab.path} sans enregistrer vos modifications ?`)) return;
  if (path === currentPath) tab.state = editor.state;
  tabs.delete(path);
  castings.delete(path);
  remarksByPath.delete(path);
  if (path === currentPath) {
    currentPath = null;
    const next = [...tabs.keys()].pop();
    if (next) {
      activate(next);
      return;
    }
  }
  render();
}

/* ---------- Affichage ---------- */

function render() {
  renderTabs();
  renderBanner();
  const tab = tabs.get(currentPath);
  ui.tabs.hidden = tabs.size === 0;
  ui.editor.hidden = !tab;
  ui.empty.hidden = Boolean(tab);
  ui.path.textContent = tab ? tab.path : '';
  ui.language.textContent = tab ? LANGUAGE_NAMES[tab.language] ?? tab.language : '';
  if (!tab) ui.position.textContent = '';
  document.title = tab ? `${tab.path.split('/').pop()} — Hodegos` : 'Hodegos';
  tree.setCurrent(currentPath);
  renderMentors();
  renderRemarksPanel();
}

function renderTabs() {
  const items = [...tabs.values()].map((tab) => {
    const wrapper = document.createElement('div');
    wrapper.className = 'app-tab';
    wrapper.setAttribute('role', 'presentation');

    const button = document.createElement('button');
    button.type = 'button';
    button.className = 'hd-tab';
    button.setAttribute('role', 'tab');
    button.setAttribute('aria-controls', 'editeur');
    button.setAttribute('aria-selected', String(tab.path === currentPath));
    button.tabIndex = tab.path === currentPath ? 0 : -1;
    button.title = tab.path;
    button.textContent = tab.path.split('/').pop();
    if (isDirty(tab)) {
      const dirty = document.createElement('span');
      dirty.className = 'app-tab__dirty';
      dirty.textContent = 'modifié';
      button.append(dirty);
    }
    button.addEventListener('click', () => activate(tab.path));

    const close = document.createElement('button');
    close.type = 'button';
    close.className = 'hd-btn hd-btn--ghost hd-btn--sm app-tab__close';
    close.setAttribute('aria-label', `Fermer ${tab.path}`);
    close.append(icon('close'));
    close.addEventListener('click', () => closeTab(tab.path));

    wrapper.append(button, close);
    return wrapper;
  });
  ui.tabs.replaceChildren(...items);

  const tab = tabs.get(currentPath);
  ui.saveState.textContent = tab ? (isDirty(tab) ? 'Modifié' : 'Enregistré') : '';
}

const BANNERS = {
  changed: {
    title: 'Ce fichier a été modifié hors de Hodegos.',
    text: (tab) => (isDirty(tab)
      ? 'Recharger remplace vos modifications non enregistrées par la version du disque.'
      : 'Rechargez pour afficher la version du disque.'),
    actions: (tab) => [
      button('Recharger', 'secondary', () => reload(tab)),
      button('Garder ma version', 'ghost', () => { tab.banner = null; render(); }),
    ],
  },
  conflict: {
    title: 'Le fichier a changé sur le disque depuis son ouverture.',
    text: () => 'Rechargez pour reprendre la version du disque, ou écrasez-la avec la vôtre.',
    actions: (tab) => [
      button('Recharger', 'secondary', () => reload(tab)),
      button('Écraser', 'danger', () => save(tab.banner.version)),
    ],
  },
  deleted: {
    title: 'Ce fichier a été supprimé hors de Hodegos.',
    text: () => 'Votre version reste lisible dans cet onglet, mais ne peut plus être enregistrée.',
    actions: (tab) => [button("Fermer l'onglet", 'secondary', () => closeTab(tab.path))],
  },
};

function renderBanner() {
  const tab = tabs.get(currentPath);
  const banner = tab?.banner && BANNERS[tab.banner.kind];
  ui.banner.hidden = !banner;
  if (!banner) {
    ui.banner.replaceChildren();
    return;
  }
  const callout = document.createElement('div');
  callout.className = 'hd-callout hd-callout--warning';
  callout.setAttribute('role', 'status');
  const title = document.createElement('p');
  title.className = 'hd-callout__title';
  title.textContent = banner.title;
  const text = document.createElement('p');
  text.textContent = banner.text(tab);
  const actions = document.createElement('div');
  actions.className = 'app-banner__actions';
  actions.append(...banner.actions(tab));
  callout.append(title, text, actions);
  ui.banner.replaceChildren(callout);
}

function button(label, variant, onClick) {
  const element = document.createElement('button');
  element.type = 'button';
  element.className = `hd-btn hd-btn--${variant} hd-btn--sm`;
  element.textContent = label;
  element.addEventListener('click', onClick);
  return element;
}

const ERROR_TEXTS = {
  not_text: "Hodegos n'ouvre que les fichiers texte UTF-8 de 2 Mo au plus.",
  path_forbidden: 'Ce chemin est hors du projet, sous .git ou exclu par .gitignore.',
  not_found: 'Fichier introuvable.',
  disconnected: 'Pas de connexion au serveur.',
};

function showError(error) {
  showMessage(ERROR_TEXTS[error.code] ?? `Erreur : ${error.message}`);
}

function showMessage(text) {
  ui.message.textContent = text;
}

/* ---------- Thème et raccourcis ---------- */

function currentTheme() {
  return document.documentElement.getAttribute('data-theme') === 'dark' ? 'dark' : 'light';
}

function renderThemeButton() {
  ui.theme.textContent = currentTheme() === 'dark' ? 'Thème clair' : 'Thème sombre';
}

ui.theme.addEventListener('click', () => {
  const next = currentTheme() === 'dark' ? 'light' : 'dark';
  window.Hodegos.setTheme(next);
  try {
    window.localStorage.setItem('hodegos.theme', next);
  } catch {
    /* Choix non mémorisé : il reste valable pour cette page. */
  }
  renderThemeButton();
});

document.addEventListener('keydown', (event) => {
  if ((event.ctrlKey || event.metaKey) && !event.altKey && event.key.toLowerCase() === 's') {
    event.preventDefault();
    save();
  }
});

window.addEventListener('beforeunload', (event) => {
  if ([...tabs.values()].some(isDirty)) event.preventDefault();
});

renderThemeButton();
render();
