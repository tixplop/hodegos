/* Hodegos — application : arborescence, onglets, éditeur, enregistrement, modifications externes. */
import { connect } from './connection.js';
import { createTree } from './tree.js';
import { createEditor, LANGUAGE_NAMES } from './editor.js';
import { icon } from './icons.js';

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
};

/**
 * Onglets ouverts, par chemin. Pour chacun : `version` connue du disque, `state` CodeMirror,
 * `savedDoc` (document au dernier enregistrement), `banner` éventuel :
 * `{ kind: 'changed' | 'conflict' | 'deleted', version }`.
 */
const tabs = new Map();
let currentPath = null;
let wasConnected = false;

const server = connect({ onPush, onState });
const editor = createEditor(ui.editor, {
  onChange: () => renderTabs(),
  onCursor: ({ line, col }) => {
    ui.position.textContent = `l.${line}, col. ${col}`;
  },
});
const tree = createTree(ui.tree, {
  list: (path) => server.request('list_dir', { path }).then((reply) => reply.payload.entries),
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
  if (message.type !== 'file_changed') return;
  const { path, version } = message.payload;
  const tab = tabs.get(path);
  if (!tab || version === tab.version) return;
  tab.banner = version === null ? { kind: 'deleted' } : { kind: 'changed', version };
  render();
}

/** Après une reconnexion, le serveur ne suit plus les fichiers : on les rouvre. */
async function resync() {
  for (const tab of tabs.values()) {
    try {
      const reply = await server.request('open_file', { path: tab.path });
      if (reply.payload.version !== tab.version) {
        tab.banner = { kind: 'changed', version: reply.payload.version };
      }
    } catch (error) {
      if (error.code === 'not_found') tab.banner = { kind: 'deleted' };
    }
  }
  render();
}

/* ---------- Fichiers ---------- */

async function openFile(path) {
  if (tabs.has(path)) {
    activate(path);
    return;
  }
  try {
    const { payload } = await server.request('open_file', { path });
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
    tab.state = editor.createState(payload.content, payload.language, tab.path);
    tab.savedDoc = tab.state.doc;
    tab.version = payload.version;
    tab.banner = null;
    if (tab.path === currentPath) editor.show(tab.state);
    render();
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
  }
  render();
}

function closeTab(path) {
  const tab = tabs.get(path);
  if (!tab) return;
  if (isDirty(tab) && !window.confirm(`Fermer ${tab.path} sans enregistrer vos modifications ?`)) return;
  if (path === currentPath) tab.state = editor.state;
  tabs.delete(path);
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
