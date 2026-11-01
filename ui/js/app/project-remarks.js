/* Vue Projet (phase 9c, maquette 09-2) : remarques des mentors de projet, filtrées par mentor,
   avec leurs citations cliquables, leurs paliers et leur fil ; historique des revues ; panneau
   des mentors de projet et « Demander une revue… », qui annonce coût et durée estimés. */
import { chooseDialog } from './dialog.js';
import { actions, composer, passContext, stepper, threadEntry } from './remarks.js';

const LEVEL_NAMES = { indice: 'Indice', question: 'Question', piste: 'Piste', exemple: 'Exemple' };
const TRIGGERS = { request: 'Demandée', create: 'Création', open: 'Ouverture', change: 'Modifications', commit: 'Commit' };
const OUTCOMES = {
  ok: null,
  provider_error: 'fournisseur indisponible',
  invalid_response: 'réponse invalide',
  stale: 'remplacée par une revue plus récente',
};

const money = new Intl.NumberFormat('fr-FR', { style: 'currency', currency: 'USD', maximumFractionDigits: 3 });
const when = new Intl.DateTimeFormat('fr-FR', { day: 'numeric', month: 'short', hour: '2-digit', minute: '2-digit' });

function element(tag, className, text) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

function date(text) {
  const parsed = new Date(text);
  return Number.isNaN(parsed.getTime()) ? text : when.format(parsed);
}

/** Coût affiché : « < 0,001 $ » plutôt que zéro pour un appel minuscule. */
function cost(value) {
  return value > 0 && value < 0.001 ? `< ${money.format(0.001)}` : money.format(value);
}

function seconds(ms) {
  return `${Math.max(1, Math.round(ms / 1000))} s`;
}

function plural(count, one, many) {
  return count === 1 ? `1 ${one}` : `${count} ${many}`;
}

/**
 * `root` : l'écran #projet. `request(type, payload)` ; `onOpenAt(path, line)` ouvre un fichier ;
 * `onShowMap()` : onglet Carte ; `mentorName(id)` ; `onCount(n)` : remarques ouvertes.
 */
export function createProjectRemarks(root, { request, onOpenAt, onShowMap, mentorName, onCount }) {
  const $ = (id) => root.querySelector(`#${id}`);
  const ui = {
    count: $('projet-compte'),
    filters: $('projet-filtres'),
    list: $('projet-liste'),
    reviews: $('projet-revues'),
    noReview: $('projet-revues-vide'),
    mentors: $('projet-mentors'),
    ask: $('projet-demander'),
  };
  /** Remarques affichées, par id, dans l'ordre d'arrivée. */
  const remarks = new Map();
  let mentors = [];
  let history = [];
  let filter = null;
  let activeId = null;
  const pending = new Set();
  const notes = new Map();
  const composing = new Set();
  const drafts = new Map();
  /** Mentors dont une revue est en cours. */
  const reviewing = new Set();

  const nameOf = (id) => mentors.find((m) => m.id === id)?.name ?? mentorName(id);

  function renderList() {
    const visible = [...remarks.values()].filter((r) => ['open', 'answered', 'to_review'].includes(r.status));
    onCount(visible.length);
    ui.count.hidden = visible.length === 0;
    ui.count.textContent = String(visible.length);

    const authors = [...new Set(visible.map((r) => r.mentor_id))];
    const chips = [{ id: null, label: 'Tous' }, ...authors.map((id) => ({ id, label: nameOf(id) }))];
    ui.filters.replaceChildren(...chips.map((chip) => {
      const button = element('button', 'hd-btn hd-btn--choice hd-btn--sm', chip.label);
      button.type = 'button';
      button.setAttribute('aria-pressed', String(filter === chip.id));
      button.addEventListener('click', () => { filter = chip.id; renderList(); });
      return button;
    }));
    ui.filters.hidden = authors.length < 2;

    const shown = visible.filter((r) => !filter || r.mentor_id === filter);
    if (!shown.length) {
      ui.list.replaceChildren(element('p', 'hd-muted app-project__note',
        mentors.length ? 'Aucune remarque sur le projet pour l\'instant. « Demander une revue… » en lance une.'
          : 'Aucun mentor de projet : donnez la portée project à un mentor (scopes: [project]) pour relire le projet.'));
      return;
    }
    const focused = root.contains(document.activeElement) ? document.activeElement.id : '';
    ui.list.replaceChildren(...shown.map((remark, index) => card(remark, index + 1)));
    const input = focused && document.getElementById(focused);
    if (input) input.focus();
  }

  const options = {
    mentorName: nameOf,
    composing,
    drafts,
    onHint: (remark) => hint(remark),
    onCompose: (remark, open) => {
      if (open) composing.add(remark.id);
      else composing.delete(remark.id);
      renderList();
      if (open) document.getElementById(`reponse-${remark.id}`)?.focus();
    },
    onDraft: (id, text) => drafts.set(id, text),
    onReply: (remark, text) => reply(remark, text),
    onDismiss: (remark, reason) => dismiss(remark, reason),
    onShowMap,
  };

  function card(remark, number) {
    const active = remark.status === 'open' || remark.status === 'answered';
    const waiting = pending.has(remark.id);
    const article = element('article', remark.id === activeId ? 'hd-card hd-card--active app-project__remark' : 'hd-card app-project__remark');
    article.addEventListener('click', () => { activeId = remark.id; });
    const head = element('div', 'hd-card__head');
    const marker = element('span', 'hd-marker app-marker-inline', String(number));
    const author = element('span', 'hd-msg__author--mentor', nameOf(remark.mentor_id));
    const badge = element('span', 'hd-badge hd-card__end',
      remark.kind === 'next_step' ? 'Étape suivante' : LEVEL_NAMES[remark.level] ?? remark.level);
    if (remark.status === 'to_review') {
      badge.className = 'hd-badge hd-badge--warning hd-card__end';
      badge.textContent = 'À réévaluer';
    }
    head.append(marker, author, badge);
    article.append(head);
    if (remark.kind !== 'next_step') article.append(stepper(remark));
    for (const entry of remark.thread) article.append(threadEntry(entry));
    if (waiting) {
      const thinking = element('p', 'hd-muted app-remark__note', 'Le mentor réfléchit…');
      thinking.setAttribute('role', 'status');
      article.append(thinking);
    } else if (notes.has(remark.id)) {
      const note = element('p', 'hd-muted app-remark__note', notes.get(remark.id));
      note.setAttribute('role', 'status');
      article.append(note);
    }
    if (remark.anchors.length) article.append(citations(remark));
    article.append(actions(remark, active, waiting, options));
    if (active && composing.has(remark.id)) article.append(composer(remark, waiting, options));
    if (remark.context?.length) article.append(passContext(remark, options));
    return article;
  }

  /** Passages cités : chacun ouvre son fichier à la ligne. */
  function citations(remark) {
    const list = element('ul', 'app-remark__links app-project__quotes');
    list.setAttribute('aria-label', 'Passages cités');
    for (const anchor of remark.anchors) {
      const item = element('li');
      const link = element('button', 'hd-link app-remark__link');
      link.type = 'button';
      const where = anchor.section_name ? `${anchor.section_name}, l.${anchor.line_hint}` : `l.${anchor.line_hint}`;
      link.textContent = `${anchor.path} › ${where}`;
      link.title = `« ${anchor.quote} »`;
      if (anchor.status === 'gone') {
        link.disabled = true;
        link.title = 'Passage disparu';
      }
      link.addEventListener('click', () => onOpenAt(anchor.path, anchor.line_hint));
      item.append(link);
      list.append(item);
    }
    return list;
  }

  function apply(remark) {
    remarks.set(remark.id, remark);
    renderList();
  }

  async function withBudget(send) {
    try {
      return await send(false);
    } catch (error) {
      if (error.code !== 'budget_exceeded') throw error;
      // Même confirmation que pour une remarque de section.
      const ok = window.confirm(
        "Le budget du mois est atteint. Envoyer quand même cette demande ? Son coût s'ajoutera au mois en cours.",
      );
      return ok ? send(true) : null;
    }
  }

  async function hint(remark) {
    pending.add(remark.id);
    notes.delete(remark.id);
    renderList();
    try {
      const reply = await withBudget((confirm) => request('hint_next', { remark_id: remark.id, confirm_over_budget: confirm }));
      if (reply) {
        if (reply.payload.thread.length === remark.thread.length) notes.set(remark.id, 'Le mentor n\'a rien ajouté pour l\'instant.');
        remarks.set(reply.payload.id, reply.payload);
      }
    } catch (error) {
      notes.set(remark.id, error.message);
    } finally {
      pending.delete(remark.id);
      renderList();
    }
  }

  async function reply(remark, text) {
    try {
      const answer = await request('remark_reply', { remark_id: remark.id, text });
      composing.delete(remark.id);
      drafts.delete(remark.id);
      pending.add(remark.id);
      apply(answer.payload);
    } catch (error) {
      notes.set(remark.id, error.message);
      renderList();
    }
  }

  async function dismiss(remark, reason) {
    try {
      const answer = await request('remark_dismiss', { remark_id: remark.id, reason });
      apply(answer.payload);
    } catch (error) {
      notes.set(remark.id, error.message);
      renderList();
    }
  }

  /* Panneau des mentors de projet */

  function renderMentors() {
    if (!mentors.length) {
      ui.mentors.replaceChildren(element('p', 'hd-muted app-project__note', 'Aucun mentor de projet actif.'));
      ui.ask.disabled = true;
      return;
    }
    ui.ask.disabled = false;
    ui.mentors.replaceChildren(...mentors.map((mentor) => {
      const row = element('div', 'hd-mentor');
      const avatar = element('span', 'hd-avatar hd-avatar--mentor', mentor.initials);
      avatar.setAttribute('aria-hidden', 'true');
      const body = element('div', 'hd-mentor__body');
      const head = element('div', 'hd-row app-mentor__head');
      head.append(element('strong', 'hd-mentor__name', mentor.name));
      if (reviewing.has(mentor.id) || mentor.reviewing) {
        head.append(element('span', 'hd-badge hd-badge--mentor', 'Relit le projet…'));
      }
      const triggers = mentor.triggers.map((t) => TRIGGERS[t] ?? t).join(', ');
      const last = mentor.last_review_at ? `dernière revue : ${date(mentor.last_review_at)}` : 'jamais relu';
      body.append(head, element('span', 'hd-mentor__why', `${triggers || 'sur demande'} · ${last}`));
      row.append(avatar, body);
      return row;
    }));
  }

  function renderHistory() {
    ui.noReview.hidden = history.length > 0;
    ui.reviews.replaceChildren(...history.map((review) => {
      const row = element('tr');
      const who = review.mentors.map((m) => {
        const problem = OUTCOMES[m.outcome];
        return `${nameOf(m.id)} : ${problem ?? plural(m.remarks, 'remarque', 'remarques')}`;
      }).join(' · ');
      row.append(
        element('td', '', date(review.at)),
        element('td', '', TRIGGERS[review.trigger] ?? review.trigger),
        element('td', 'app-project__detail', who),
        element('td', 'hd-mono app-project__num', cost(review.cost)),
        element('td', 'hd-mono app-project__num', seconds(review.duration_ms)),
      );
      return row;
    }));
  }

  async function loadReviews() {
    try {
      const reply = await request('reviews_get', null);
      mentors = reply.payload.mentors;
      history = reply.payload.items;
      renderMentors();
      renderHistory();
    } catch (error) {
      ui.mentors.replaceChildren(element('p', 'app-settings__error', error.message));
    }
  }

  /* Revue demandée */

  async function askReview() {
    const chosen = new Set(mentors.map((m) => m.id));
    const boxes = element('fieldset', 'app-project__choice');
    boxes.append(element('legend', 'hd-label', 'Mentors'));
    const summary = element('p', '', 'Estimation en cours…');
    const costLine = element('p', 'app-project__estimate', '');
    let estimating = 0;
    async function estimate() {
      const ticket = ++estimating;
      if (!chosen.size) {
        summary.textContent = 'Choisissez au moins un mentor.';
        costLine.textContent = '';
        return;
      }
      summary.textContent = 'Estimation en cours…';
      try {
        const reply = await request('review_estimate', { mentor_ids: [...chosen] });
        if (ticket !== estimating) return;
        const { mentors: items, total_cost: total, duration_ms: duration, budget_left: left } = reply.payload;
        summary.textContent = items.map((m) => `${nameOf(m.id)} : ${m.detail}, ${m.tokens.toLocaleString('fr-FR')} jetons estimés`).join(' ; ') + '.';
        const price = total === null ? 'coût inconnu (prix du modèle introuvable)' : `coût estimé : ${cost(total)}`;
        const basis = items.some((m) => m.basis === 'last') ? ' (d\'après les dernières revues)' : '';
        costLine.textContent = `${price}${basis} · durée estimée : ${seconds(duration)} · budget restant : ${money.format(left)}.`;
      } catch (error) {
        if (ticket === estimating) summary.textContent = error.message;
      }
    }
    for (const mentor of mentors) {
      const label = element('label', 'hd-row app-project__check');
      const box = element('input');
      box.type = 'checkbox';
      box.checked = true;
      box.addEventListener('change', () => {
        if (box.checked) chosen.add(mentor.id);
        else chosen.delete(mentor.id);
        estimate();
      });
      label.append(box, `${mentor.name} (modèle ${mentor.model === 'strong' ? 'fort' : 'rapide'})`);
      boxes.append(label);
    }
    const note = element('p', 'app-project__note', 'Chaque mentor reçoit la carte du projet, ses fichiers clés et le diff depuis sa dernière revue ; jamais un fichier ignoré par git ni un secret.');
    estimate();
    const choice = await chooseDialog({
      title: 'Revue du projet',
      body: [boxes, summary, costLine, note],
      choices: [
        { value: 'cancel', label: 'Annuler', variant: 'secondary' },
        { value: 'run', label: 'Lancer la revue', variant: 'primary' },
      ],
      focus: 'run',
      returnFocus: ui.ask,
    });
    if (choice !== 'run' || !chosen.size) return;
    try {
      const started = await withBudget((confirm) => request('review_request', { mentor_ids: [...chosen], confirm_over_budget: confirm }));
      if (started) reviewStarted(started.payload);
    } catch (error) {
      ui.mentors.prepend(element('p', 'app-settings__error', error.message));
    }
  }

  function reviewStarted({ mentor_ids: ids }) {
    for (const id of ids) reviewing.add(id);
    renderMentors();
  }

  ui.ask.addEventListener('click', askReview);

  return {
    /** Remarques et revues, à l'ouverture de la vue. */
    async load() {
      try {
        const reply = await request('project_remarks_get', null);
        remarks.clear();
        for (const remark of reply.payload.items) remarks.set(remark.id, remark);
      } catch (error) {
        ui.list.replaceChildren(element('p', 'app-settings__error', error.message));
      }
      await loadReviews();
      renderList();
    },
    reviewStarted,
    /** Fin d'une revue : nouvelles remarques, historique et panneau à jour. */
    reviewDone({ outcomes }) {
      for (const id of Object.keys(outcomes)) reviewing.delete(id);
      loadReviews();
    },
    remarksArrived(items) {
      for (const remark of items) remarks.set(remark.id, remark);
      renderList();
    },
    /** `remark_update` d'une remarque de projet (relance après une réponse). */
    remarkUpdated(remark) {
      pending.delete(remark.id);
      apply(remark);
    },
  };
}
