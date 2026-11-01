/* Réglages personnels (phase 8d, maquette 08-4) : modèles et effort par niveau, clé d'API,
   budget. Les changements restent en attente jusqu'à « Enregistrer » ; un niveau qui
   change est réévalué après confirmation du coût estimé (ADR 0007). La clé n'est jamais
   renvoyée par le serveur : seuls son origine et ses quatre derniers caractères le sont. */
import { chooseDialog } from './dialog.js';

const LEVELS = {
  fast: { name: 'niveau rapide', title: 'Niveau rapide' },
  strong: { name: 'niveau fort', title: 'Niveau fort' },
};

const EFFORTS = { default: 'Par défaut', low: 'Faible', medium: 'Moyen', high: 'Élevé' };

const BASIS = {
  same: 'le dernier passage de ce réglage',
  tokens: 'le prix du modèle et les jetons du dernier passage du niveau',
  prompts: 'le prix du modèle et la taille des prompts des cas (aucun passage mesuré)',
};

const money = new Intl.NumberFormat('fr-FR', { style: 'currency', currency: 'USD', maximumFractionDigits: 3 });
const percent = new Intl.NumberFormat('fr-FR', { style: 'percent', maximumFractionDigits: 0 });
const day = new Intl.DateTimeFormat('fr-FR', { day: 'numeric', month: 'long' });
const month = new Intl.DateTimeFormat('fr-FR', { month: 'long', year: 'numeric', timeZone: 'UTC' });

function element(tag, className, text) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

function mono(text) {
  return element('span', 'hd-mono', text);
}

function seconds(ms) {
  return `${(ms / 1000).toLocaleString('fr-FR', { maximumFractionDigits: 1 })} s`;
}

function duration(ms) {
  if (ms < 90_000) return seconds(ms);
  return `${Math.round(ms / 60_000)} min`;
}

function date(text) {
  const parsed = new Date(`${text}T12:00:00Z`);
  return Number.isNaN(parsed.getTime()) ? text : day.format(parsed);
}

function capitalize(text) {
  return text.charAt(0).toUpperCase() + text.slice(1);
}

/** « l'évaluation du 30 octobre », « l'évaluation livrée du 30 octobre (ADR 0003) » */
function evalLabel(result) {
  if (result.origin === 'builtin') return `l'évaluation livrée du ${date(result.date)} (ADR 0003)`;
  return `l'évaluation du ${date(result.date)}${result.partial ? ' (interrompue)' : ''}`;
}

/** « L'évaluation du 30 octobre : notion 15 cas sur 15, aucune solution divulguée, 2,9 s en médiane. » */
export function evalSummary(result) {
  const leaks = result.divulged === 0 ? 'aucune solution divulguée'
    : result.divulged === 1 ? '1 solution divulguée' : `${result.divulged} solutions divulguées`;
  const parts = [`notion ${result.notion} cas sur ${result.cases}`, leaks];
  if (result.median_ms) parts.push(`${seconds(result.median_ms)} en médiane`);
  return `${capitalize(evalLabel(result))} : ${parts.join(', ')}.`;
}

/**
 * Écran des réglages dans `root`. `request(type, payload)` envoie une demande au serveur ;
 * `onSettings(settings)` suit les réglages reçus (barre d'état) ; `mentors` est la page
 * des mentors (`show()`, `hide()`), dans le même écran.
 */
export function createSettings(root, { request, onSettings, mentors, reviews }) {
  const $ = (id) => root.querySelector(`#${id}`);
  const levels = Object.fromEntries(Object.keys(LEVELS).map((key) => [key, {
    card: $(`niveau-${key}`),
    model: $(`modele-${key}`),
    effort: $(`effort-${key}`),
    badge: $(`niveau-${key}`).querySelector('[data-role="badge"]'),
    note: $(`niveau-${key}`).querySelector('[data-role="note"]'),
    callout: $(`niveau-${key}`).querySelector('[data-role="callout"]'),
  }]));
  const ui = {
    where: $('reglages-ou'),
    navKey: $('nav-cle'),
    navBudget: $('nav-budget'),
    key: $('cle'),
    keyBadge: $('cle-badge'),
    keyNote: $('cle-note'),
    keyReplace: $('cle-remplacer'),
    keySave: $('cle-enregistrer'),
    keyTest: $('cle-tester'),
    cap: $('plafond'),
    warn: $('seuil'),
    month: $('budget-mois'),
    spent: $('depense'),
    bar: $('depense-barre'),
    error: $('reglages-erreur'),
    pending: $('reglages-attente'),
    cancel: $('reglages-annuler'),
    save: $('reglages-enregistrer'),
  };

  /** Derniers réglages reçus du serveur. */
  let saved = null;
  /** Fiches des modèles vérifiés : `{ state: 'checking' | 'ok' | 'error', info, message }`. */
  const models = new Map();
  /** Progression de l'évaluation en cours : `{ level, done, total, cost }`. */
  let progress = null;
  let editingKey = false;
  let keyTest = null;
  let saving = false;

  function draft() {
    return {
      fast: levels.fast.model.value.trim(),
      strong: levels.strong.model.value.trim(),
      fast_effort: levels.fast.effort.value,
      strong_effort: levels.strong.effort.value,
      monthly_cap_usd: Number(ui.cap.value),
      warn_at: Number(ui.warn.value) / 100,
    };
  }

  function changedLevels() {
    if (!saved) return [];
    const now = draft();
    return Object.keys(LEVELS).filter((key) => now[key] !== (saved[key] ?? '') || now[`${key}_effort`] !== saved[`${key}_effort`]);
  }

  function changeCount() {
    if (!saved) return 0;
    const now = draft();
    const fields = [
      now.fast !== (saved.fast ?? ''),
      now.fast_effort !== saved.fast_effort,
      now.strong !== (saved.strong ?? ''),
      now.strong_effort !== saved.strong_effort,
      Math.abs(now.monthly_cap_usd - saved.budget.monthly_cap_usd) > 1e-9,
      Math.abs(now.warn_at - saved.budget.warn_at) > 1e-9,
    ];
    return fields.filter(Boolean).length;
  }

  /** Remet les champs aux valeurs lues. */
  function reset() {
    if (!saved) return;
    for (const key of Object.keys(LEVELS)) {
      levels[key].model.value = saved[key] ?? '';
      levels[key].effort.value = saved[`${key}_effort`];
      check(key);
    }
    ui.cap.value = String(saved.budget.monthly_cap_usd);
    ui.warn.value = String(Math.round(saved.budget.warn_at * 100));
    showError('');
    render();
  }

  async function check(key) {
    const id = levels[key].model.value.trim();
    if (!id || models.get(id)?.state === 'ok' || models.get(id)?.state === 'checking') {
      render();
      return;
    }
    models.set(id, { state: 'checking' });
    render();
    try {
      const reply = await request('model_check', { id });
      models.set(id, { state: 'ok', info: reply.payload });
    } catch (error) {
      models.set(id, { state: 'error', message: error.message });
    }
    render();
  }

  function render() {
    if (!saved) return;
    for (const key of Object.keys(LEVELS)) renderLevel(key);
    renderKey();
    renderBudget();
    const count = changeCount();
    ui.pending.textContent = count === 0 ? '' : count === 1 ? '1 changement non enregistré' : `${count} changements non enregistrés`;
    ui.cancel.disabled = count === 0 || saving;
    ui.save.disabled = count === 0 || saving || !saved.editable;
    ui.save.textContent = changedLevels().length ? 'Enregistrer et évaluer…' : 'Enregistrer';
    ui.where.replaceChildren(
      mono(saved.config_dir ? `${saved.config_dir}/config.toml` : '~/.config/hodegos/config.toml'),
      ' et ',
      mono('credentials.toml'),
      ', jamais dans le projet.',
    );
  }

  function setBadge(badge, text, variant) {
    badge.textContent = text;
    badge.className = `hd-badge hd-card__end${variant ? ` hd-badge--${variant}` : ''}`;
  }

  function renderLevel(key) {
    const level = levels[key];
    const model = level.model.value.trim();
    const effort = level.effort.value;
    const before = { model: saved[key] ?? '', effort: saved[`${key}_effort`] };
    const changed = model !== before.model || effort !== before.effort;
    const checked = models.get(model);
    const info = checked?.state === 'ok' ? checked.info : null;
    const last = saved.last_eval[key];
    const running = saved.eval_running === key;

    // L'effort n'est proposé que si le modèle accepte `reasoning`.
    const noEffort = info && info.exists && !info.reasoning;
    level.effort.disabled = Boolean(noEffort) || !saved.editable;
    if (noEffort && effort !== 'default') level.effort.value = 'default';
    level.model.readOnly = !saved.editable;

    level.card.classList.toggle('hd-card--active', changed);
    if (running) setBadge(level.badge, 'Évaluation en cours', 'info');
    else if (changed) setBadge(level.badge, 'Modifié', 'warning');
    else if (!last) setBadge(level.badge, 'Jamais évalué', '');
    else if (last.passed) setBadge(level.badge, 'Évalué', 'success');
    else setBadge(level.badge, 'Sous le seuil', 'warning');

    const parts = [];
    let refused = false;
    if (!model) {
      parts.push('Renseignez l\'identifiant OpenRouter du modèle.');
      refused = true;
    } else if (checked?.state === 'checking') {
      parts.push('Vérification du modèle…');
    } else if (checked?.state === 'error') {
      parts.push(`Catalogue injoignable : ${checked.message}`);
    } else if (info && !info.exists) {
      parts.push('Modèle inconnu d\'OpenRouter.');
      refused = true;
    } else if (info && !info.structured_outputs) {
      parts.push('Ce modèle n\'accepte pas les sorties structurées : Hodegos ne peut pas valider ses remarques.');
      refused = true;
    } else if (info) {
      parts.push('Sorties structurées acceptées.');
    }
    if (!changed && last) parts.push(evalSummary(last));
    else if (!changed && !last && model) parts.push('Ce réglage n\'a jamais été évalué.');
    if (noEffort) parts.push('Ce modèle ne propose pas de réglage d\'effort.');
    level.note.textContent = parts.join(' ');
    level.note.classList.toggle('app-settings__note--error', refused);
    level.model.setAttribute('aria-invalid', String(refused));

    level.callout.replaceChildren();
    if (running && progress?.level === key) {
      level.callout.append(callout('info', 'Évaluation en cours', [
        `${progress.done} cas sur ${progress.total} · ${money.format(progress.cost)}. Les passes continuent avec le nouveau réglage.`,
      ]));
    } else if (changed) {
      const title = model !== before.model
        ? `Modèle changé : de « ${before.model || 'aucun'} » à « ${model} »`
        : `Effort changé : de « ${EFFORTS[before.effort]} » à « ${EFFORTS[effort]} »`;
      const lost = last ? `${capitalize(evalLabel(last))} ne vaut plus pour ce réglage : ` : '';
      level.callout.append(callout('warning', title, [
        `${lost}« Enregistrer » lance l'évaluation du ${LEVELS[key].name}, après confirmation du coût et de la durée estimés (ADR 0007). Un réglage déjà évalué et retenu ne l'est pas de nouveau.`,
      ]));
    } else if (last && !last.passed && !running) {
      const actions = [];
      if (last.previous && saved.editable) {
        const revert = element('button', 'hd-btn hd-btn--secondary hd-btn--sm', 'Revenir à l\'ancien réglage');
        revert.type = 'button';
        revert.title = `${last.previous.model} · ${EFFORTS[last.previous.effort]}`;
        revert.addEventListener('click', () => {
          level.model.value = last.previous.model;
          level.effort.value = last.previous.effort;
          save();
        });
        actions.push(revert);
      }
      level.callout.append(callout('warning', 'Évaluation sous le seuil', [
        'Le réglage reste appliqué, mais il ne remplit pas les critères de l\'ADR 0003 : aucune solution divulguée et la notion dans 70 % des cas au moins.',
      ], actions));
    }
  }

  function callout(variant, title, paragraphs, actions = []) {
    const box = element('div', `hd-callout${variant === 'warning' ? ' hd-callout--warning' : ''}`);
    box.append(element('div', 'hd-callout__title', title));
    for (const text of paragraphs) box.append(element('p', '', text));
    if (actions.length) {
      const row = element('div', 'app-settings__callout-actions');
      row.append(...actions);
      box.append(row);
    }
    return box;
  }

  function renderKey() {
    const source = saved.key_source;
    ui.key.readOnly = !editingKey;
    if (!editingKey) {
      ui.key.value = '';
      ui.key.placeholder = saved.key_hint ? `••••••••${saved.key_hint}` : 'Aucune clé';
    }
    ui.keyReplace.hidden = editingKey;
    ui.keySave.hidden = !editingKey;
    ui.keyReplace.disabled = source === 'env' || !saved.editable;
    ui.keyReplace.textContent = source === 'none' ? 'Ajouter une clé' : 'Remplacer la clé';
    ui.keyTest.disabled = source === 'none';

    const note = [];
    if (source === 'env') {
      note.push('Lue dans ', mono('OPENROUTER_API_KEY'), `, qui finit par …${saved.key_hint} et prime sur `, mono('credentials.toml'), ' : modifiez la variable pour changer de clé.');
    } else if (source === 'file') {
      note.push('Finit par ', mono(`…${saved.key_hint}`), ', lue dans ', mono('credentials.toml'), ' (droits 600).');
    } else {
      note.push('Aucune clé : les mentors ne répondront pas tant qu\'elle manque.');
    }
    note.push(' La clé n\'est jamais renvoyée au navigateur.');
    ui.keyNote.replaceChildren(...note);

    ui.keyBadge.hidden = !keyTest;
    if (keyTest) {
      ui.keyBadge.textContent = keyTest.ok ? 'Connexion réussie' : 'Connexion refusée';
      ui.keyBadge.className = `hd-badge hd-card__end hd-badge--${keyTest.ok ? 'success' : 'danger'}`;
      ui.keyBadge.title = keyTest.message;
    }
    ui.navKey.textContent = source === 'env' ? 'variable' : source === 'file' ? `…${saved.key_hint}` : 'absente';
  }

  function renderBudget() {
    const { used_usd: used } = saved.budget;
    const cap = Number(ui.cap.value) || saved.budget.monthly_cap_usd;
    const share = cap > 0 ? used / cap : 1;
    ui.month.textContent = `${month.format(new Date())} (UTC)`;
    ui.spent.textContent = `Dépensé : ${money.format(used)} (${percent.format(share)})`;
    ui.bar.setAttribute('aria-label', `Budget utilisé : ${percent.format(share)}`);
    ui.bar.firstElementChild.style.width = `${Math.min(100, share * 100)}%`;
    ui.navBudget.textContent = percent.format(share);
    for (const input of [ui.cap, ui.warn]) input.readOnly = !saved.editable;
  }

  function showError(text) {
    ui.error.hidden = !text;
    ui.error.textContent = text;
  }

  async function save(confirm = false) {
    if (saving) return;
    const now = draft();
    if (!(now.monthly_cap_usd >= 0) || !(now.warn_at >= 0 && now.warn_at <= 1)) {
      showError('Budget : plafond positif, seuil entre 0 et 100 %.');
      return;
    }
    saving = true;
    render();
    let reply;
    try {
      reply = await request('settings_set', {
        fast: now.fast,
        strong: now.strong,
        fast_effort: now.fast_effort,
        strong_effort: now.strong_effort,
        budget: { monthly_cap_usd: now.monthly_cap_usd, warn_at: now.warn_at },
        confirm_eval: confirm,
      });
    } catch (error) {
      saving = false;
      showError(error.message);
      render();
      return;
    }
    saving = false;
    if (reply.type === 'eval_required') {
      render();
      if (await confirmEval(reply.payload)) await save(true);
      return;
    }
    showError('');
    apply(reply.payload, true);
  }

  /** Dialogue avant la dépense : niveau, cas, coût et durée estimés, plafond. */
  async function confirmEval(estimate) {
    const level = LEVELS[estimate.level];
    const text = element('p', '', `Le nouveau réglage du ${level.name} s'applique tout de suite et sera évalué sur ${estimate.cases} cas intégrés. Les passes continuent pendant l'évaluation ; un résultat sous le seuil vous proposera de revenir à l'ancien réglage.`);
    const costs = element('p', '', `Coût estimé : ${money.format(estimate.estimated_cost)} · durée estimée : ${duration(estimate.estimated_duration_ms)}. Plafond de l'évaluation : ${money.format(estimate.max_cost)}. La dépense compte dans le budget du mois.`);
    const basis = element('p', 'hd-muted', `Estimation d'après ${BASIS[estimate.basis]}.`);
    const choice = await chooseDialog({
      title: `Évaluer le ${level.name} ?`,
      body: [text, costs, basis],
      choices: [
        { value: 'cancel', label: 'Annuler', variant: 'secondary' },
        { value: 'confirm', label: 'Enregistrer et évaluer', variant: 'primary' },
      ],
      focus: 'confirm',
      returnFocus: ui.save,
    });
    return choice === 'confirm';
  }

  /** Nouveaux réglages du serveur ; `own` : réponse à notre enregistrement. */
  function apply(settings, own = false) {
    const pending = saved && !own ? changeCount() : 0;
    saved = settings;
    if (!saved.eval_running) progress = null;
    onSettings(saved);
    reviews.apply(saved);
    // Un changement diffusé par le serveur ne remplace pas une saisie en cours.
    if (pending === 0) reset();
    else render();
  }

  ui.keyReplace.addEventListener('click', () => {
    editingKey = true;
    keyTest = null;
    render();
    ui.key.focus();
  });
  ui.keySave.addEventListener('click', async () => {
    const key = ui.key.value.trim();
    if (!key) {
      editingKey = false;
      render();
      return;
    }
    try {
      const reply = await request('api_key_set', { key });
      editingKey = false;
      ui.key.value = '';
      showError('');
      apply(reply.payload);
    } catch (error) {
      showError(error.message);
    }
  });
  ui.key.addEventListener('keydown', (event) => {
    if (event.key === 'Enter' && editingKey) ui.keySave.click();
    if (event.key === 'Escape' && editingKey) {
      editingKey = false;
      render();
    }
  });
  ui.keyTest.addEventListener('click', async () => {
    try {
      keyTest = (await request('provider_test', null)).payload;
    } catch (error) {
      keyTest = { ok: false, message: error.message };
    }
    render();
  });
  for (const key of Object.keys(LEVELS)) {
    levels[key].model.addEventListener('input', render);
    levels[key].model.addEventListener('change', () => check(key));
    levels[key].effort.addEventListener('change', render);
  }
  for (const input of [ui.cap, ui.warn]) input.addEventListener('input', render);
  ui.cancel.addEventListener('click', reset);
  ui.save.addEventListener('click', () => save());
  const pages = { modeles: $('page-modeles'), revues: $('page-revues') };
  for (const link of root.querySelectorAll('.app-settings__nav a[href^="#"]')) {
    link.addEventListener('click', (event) => {
      event.preventDefault();
      for (const other of root.querySelectorAll('.app-settings__nav a')) other.removeAttribute('aria-current');
      link.setAttribute('aria-current', 'page');
      const page = link.dataset.page;
      for (const [name, node] of Object.entries(pages)) node.hidden = name !== page;
      if (page === 'mentors') {
        mentors.show();
        return;
      }
      mentors.hide();
      root.querySelector(link.getAttribute('href'))?.scrollIntoView({ block: 'start' });
    });
  }

  return {
    /** Relit les réglages (à l'ouverture de l'écran). */
    async load() {
      try {
        apply((await request('settings_get', null)).payload);
      } catch (error) {
        showError(error.message);
      }
    },
    apply,
    progress(payload) {
      progress = payload;
      render();
    },
    /** Changements non enregistrés. */
    get pending() {
      return changeCount();
    },
  };
}
