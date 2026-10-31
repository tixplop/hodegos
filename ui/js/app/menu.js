/* Menu contextuel (hd-menu) : un seul ouvert à la fois, fermé par Échap, un clic
   ailleurs, le défilement ou le choix d'un élément ; flèches pour se déplacer. */

let current = null;

/**
 * `items` : `{ label, danger, onSelect }` ou `'sep'`. `returnFocus` reprend le focus
 * à la fermeture, sauf quand un élément est choisi (l'action place le sien).
 */
export function openMenu({ x, y, label, items, returnFocus }) {
  closeMenu(false);
  const menu = document.createElement('ul');
  menu.className = 'hd-menu';
  menu.setAttribute('role', 'menu');
  if (label) menu.setAttribute('aria-label', label);
  const buttons = [];
  for (const entry of items) {
    const li = document.createElement('li');
    if (entry === 'sep') {
      li.className = 'hd-menu__sep';
      li.setAttribute('role', 'separator');
      menu.append(li);
      continue;
    }
    li.setAttribute('role', 'none');
    const button = document.createElement('button');
    button.type = 'button';
    button.className = entry.danger ? 'hd-menu__item hd-menu__item--danger' : 'hd-menu__item';
    button.setAttribute('role', 'menuitem');
    button.tabIndex = -1;
    button.textContent = entry.label;
    button.addEventListener('click', () => {
      closeMenu(false);
      entry.onSelect();
    });
    li.append(button);
    menu.append(li);
    buttons.push(button);
  }
  document.body.append(menu);

  // Gardé dans la fenêtre.
  const rect = menu.getBoundingClientRect();
  menu.style.left = `${Math.max(8, Math.min(x, window.innerWidth - rect.width - 8))}px`;
  menu.style.top = `${Math.max(8, Math.min(y, window.innerHeight - rect.height - 8))}px`;

  menu.addEventListener('keydown', (event) => {
    const index = buttons.indexOf(document.activeElement);
    const move = { ArrowDown: 1, ArrowUp: -1 }[event.key];
    if (move) {
      event.preventDefault();
      buttons[(index + move + buttons.length) % buttons.length].focus();
    } else if (event.key === 'Home' || event.key === 'End') {
      event.preventDefault();
      buttons[event.key === 'Home' ? 0 : buttons.length - 1].focus();
    } else if (event.key === 'Escape' || event.key === 'Tab') {
      event.preventDefault();
      closeMenu();
    }
  });
  const outside = (event) => {
    if (!menu.contains(event.target)) closeMenu(false);
  };
  const dismiss = () => closeMenu(false);
  document.addEventListener('pointerdown', outside, true);
  document.addEventListener('scroll', dismiss, true);
  window.addEventListener('blur', dismiss);
  window.addEventListener('resize', dismiss);
  current = {
    menu,
    returnFocus,
    cleanup() {
      document.removeEventListener('pointerdown', outside, true);
      document.removeEventListener('scroll', dismiss, true);
      window.removeEventListener('blur', dismiss);
      window.removeEventListener('resize', dismiss);
    },
  };
  buttons[0]?.focus();
}

export function closeMenu(restoreFocus = true) {
  if (!current) return;
  const { menu, returnFocus, cleanup } = current;
  current = null;
  cleanup();
  menu.remove();
  if (restoreFocus && returnFocus?.isConnected) returnFocus.focus();
}
