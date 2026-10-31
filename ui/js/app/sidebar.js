/* Largeur de l'arbre des fichiers : poignée entre l'arbre et l'éditeur, comme celle du terminal. */
const WIDTH_KEY = 'hodegos.treeWidth';
const DEFAULT_WIDTH = 260;
const MIN_WIDTH = 160;
const MIN_EDITOR = 320;
const STEP = 16;
// Point de rupture d'app.css où les colonnes passent les unes sous les autres.
const NARROW = window.matchMedia('(max-width: 720px)');

function stored() {
  try {
    return Number(window.localStorage.getItem(WIDTH_KEY)) || null;
  } catch {
    return null;
  }
}

function store(value) {
  try {
    window.localStorage.setItem(WIDTH_KEY, String(value));
  } catch {
    /* Largeur non mémorisée : elle vaut pour cette page. */
  }
}

/**
 * Rend `sidebar` redimensionnable par `handle` (role="separator"). La largeur
 * maximale laisse 320 px à l'éditeur à côté de `panel`, et 50 % de la fenêtre au plus.
 */
export function createSidebarResizer({ sidebar, handle, panel }) {
  const columns = sidebar.parentElement;
  let preferred = stored() ?? DEFAULT_WIDTH;

  function bounds() {
    const total = columns.clientWidth;
    const room = total - panel.getBoundingClientRect().width - handle.offsetWidth - MIN_EDITOR;
    return { min: MIN_WIDTH, max: Math.max(MIN_WIDTH, Math.min(total * 0.5, room)) };
  }

  // La largeur voulue reste mémorisée ; une fenêtre trop étroite ne fait que la borner.
  function apply() {
    if (NARROW.matches || columns.clientWidth === 0) {
      sidebar.style.width = '';
      return;
    }
    const { min, max } = bounds();
    const value = Math.round(Math.min(Math.max(preferred, min), max));
    sidebar.style.width = `${value}px`;
    handle.setAttribute('aria-valuenow', String(value));
    handle.setAttribute('aria-valuemin', String(min));
    handle.setAttribute('aria-valuemax', String(Math.round(max)));
  }

  function setWidth(width) {
    const { min, max } = bounds();
    preferred = Math.round(Math.min(Math.max(width, min), max));
    store(preferred);
    apply();
  }

  // Fenêtre redimensionnée, ou IDE de nouveau affiché après les réglages.
  new ResizeObserver(apply).observe(columns);
  NARROW.addEventListener('change', apply);

  handle.addEventListener('pointerdown', (event) => {
    if (event.button !== 0) return;
    event.preventDefault();
    handle.setPointerCapture(event.pointerId);
    const startX = event.clientX;
    const startWidth = sidebar.getBoundingClientRect().width;
    const move = (e) => setWidth(startWidth + (e.clientX - startX));
    const up = () => {
      handle.removeEventListener('pointermove', move);
      handle.removeEventListener('pointerup', up);
    };
    handle.addEventListener('pointermove', move);
    handle.addEventListener('pointerup', up);
  });
  handle.addEventListener('dblclick', () => setWidth(DEFAULT_WIDTH));
  handle.addEventListener('keydown', (event) => {
    const width = sidebar.getBoundingClientRect().width;
    if (event.key === 'ArrowRight') setWidth(width + STEP);
    else if (event.key === 'ArrowLeft') setWidth(width - STEP);
    else if (event.key === 'Enter') setWidth(DEFAULT_WIDTH);
    else return;
    event.preventDefault();
  });

  apply();
}
