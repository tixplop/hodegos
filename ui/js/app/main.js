/* Hodegos — application : arborescence (création, suppression, déplacement, statut git), onglets, éditeur,
   Markdown en source, aperçu ou lecture, enregistrement, modifications externes, interlocuteurs
   de la section courante, remarques, état des passes, réglages personnels et vue Projet. */
import { connect } from './connection.js';
import { createTree } from './tree.js';
import { confirmDeletion } from './deletion.js';
import { chooseDialog } from './dialog.js';
import { createEditor, LANGUAGE_NAMES, locate } from './editor.js';
import { icon } from './icons.js';
import { createReader } from './md-read.js';
import { createPanel } from './panel.js';
import { createProjectView } from './project.js';
import { createProjectRemarks } from './project-remarks.js';
import { renderRemarks } from './remarks.js';
import { createMentorsSettings } from './mentors-settings.js';
import { chooseDestination } from './move.js';
import { createReviewSettings } from './review-settings.js';
import { createSettings } from './settings.js';
import { createSidebarResizer } from './sidebar.js';
import { createTerminalPanel } from './terminal.js';

const $ = (id) => document.getElementById(id);
const ui = {
  path: $('chemin'),
  theme: $('theme'),
  openSettings: $('ouvrir-reglages'),
  openProject: $('ouvrir-projet'),
  project: $('projet'),
  openTerminal: $('ouvrir-terminal'),
  terminals: $('terminaux'),
  ide: $('ide'),
  settings: $('reglages'),
  evaluation: $('evaluation'),
  tree: $('arborescence'),
  tabBar: $('barre-onglets'),
  tabs: $('onglets'),
  modes: $('modes'),
  reading: $('lecture'),
  banner: $('bandeau'),
  empty: $('vide'),
  editor: $('editeur'),
  position: $('position'),
  language: $('langage'),
  git: $('git'),
  gitLegend: $('legende-git'),
  newFile: $('nouveau-fichier'),
  newDir: $('nouveau-dossier'),
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

/** Modes d'affichage du Markdown ; Ctrl+E bascule entre le dernier mode d'édition et la lecture. */
const MODES = ['source', 'preview', 'reading'];
const EDIT_MODES = ['source', 'preview'];
const MODE_LABELS = { source: 'source', preview: 'aperçu en direct', reading: 'lecture' };

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
/** Passe en cours : `{ path, sections, mentors, since }`, `since` étant la dernière passe
    connue à son départ ; elle s'achève au `status` qui en annonce une autre. */
let passing = null;
/** `at` de la dernière passe annoncée par `status`. */
let lastPassAt = null;
/** Dernier `git_status` reçu : `null` sans dépôt, `undefined` avant le premier. */
let gitStatus;
/** Dernier curseur envoyé : `{ path, line }`. */
let sentCursor = null;
let cursorTimer = null;
/** Mode d'affichage du Markdown, global et mémorisé comme le thème. */
let markdownMode = stored('hodegos.markdown.mode', MODES, 'preview');
let editMode = stored('hodegos.markdown.edit', EDIT_MODES, 'preview');
/** Derniers réglages reçus : évaluation en cours, résultats sous le seuil. */
let lastSettings = null;

const server = connect({ onPush, onState });
const editor = createEditor(ui.editor, {
  onChange: () => renderTabs(),
  onCursor: ({ line, col }) => {
    ui.position.textContent = `l.${line}, col. ${col}`;
    scheduleCursor(line, col);
  },
  onAnchor: (id) => (id.startsWith('rp_') ? openProjectRemark(id) : selectRemark(id, false)),
  onLink: openFile,
});
const reader = createReader(ui.reading, {
  onLink: openFile,
  // Double-clic : retour en aperçu sur ce bloc.
  onBlock: (pos) => {
    setMarkdownMode('preview');
    editor.reveal(pos);
  },
  onAnchor: (id) => (id.startsWith('rp_') ? openProjectRemark(id) : selectRemark(id, false)),
});
for (const choice of ui.modes.querySelectorAll('[data-value]')) {
  choice.addEventListener('click', () => setMarkdownMode(choice.dataset.value));
}
const mentorsPage = createMentorsSettings($('mentors-liste'), $('mentors-fiche'), {
  request: (type, payload) => server.request(type, payload),
  onCount: (count) => { $('nav-mentors').textContent = String(count); },
});
const settings = createSettings(ui.settings, {
  request: (type, payload) => server.request(type, payload),
  onSettings: renderEvaluation,
  mentors: mentorsPage,
  reviews: createReviewSettings($('page-revues'), {
    request: (type, payload) => server.request(type, payload),
  }),
});
ui.openSettings.addEventListener('click', () => showSettings(ui.settings.hidden));
const projectView = createProjectView(ui.project, {
  request: (type, payload) => server.request(type, payload),
});
const projectRemarks = createProjectRemarks(ui.project, {
  request: (type, payload) => server.request(type, payload),
  onOpenAt: (path, line) => {
    showProject(false);
    openFile(path, line);
  },
  onShowMap: () => showProjectTab('carte'),
  mentorName: (id) => mentorNames.get(id) ?? id,
  onCount: () => {},
  onChange: () => {
    renderRemarksPanel();
    tree.setProjectPaths(projectRemarks.citedPaths());
    if (lastProjectStatus) renderProjectStatus(lastProjectStatus);
  },
});
ui.openProject.addEventListener('click', () => showProject(ui.project.hidden));
for (const tab of ui.project.querySelectorAll('[role="tab"]')) {
  tab.addEventListener('click', () => showProjectTab(tab.dataset.tab));
}
const terminalPanel = createTerminalPanel({
  section: $('terminal'),
  handle: $('terminal-poignee'),
  request: (type, payload) => server.request(type, payload),
  send: (type, payload) => server.send(type, payload),
  onCount: (count) => {
    ui.terminals.hidden = count === 0;
    ui.terminals.textContent = count === 1 ? '1 terminal' : `${count} terminaux`;
  },
  onToggle: (open) => ui.openTerminal.setAttribute('aria-expanded', String(open)),
});
ui.openTerminal.addEventListener('click', () => {
  backToIde();
  terminalPanel.toggle();
});
const panel = createPanel(ui.mentors, { onActivate: activateMentor, onAutomatic: automaticMentor });
createSidebarResizer({
  sidebar: $('fichiers'),
  handle: $('arbre-poignee'),
  panel: ui.ide.querySelector('.app-panel'),
});
const tree = createTree(ui.tree, {
  list: (path) => server.request('list_dir', { path }).then((reply) => {
    for (const entry of reply.payload.entries) rememberMentors(entry.mentors);
    return reply.payload.entries;
  }),
  create: (path, kind) => server.request('create_entry', { path, kind }),
  move: (from, toDir) => server.request('move_entry', { from, to_dir: toDir }),
  onOpen: openFile,
  onCreated: (path, kind) => {
    if (kind === 'file') openFile(path);
  },
  onDelete: deleteEntry,
  onMoveTo: moveTo,
  onMoved: followMove,
  onError: showError,
  zone: $('fichiers'),
  head: $('fichiers').querySelector('.app-tree__head'),
  problem: $('arbre-erreur'),
});
ui.newFile.append(icon('file-plus'));
ui.newDir.append(icon('folder-plus'));
ui.newFile.addEventListener('click', () => tree.startCreate('', 'file'));
ui.newDir.addEventListener('click', () => tree.startCreate('', 'dir'));

/* ---------- Connexion ---------- */

function onState(state) {
  if (state === 'open') {
    setBadge('Connecté', 'success');
    tree.load();
    settings.load();
    mentorsPage.count();
    projectRemarks.load();
    if (wasConnected) resync();
    wasConnected = true;
  } else {
    setBadge('Déconnecté', 'danger');
    terminalPanel.disconnected();
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
    mentorsPage.reloaded();
    return;
  }
  if (message.type === 'pass_started') {
    const { path, sections, mentors } = message.payload;
    rememberMentors(mentors);
    passing = { path, sections, mentors: new Set(mentors.map((m) => m.id)), since: lastPassAt };
    const names = mentors.map((m) => m.name).join(', ');
    ui.pass.textContent = `Passe en cours : ${names} sur ${sections.join(', ')}…`;
    if (path === currentPath) renderMentors();
    return;
  }
  if (message.type === 'remarks') {
    const { path, items } = message.payload;
    remarksByPath.set(path, [...(remarksByPath.get(path) ?? []), ...items]);
    if (path === currentPath) renderRemarksPanel();
    return;
  }
  // Remarque de projet (phase 9c) : la vue Projet la tient à jour.
  if (message.type === 'remark_update' && message.payload.scope === 'project') {
    projectRemarks.remarkUpdated(message.payload);
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
  if (message.type === 'term_output') {
    terminalPanel.output(message.payload);
    return;
  }
  if (message.type === 'term_exited') {
    terminalPanel.exited(message.payload);
    return;
  }
  if (message.type === 'settings') {
    settings.apply(message.payload);
    return;
  }
  if (message.type === 'map_updated') {
    projectView.updated(!ui.project.hidden && projectTab === 'carte');
    return;
  }
  if (message.type === 'project_status') {
    renderProjectStatus(message.payload);
    return;
  }
  if (message.type === 'review_started') {
    projectRemarks.reviewStarted(message.payload);
    return;
  }
  if (message.type === 'review_done') {
    projectRemarks.reviewDone(message.payload);
    return;
  }
  if (message.type === 'project_remarks') {
    projectRemarks.remarksArrived(message.payload.items);
    return;
  }

  if (message.type === 'eval_progress') {
    settings.progress(message.payload);
    renderEvaluation(null, message.payload);
    return;
  }
  if (message.type === 'eval_done') {
    const { level, passed } = message.payload;
    showMessage(`Évaluation du ${LEVEL_NAMES[level]} terminée : ${passed ? 'retenu' : 'sous le seuil'}.`);
    return;
  }
  if (message.type === 'dir') {
    // Entrées d'un dossier affiché changées sur le disque (terminal, autre outil).
    tree.refresh();
    return;
  }
  if (message.type === 'git_status') {
    gitStatus = message.payload;
    tree.setGit(gitStatus);
    renderGit();
    return;
  }
  if (message.type === 'status') {
    applyStatus(message.payload);
    // Premier status après `hodegos new` : la vision s'ouvre d'elle-même.
    const initial = message.payload.initial_file;
    if (initial && !currentPath) openFile(initial);
    return;
  }
  if (message.type !== 'file_changed') return;
  const { path, version } = message.payload;
  const tab = tabs.get(path);
  if (!tab || version === tab.version) return;
  tab.banner = version === null ? { kind: 'deleted' } : { kind: 'changed', version };
  render();
}

/* ---------- Réglages ---------- */

const LEVEL_NAMES = { fast: 'niveau rapide', strong: 'niveau fort' };

/** L'IDE est affiché : ni les réglages ni la vue Projet. */
function inIde() {
  return ui.settings.hidden && ui.project.hidden;
}

function backToIde() {
  if (!ui.settings.hidden) showSettings(false);
  if (!ui.project.hidden) showProject(false);
}

/** Onglet de la vue Projet : `remarques`, `carte` ou `historique`. */
let projectTab = 'remarques';

function showProjectTab(name) {
  projectTab = name;
  for (const tab of ui.project.querySelectorAll('[role="tab"]')) {
    const selected = tab.dataset.tab === name;
    tab.setAttribute('aria-selected', String(selected));
    tab.tabIndex = selected ? 0 : -1;
    $(tab.getAttribute('aria-controls')).hidden = !selected;
  }
  if (name === 'carte') projectView.load();
}

/** La vue Projet remplace l'IDE, comme les réglages : remarques, carte et historique. */
function showProject(open) {
  if (open && !ui.settings.hidden) showSettings(false);
  ui.ide.hidden = open;
  ui.project.hidden = !open;
  ui.openProject.setAttribute('aria-expanded', String(open));
  $('ouvrir-projet-texte').textContent = open ? 'Revenir à l\'éditeur' : 'Projet';
  if (open) {
    projectRemarks.load();
    showProjectTab(projectTab);
    ui.path.textContent = 'Projet';
    document.title = 'Projet — Hodegos';
    ui.project.querySelector('[role="tab"][aria-selected="true"]')?.focus();
  } else {
    render();
    if (currentPath && !isReading()) editor.focus();
  }
}

/** L'écran des réglages remplace l'IDE ; il garde ses changements en attente quand on le quitte. */
function showSettings(open) {
  if (open && !ui.project.hidden) showProject(false);
  ui.ide.hidden = open;
  ui.settings.hidden = !open;
  ui.openSettings.setAttribute('aria-expanded', String(open));
  ui.openSettings.textContent = open ? 'Revenir à l\'éditeur' : 'Réglages';
  if (open) {
    settings.load();
    ui.path.textContent = 'Réglages';
    document.title = 'Réglages — Hodegos';
    ui.settings.querySelector('.app-settings__nav a')?.focus();
  } else {
    render();
    if (currentPath && !isReading()) editor.focus();
  }
}

/** Barre d'état : évaluation en cours, ou réglage sous le seuil. */
function renderEvaluation(next, progress = null) {
  if (next) lastSettings = next;
  ui.evaluation.replaceChildren();
  ui.evaluation.className = 'app-eval';
  const running = lastSettings?.eval_running;
  if (running) {
    const done = progress && progress.level === running ? ` : ${progress.done}/${progress.total} cas` : '…';
    ui.evaluation.append(`Évaluation du ${LEVEL_NAMES[running]}${done}`);
    ui.evaluation.hidden = false;
    return;
  }
  const below = Object.keys(LEVEL_NAMES).filter((level) => lastSettings?.last_eval[level]?.passed === false);
  ui.evaluation.hidden = below.length === 0;
  if (!below.length) return;
  const badge = document.createElement('span');
  badge.className = 'hd-badge hd-badge--warning';
  badge.textContent = below.length === 2 ? 'Deux niveaux sous le seuil' : `${capitalize(LEVEL_NAMES[below[0]])} sous le seuil`;
  const open = document.createElement('button');
  open.type = 'button';
  open.textContent = 'Voir les réglages';
  open.addEventListener('click', () => showSettings(true));
  ui.evaluation.append(badge, open);
}

function capitalize(text) {
  return text.charAt(0).toUpperCase() + text.slice(1);
}

/* ---------- Git et suppression ---------- */

function plural(count, one, many, none) {
  if (count === 0 && none) return none;
  return count === 1 ? `1 ${one}` : `${count} ${many}`;
}

function renderGit() {
  ui.git.replaceChildren();
  ui.gitLegend.hidden = !gitStatus;
  if (gitStatus === undefined) return;
  if (gitStatus === null) {
    ui.git.className = '';
    ui.git.textContent = 'Pas de dépôt git';
    return;
  }
  const branch = document.createElement('span');
  branch.className = 'hd-mono';
  branch.textContent = gitStatus.detached ? 'HEAD détachée' : gitStatus.branch;
  const parts = [plural(gitStatus.changes, 'changement', 'changements', 'aucun changement')];
  if (gitStatus.conflicts) parts.push(plural(gitStatus.conflicts, 'conflit', 'conflits'));
  ui.git.className = 'app-git';
  ui.git.append(icon('branch'), branch, ` · ${parts.join(' · ')}`);
}

/** « Supprimer… » : aperçu, confirmation qui dit ce qui est récupérable, puis suppression. */
async function deleteEntry(path, button) {
  let preview;
  try {
    preview = (await server.request('delete_preview', { path })).payload;
  } catch (error) {
    showError(error);
    return;
  }
  const affected = [...tabs.keys()].filter((p) => p === path || p.startsWith(`${path}/`));
  const confirmed = await confirmDeletion(preview, {
    repo: Boolean(gitStatus),
    tabs: affected.length,
    dirty: affected.filter((p) => isDirty(tabs.get(p))).length,
    returnFocus: button,
  });
  if (!confirmed) return;
  try {
    await server.request('delete_entry', { path });
  } catch (error) {
    showError(error);
    return;
  }
  for (const p of affected) dropTab(p);
  tree.refresh();
  showMessage(`Supprimé : ${path}`);
}

/** « Déplacer vers… » : choix du dossier au clavier, puis le même déplacement qu'au glisser. */
async function moveTo(path, button) {
  let reply;
  try {
    reply = await server.request('list_dirs', null);
  } catch (error) {
    showError(error);
    return;
  }
  const { paths, truncated } = reply.payload;
  const kind = button.dataset.kind;
  const dir = await chooseDestination({ path, kind, dirs: paths, truncated, returnFocus: button });
  if (dir !== null) tree.moveEntry(path, dir);
}

/** Les onglets des fichiers déplacés suivent leur nouveau chemin, modifications comprises. */
function followMove({ from, to }) {
  const moved = (path) => (path === from || path.startsWith(`${from}/`) ? to + path.slice(from.length) : null);
  if (currentPath && tabs.has(currentPath)) tabs.get(currentPath).state = editor.state;
  const entries = [...tabs.entries()].map(([path, tab]) => {
    const next = moved(path);
    if (!next) return [path, tab];
    tab.path = next;
    tab.state = editor.retarget(tab.state, next);
    if (remarksByPath.has(path)) {
      const remarks = remarksByPath.get(path).map((remark) => ({ ...remark, path: next }));
      remarksByPath.delete(path);
      remarksByPath.set(next, remarks);
    }
    // Le serveur pousse le casting du nouveau chemin.
    castings.delete(path);
    return [next, tab];
  });
  tabs.clear();
  for (const [path, tab] of entries) tabs.set(path, tab);
  const current = moved(currentPath ?? '');
  if (current) {
    currentPath = current;
    if (sentCursor) sentCursor = { ...sentCursor, path: current };
  }
  render();
  showMessage(`Déplacé : ${from} → ${to}`);
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

/** Rend `mentor` titulaire et lui demande une relecture de la section du curseur. */
async function activateMentor(mentor, button) {
  const casting = castings.get(currentPath);
  const tab = tabs.get(currentPath);
  if (!casting || !tab) return;
  if (isDirty(tab)) {
    const name = document.createElement('span');
    name.className = 'hd-mono';
    name.textContent = tab.path.split('/').pop();
    const text = document.createElement('p');
    text.textContent = `${mentor.name} relit la version enregistrée. Que voulez-vous qu'il lise ?`;
    const choice = await chooseDialog({
      title: [name, ' a des modifications non enregistrées'],
      body: [text],
      choices: [
        { value: 'cancel', label: 'Annuler', variant: 'ghost' },
        { value: 'saved', label: 'Version enregistrée', variant: 'secondary' },
        { value: 'save', label: 'Enregistrer et demander', variant: 'primary' },
      ],
      focus: 'save',
      returnFocus: button,
    });
    if (choice !== 'saved' && choice !== 'save') return;
    // Une seule passe : l'enregistrement n'en lance pas, l'activation s'en charge.
    if (choice === 'save' && !(await save(undefined, { skipPass: true }))) return;
  }
  await requestActivation(tab.path, casting.section_id, mentor, false, button);
}

async function requestActivation(path, sectionId, mentor, confirm, button) {
  try {
    const reply = await server.request('cast_override', {
      path,
      section_id: sectionId,
      mentor_id: mentor.id,
      activate: true,
      confirm_over_budget: confirm,
    });
    applyCasting(reply.payload);
  } catch (error) {
    if (error.code !== 'budget_exceeded' || confirm) {
      showError(error);
      return;
    }
    const text = document.createElement('p');
    text.textContent = `Le budget du mois est atteint. Demander quand même une relecture à ${mentor.name} ? Son coût s'ajoutera au mois en cours.`;
    const choice = await chooseDialog({
      title: 'Budget du mois atteint',
      body: [text],
      choices: [
        { value: 'cancel', label: 'Annuler', variant: 'secondary' },
        { value: 'confirm', label: 'Demander quand même', variant: 'primary' },
      ],
      focus: 'cancel',
      returnFocus: button,
    });
    if (choice === 'confirm') await requestActivation(path, sectionId, mentor, true, button);
    else showMessage('Budget du mois atteint : aucune relecture demandée.');
  }
}

/** Revient au choix automatique : rien n'est relu. */
async function automaticMentor() {
  const casting = castings.get(currentPath);
  if (!casting) return;
  try {
    const reply = await server.request('cast_override', {
      path: currentPath,
      section_id: casting.section_id,
      mentor_id: null,
    });
    applyCasting(reply.payload);
  } catch (error) {
    showError(error);
  }
}

function renderMentors() {
  const casting = castings.get(currentPath) ?? null;
  const reviewing = passing && casting && passing.path === casting.path
    && passing.sections.includes(casting.section_id) ? passing.mentors : new Set();
  panel.render(casting, reviewing);
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
    onOpenAt: (path, line) => openFile(path, line),
    onShowMap: () => {
      showProject(true);
      showProjectTab('carte');
    },
  });
  if (!currentPath) return;
  const projectItems = projectRemarkItems(currentPath);
  if (projectItems.length) ui.remarks.append(projectSection(projectItems));
  const anchors = items
    .filter((item) => item.position)
    .map(({ remark, number, position }) => ({
      id: remark.id,
      number,
      from: position.from,
      to: position.to,
      active: remark.id === activeRemark,
    }))
    .concat(projectItems.filter((item) => item.position).map(({ remark, number, position }) => ({
      id: remark.id,
      number: `P${number}`,
      from: position.from,
      to: position.to,
      active: remark.id === activeRemark,
      project: true,
    })));
  editor.showAnchors(anchors);
  if (isReading()) reader.render(editor.state, currentPath, anchors);
}

/* ---------- Remarques de projet dans le fichier ouvert (phase 9d) ---------- */

/** Passages du fichier `path` cités par une remarque de projet. */
function projectRemarkItems(path) {
  const tab = tabs.get(path);
  if (!tab) return [];
  const doc = path === currentPath ? editor.state.doc : tab.state.doc;
  return projectRemarks.citing(path).flatMap((remark) => remark.anchors
    .filter((a) => a.path === path && a.status === 'ok')
    .map((anchor) => ({ remark, anchor, position: locate(doc, anchor.quote, anchor.line_hint) })))
    .sort((a, b) => a.anchor.line_hint - b.anchor.line_hint)
    .map((item, index) => ({ ...item, number: index + 1 }));
}

/** Section repliée « Remarques de projet sur ce fichier » : cartes courtes vers la vue Projet. */
function projectSection(items) {
  const details = document.createElement('details');
  details.className = 'app-project-file';
  const summary = document.createElement('summary');
  summary.textContent = `Remarques de projet sur ce fichier (${items.length})`;
  const list = document.createElement('div');
  list.className = 'app-project-file__list';
  for (const { remark, number } of items) {
    const card = document.createElement('article');
    card.className = remark.id === activeRemark ? 'hd-card hd-card--active app-project-file__card' : 'hd-card app-project-file__card';
    const head = document.createElement('div');
    head.className = 'hd-card__head';
    const marker = document.createElement('span');
    marker.className = 'hd-marker app-marker-project app-marker-inline';
    marker.textContent = `P${number}`;
    const author = document.createElement('span');
    author.className = 'hd-msg__author--mentor';
    author.textContent = projectRemarks.nameOf(remark.mentor_id);
    head.append(marker, author);
    if (remark.status === 'to_review') {
      const badge = document.createElement('span');
      badge.className = 'hd-badge hd-badge--warning hd-card__end';
      badge.textContent = 'À réévaluer';
      head.append(badge);
    }
    const text = document.createElement('p');
    text.className = 'app-project-file__text';
    text.textContent = remark.thread[0]?.text ?? '';
    const open = document.createElement('button');
    open.type = 'button';
    open.className = 'hd-btn hd-btn--ghost hd-btn--sm';
    open.textContent = 'Voir dans la vue Projet';
    open.addEventListener('click', () => openProjectRemark(remark.id));
    card.append(head, text, open);
    list.append(card);
  }
  details.append(summary, list);
  return details;
}

/** Ouvre la vue Projet sur une remarque de projet ; le fil s'y traite. */
function openProjectRemark(id) {
  showProject(true);
  showProjectTab('remarques');
  projectRemarks.focus(id);
}

/** `project_status` : compteur du bouton Projet, barre d'état, panneau des mentors. */
/** Dernier `project_status` reçu, redessiné quand les noms des mentors arrivent. */
let lastProjectStatus = null;

function renderProjectStatus(status) {
  const first = lastProjectStatus !== status;
  lastProjectStatus = status;
  if (first) projectRemarks.status(status);
  const count = status.open + status.to_review;
  const counter = $('projet-compteur');
  counter.hidden = count === 0;
  counter.textContent = String(count);
  ui.openProject.setAttribute('aria-label', count ? `Projet, ${count} remarques de projet` : 'Projet');
  const name = (id) => projectRemarks.nameOf(id);
  const line = $('revues');
  if (status.reviewing.length) {
    line.textContent = `Relit le projet : ${status.reviewing.map(name).join(', ')}…`;
  } else if (status.pending.length) {
    const calm = status.pending.every((p) => p.trigger === 'change');
    line.textContent = `Revue du projet prévue${calm ? ' au calme' : ''} : ${status.pending.map((p) => name(p.mentor_id)).join(', ')}`;
  } else {
    line.textContent = status.to_review ? `${status.to_review} remarque${status.to_review > 1 ? 's' : ''} de projet à réévaluer` : '';
  }
}

/** Met en avant une remarque ; `reveal` amène aussi son passage à l'écran. */
function selectRemark(id, reveal) {
  activeRemark = id;
  renderRemarksPanel();
  if (!reveal) return;
  const item = remarkItems(currentPath).find((i) => i.remark.id === id);
  if (!item?.position) return;
  if (isReading()) reader.reveal(id);
  else editor.reveal(item.position.from);
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
  lastPassAt = last?.at ?? null;
  if (passing && lastPassAt !== passing.since) {
    const { path } = passing;
    passing = null;
    if (path === currentPath) renderMentors();
  }
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

/** Ouvre `path` dans un onglet ; `line` (à partir de 1) : y amène le curseur (« Voir aussi »). */
async function openFile(path, line = null) {
  if (tabs.has(path)) {
    activate(path);
    if (line) revealLine(line);
    return;
  }
  try {
    const { payload } = await server.request('open_file', { path });
    remarksByPath.set(path, payload.remarks);
    projectRemarks.remarksArrived(payload.project_remarks ?? []);
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
    if (line) revealLine(line);
  } catch (error) {
    showError(error);
  }
}

/** Amène la ligne `line` du document affiché à l'écran, en édition comme en lecture. */
function revealLine(line) {
  const { doc } = editor.state;
  const pos = doc.line(Math.min(Math.max(line, 1), doc.lines)).from;
  if (isReading()) reader.scrollTo(pos);
  else editor.reveal(pos);
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

/**
 * Enregistre l'onglet courant ; `baseVersion` force l'écrasement après un conflit,
 * `skipPass` enregistre sans passe. Renvoie `true` si le fichier est enregistré.
 */
async function save(baseVersion, { skipPass = false } = {}) {
  const tab = tabs.get(currentPath);
  if (!tab) return false;
  if (tab.banner?.kind === 'deleted') {
    showMessage('Ce fichier a été supprimé ; il ne peut plus être enregistré.');
    return false;
  }
  if (tab.saving) return false;
  tab.saving = true;
  const doc = editor.state.doc;
  let saved = false;
  try {
    const { payload } = await server.request('save', {
      path: tab.path,
      content: doc.toString(),
      base_version: baseVersion ?? tab.version,
      skip_pass: skipPass,
    });
    tab.version = payload.version;
    tab.savedDoc = doc;
    tab.banner = null;
    saved = true;
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
  return saved;
}

function isDirty(tab) {
  const state = tab.path === currentPath ? editor.state : tab.state;
  return !state.doc.eq(tab.savedDoc);
}

/* ---------- Markdown : source, aperçu, lecture ---------- */

function isMarkdown() {
  return tabs.get(currentPath)?.language === 'markdown';
}

function isReading() {
  return isMarkdown() && markdownMode === 'reading';
}

function languageLabel(tab) {
  const name = LANGUAGE_NAMES[tab.language] ?? tab.language;
  return tab.language === 'markdown' && markdownMode !== 'source' ? `${name} · ${MODE_LABELS[markdownMode]}` : name;
}

/** Aperçu de l'éditeur et sélecteur de mode, selon l'onglet courant. */
function syncMode() {
  const markdown = isMarkdown();
  editor.setPreview(markdown && markdownMode === 'preview');
  ui.modes.hidden = !markdown;
  for (const choice of ui.modes.querySelectorAll('[data-value]')) {
    choice.setAttribute('aria-pressed', String(choice.dataset.value === markdownMode));
  }
}

function setMarkdownMode(mode) {
  if (!MODES.includes(mode) || mode === markdownMode) return;
  const wasReading = isReading();
  markdownMode = mode;
  store('hodegos.markdown.mode', mode);
  if (EDIT_MODES.includes(mode)) {
    editMode = mode;
    store('hodegos.markdown.edit', mode);
  }
  render();
  if (isReading()) {
    reader.scrollTo(editor.state.selection.main.head);
    ui.reading.focus();
  } else if (wasReading) {
    editor.focus();
  }
}

function stored(key, allowed, fallback) {
  try {
    const value = window.localStorage.getItem(key);
    return allowed.includes(value) ? value : fallback;
  } catch {
    return fallback;
  }
}

function store(key, value) {
  try {
    window.localStorage.setItem(key, value);
  } catch {
    /* Choix non mémorisé : il reste valable pour cette page. */
  }
}

/* ---------- Onglets ---------- */

function activate(path) {
  const previous = tabs.get(currentPath);
  if (previous) previous.state = editor.state;
  currentPath = path;
  const tab = tabs.get(path);
  if (tab) {
    editor.show(tab.state);
    syncMode();
    if (isReading()) ui.reading.focus();
    else editor.focus();
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
  dropTab(path);
}

/** Ferme un onglet sans rien demander (fichier supprimé, onglet déjà confirmé). */
function dropTab(path) {
  const tab = tabs.get(path);
  if (!tab) return;
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
  syncMode();
  ui.tabBar.hidden = tabs.size === 0;
  ui.editor.hidden = !tab || isReading();
  ui.reading.hidden = !tab || !isReading();
  ui.empty.hidden = Boolean(tab);
  ui.language.textContent = tab ? languageLabel(tab) : '';
  if (!tab) ui.position.textContent = '';
  // Les réglages et la vue Projet gardent leur titre.
  if (inIde()) {
    ui.path.textContent = tab ? tab.path : '';
    document.title = tab ? `${tab.path.split('/').pop()} — Hodegos` : 'Hodegos';
  }
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
    button.setAttribute('aria-controls', isReading() ? 'lecture' : 'editeur');
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
  store('hodegos.theme', next);
  renderThemeButton();
});

document.addEventListener('keydown', (event) => {
  if ((event.ctrlKey || event.metaKey) && !event.altKey && event.key.toLowerCase() === 's') {
    event.preventDefault();
    if (inIde()) save();
  }
  // Ctrl+E : dernier mode d'édition ⇄ lecture, pour un onglet Markdown.
  if ((event.ctrlKey || event.metaKey) && !event.altKey && !event.shiftKey && event.key.toLowerCase() === 'e' && isMarkdown() && inIde()) {
    event.preventDefault();
    setMarkdownMode(markdownMode === 'reading' ? editMode : 'reading');
  }
});

// Ctrl+` : terminal ouvert ou replié, même quand le focus est dans le terminal.
document.addEventListener('keydown', (event) => {
  if (!event.ctrlKey || event.altKey || event.metaKey) return;
  if (event.key !== '`' && event.code !== 'Backquote') return;
  event.preventDefault();
  event.stopPropagation();
  backToIde();
  terminalPanel.toggle();
}, true);

window.addEventListener('beforeunload', (event) => {
  if ([...tabs.values()].some(isDirty)) event.preventDefault();
});

renderThemeButton();
render();
