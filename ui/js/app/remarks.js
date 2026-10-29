/* Cartes des remarques dans le panneau : mentor, paliers, fil, passage visé, actions. */

const LEVEL_NAMES = { indice: 'Indice', question: 'Question', piste: 'Piste', exemple: 'Exemple' };

/**
 * `items` : `[{ remark, number, located }]`. Options :
 * - `activeId`, `mentorName(id)`, `onSelect(remark)` : sélection et signature ;
 * - `pending` (ids en attente du mentor), `notes` (id → texte), `composing` (ids dont le
 *   champ de réponse est ouvert), `drafts` (id → texte saisi) ;
 * - `onHint(remark)`, `onCompose(remark, open)`, `onDraft(id, text)`, `onReply(remark, text)`,
 *   `onDismiss(remark, reason)`.
 */
export function renderRemarks(root, items, options) {
  if (!items.length) {
    const empty = document.createElement('p');
    empty.className = 'hd-muted app-panel__note';
    empty.textContent = 'Aucune remarque pour ce fichier.';
    root.replaceChildren(empty);
    return;
  }
  // Le champ de réponse en cours de saisie garde le focus d'un rendu à l'autre.
  const focused = root.contains(document.activeElement) ? document.activeElement.id : '';

  const title = document.createElement('p');
  title.className = 'hd-nav__label';
  title.textContent = `Remarques (${items.length})`;
  root.replaceChildren(title, ...items.map((item) => card(item, options)));

  const input = focused && document.getElementById(focused);
  if (input) {
    input.focus();
    input.setSelectionRange?.(input.value.length, input.value.length);
  }
}

function card({ remark, number, located }, options) {
  const { activeId, mentorName, pending, notes } = options;
  const active = remark.status === 'open' || remark.status === 'answered';
  const detached = remark.status === 'detached';
  const waiting = pending.has(remark.id);

  const article = document.createElement('article');
  article.className = remark.id === activeId ? 'hd-card hd-card--active' : 'hd-card';

  const head = document.createElement('div');
  head.className = 'hd-card__head';
  const marker = document.createElement('span');
  marker.className = remark.id === activeId ? 'hd-marker hd-marker--active app-marker-inline' : 'hd-marker app-marker-inline';
  marker.textContent = String(number);
  const author = document.createElement('span');
  author.className = 'hd-msg__author--mentor';
  author.textContent = mentorName(remark.mentor_id);
  const badge = document.createElement('span');
  badge.className = detached ? 'hd-badge hd-badge--warning hd-card__end' : 'hd-badge hd-card__end';
  badge.textContent = detached ? 'Détachée' : LEVEL_NAMES[remark.level] ?? remark.level;
  head.append(marker, author, badge);
  article.append(head);

  if (!detached) article.append(stepper(remark));
  for (const entry of remark.thread) article.append(threadEntry(entry));

  if (waiting) {
    const thinking = document.createElement('p');
    thinking.className = 'hd-muted app-remark__note';
    thinking.setAttribute('role', 'status');
    thinking.textContent = 'Le mentor réfléchit…';
    article.append(thinking);
  } else if (notes.has(remark.id)) {
    const note = document.createElement('p');
    note.className = 'hd-muted app-remark__note';
    note.setAttribute('role', 'status');
    note.textContent = notes.get(remark.id);
    article.append(note);
  }

  article.append(where(remark, located, detached, options));
  article.append(actions(remark, active, waiting, options));
  if (active && options.composing.has(remark.id)) article.append(composer(remark, waiting, options));
  return article;
}

/** Paliers d'aide : ceux déjà donnés, le palier courant, ceux qui restent. */
function stepper(remark) {
  const list = document.createElement('ol');
  list.className = 'hd-stepper';
  list.setAttribute('aria-label', "Paliers d'aide");
  for (const level of window.Hodegos.hintLadder) {
    const item = document.createElement('li');
    const step = document.createElement('span');
    const name = LEVEL_NAMES[level];
    if (level === remark.level) {
      step.className = 'hd-step hd-step--current';
      step.setAttribute('aria-current', 'step');
      step.textContent = name;
    } else if (remark.levels_given.includes(level)) {
      step.className = 'hd-step hd-step--done';
      step.textContent = `${name} ✓`;
    } else {
      step.className = 'hd-step';
      step.textContent = name;
    }
    item.append(step);
    list.append(item);
  }
  return list;
}

function threadEntry(entry) {
  if (entry.author === 'user') {
    const message = document.createElement('p');
    message.className = 'hd-msg hd-msg--reply app-remark__text';
    const author = document.createElement('span');
    author.className = 'hd-msg__author';
    author.textContent = 'Vous';
    message.append(author, ` · ${entry.text}`);
    return message;
  }
  const block = document.createElement('div');
  block.className = 'hd-msg app-remark__text';
  block.append(...formatted(entry.text));
  return block;
}

/**
 * Message du mentor : blocs entre ``` en `pre`, code en ligne entre backticks en police
 * à chasse fixe, le reste en texte. Jamais de HTML venu du modèle.
 */
function formatted(text) {
  const nodes = [];
  text.split(/^```[^\n]*$/m).forEach((part, index) => {
    if (index % 2 === 1) {
      const pre = document.createElement('pre');
      pre.className = 'hd-mono app-remark__code';
      pre.textContent = part.replace(/^\n/, '').replace(/\n$/, '');
      nodes.push(pre);
      return;
    }
    const trimmed = part.replace(/^\n+/, '').replace(/\n+$/, '');
    if (!trimmed) return;
    const paragraph = document.createElement('p');
    trimmed.split('`').forEach((piece, i) => {
      if (i % 2 === 1) {
        const code = document.createElement('span');
        code.className = 'hd-mono app-remark__inline';
        code.textContent = piece;
        paragraph.append(code);
      } else if (piece) {
        paragraph.append(piece);
      }
    });
    nodes.push(paragraph);
  });
  return nodes;
}

function where(remark, located, detached, { onSelect }) {
  const row = document.createElement('div');
  row.className = 'hd-row';
  if (located) {
    const go = document.createElement('button');
    go.type = 'button';
    go.className = 'hd-btn hd-btn--ghost hd-btn--sm';
    go.textContent = `Aller à l.${remark.anchor.line_hint}`;
    go.addEventListener('click', () => onSelect(remark));
    row.append(go);
  } else {
    const missing = document.createElement('span');
    missing.className = 'hd-muted app-remark__missing';
    missing.textContent = detached
      ? 'Passage modifié : remarque détachée de votre texte.'
      : 'Passage cité introuvable dans le fichier.';
    row.append(missing);
  }
  const section = document.createElement('span');
  section.className = 'hd-id';
  section.textContent = remark.anchor.section_id;
  row.append(section);
  return row;
}

function actions(remark, active, waiting, { onHint, onCompose, onDismiss, composing }) {
  const row = document.createElement('div');
  row.className = 'hd-row app-remark__actions';
  if (active) {
    const next = window.Hodegos.nextHint(remark.level);
    // Au dernier palier, le bouton disparaît.
    if (next) {
      const hint = smallButton('Palier suivant', 'secondary', () => onHint(remark));
      hint.title = `Passer au palier ${LEVEL_NAMES[next].toLowerCase()}`;
      hint.disabled = waiting;
      row.append(hint);
    }
    const open = composing.has(remark.id);
    const reply = smallButton('Répondre', 'ghost', () => onCompose(remark, !open));
    reply.setAttribute('aria-expanded', String(open));
    row.append(
      reply,
      smallButton('Résolu', 'ghost', () => onDismiss(remark, 'resolved')),
      smallButton('Ignorer', 'ghost', () => onDismiss(remark, 'ignored')),
    );
  } else {
    row.append(smallButton('Fermer', 'ghost', () => onDismiss(remark, 'ignored')));
  }
  return row;
}

function composer(remark, waiting, { mentorName, drafts, onDraft, onReply }) {
  const form = document.createElement('form');
  form.className = 'hd-composer';
  const id = `reponse-${remark.id}`;
  const label = document.createElement('label');
  label.className = 'hd-label';
  label.htmlFor = id;
  label.textContent = `Répondre à ${mentorName(remark.mentor_id)}`;
  const row = document.createElement('div');
  row.className = 'hd-composer__row';
  const input = document.createElement('input');
  input.className = 'hd-input';
  input.id = id;
  input.type = 'text';
  input.maxLength = 2000;
  input.placeholder = 'Votre raisonnement ou votre question';
  input.value = drafts.get(remark.id) ?? '';
  input.addEventListener('input', () => onDraft(remark.id, input.value));
  const send = document.createElement('button');
  send.type = 'submit';
  send.className = 'hd-btn hd-btn--strong';
  send.textContent = 'Envoyer';
  send.disabled = waiting;
  form.addEventListener('submit', (event) => {
    event.preventDefault();
    if (!waiting) onReply(remark, input.value);
  });
  row.append(input, send);
  form.append(label, row);
  return form;
}

function smallButton(label, variant, onClick) {
  const element = document.createElement('button');
  element.type = 'button';
  element.className = `hd-btn hd-btn--${variant} hd-btn--sm`;
  element.textContent = label;
  element.addEventListener('click', onClick);
  return element;
}
