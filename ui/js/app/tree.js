/* Arborescence du projet, chargée dossier par dossier (list_dir) : création dans l'arbre,
   suppression par le menu contextuel, déplacement par glisser-déposer ou « Déplacer vers… »,
   statut git (lettre et couleur, point sur les dossiers). */
import { icon } from './icons.js';
import { openMenu } from './menu.js';

/** Lettre git : mot pour les lecteurs d'écran et l'infobulle, couleur du design system. */
export const GIT_LETTERS = {
  M: { word: 'Modifié', tone: 'warning' },
  R: { word: 'Renommé', tone: 'warning' },
  A: { word: "Ajouté à l'index", tone: 'success' },
  U: { word: 'Non suivi', tone: 'success' },
  C: { word: 'En conflit', tone: 'danger' },
};

const CREATE_ERRORS = {
  already_exists: 'Ce nom est déjà pris dans ce dossier.',
  path_forbidden: 'Ce chemin est sous .git, exclu par .gitignore ou hors du projet.',
  not_found: "Ce dossier n'existe plus.",
  bad_message: 'Nom invalide.',
};

const MOVE_ERRORS = {
  already_exists: 'Ce nom est déjà pris dans le dossier de destination.',
  path_forbidden: 'Destination sous .git, exclue par .gitignore, hors du projet ou atteinte par un lien symbolique.',
  not_found: "Ce fichier ou ce dossier n'existe plus.",
  invalid_move: 'Un dossier ne peut pas aller dans lui-même ni dans un de ses sous-dossiers.',
  internal: "Déplacement annulé : les remarques n'ont pas pu suivre. Voir le journal du serveur.",
};

/** Délai de regroupement des rechargements (plusieurs `git_status` rapprochés). */
const REFRESH_DELAY_MS = 50;

/** Type des données glissées depuis l'arbre ; tout autre dépôt est ignoré. */
const DRAG_TYPE = 'application/x-hodegos-entry';
/** Survol d'un dossier replié avant qu'il s'ouvre pendant un glisser. */
const EXPAND_DELAY_MS = 600;

/** Dossier parent d'un chemin (`''` : racine). */
export function parentOf(path) {
  const slash = path.lastIndexOf('/');
  return slash < 0 ? '' : path.slice(0, slash);
}

/** `dir` peut recevoir l'entrée `path` : ni elle-même, ni un descendant, ni son dossier actuel. */
export function canMoveInto(path, dir) {
  return dir !== parentOf(path) && dir !== path && !dir.startsWith(`${path}/`);
}

let inputs = 0;

/**
 * `list(path)` renvoie une promesse des entrées du dossier ; `create(path, kind)` crée une
 * entrée ; `move(from, toDir)` la déplace et renvoie la réponse `entry_moved` ; `onOpen(path)`
 * au clic sur un fichier ; `onCreated(path, kind)` après une création ; `onDelete(path,
 * button)` quand « Supprimer… » est choisi ; `onMoveTo(path, button)` pour « Déplacer vers… » ;
 * `onMoved({ from, to, kind })` après un déplacement. `zone` reçoit les dépôts (hors d'une
 * ligne : la racine, signalée sur `head`) ; `problem` affiche les refus de déplacement.
 */
export function createTree(root, {
  list, create, move, onOpen, onCreated, onDelete, onMoveTo, onMoved, onError, zone, head, problem,
}) {
  let current = null;
  /** Dossiers dépliés : ils le restent quand l'arbre est rechargé. */
  const expanded = new Set();
  /** Statut git : lettre par fichier, dossiers qui contiennent un changement. */
  let git = null;
  let letters = new Map();
  let changedDirs = new Set();
  /** Saisie d'un nom en cours : `{ li }` ; un rechargement attend sa fin. */
  let editing = null;
  let refreshPending = false;
  let refreshTimer = null;
  /** Élément à refocaliser après un rechargement. */
  let focusPath = null;
  /** Dernière demande de contenu par liste : une réponse plus ancienne est ignorée. */
  const fills = new WeakMap();
  /** Entrée glissée depuis l'arbre : `{ path }` ; `null` hors d'un glisser. */
  let dragging = null;
  /** Élément en surbrillance de dépôt, et dossier replié qui s'ouvrira s'il reste survolé. */
  let dropMark = null;
  let expandTimer = null;
  let expandTarget = null;

  root.dataset.path = '';
  root.dataset.depth = '0';

  async function fill(ul, path, depth, quiet = false) {
    const token = (fills.get(ul) ?? 0) + 1;
    fills.set(ul, token);
    try {
      const entries = await list(path);
      if (fills.get(ul) !== token || !ul.isConnected) return;
      ul.replaceChildren(...entries.map((entry) => item(entry, path, depth)));
      markCurrent();
      if (focusPath) {
        const button = buttonFor(focusPath);
        if (button) {
          button.focus();
          focusPath = null;
        }
      }
    } catch (error) {
      if (!quiet) onError(error);
    }
  }

  function expand(li, path, depth, quiet = false) {
    const ul = document.createElement('ul');
    ul.dataset.path = path;
    ul.dataset.depth = String(depth);
    li.append(ul);
    return fill(ul, path, depth, quiet).then(() => ul);
  }

  function item(entry, parent, depth) {
    const path = parent ? `${parent}/${entry.name}` : entry.name;
    const li = document.createElement('li');
    const button = document.createElement('button');
    button.type = 'button';
    button.className = 'hd-nav__item app-tree__item';
    button.style.setProperty('--depth', String(depth));
    button.dataset.path = path;
    button.dataset.kind = entry.kind;
    button.draggable = true;
    button.title = path;
    const name = document.createElement('span');
    name.className = 'app-tree__name';
    name.textContent = entry.name;

    if (entry.kind === 'dir') {
      button.setAttribute('aria-expanded', String(expanded.has(path)));
      button.append(icon('chevron'), name);
      if (changedDirs.has(path)) {
        const dot = document.createElement('span');
        dot.className = 'hd-dot app-tree__dot';
        dot.title = 'Contient des changements';
        const label = document.createElement('span');
        label.className = 'app-sr-only';
        label.textContent = ', contient des changements';
        button.append(dot, label);
      }
      button.addEventListener('click', () => {
        const open = button.getAttribute('aria-expanded') === 'true';
        button.setAttribute('aria-expanded', String(!open));
        if (open) {
          expanded.delete(path);
          li.querySelector('ul')?.remove();
          return;
        }
        expanded.add(path);
        expand(li, path, depth + 1);
      });
    } else {
      button.classList.add('hd-mono', 'app-tree__file');
      button.append(name);
      const end = document.createElement('span');
      end.className = 'hd-row app-tree__end';
      if (entry.mentors?.length) {
        const meta = document.createElement('span');
        meta.className = 'hd-nav__meta';
        meta.textContent = entry.mentors.map((m) => m.initials).join(' · ');
        meta.title = entry.mentors.map((m) => m.name).join(', ');
        end.append(meta);
      }
      const letter = letters.get(path);
      if (git) {
        const mark = document.createElement('span');
        mark.className = 'app-tree__git';
        if (letter) {
          const { word, tone } = GIT_LETTERS[letter];
          button.classList.add(`app-tree__item--${tone}`);
          mark.classList.add(`app-tree__git--${tone}`);
          mark.textContent = letter;
          mark.title = word;
          mark.setAttribute('aria-label', word);
        } else {
          mark.setAttribute('aria-hidden', 'true');
        }
        end.append(mark);
      }
      if (end.childElementCount) button.append(end);
      button.addEventListener('click', () => onOpen(path));
    }
    button.addEventListener('dragstart', (event) => {
      dragging = { path };
      event.dataTransfer.effectAllowed = 'move';
      event.dataTransfer.setData(DRAG_TYPE, path);
      event.dataTransfer.setData('text/plain', path);
      clearProblem();
    });
    button.addEventListener('dragend', endDrag);
    button.addEventListener('contextmenu', (event) => {
      event.preventDefault();
      showMenu(button, event.clientX, event.clientY);
    });
    button.addEventListener('keydown', (event) => {
      if (event.key === 'ContextMenu' || (event.shiftKey && event.key === 'F10')) {
        event.preventDefault();
        const rect = button.getBoundingClientRect();
        showMenu(button, rect.left + 24, rect.bottom);
      }
    });
    li.append(button);
    // Un dossier déplié le reste quand l'arbre est rechargé ; sa liste suit le bouton.
    if (entry.kind === 'dir' && expanded.has(path)) expand(li, path, depth + 1, true);
    return li;
  }

  function showMenu(button, x, y) {
    const { path, kind } = button.dataset;
    const moveTo = { label: 'Déplacer vers…', onSelect: () => onMoveTo(path, button) };
    const remove = { label: 'Supprimer…', danger: true, onSelect: () => onDelete(path, button) };
    const items = kind === 'dir'
      ? [
        { label: 'Nouveau fichier', onSelect: () => startCreate(path, 'file') },
        { label: 'Nouveau dossier', onSelect: () => startCreate(path, 'dir') },
        'sep',
        moveTo,
        remove,
      ]
      : [moveTo, remove];
    openMenu({ x, y, label: path, items, returnFocus: button });
  }

  function buttonFor(path) {
    return [...root.querySelectorAll('.app-tree__item')].find((b) => b.dataset.path === path) ?? null;
  }

  function listFor(path) {
    if (path === '') return root;
    return [...root.querySelectorAll('ul')].find((ul) => ul.dataset.path === path) ?? null;
  }

  /** Saisie du nom d'une entrée nouvelle, en tête du dossier `parent` (`''` : racine). */
  async function startCreate(parent, kind) {
    endEdit();
    // Aucun rechargement entre le dépliage du dossier et la saisie.
    editing = { li: null };
    let ul = listFor(parent);
    if (!ul) {
      const button = buttonFor(parent);
      if (!button) {
        endEdit();
        return;
      }
      expanded.add(parent);
      button.setAttribute('aria-expanded', 'true');
      const depth = Number(button.style.getPropertyValue('--depth')) + 1;
      ul = await expand(button.parentElement, parent, depth);
    }
    const depth = Number(ul.dataset.depth);
    const id = `nouvelle-entree-${++inputs}`;
    const li = document.createElement('li');
    li.className = 'app-tree__new';
    li.style.setProperty('--depth', String(depth));
    const label = document.createElement('label');
    label.className = 'app-sr-only';
    label.htmlFor = id;
    label.textContent = kind === 'dir' ? 'Nom du nouveau dossier' : 'Nom du nouveau fichier';
    const input = document.createElement('input');
    input.className = 'hd-input app-tree__input';
    input.id = id;
    input.type = 'text';
    input.autocomplete = 'off';
    input.spellcheck = false;
    const hint = document.createElement('span');
    hint.className = 'app-tree__hint';
    hint.id = `${id}-aide`;
    hint.setAttribute('aria-live', 'polite');
    hint.textContent = 'Entrée pour créer · Échap pour annuler';
    input.setAttribute('aria-describedby', hint.id);
    li.append(label, input, hint);
    ul.prepend(li);
    editing = { li };
    input.focus();

    const problem = (text) => {
      hint.textContent = text;
      hint.classList.add('app-tree__hint--error');
      input.setAttribute('aria-invalid', 'true');
    };
    input.addEventListener('input', () => {
      if (!input.hasAttribute('aria-invalid')) return;
      input.removeAttribute('aria-invalid');
      hint.classList.remove('app-tree__hint--error');
      hint.textContent = 'Entrée pour créer · Échap pour annuler';
    });
    input.addEventListener('blur', () => {
      if (!input.disabled && !input.value.trim()) endEdit();
    });
    input.addEventListener('keydown', async (event) => {
      if (event.key === 'Escape') {
        event.preventDefault();
        endEdit();
        buttonFor(parent)?.focus();
        return;
      }
      if (event.key !== 'Enter' || input.disabled) return;
      event.preventDefault();
      const name = input.value.trim();
      if (!name) return problem('Saisissez un nom.');
      if (name.includes('/')) return problem('Un nom ne peut pas contenir « / ».');
      if (name === '.' || name === '..') return problem('Nom invalide.');
      const path = parent ? `${parent}/${name}` : name;
      input.disabled = true;
      try {
        await create(path, kind);
      } catch (error) {
        input.disabled = false;
        input.focus();
        problem(CREATE_ERRORS[error.code] ?? error.message);
        return;
      }
      endEdit();
      focusPath = path;
      await fill(ul, parent, depth);
      onCreated(path, kind);
    });
  }

  function endEdit() {
    if (!editing) return;
    // Retirer la ligne fait perdre le focus au champ : son blur rappelle endEdit.
    const { li } = editing;
    editing = null;
    li?.remove();
    if (refreshPending) refresh();
  }

  /** Recharge la racine et les dossiers dépliés ; attend la fin d'une saisie de nom. */
  function refresh() {
    if (editing) {
      refreshPending = true;
      return;
    }
    refreshPending = false;
    clearTimeout(refreshTimer);
    refreshTimer = setTimeout(() => {
      const active = document.activeElement;
      if (!focusPath && root.contains(active) && active.dataset.path) focusPath = active.dataset.path;
      fill(root, '', 0, true);
    }, REFRESH_DELAY_MS);
  }

  /* ---------- Déplacer ---------- */

  /** Dossier visé sous le pointeur : le dossier survolé, celui d'un fichier, sinon la racine. */
  function dropTarget(event) {
    const button = event.target.closest?.('.app-tree__item');
    if (!button || !zone.contains(button)) return { dir: '', mark: head };
    if (button.dataset.kind === 'dir') return { dir: button.dataset.path, mark: button, button };
    const dir = parentOf(button.dataset.path);
    return { dir, mark: dir ? buttonFor(dir) : head };
  }

  function markDrop(element) {
    if (dropMark === element) return;
    dropMark?.classList.remove('app-tree__drop');
    dropMark = element;
    dropMark?.classList.add('app-tree__drop');
  }

  /** Ouvre un dossier replié survolé assez longtemps. */
  function scheduleExpand(button) {
    if (expandTarget === button) return;
    clearTimeout(expandTimer);
    expandTarget = button;
    if (!button || button.getAttribute('aria-expanded') === 'true') return;
    expandTimer = setTimeout(() => {
      if (dragging && button.isConnected && button.getAttribute('aria-expanded') !== 'true') button.click();
    }, EXPAND_DELAY_MS);
  }

  function endDrag() {
    dragging = null;
    markDrop(null);
    clearTimeout(expandTimer);
    expandTarget = null;
  }

  // Fichiers venus de l'extérieur du navigateur : ignorés (pas d'import), sans que la page
  // les ouvre à la place de l'IDE. L'éditeur garde ses propres dépôts.
  window.addEventListener('dragover', (event) => {
    if (event.defaultPrevented || !event.dataTransfer.types.includes('Files')) return;
    event.preventDefault();
    event.dataTransfer.dropEffect = 'none';
  });
  window.addEventListener('drop', (event) => {
    if (!event.defaultPrevented && event.dataTransfer.types.includes('Files')) event.preventDefault();
  });

  zone.addEventListener('dragover', (event) => {
    if (!dragging || !event.dataTransfer.types.includes(DRAG_TYPE)) return;
    const target = dropTarget(event);
    scheduleExpand(target.button ?? null);
    if (!canMoveInto(dragging.path, target.dir)) {
      // Sans preventDefault, le navigateur montre le curseur « interdit ».
      event.dataTransfer.dropEffect = 'none';
      markDrop(null);
      return;
    }
    event.preventDefault();
    event.dataTransfer.dropEffect = 'move';
    markDrop(target.mark);
  });
  zone.addEventListener('dragleave', (event) => {
    if (!zone.contains(event.relatedTarget)) {
      markDrop(null);
      scheduleExpand(null);
    }
  });
  zone.addEventListener('drop', (event) => {
    if (!dragging || !event.dataTransfer.types.includes(DRAG_TYPE)) return;
    event.preventDefault();
    const { path } = dragging;
    const { dir } = dropTarget(event);
    endDrag();
    if (canMoveInto(path, dir)) moveEntry(path, dir);
  });

  /** Déplace `from` dans `toDir` ; le refus du serveur s'affiche sous l'arbre. */
  async function moveEntry(from, toDir) {
    clearProblem();
    let reply;
    try {
      reply = await move(from, toDir);
    } catch (error) {
      showProblem(MOVE_ERRORS[error.code] ?? error.message);
      return false;
    }
    const { to } = reply.payload;
    // Les dossiers dépliés le restent à leur nouvel endroit ; la destination s'ouvre.
    for (const path of [...expanded]) {
      if (path === from || path.startsWith(`${from}/`)) {
        expanded.delete(path);
        expanded.add(to + path.slice(from.length));
      }
    }
    for (let dir = toDir; dir; dir = parentOf(dir)) expanded.add(dir);
    if (current === from || current?.startsWith(`${from}/`)) current = to + current.slice(from.length);
    focusPath = to;
    refresh();
    onMoved(reply.payload);
    return true;
  }

  function showProblem(text) {
    problem.textContent = text;
    problem.hidden = false;
  }

  function clearProblem() {
    problem.hidden = true;
    problem.textContent = '';
  }

  function markCurrent() {
    for (const button of root.querySelectorAll('.app-tree__file')) {
      if (button.dataset.path === current) button.setAttribute('aria-current', 'page');
      else button.removeAttribute('aria-current');
    }
  }

  return {
    load: () => fill(root, '', 0),
    refresh,
    startCreate,
    moveEntry,
    setCurrent(path) {
      current = path;
      markCurrent();
    },
    /** Statut git (`null` sans dépôt) ; l'arbre est rechargé pour suivre les fichiers. */
    setGit(status) {
      git = status;
      letters = new Map((status?.entries ?? []).map((e) => [e.path, e.status]));
      changedDirs = new Set();
      for (const path of letters.keys()) {
        const parts = path.split('/');
        for (let i = 1; i < parts.length; i += 1) changedDirs.add(parts.slice(0, i).join('/'));
      }
      refresh();
    },
  };
}
