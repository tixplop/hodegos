/* Dialogues (hd-dialog) : <dialog> natif, Échap annule. */

let counter = 0;

/**
 * `title` et `body` : nœuds ou textes. `choices` : `{ value, label, variant }` dans l'ordre
 * d'affichage (`variant` : primary, secondary, ghost ou danger) ; `focus` : valeur du
 * bouton qui a le focus ; `alert` : action destructrice. Renvoie une promesse de la
 * valeur choisie, `null` si le dialogue est fermé par Échap.
 */
export function chooseDialog({ title, body, choices, focus, alert = false, returnFocus }) {
  const id = `dialogue-${++counter}`;
  const dialog = document.createElement('dialog');
  dialog.className = 'hd-dialog';
  if (alert) dialog.setAttribute('role', 'alertdialog');
  dialog.setAttribute('aria-labelledby', `${id}-titre`);
  dialog.setAttribute('aria-describedby', `${id}-texte`);

  const heading = document.createElement('h2');
  heading.className = 'hd-dialog__title';
  heading.id = `${id}-titre`;
  heading.append(...[title].flat());

  const text = document.createElement('div');
  text.className = 'hd-dialog__body';
  text.id = `${id}-texte`;
  text.append(...body);

  const actions = document.createElement('div');
  actions.className = 'hd-dialog__actions';
  let focused = null;
  for (const choice of choices) {
    const button = document.createElement('button');
    button.type = 'button';
    button.className = `hd-btn hd-btn--${choice.variant}`;
    button.textContent = choice.label;
    button.addEventListener('click', () => dialog.close(choice.value));
    actions.append(button);
    if (choice.value === focus) focused = button;
  }

  dialog.append(heading, text, actions);
  document.body.append(dialog);

  return new Promise((resolve) => {
    dialog.addEventListener('close', () => {
      dialog.remove();
      if (returnFocus?.isConnected) returnFocus.focus();
      resolve(dialog.returnValue || null);
    });
    dialog.showModal();
    focused?.focus();
  });
}

/**
 * Confirmation d'une action destructrice : « Annuler » a le focus. Renvoie une promesse :
 * `true` si l'action est confirmée.
 */
export async function confirmDialog({ title, body, confirmLabel, returnFocus }) {
  const choice = await chooseDialog({
    title,
    body,
    choices: [
      { value: 'cancel', label: 'Annuler', variant: 'secondary' },
      { value: 'confirm', label: confirmLabel, variant: 'danger' },
    ],
    focus: 'cancel',
    alert: true,
    returnFocus,
  });
  return choice === 'confirm';
}
