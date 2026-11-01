/* Panneau des interlocuteurs : titulaire, consultant, raisons, mentor activé. */

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

const CONTEXTS = { neighborhood: 'voisinage', project: 'projet' };

function mentorRow(mentor, badgeText, badgeVariant, reasons, reviewing) {
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
  // Contexte reçu en plus de la section (phase 9b).
  const context = CONTEXTS[mentor.context];
  if (context) {
    const scope = document.createElement('span');
    scope.className = 'hd-badge';
    scope.textContent = `Contexte : ${context}`;
    scope.title = mentor.context === 'project'
      ? 'Reçoit le voisinage du fichier et la carte du projet'
      : 'Reçoit le voisinage du fichier (imports, tests, documents qui le citent, même dossier)';
    head.append(scope);
  }
  if (reviewing) {
    const busy = document.createElement('span');
    busy.className = 'hd-badge hd-badge--mentor';
    busy.textContent = 'Relit la section…';
    head.append(busy);
  }

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

/**
 * `onActivate(mentor, button)` rend le mentor titulaire et lui demande une relecture ;
 * `onAutomatic()` revient au choix automatique.
 */
export function createPanel(root, { onActivate, onAutomatic }) {
  function note(text) {
    const p = document.createElement('p');
    p.className = 'hd-muted app-panel__note';
    p.textContent = text;
    return p;
  }

  return {
    /**
     * Affiche un `casting`, ou l'état vide si `casting` est nul ; `reviewing` : mentors
     * dont une passe sur cette section est en cours.
     */
    render(casting, reviewing = new Set()) {
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
        items.push(mentorRow(
          casting.titular,
          'Titulaire',
          'info',
          casting.reasons[casting.titular.id],
          reviewing.has(casting.titular.id),
        ));
      } else {
        items.push(note('Aucun mentor pour cette section.'));
      }
      for (const consultant of casting.consultants) {
        items.push(mentorRow(consultant, 'Consultant', null, casting.reasons[consultant.id], reviewing.has(consultant.id)));
      }

      const actions = document.createElement('div');
      actions.className = 'hd-row';
      if (casting.available.length) {
        const label = document.createElement('span');
        label.className = 'app-available__label';
        label.textContent = 'Disponibles :';
        actions.append(label);
        for (const mentor of casting.available) {
          const button = ghostButton(`Activer ${mentor.name}`, () => onActivate(mentor, button));
          actions.append(button);
        }
      }
      if (casting.forced === 'user') {
        actions.append(ghostButton('Revenir au choix automatique', () => onAutomatic()));
      }
      if (actions.childElementCount) items.push(actions);
      if (casting.available.length) {
        items.push(note('Activer un mentor le rend titulaire et lui demande aussitôt une relecture de la section du curseur.'));
      }
      root.replaceChildren(...items);
    },
  };
}
