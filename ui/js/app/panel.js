/* Panneau des interlocuteurs : titulaire, consultant, raisons, choix forcé. */

/** « chemin docs/architecture/** » → « Chemin » suivi du motif en mono. */
function reasonNode(reason) {
  const span = document.createElement('span');
  const match = /^chemin (.+)$/.exec(reason);
  if (match) {
    const code = document.createElement('span');
    code.className = 'hd-mono';
    code.textContent = match[1];
    span.append('Chemin ', code);
  } else {
    span.textContent = reason.charAt(0).toUpperCase() + reason.slice(1);
  }
  return span;
}

function mentorRow(mentor, badgeText, badgeVariant, reasons) {
  const row = document.createElement('div');
  row.className = 'hd-mentor';
  const avatar = document.createElement('span');
  avatar.className = 'hd-avatar hd-avatar--mentor';
  avatar.setAttribute('aria-hidden', 'true');
  avatar.textContent = mentor.initials;

  const body = document.createElement('div');
  body.className = 'hd-mentor__body';
  const head = document.createElement('div');
  head.className = 'hd-row app-mentor__head';
  const name = document.createElement('strong');
  name.className = 'hd-mentor__name';
  name.textContent = mentor.name;
  const badge = document.createElement('span');
  badge.className = `hd-badge${badgeVariant ? ` hd-badge--${badgeVariant}` : ''}`;
  badge.textContent = badgeText;
  head.append(name, badge);

  const why = document.createElement('span');
  why.className = 'hd-mentor__why';
  (reasons ?? []).forEach((reason, index) => {
    if (index > 0) why.append(' · ');
    why.append(reasonNode(reason));
  });
  body.append(head, why);
  row.append(avatar, body);
  return row;
}

function ghostButton(label, onClick) {
  const button = document.createElement('button');
  button.type = 'button';
  button.className = 'hd-btn hd-btn--ghost hd-btn--sm';
  button.textContent = label;
  button.addEventListener('click', onClick);
  return button;
}

/** `onForce(mentorId | null)` impose un titulaire, ou revient au choix automatique. */
export function createPanel(root, { onForce }) {
  function note(text) {
    const p = document.createElement('p');
    p.className = 'hd-muted app-panel__note';
    p.textContent = text;
    return p;
  }

  return {
    /** Affiche un `casting`, ou l'état vide si `casting` est nul. */
    render(casting) {
      if (!casting) {
        root.replaceChildren(note('Ouvrez un fichier pour voir qui vous accompagne.'));
        return;
      }
      const items = [];
      if (casting.section_id) {
        const section = document.createElement('div');
        section.className = 'hd-row';
        const id = document.createElement('span');
        id.className = 'hd-id';
        id.textContent = casting.section_id;
        section.append(id);
        if (casting.section_name) {
          const name = document.createElement('span');
          name.className = 'hd-muted';
          name.textContent = casting.section_name;
          section.append(name);
        }
        items.push(section);
      }

      if (casting.titular) {
        // Un choix forcé se lit dans les raisons (« Choisi par vous »), pas dans le badge.
        items.push(mentorRow(casting.titular, 'Titulaire', 'info', casting.reasons[casting.titular.id]));
      } else {
        items.push(note('Aucun mentor pour cette section.'));
      }
      for (const consultant of casting.consultants) {
        items.push(mentorRow(consultant, 'Consultant', null, casting.reasons[consultant.id]));
      }

      const actions = document.createElement('div');
      actions.className = 'hd-row';
      if (casting.available.length) {
        const label = document.createElement('span');
        label.className = 'app-available__label';
        label.textContent = 'Disponibles :';
        actions.append(label);
        for (const mentor of casting.available) {
          actions.append(ghostButton(`Choisir ${mentor.name}`, () => onForce(mentor.id)));
        }
      }
      if (casting.forced === 'user') {
        actions.append(ghostButton('Revenir au choix automatique', () => onForce(null)));
      }
      if (actions.childElementCount) items.push(actions);
      root.replaceChildren(...items);
    },
  };
}
