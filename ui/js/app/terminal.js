/* Terminal intégré et confiné (phase 8f, maquette 08-6, ADR 0005) : panneau sous
   l'éditeur, onglets, xterm.js chargé à la première ouverture. Ce qui passe dans le
   terminal n'est jamais envoyé aux mentors.

   CSP (prototype du 31 octobre 2026) : rendu canvas, sans styles par cellule ; les
   deux <style> que xterm.js crée deviennent des éléments inertes `hodegos-style`, dont
   le texte alimente une feuille adoptée par le shadow root du terminal. Le front ne
   crée jamais de <style> lui-même : la cale ne change rien ailleurs. */

const HEIGHT_KEY = 'hodegos.terminal.height';
const MIN_HEIGHT = 120;
const STEP = 24;

let lib = null;
let baseSheet = null;

/** `createElement('style')` → élément inerte, synchronisé avec une feuille adoptée. */
function installStyleShim() {
  const create = Document.prototype.createElement;
  Document.prototype.createElement = function createElement(tag, options) {
    return create.call(this, String(tag).toLowerCase() === 'style' ? 'hodegos-style' : tag, options);
  };
}

/** Feuilles adoptées d'un shadow root : celle de xterm.js, puis ses `hodegos-style`. */
function adoptStyles(root) {
  const sheets = new Map();
  const sync = () => {
    for (const node of root.querySelectorAll('hodegos-style')) {
      if (!sheets.has(node)) sheets.set(node, new CSSStyleSheet());
      sheets.get(node).replaceSync(node.textContent);
    }
    for (const node of sheets.keys()) if (!node.isConnected) sheets.delete(node);
    root.adoptedStyleSheets = [baseSheet, ...sheets.values()];
  };
  new MutationObserver(sync).observe(root, { subtree: true, childList: true, characterData: true });
  sync();
}

async function loadLibrary() {
  if (lib) return lib;
  installStyleShim();
  lib = await import('/vendor/xterm.js');
  baseSheet = new CSSStyleSheet();
  baseSheet.replaceSync(`${lib.xtermCss}\n.app-term__screen { height: 100%; padding: 4px 0 0 8px; box-sizing: border-box; }`);
  return lib;
}

/** Couleurs du design system, relues à chaque changement de thème. */
function theme() {
  const css = getComputedStyle(document.documentElement);
  const token = (name) => css.getPropertyValue(`--${name}`).trim();
  return {
    background: token('surface-0'),
    foreground: token('ink'),
    cursor: token('ink'),
    cursorAccent: token('surface-0'),
    selectionBackground: token('accent-soft'),
    black: token('ink'),
    red: token('danger'),
    green: token('success'),
    yellow: token('warning'),
    blue: token('accent'),
    magenta: token('mentor'),
    cyan: token('syntax-string'),
    white: token('ink-muted'),
    brightBlack: token('ink-muted'),
    brightRed: token('danger'),
    brightGreen: token('success'),
    brightYellow: token('warning'),
    brightBlue: token('accent'),
    brightMagenta: token('mentor'),
    brightCyan: token('syntax-string'),
    brightWhite: token('ink'),
  };
}

function toBase64(text) {
  const bytes = new TextEncoder().encode(text);
  let binary = '';
  for (let i = 0; i < bytes.length; i += 0x8000) binary += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
  return btoa(binary);
}

function fromBase64(data) {
  const binary = atob(data);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i += 1) bytes[i] = binary.charCodeAt(i);
  return bytes;
}

function stored(key) {
  try {
    return Number(window.localStorage.getItem(key)) || null;
  } catch {
    return null;
  }
}

function store(key, value) {
  try {
    window.localStorage.setItem(key, String(value));
  } catch {
    /* Hauteur non mémorisée : elle vaut pour cette page. */
  }
}

/**
 * Panneau du terminal. `request` et `send` passent par la WebSocket ; `onCount(n)`
 * suit le nombre de terminaux ouverts ; `onToggle(open)` quand le panneau s'ouvre ou
 * se replie.
 */
export function createTerminalPanel({ section, handle, request, send, onCount, onToggle }) {
  const tabs = section.querySelector('#terminal-onglets');
  const views = section.querySelector('#terminal-vues');
  const message = section.querySelector('#terminal-message');
  const terminals = new Map();
  let active = null;
  let opening = false;

  const container = section.parentElement;
  const resize = new ResizeObserver(() => fitActive());
  resize.observe(views);
  new MutationObserver(() => {
    for (const t of terminals.values()) t.term.options.theme = theme();
  }).observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme'] });

  function setHeight(height) {
    const max = Math.max(MIN_HEIGHT, container.clientHeight * 0.75);
    const value = Math.round(Math.min(Math.max(height, MIN_HEIGHT), max));
    section.style.height = `${value}px`;
    handle.setAttribute('aria-valuenow', String(value));
    handle.setAttribute('aria-valuemin', String(MIN_HEIGHT));
    handle.setAttribute('aria-valuemax', String(Math.round(max)));
    store(HEIGHT_KEY, value);
  }

  function fitActive() {
    const t = terminals.get(active);
    if (t && !section.hidden) t.fit.fit();
  }

  function showMessage(text) {
    message.hidden = !text;
    message.textContent = text ?? '';
  }

  function renderTabs() {
    const items = [...terminals.values()].map((t) => {
      const tab = document.createElement('button');
      tab.type = 'button';
      tab.className = 'hd-tab app-term__tab';
      tab.setAttribute('role', 'tab');
      tab.setAttribute('aria-selected', String(t.id === active));
      tab.setAttribute('aria-controls', `terminal-vue-${t.id}`);
      tab.tabIndex = t.id === active ? 0 : -1;
      tab.textContent = t.exited === undefined ? t.label : `${t.label} (terminé)`;
      tab.addEventListener('click', () => select(t.id));
      return tab;
    });
    tabs.replaceChildren(...items);
    const running = [...terminals.values()].filter((t) => t.exited === undefined).length;
    onCount(running);
  }

  function select(id) {
    active = id;
    for (const t of terminals.values()) t.view.hidden = t.id !== id;
    renderTabs();
    requestAnimationFrame(() => {
      fitActive();
      terminals.get(id)?.term.focus();
    });
  }

  async function create() {
    if (opening) return;
    opening = true;
    showMessage('');
    try {
      const { Terminal, FitAddon, CanvasAddon } = await loadLibrary();
      const view = document.createElement('div');
      view.className = 'app-term__view';
      views.append(view);
      const root = view.attachShadow({ mode: 'open' });
      adoptStyles(root);
      const screen = document.createElement('div');
      screen.className = 'app-term__screen';
      root.append(screen);
      const term = new Terminal({
        fontFamily: '"JetBrains Mono", ui-monospace, monospace',
        fontSize: 13,
        lineHeight: 1.2,
        cursorBlink: true,
        scrollback: 5000,
        theme: theme(),
      });
      const fit = new FitAddon();
      term.loadAddon(fit);
      term.open(screen);
      term.loadAddon(new CanvasAddon());
      fit.fit();
      let reply;
      try {
        reply = await request('term_open', { cols: term.cols, rows: term.rows });
      } catch (error) {
        term.dispose();
        view.remove();
        showMessage(error.message);
        return;
      }
      const { term_id: id, shell, warning } = reply.payload;
      const number = terminals.size === 0 ? 1 : Math.max(...[...terminals.values()].map((t) => t.number)) + 1;
      view.id = `terminal-vue-${id}`;
      view.setAttribute('role', 'tabpanel');
      const entry = { id, number, label: `${shell} · ${number}`, term, fit, view, exited: undefined };
      terminals.set(id, entry);
      term.onData((data) => {
        if (entry.exited === undefined) send('term_input', { term_id: id, data: toBase64(data) });
      });
      term.onResize(({ cols, rows }) => {
        if (entry.exited === undefined) send('term_resize', { term_id: id, cols, rows });
      });
      if (warning) showMessage(warning);
      select(id);
    } finally {
      opening = false;
    }
  }

  function close(id) {
    const t = terminals.get(id);
    if (!t) return;
    if (t.exited === undefined) send('term_close', { term_id: id });
    t.term.dispose();
    t.view.remove();
    terminals.delete(id);
    const next = [...terminals.keys()].pop();
    if (next === undefined) {
      active = null;
      renderTabs();
      hide();
      return;
    }
    select(next);
  }

  function show() {
    section.hidden = false;
    handle.hidden = false;
    setHeight(stored(HEIGHT_KEY) ?? Math.round(container.clientHeight * 0.4));
    onToggle(true);
    if (terminals.size === 0) create();
    else select(active ?? [...terminals.keys()][0]);
  }

  /** Replie le panneau sans tuer les terminaux. */
  function hide() {
    section.hidden = true;
    handle.hidden = true;
    onToggle(false);
  }

  section.querySelector('#terminal-nouveau').addEventListener('click', () => create());
  section.querySelector('#terminal-reduire').addEventListener('click', hide);
  section.querySelector('#terminal-fermer').addEventListener('click', () => {
    if (active !== null) close(active);
  });
  tabs.addEventListener('keydown', (event) => {
    if (event.key !== 'ArrowRight' && event.key !== 'ArrowLeft') return;
    const ids = [...terminals.keys()];
    const index = ids.indexOf(active);
    const next = ids[(index + (event.key === 'ArrowRight' ? 1 : ids.length - 1)) % ids.length];
    if (next !== undefined) {
      select(next);
      tabs.querySelector('[aria-selected="true"]')?.focus();
    }
  });

  // Poignée : glisser, ou flèches haut et bas.
  handle.addEventListener('pointerdown', (event) => {
    event.preventDefault();
    handle.setPointerCapture(event.pointerId);
    const startY = event.clientY;
    const startHeight = section.getBoundingClientRect().height;
    const move = (e) => setHeight(startHeight + (startY - e.clientY));
    const up = () => {
      handle.removeEventListener('pointermove', move);
      handle.removeEventListener('pointerup', up);
    };
    handle.addEventListener('pointermove', move);
    handle.addEventListener('pointerup', up);
  });
  handle.addEventListener('keydown', (event) => {
    const height = section.getBoundingClientRect().height;
    if (event.key === 'ArrowUp') setHeight(height + STEP);
    else if (event.key === 'ArrowDown') setHeight(height - STEP);
    else return;
    event.preventDefault();
  });

  return {
    /** Ctrl+` : ouvre (et crée un premier terminal) ou replie. */
    toggle() {
      if (section.hidden) show();
      else hide();
    },
    show,
    output({ term_id: id, data }) {
      terminals.get(id)?.term.write(fromBase64(data));
    },
    exited({ term_id: id, code }) {
      const t = terminals.get(id);
      if (!t || t.exited !== undefined) return;
      t.exited = code;
      t.term.write(`\r\n\x1b[2m[Processus terminé${code === null ? '' : `, code ${code}`}]\x1b[0m\r\n`);
      renderTabs();
    },
    /** WebSocket perdue : le serveur a tué les terminaux de cette fenêtre. */
    disconnected() {
      for (const t of terminals.values()) {
        if (t.exited === undefined) {
          t.exited = null;
          t.term.write('\r\n\x1b[2m[Connexion perdue : terminal fermé]\x1b[0m\r\n');
        }
      }
      renderTabs();
    },
  };
}
