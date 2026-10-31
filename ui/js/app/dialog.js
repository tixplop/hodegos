/* Confirmation (hd-dialog) : <dialog> natif, « Annuler » a le focus, Échap annule. */

let counter = 0;

/**
 * `title` et `body` : nœuds ou textes. Renvoie une promesse : `true` si l'action est
 * confirmée.
 */
export function confirmDialog({ title, body, confirmLabel, returnFocus }) {
  const id = `dialogue-${++counter}`;
  const dialog = document.createElement('dialog');
  dialog.className = 'hd-dialog';
  dialog.setAttribute('role', 'alertdialog');
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

  const cancel = document.createElement('button');
  cancel.type = 'button';
  cancel.className = 'hd-btn hd-btn--secondary';
  cancel.textContent = 'Annuler';
  const confirm = document.createElement('button');
  confirm.type = 'button';
  confirm.className = 'hd-btn hd-btn--danger';
  confirm.textContent = confirmLabel;
  const actions = document.createElement('div');
  actions.className = 'hd-dialog__actions';
  actions.append(cancel, confirm);

  dialog.append(heading, text, actions);
  document.body.append(dialog);

  return new Promise((resolve) => {
    cancel.addEventListener('click', () => dialog.close('cancel'));
    confirm.addEventListener('click', () => dialog.close('confirm'));
    dialog.addEventListener('close', () => {
      dialog.remove();
      if (returnFocus?.isConnected) returnFocus.focus();
      resolve(dialog.returnValue === 'confirm');
    });
    dialog.showModal();
    cancel.focus();
  });
}
