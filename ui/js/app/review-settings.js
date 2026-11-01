/* Réglages · Revues du projet (phase 9e, maquette 09-5) : déclencheurs des revues
   automatiques, rythme (calme, intervalle, plafond quotidien), plafonds de connaissance du
   projet. Changements en attente jusqu'à « Enregistrer » ; `config.toml` est réécrit en
   gardant ses commentaires, et les réglages s'appliquent sans redémarrage. */

const TRIGGERS = [
  ['create', 'À la création du projet', 'Après hodegos new : une première lecture de la vision et de ce qui existe.'],
  ['open', 'À l\'ouverture du projet', 'Seulement si des fichiers ont changé depuis la dernière revue (commit, autre éditeur, terminal).'],
  ['change', 'Après des modifications', 'La carte est recalculée à chaque enregistrement ; la revue part quand l\'édition est au calme.'],
  ['commit', 'Après un commit', 'En plus des modifications : utile si vous commitez souvent depuis le terminal.'],
];

/** Bornes, comme celles du serveur ; calme et intervalle en minutes, décimales permises. */
const FIELDS = {
  calm: { min: 0, max: 60, label: 'Le calme', unit: 'minutes', decimal: true },
  interval: { min: 0, max: 1440, label: 'L\'intervalle', unit: 'minutes', decimal: true },
  daily: { min: 0, max: 200, label: 'Le nombre de revues par jour', unit: '' },
  map: { min: 1000, max: 32000, label: 'Le plafond de la carte', unit: 'jetons' },
  neighborhood: { min: 500, max: 16000, label: 'Le plafond du voisinage', unit: 'jetons' },
  keys: { min: 2000, max: 32000, label: 'Le plafond des fichiers clés', unit: 'jetons' },
};

const money = new Intl.NumberFormat('fr-FR', { style: 'currency', currency: 'USD', maximumFractionDigits: 2 });

function element(tag, className, text) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

/** `page` : #page-revues ; `request(type, payload)`. */
export function createReviewSettings(page, { request }) {
  const $ = (id) => page.querySelector(`#${id}`);
  const ui = {
    enabled: $('revues-actives'),
    triggers: $('revues-declencheurs'),
    inputs: {
      calm: $('revues-calme'),
      interval: $('revues-intervalle'),
      daily: $('revues-jour'),
      map: $('revues-carte'),
      neighborhood: $('revues-voisinage'),
      keys: $('revues-cles'),
    },
    spend: $('revues-depense'),
    error: $('revues-erreur'),
    pending: $('revues-attente'),
    cancel: $('revues-annuler'),
    save: $('revues-enregistrer'),
  };
  /** Derniers réglages reçus. */
  let saved = null;
  const boxes = new Map();

  for (const [value, title, note] of TRIGGERS) {
    const label = element('label', 'app-reviews__trigger');
    const box = element('input');
    box.type = 'checkbox';
    box.value = value;
    box.addEventListener('change', render);
    boxes.set(value, box);
    const text = element('span', 'app-reviews__trigger-text');
    text.append(element('strong', '', title), element('span', 'app-settings__note', note));
    label.append(box, text);
    ui.triggers.append(label);
  }

  /** Valeurs saisies, en unités du serveur ; `errors` : champ → message. */
  function read() {
    const errors = {};
    const value = (key) => {
      const raw = ui.inputs[key].value.trim().replace(',', '.');
      const number = Number(raw);
      const field = FIELDS[key];
      const whole = field.decimal ? Number.isFinite(number) : Number.isInteger(number);
      if (raw === '' || !whole || number < field.min || number > field.max) {
        errors[key] = `${field.label} va de ${field.min} à ${field.max}${field.unit ? ` ${field.unit}` : ''}.`;
      }
      return number;
    };
    const payload = {
      project_review: {
        enabled: ui.enabled.checked,
        triggers: TRIGGERS.map(([v]) => v).filter((v) => boxes.get(v).checked),
        calm_secs: Math.round(value('calm') * 60),
        min_interval_secs: Math.round(value('interval') * 60),
        daily_max: value('daily'),
      },
      context: {
        map_cap: value('map'),
        neighborhood_cap: value('neighborhood'),
        key_files_cap: value('keys'),
      },
    };
    return { payload, errors };
  }

  function changes() {
    if (!saved) return 0;
    const { payload } = read();
    const a = payload.project_review;
    const b = saved.project_review;
    let count = 0;
    if (a.enabled !== b.enabled) count += 1;
    if (a.triggers.join() !== [...b.triggers].sort((x, y) => TRIGGERS.findIndex(([v]) => v === x) - TRIGGERS.findIndex(([v]) => v === y)).join()) count += 1;
    if (a.calm_secs !== b.calm_secs) count += 1;
    if (a.min_interval_secs !== b.min_interval_secs) count += 1;
    if (a.daily_max !== b.daily_max) count += 1;
    for (const key of ['map_cap', 'neighborhood_cap', 'key_files_cap']) {
      if (payload.context[key] !== saved.context[key]) count += 1;
    }
    return count;
  }

  function render() {
    const { errors } = read();
    for (const [key] of Object.entries(FIELDS)) {
      const note = page.querySelector(`[data-error="${key}"]`);
      note.textContent = errors[key] ?? '';
      note.classList.toggle('app-settings__note--error', Boolean(errors[key]));
      ui.inputs[key].setAttribute('aria-invalid', String(Boolean(errors[key])));
    }
    for (const box of boxes.values()) box.disabled = !ui.enabled.checked;
    const count = changes();
    ui.pending.textContent = count ? `${count} changement${count > 1 ? 's' : ''} en attente` : '';
    ui.cancel.disabled = count === 0;
    ui.save.disabled = count === 0 || Object.keys(errors).length > 0 || !saved?.editable;
  }

  function reset() {
    if (!saved) return;
    const review = saved.project_review;
    ui.enabled.checked = review.enabled;
    for (const [value, box] of boxes) box.checked = review.triggers.includes(value);
    // Minutes avec leurs décimales : 5 s ne s'affiche pas « 0 » (et ne compte pas comme changé).
    const minutes = (secs) => String(Math.round((secs / 60) * 100) / 100);
    ui.inputs.calm.value = minutes(review.calm_secs);
    ui.inputs.interval.value = minutes(review.min_interval_secs);
    ui.inputs.daily.value = String(review.daily_max);
    ui.inputs.map.value = String(saved.context.map_cap);
    ui.inputs.neighborhood.value = String(saved.context.neighborhood_cap);
    ui.inputs.keys.value = String(saved.context.key_files_cap);
    const { reviews, cost } = saved.auto_spend;
    ui.spend.textContent = `Ce mois-ci, pour ce projet : ${reviews} revue${reviews > 1 ? 's' : ''} automatique${reviews > 1 ? 's' : ''}, ${money.format(cost)}.`;
    ui.error.hidden = true;
    render();
  }

  async function save() {
    const { payload, errors } = read();
    if (Object.keys(errors).length) return;
    ui.save.disabled = true;
    try {
      const reply = await request('project_review_set', payload);
      saved = reply.payload;
      reset();
    } catch (error) {
      ui.error.textContent = error.message;
      ui.error.hidden = false;
      render();
    }
  }

  for (const input of [ui.enabled, ...Object.values(ui.inputs)]) input.addEventListener('input', render);
  ui.enabled.addEventListener('change', render);
  ui.cancel.addEventListener('click', reset);
  ui.save.addEventListener('click', save);

  return {
    /** Réglages reçus ; une saisie en cours n'est pas remplacée. */
    apply(settings) {
      const pending = changes();
      saved = settings;
      if (pending === 0) reset();
      else render();
    },
  };
}
