/* « Déplacer vers… » : alternative au glisser-déposer (WCAG 2.5.7). Un champ filtre la liste
   des dossiers du projet ; flèches pour choisir, Entrée déplace, Échap annule. */
import { chooseDialog } from './dialog.js';
import { canMoveInto } from './tree.js';

let counter = 0;

/**
 * `dirs` : dossiers du projet (`list_dirs`), `truncated` s'il en manque. Renvoie une
 * promesse du dossier choisi (`''` : racine), `null` si le dialogue est fermé.
 */
export function chooseDestination({ path, kind, dirs, truncated, returnFocus }) {
  const id = `deplacer-${++counter}`;
  const name = path.split('/').pop();
  const targets = ['', ...dirs].filter((dir) => canMoveInto(path, dir));

  const label = document.createElement('label');
  label.className = 'hd-label';
  label.htmlFor = `${id}-filtre`;
  label.textContent = 'Dossier de destination';
  const input = document.createElement('input');
  input.className = 'hd-input app-move__filter';
  input.id = `${id}-filtre`;
  input.type = 'text';
  input.autocomplete = 'off';
  input.spellcheck = false;
  input.placeholder = 'Filtrer les dossiers';
  input.setAttribute('role', 'combobox');
  input.setAttribute('aria-expanded', 'true');
  input.setAttribute('aria-autocomplete', 'list');
  input.setAttribute('aria-controls', `${id}-liste`);
  const listbox = document.createElement('ul');
  listbox.className = 'app-move__list';
  listbox.id = `${id}-liste`;
  listbox.setAttribute('role', 'listbox');
  listbox.setAttribute('aria-label', 'Dossiers');
  const count = document.createElement('p');
  count.className = 'hd-muted app-move__count';
  count.setAttribute('aria-live', 'polite');

  let shown = [];
  let active = 0;

  function filter() {
    const query = input.value.trim().toLowerCase();
    shown = targets.filter((dir) => (dir || 'racine du projet').toLowerCase().includes(query));
    active = 0;
    listbox.replaceChildren(...shown.map((dir, index) => {
      const option = document.createElement('li');
      option.className = 'app-move__option';
      option.id = `${id}-option-${index}`;
      option.setAttribute('role', 'option');
      if (dir) {
        option.classList.add('hd-mono');
        option.textContent = dir;
      } else {
        option.textContent = 'Racine du projet';
      }
      option.addEventListener('mousedown', (event) => event.preventDefault());
      option.addEventListener('click', () => {
        active = index;
        input.closest('dialog')?.close('move');
      });
      return option;
    }));
    const found = shown.length === 0
      ? 'Aucun dossier ne correspond.'
      : `${shown.length} ${shown.length === 1 ? 'dossier' : 'dossiers'}`;
    count.textContent = truncated ? `${found} · liste limitée aux 5 000 premiers dossiers.` : found;
    select(0);
  }

  function select(index) {
    const options = listbox.children;
    if (options.length === 0) {
      input.removeAttribute('aria-activedescendant');
      return;
    }
    active = (index + options.length) % options.length;
    for (const [i, option] of [...options].entries()) {
      option.setAttribute('aria-selected', String(i === active));
    }
    input.setAttribute('aria-activedescendant', options[active].id);
    options[active].scrollIntoView({ block: 'nearest' });
  }

  input.addEventListener('input', filter);
  input.addEventListener('keydown', (event) => {
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();
      select(active + (event.key === 'ArrowDown' ? 1 : -1));
    } else if (event.key === 'Home' && event.ctrlKey) {
      event.preventDefault();
      select(0);
    } else if (event.key === 'End' && event.ctrlKey) {
      event.preventDefault();
      select(shown.length - 1);
    } else if (event.key === 'Enter') {
      event.preventDefault();
      if (shown.length) input.closest('dialog')?.close('move');
    }
  });

  const title = document.createElement('code');
  title.className = 'hd-mono';
  title.textContent = name;
  const pending = chooseDialog({
    title: ['Déplacer ', title, kind === 'dir' ? ' et son contenu' : ''],
    body: [label, input, listbox, count],
    choices: [
      { value: 'cancel', label: 'Annuler', variant: 'ghost' },
      { value: 'move', label: 'Déplacer', variant: 'primary' },
    ],
    returnFocus,
  });
  filter();
  input.focus();
  return pending.then((choice) => (choice === 'move' && shown.length ? shown[active] : null));
}
