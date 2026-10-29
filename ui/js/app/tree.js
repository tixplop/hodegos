/* Arborescence du projet, chargée dossier par dossier (list_dir). */
import { icon } from './icons.js';

/**
 * `list(path)` renvoie une promesse des entrées du dossier ;
 * `onOpen(path)` est appelé au clic sur un fichier.
 */
export function createTree(root, { list, onOpen, onError }) {
  let current = null;

  async function fill(ul, path, depth) {
    try {
      const entries = await list(path);
      ul.replaceChildren(...entries.map((entry) => item(entry, path, depth)));
      markCurrent();
    } catch (error) {
      onError(error);
    }
  }

  function item(entry, parent, depth) {
    const path = parent ? `${parent}/${entry.name}` : entry.name;
    const li = document.createElement('li');
    const button = document.createElement('button');
    button.type = 'button';
    button.className = 'hd-nav__item app-tree__item';
    button.style.setProperty('--depth', String(depth));
    button.dataset.path = path;
    button.title = path;
    const name = document.createElement('span');
    name.className = 'app-tree__name';
    name.textContent = entry.name;

    if (entry.kind === 'dir') {
      button.setAttribute('aria-expanded', 'false');
      button.append(icon('chevron'), name);
      button.addEventListener('click', () => {
        const open = button.getAttribute('aria-expanded') === 'true';
        button.setAttribute('aria-expanded', String(!open));
        if (open) {
          li.querySelector('ul')?.remove();
          return;
        }
        const ul = document.createElement('ul');
        li.append(ul);
        fill(ul, path, depth + 1);
      });
    } else {
      button.classList.add('hd-mono', 'app-tree__file');
      button.append(name);
      if (entry.mentors?.length) {
        const meta = document.createElement('span');
        meta.className = 'hd-nav__meta';
        meta.textContent = entry.mentors.map((m) => m.initials).join(' · ');
        meta.title = entry.mentors.map((m) => m.name).join(', ');
        button.append(meta);
      }
      button.addEventListener('click', () => onOpen(path));
    }
    li.append(button);
    return li;
  }

  function markCurrent() {
    for (const button of root.querySelectorAll('.app-tree__file')) {
      if (button.dataset.path === current) button.setAttribute('aria-current', 'page');
      else button.removeAttribute('aria-current');
    }
  }

  return {
    load: () => fill(root, '', 0),
    setCurrent(path) {
      current = path;
      markCurrent();
    },
  };
}
