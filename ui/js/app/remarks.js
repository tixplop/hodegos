/* Cartes des remarques dans le panneau : mentor, palier, message, passage visé. */

const LEVEL_NAMES = { indice: 'Indice', question: 'Question', piste: 'Piste', exemple: 'Exemple' };

/**
 * `items` : `[{ remark, number, located }]` ; `mentorName(id)` donne le nom affiché ;
 * `onSelect(remark)` amène le passage à l'écran.
 */
export function renderRemarks(root, items, { activeId, mentorName, onSelect }) {
  if (!items.length) {
    const empty = document.createElement('p');
    empty.className = 'hd-muted app-panel__note';
    empty.textContent = 'Aucune remarque pour ce fichier.';
    root.replaceChildren(empty);
    return;
  }
  const title = document.createElement('p');
  title.className = 'hd-nav__label';
  title.textContent = `Remarques (${items.length})`;

  const cards = items.map(({ remark, number, located }) => {
    const card = document.createElement('article');
    card.className = remark.id === activeId ? 'hd-card hd-card--active' : 'hd-card';

    const head = document.createElement('div');
    head.className = 'hd-card__head';
    const marker = document.createElement('span');
    marker.className = remark.id === activeId ? 'hd-marker hd-marker--active app-marker-inline' : 'hd-marker app-marker-inline';
    marker.textContent = String(number);
    const author = document.createElement('span');
    author.className = 'hd-msg__author--mentor';
    author.textContent = mentorName(remark.mentor_id);
    const level = document.createElement('span');
    level.className = 'hd-badge hd-card__end';
    level.textContent = LEVEL_NAMES[remark.level] ?? remark.level;
    head.append(marker, author, level);

    const message = document.createElement('p');
    message.className = 'hd-msg';
    const lastFromMentor = [...remark.thread].reverse().find((entry) => entry.author === 'mentor');
    message.textContent = lastFromMentor?.text ?? '';

    const where = document.createElement('div');
    where.className = 'hd-row';
    if (located) {
      const go = document.createElement('button');
      go.type = 'button';
      go.className = 'hd-btn hd-btn--ghost hd-btn--sm';
      go.textContent = `Aller à l.${remark.anchor.line_hint}`;
      go.addEventListener('click', () => onSelect(remark));
      where.append(go);
    } else {
      const missing = document.createElement('span');
      missing.className = 'hd-muted app-remark__missing';
      missing.textContent = 'Passage cité introuvable dans le fichier.';
      where.append(missing);
    }
    const section = document.createElement('span');
    section.className = 'hd-id';
    section.textContent = remark.anchor.section_id;
    where.append(section);

    card.append(head, message, where);
    return card;
  });
  root.replaceChildren(title, ...cards);
}
