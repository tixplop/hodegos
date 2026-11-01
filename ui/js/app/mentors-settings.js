/* Réglages des mentors (phase 8e, maquette 08-5 ; portées en phase 9e, maquette 09-4) :
   liste, fiche en formulaire ou en fichier brut, personnaliser, créer, dupliquer,
   désactiver, revenir au niveau inférieur, prompt assemblé. Le serveur valide avec les règles du chargeur et renvoie les erreurs
   champ par champ ; un mentor livré n'est jamais écrit. */
import { confirmDialog } from './dialog.js';
import { createTextEditor } from './editor.js';

const ORIGINS = { builtin: 'Livré', user: 'Personnel', project: 'Dépôt' };
const ORIGIN_OF = { builtin: 'livré', user: 'personnel', project: 'du dépôt' };
const MODES = { proactive: 'Proactif', 'strong-signal': 'Signal fort', 'on-demand': 'À la demande' };
const MODELS = { fast: 'Rapide', strong: 'Fort' };
const LEVELS = { indice: 'Indice', question: 'Question', piste: 'Piste', exemple: 'Exemple' };
const TRIGGERS = { paths: 'Chemins', languages: 'Langages', headings: 'Titres', keywords: 'Mots-clés' };
const CONTEXTS = { section: 'Section', neighborhood: 'Voisinage', project: 'Projet' };
const CONTEXT_NOTES = {
  section: 'Section = la section, son diff et 20 lignes autour.',
  neighborhood: 'Voisinage = la section, plus les fichiers importés, ceux qui l\'importent, son test et les documents qui le citent.',
  project: 'Projet = la section, son voisinage et la carte du projet.',
};
const REVIEW_TRIGGERS = {
  create: 'À la création du projet',
  open: 'À l\'ouverture, si des fichiers ont changé depuis la dernière revue',
  change: 'Après des modifications, une fois l\'édition au calme',
  commit: 'Après un commit',
};

/** Champs affichés dans le formulaire ; les autres erreurs vont dans l'encadré. */
const FORM_FIELDS = ['id', 'name', 'initials', 'role', 'mode', 'priority', 'model', 'entry_level', 'max_remarks', 'scopes', 'context'];

const EMPTY_FORM = {
  id: '',
  name: '',
  initials: '',
  role: '',
  enabled: true,
  mode: 'proactive',
  model: 'fast',
  priority: 50,
  entry_level: 'indice',
  max_remarks: 2,
  on_open: false,
  context: 'section',
  scopes: ['section'],
  project: { key_files: [], max_remarks: 3, triggers: ['create', 'open', 'change'] },
  triggers: { paths: [], languages: [], headings: [], keywords: [] },
};

let ids = 0;

function element(tag, className, text) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

function button(label, variant, onClick) {
  const node = element('button', `hd-btn hd-btn--${variant} hd-btn--sm`, label);
  node.type = 'button';
  node.addEventListener('click', onClick);
  return node;
}

function mono(text) {
  return element('span', 'hd-mono', text);
}

function minimalOverride(id) {
  return `---\nid: ${id}\nenabled: false\n---\n`;
}

/**
 * Page des mentors : liste dans `listRoot`, fiche dans `ficheRoot`. `request` envoie une
 * demande au serveur ; `onCount(count)` suit le nombre de mentors (navigation).
 */
export function createMentorsSettings(listRoot, ficheRoot, { request, onCount }) {
  const items = listRoot.querySelector('#mentors-items');
  const newButton = listRoot.querySelector('#mentor-nouveau');

  let list = { items: [], errors: [] };
  /** Mentor affiché : `{ id, origin }`, ou `{ creating: true }`. */
  let selected = null;
  /** `mentor_file` du mentor affiché (absent en création). */
  let file = null;
  /** Saisie du formulaire, posture comprise ; `raw` : fichier brut en cours. */
  let form = null;
  let creating = null;
  let rawMode = false;
  let dirty = false;
  let errors = [];
  let message = '';
  let prompt = null;
  let shown = false;
  /** Opération en cours (enregistrer, personnaliser, supprimer) : elle rouvre la fiche
      elle-même, le rechargement à chaud qu'elle provoque ne doit pas la devancer. */
  let busy = 0;

  async function operation(work) {
    busy += 1;
    try {
      await work();
    } finally {
      busy -= 1;
    }
  }

  const posture = element('div', 'app-mentors__editor');
  const editor = createTextEditor(posture, {
    label: 'Posture du mentor',
    onChange: () => {
      dirty = true;
      renderFooter();
    },
  });

  async function loadList() {
    try {
      list = (await request('mentors_list', null)).payload;
    } catch (error) {
      message = error.message;
    }
    onCount(list.items.length);
    renderList();
  }

  function describe(item) {
    const parts = [ORIGINS[item.origin]];
    if (item.overrides.length) parts[0] += `, remplace le ${ORIGIN_OF[item.overrides.at(-1)]}`;
    if (item.mode) parts.push(MODES[item.mode].toLowerCase());
    if (item.model) parts.push(MODELS[item.model].toLowerCase());
    if (item.on_open) parts.push('à l\'ouverture');
    if (item.scopes?.includes('section') && item.context && item.context !== 'section') {
      parts.push(`contexte ${CONTEXTS[item.context].toLowerCase()}`);
    }
    if (item.scopes?.length) {
      parts.unshift(item.scopes.map((s) => (s === 'section' ? 'Section' : 'Projet')).join(' + '));
    }
    return parts.join(' · ');
  }

  function renderList() {
    const rows = list.items.map((item) => row({
      key: { id: item.id, origin: item.origin },
      initials: item.initials || item.id.slice(0, 2).toUpperCase(),
      title: item.name,
      badge: item.enabled ? ['Actif', 'success'] : ['Désactivé', ''],
      meta: describe(item),
    }));
    for (const error of list.errors) {
      rows.push(row({
        key: { id: error.id, origin: error.origin },
        initials: error.id.slice(0, 2).toUpperCase(),
        title: error.id,
        badge: ['En erreur', 'danger'],
        meta: `${ORIGINS[error.origin]} · ${error.path}`,
        error: error.errors.map((e) => e.message).join(' ; '),
      }));
    }
    items.replaceChildren(...rows);
  }

  function row({ key, initials, title, badge, meta, error }) {
    const li = element('li');
    const card = element('button', 'hd-card app-mentors__item');
    card.type = 'button';
    const current = selected && selected.id === key.id && selected.origin === key.origin;
    if (current) {
      card.classList.add('hd-card--active');
      card.setAttribute('aria-current', 'true');
    }
    if (error) card.classList.add('app-mentors__item--error');
    const head = element('span', 'hd-row app-mentors__item-head');
    const avatar = element('span', `hd-avatar hd-avatar--sm${error ? '' : ' hd-avatar--mentor'}`, initials);
    avatar.setAttribute('aria-hidden', 'true');
    const status = element('span', `hd-badge app-mentors__badge${badge[1] ? ` hd-badge--${badge[1]}` : ''}`, badge[0]);
    head.append(avatar, element('strong', '', title), status);
    card.append(head, element('span', 'app-mentors__meta', meta));
    if (error) card.append(element('span', 'app-mentors__error', error));
    card.addEventListener('click', () => open(key));
    li.append(card);
    return li;
  }

  async function guard() {
    if (!dirty) return true;
    return confirmDialog({
      title: 'Abandonner les changements ?',
      body: [element('p', '', 'Les changements de ce mentor ne sont pas enregistrés.')],
      confirmLabel: 'Abandonner',
    });
  }

  async function open(key, { force = false } = {}) {
    if (!force && !(await guard())) return;
    try {
      file = (await request('mentor_get', key)).payload;
    } catch (error) {
      message = error.message;
      renderFiche();
      return;
    }
    selected = key;
    creating = null;
    form = file.form ? structuredClone(file.form) : null;
    rawMode = !file.form && file.editable && file.errors.length > 0;
    errors = file.errors;
    dirty = false;
    prompt = null;
    message = '';
    editor.set(rawMode ? file.content : (file.body ?? file.content), { readOnly: !file.editable, language: 'markdown' });
    renderList();
    renderFiche();
  }

  /** Création, vide ou par duplication de `source` (formulaire et posture). */
  async function create(source = null) {
    if (!(await guard())) return;
    const base = source ? structuredClone(source.form) : structuredClone(EMPTY_FORM);
    if (source) {
      base.id = `${source.form.id}-copie`;
      base.name = `${source.form.name} (copie)`;
      base.enabled = true;
    }
    selected = null;
    file = null;
    creating = { origin: source?.origin === 'project' ? 'project' : 'user' };
    form = base;
    rawMode = false;
    errors = [];
    dirty = true;
    prompt = null;
    message = '';
    editor.set(source ? source.body : '', { language: 'markdown' });
    renderList();
    renderFiche();
    ficheRoot.querySelector('#mentor-id')?.focus();
  }

  /* ---------- Fiche ---------- */

  function errorFor(field) {
    return errors.filter((e) => e.field === field || e.field.startsWith(`${field}[`));
  }

  function field(label, control, name, wide = false) {
    const box = element('div', `app-settings__field app-mentors__field${wide ? ' app-settings__field--wide' : ''}`);
    const id = `mentor-${name.replace(/[^a-z_]/g, '-')}`;
    control.id = id;
    const caption = element('label', 'hd-label', label);
    caption.htmlFor = id;
    box.append(caption, control);
    const own = errorFor(name);
    if (own.length) {
      const note = element('span', 'app-mentors__field-error', own.map((e) => e.message).join(' ; '));
      note.id = `${id}-erreur`;
      control.setAttribute('aria-invalid', 'true');
      control.setAttribute('aria-describedby', note.id);
      box.append(note);
    }
    return box;
  }

  function input(name, type = 'text') {
    const node = element('input', 'hd-input');
    node.type = type;
    node.value = String(form[name]);
    node.readOnly = !editable();
    node.addEventListener('input', () => {
      form[name] = type === 'number' ? Number(node.value) : node.value;
      dirty = true;
      renderFooter();
    });
    return node;
  }

  function select(name, options, cast = String) {
    const node = element('select', 'hd-input');
    for (const [value, label] of Object.entries(options)) {
      const option = element('option', '', label);
      option.value = value;
      node.append(option);
    }
    node.value = String(form[name]);
    node.disabled = !editable();
    node.addEventListener('change', () => {
      form[name] = cast(node.value);
      dirty = true;
      renderFooter();
    });
    return node;
  }

  function editable() {
    return Boolean(creating) || Boolean(file?.editable);
  }

  /** Liste modifiable d'un déclencheur : étiquettes retirables et champ « Ajouter ». */
  function tagList(key) {
    const box = element('div', 'hd-row app-mentors__tags');
    const label = element('span', 'hd-label app-mentors__tags-label', TRIGGERS[key]);
    label.id = `mentor-tags-${key}-${++ids}`;
    box.setAttribute('role', 'group');
    box.setAttribute('aria-labelledby', label.id);
    box.append(label);
    const values = form.triggers[key];
    if (!values.length) box.append(element('span', 'hd-muted app-mentors__none', 'Aucun'));
    values.forEach((value, index) => {
      const invalid = errors.some((e) => e.field === `triggers.${key}[${index}]`);
      const tag = element('span', `hd-id app-mentors__tag${invalid ? ' app-mentors__tag--error' : ''}`, value);
      if (editable()) {
        const remove = element('button', 'app-mentors__remove', '×');
        remove.type = 'button';
        remove.setAttribute('aria-label', `Retirer ${value}`);
        remove.addEventListener('click', () => {
          values.splice(index, 1);
          dirty = true;
          renderFiche();
        });
        tag.append(remove);
      }
      box.append(tag);
    });
    if (editable()) {
      const add = element('input', 'hd-input app-mentors__add');
      add.type = 'text';
      add.setAttribute('aria-label', `Ajouter : ${TRIGGERS[key].toLowerCase()}`);
      const commit = () => {
        const value = add.value.trim();
        if (!value) return;
        values.push(value);
        dirty = true;
        renderFiche();
        ficheRoot.querySelector(`[aria-label="${add.getAttribute('aria-label')}"]`)?.focus();
      };
      add.addEventListener('keydown', (event) => {
        if (event.key === 'Enter') {
          event.preventDefault();
          commit();
        }
      });
      box.append(add, button('Ajouter', 'ghost', commit));
    }
    for (const e of errors.filter((x) => x.field.startsWith(`triggers.${key}`))) {
      box.append(element('span', 'app-mentors__field-error', e.message));
    }
    return box;
  }

  /** Portées : cases « Section » et « Projet » (au moins une). */
  function scopesRow() {
    const box = element('fieldset', 'app-mentors__scopes');
    box.append(element('legend', '', 'Portées'));
    const row = element('div', 'hd-row');
    for (const [scope, label] of [['section', 'Section'], ['project', 'Projet']]) {
      const check = element('label', 'hd-row app-mentors__check');
      const input = element('input');
      input.type = 'checkbox';
      input.checked = form.scopes.includes(scope);
      input.disabled = !editable();
      input.addEventListener('change', () => {
        const next = new Set(form.scopes);
        if (input.checked) next.add(scope);
        else next.delete(scope);
        form.scopes = ['section', 'project'].filter((s) => next.has(s));
        dirty = true;
        renderFiche();
      });
      check.append(input, label);
      row.append(check);
    }
    box.append(row, element('p', 'app-mentors__hint', 'Un mentor relit les sections qui le concernent, le projet lors d\'une revue, ou les deux.'));
    for (const e of errorFor('scopes')) box.append(element('span', 'app-mentors__field-error', e.message));
    return box;
  }

  function contextSelect() {
    const node = select('context', CONTEXTS);
    node.addEventListener('change', () => renderFiche());
    return node;
  }

  /** « Comme mentor de projet » : déclencheurs, fichiers clés, remarques par revue. */
  function projectBlock() {
    const project = form.project;
    const box = element('fieldset', 'app-mentors__triggers');
    box.append(element('legend', '', 'Comme mentor de projet'));
    const triggers = element('div', 'app-mentors__review-triggers');
    triggers.append(element('span', 'hd-label', 'Revues automatiques'));
    for (const [value, label] of Object.entries(REVIEW_TRIGGERS)) {
      const check = element('label', 'hd-row app-mentors__check');
      const input = element('input');
      input.type = 'checkbox';
      input.checked = project.triggers.includes(value);
      input.disabled = !editable();
      input.addEventListener('change', () => {
        const next = new Set(project.triggers);
        if (input.checked) next.add(value);
        else next.delete(value);
        project.triggers = Object.keys(REVIEW_TRIGGERS).filter((t) => next.has(t));
        dirty = true;
        renderFooter();
      });
      check.append(input, label);
      triggers.append(check);
    }
    triggers.append(element('p', 'app-mentors__hint', 'Délai, fréquence et plafond : Réglages · Revues du projet.'));
    box.append(triggers);

    // Fichiers clés : étiquettes retirables, comme les chemins des déclencheurs.
    const files = element('div', 'hd-row app-mentors__tags');
    const label = element('span', 'hd-label app-mentors__tags-label', 'Fichiers clés');
    label.id = `mentor-cles-${++ids}`;
    files.setAttribute('role', 'group');
    files.setAttribute('aria-labelledby', label.id);
    files.append(label);
    if (!project.key_files.length) files.append(element('span', 'hd-muted app-mentors__none', 'Aucun (la carte seule)'));
    project.key_files.forEach((value, index) => {
      const invalid = errors.some((e) => e.field === `project.key_files[${index}]`);
      const tag = element('span', `hd-id app-mentors__tag${invalid ? ' app-mentors__tag--error' : ''}`, value);
      if (editable()) {
        const remove = element('button', 'app-mentors__remove', '×');
        remove.type = 'button';
        remove.setAttribute('aria-label', `Retirer ${value}`);
        remove.addEventListener('click', () => {
          project.key_files.splice(index, 1);
          dirty = true;
          renderFiche();
        });
        tag.append(remove);
      }
      files.append(tag);
    });
    if (editable()) {
      const add = element('input', 'hd-input app-mentors__add');
      add.type = 'text';
      add.setAttribute('aria-label', 'Ajouter : fichier clé');
      add.placeholder = 'docs/architecture/**';
      const commit = () => {
        const value = add.value.trim();
        if (!value) return;
        project.key_files.push(value);
        dirty = true;
        renderFiche();
        ficheRoot.querySelector('[aria-label="Ajouter : fichier clé"]')?.focus();
      };
      add.addEventListener('keydown', (event) => {
        if (event.key === 'Enter') {
          event.preventDefault();
          commit();
        }
      });
      files.append(add, button('Ajouter', 'ghost', commit));
    }
    box.append(files);

    const count = element('select', 'hd-input');
    for (const n of [1, 2, 3]) {
      const option = element('option', '', String(n));
      option.value = String(n);
      count.append(option);
    }
    count.value = String(project.max_remarks);
    count.disabled = !editable();
    count.addEventListener('change', () => {
      project.max_remarks = Number(count.value);
      dirty = true;
      renderFooter();
    });
    box.append(field('Remarques par revue', count, 'project.max_remarks'));
    for (const e of errors.filter((x) => x.field.startsWith('project.') && x.field !== 'project.max_remarks')) {
      box.append(element('span', 'app-mentors__field-error', `${e.field} : ${e.message}`));
    }
    return box;
  }

  function header() {
    const head = element('div', 'hd-row app-mentors__head');
    const titles = element('div', 'app-mentors__titles');
    const item = list.items.find((i) => selected && i.id === selected.id);
    const name = creating ? 'Nouveau mentor' : (form?.name || file.id);
    titles.append(element('h1', 'app-settings__h1', name));
    const where = element('span', 'app-mentors__where');
    if (creating) {
      where.textContent = 'Il sera écrit dans vos mentors personnels ou dans le dépôt du projet.';
    } else {
      where.append(file.path ? mono(file.path) : 'Mentor livré, intégré à Hodegos');
      if (item?.origin === file.origin && item.overrides.length) {
        where.append(` · remplace le mentor ${ORIGIN_OF[item.overrides.at(-1)]} de même identifiant`);
      }
      if (item && item.origin !== file.origin) where.append(` · remplacé par le mentor ${ORIGIN_OF[item.origin]}`);
    }
    titles.append(where);
    head.append(titles, actions(item));
    return head;
  }

  function actions(item) {
    const row = element('div', 'hd-row app-mentors__actions');
    if (creating) return row;
    if (!file.editable) {
      if (form) row.append(button('Dupliquer', 'ghost', () => create({ form, body: file.body, origin: 'user' })));
      if (item?.origin === 'builtin' && item.enabled) {
        row.append(button('Désactiver', 'ghost', () => save({ origin: 'user', id: file.id, content: minimalOverride(file.id) })));
      }
      row.append(
        button('Personnaliser dans le dépôt', 'secondary', () => personalize('project')),
        button('Personnaliser', 'primary', () => personalize('user')),
      );
      return row;
    }
    if (form) {
      row.append(button('Dupliquer', 'ghost', () => create({ form, body: editor.value, origin: file.origin })));
      row.append(button(form.enabled ? 'Désactiver' : 'Réactiver', 'ghost', () => {
        form.enabled = !form.enabled;
        save({ origin: file.origin, id: file.id, form, body: editor.value });
      }));
    }
    const lower = item && item.id === file.id ? item.overrides.filter((o) => o !== file.origin).at(-1) : null;
    const label = !form && !errors.length ? 'Réactiver' : lower ? `Revenir au mentor ${ORIGIN_OF[lower]}` : 'Supprimer le mentor';
    row.append(button(label, 'secondary', (event) => remove(event.currentTarget, lower)));
    return row;
  }

  function renderFiche() {
    if (!shown) return;
    const parts = [];
    if (!selected && !creating) {
      ficheRoot.replaceChildren(element('p', 'hd-muted', message || 'Choisissez un mentor dans la liste.'));
      return;
    }
    parts.push(header());

    if (creating) {
      const origin = element('select', 'hd-input');
      for (const [value, label] of [['user', 'Mentors personnels'], ['project', 'Dépôt du projet (.hodegos/mentors/)']]) {
        const option = element('option', '', label);
        option.value = value;
        origin.append(option);
      }
      origin.value = creating.origin;
      origin.addEventListener('change', () => { creating.origin = origin.value; });
      const grid = element('div', 'app-mentors__grid');
      grid.append(field('Identifiant (minuscules, chiffres, -)', input('id'), 'id'), field('Enregistrer dans', origin, 'origine'));
      parts.push(grid);
    } else if (errorFor('id').length) {
      parts.push(element('p', 'app-mentors__field-error', errorFor('id').map((e) => e.message).join(' ; ')));
    }

    if (!form && !rawMode && file && !file.errors.length) {
      parts.push(element('p', 'app-settings__note', 'Cette surcharge minimale désactive le mentor de niveau inférieur. « Réactiver » la supprime.'));
    }

    if (form && !rawMode) {
      const grid = element('div', 'app-mentors__grid');
      grid.append(
        field('Nom', input('name'), 'name'),
        field('Initiales', input('initials'), 'initials'),
        field('Rôle', input('role'), 'role', true),
        field('Mode', select('mode', MODES), 'mode'),
        field('Priorité (0-100)', input('priority', 'number'), 'priority'),
        field('Niveau de modèle', select('model', MODELS), 'model'),
        field('Palier d\'entrée', select('entry_level', LEVELS), 'entry_level'),
        field('Remarques par passe', select('max_remarks', { 1: '1', 2: '2' }, Number), 'max_remarks'),
      );
      grid.append(element('p', 'app-mentors__hint', 'L\'effort suit le niveau de modèle (Réglages · Modèles et effort).'));
      parts.push(grid);
      parts.push(scopesRow());
      if (!form.scopes.includes('section')) {
        if (form.scopes.includes('project')) parts.push(projectBlock());
      }

      const triggers = element('fieldset', 'app-mentors__triggers');
      triggers.hidden = !form.scopes.includes('section');
      triggers.append(element('legend', '', 'Comme mentor de section'));
      triggers.append(field('Contexte envoyé', contextSelect(), 'context'));
      triggers.append(element('p', 'app-mentors__hint', CONTEXT_NOTES[form.context] ?? ''));
      for (const key of Object.keys(TRIGGERS)) triggers.append(tagList(key));
      const onOpen = element('label', 'hd-row app-mentors__check');
      const box = element('input');
      box.type = 'checkbox';
      box.checked = form.on_open;
      box.disabled = !editable();
      box.addEventListener('change', () => { form.on_open = box.checked; dirty = true; renderFooter(); });
      onOpen.append(box, 'Passe à la première ouverture d\'un fichier (', mono('on_open'), ')');
      triggers.append(onOpen);
      parts.push(triggers);
      if (form.scopes.includes('section') && form.scopes.includes('project')) parts.push(projectBlock());
    }

    const editorBox = element('div', 'app-mentors__posture');
    const caption = element('span', 'hd-label');
    if (rawMode) caption.append('Fichier ', mono(file?.path ?? ''), ' tel quel (frontmatter et posture)');
    else caption.append('Posture, commune aux deux portées (Markdown ; ', mono('{language}'), ', ', mono('{name}'), ' et ', mono('{scope}'), ' — « section » ou « projet » — sont remplacés)');
    editorBox.append(caption, posture);
    if (form || rawMode || creating) parts.push(editorBox);

    const general = errors.filter((e) => rawMode || !form
      || !(FORM_FIELDS.includes(e.field) || e.field.startsWith('triggers') || e.field.startsWith('project')));
    if (general.length) {
      const box = element('div', 'hd-callout hd-callout--warning');
      box.setAttribute('role', 'alert');
      box.append(element('div', 'hd-callout__title', 'Le chargeur refuse ce fichier'));
      const ul = element('ul', 'app-mentors__errors');
      for (const e of general) ul.append(element('li', '', `${e.field} : ${e.message}`));
      box.append(ul);
      parts.push(box);
    }

    if (editable() && !creating && (file.form || rawMode)) {
      const toggle = element('label', 'hd-row app-mentors__check');
      const box = element('input');
      box.type = 'checkbox';
      box.checked = rawMode;
      box.addEventListener('change', () => switchRaw(box.checked, box));
      toggle.append(box, 'Modifier le fichier brut');
      parts.push(toggle);
      if (!rawMode) {
        parts.push(element('p', 'app-settings__note', 'En mode formulaire, le frontmatter est réécrit : ses commentaires YAML ne sont pas conservés. Le fichier brut les garde.'));
      }
    }

    if (form || rawMode) parts.push(promptBox());
    const footer = element('div', 'hd-row app-mentors__footer');
    parts.push(footer);
    ficheRoot.replaceChildren(...parts);
    renderFooter();
  }

  function promptBox() {
    const box = element('div', 'hd-callout');
    const title = element('div', 'hd-callout__title', 'Prompt assemblé');
    title.append(button(prompt ? 'Masquer' : 'Afficher', 'ghost', togglePrompt));
    box.append(title, element('p', 'app-settings__note', 'Cadre commun (règles absolues, paliers, format JSON), en lecture seule, puis cette posture. Une posture ne peut pas lever la règle « pas de solution ».'));
    if (prompt) {
      if (prompt.note) box.append(element('p', 'app-settings__note', prompt.note));
      box.append(element('pre', 'app-mentors__prompt', prompt.text));
    }
    return box;
  }

  async function togglePrompt() {
    if (prompt) {
      prompt = null;
      renderFiche();
      return;
    }
    const id = creating ? form.id : file.id;
    const payload = { id };
    let note = '';
    if (rawMode && dirty) payload.content = editor.value;
    else if (dirty || creating) note = 'Prompt du mentor enregistré : enregistrez pour voir vos changements.';
    try {
      prompt = { text: (await request('mentor_prompt', payload)).payload.text, note };
    } catch (error) {
      prompt = { text: '', note: error.message };
    }
    renderFiche();
  }

  async function switchRaw(on, box) {
    if (dirty && !(await confirmDialog({
      title: on ? 'Passer au fichier brut ?' : 'Revenir au formulaire ?',
      body: [element('p', '', 'Vos changements non enregistrés seront abandonnés.')],
      confirmLabel: 'Abandonner et changer',
      returnFocus: box,
    }))) {
      box.checked = !on;
      return;
    }
    rawMode = on;
    form = file.form ? structuredClone(file.form) : null;
    dirty = false;
    errors = [];
    prompt = null;
    editor.set(on ? file.content : (file.body ?? ''), { language: 'markdown' });
    renderFiche();
  }

  function renderFooter() {
    const footer = ficheRoot.querySelector('.app-mentors__footer');
    if (!footer) return;
    footer.replaceChildren();
    if (!editable()) {
      footer.append(element('span', 'app-mentors__note', 'Un mentor livré se lit ; « Personnaliser » le copie pour le modifier.'));
      return;
    }
    const note = element('span', 'app-mentors__note', message || 'Enregistrer écrit le fichier ; le mentor s\'applique à la passe suivante.');
    if (message) note.setAttribute('role', 'alert');
    const cancel = button('Annuler', 'secondary', () => (creating ? cancelCreate() : open(selected, { force: true })));
    cancel.disabled = !dirty;
    const saveButton = button('Enregistrer', 'primary', () => submit());
    saveButton.disabled = !dirty;
    footer.append(note, cancel, saveButton);
  }

  function cancelCreate() {
    creating = null;
    form = null;
    dirty = false;
    renderFiche();
  }

  function submit() {
    if (creating) return save({ origin: creating.origin, id: form.id.trim(), form: { ...form, id: form.id.trim() }, body: editor.value });
    if (rawMode) return save({ origin: file.origin, id: file.id, content: editor.value });
    return save({ origin: file.origin, id: file.id, form, body: editor.value });
  }

  function save(payload) {
    return operation(() => saveNow(payload));
  }

  async function saveNow(payload) {
    message = '';
    try {
      const reply = await request('mentor_save', payload);
      dirty = false;
      errors = [];
      await loadList();
      await open({ id: reply.payload.id, origin: reply.payload.origin }, { force: true });
    } catch (error) {
      errors = error.errors ?? [];
      message = errors.length ? 'Le chargeur refuse ce mentor : corrigez les champs signalés.' : error.message;
      renderFiche();
    }
  }

  function personalize(to) {
    return operation(() => personalizeNow(to));
  }

  async function personalizeNow(to) {
    try {
      const reply = await request('mentor_eject', { id: file.id, to });
      await loadList();
      await open({ id: reply.payload.id, origin: to }, { force: true });
    } catch (error) {
      message = error.message;
      renderFiche();
    }
  }

  async function remove(trigger, lower) {
    const name = mono(file.path ?? file.id);
    const text = lower
      ? `Le fichier sera supprimé ; le mentor ${ORIGIN_OF[lower]} reprend sa place.`
      : 'Le fichier sera supprimé ; ce mentor n\'existera plus.';
    const confirmed = await confirmDialog({
      title: ['Supprimer ', name, ' ?'],
      body: [element('p', '', text)],
      confirmLabel: 'Supprimer',
      returnFocus: trigger,
    });
    if (!confirmed) return;
    await operation(() => deleteNow());
  }

  async function deleteNow() {
    try {
      await request('mentor_delete', { id: file.id, origin: file.origin });
      dirty = false;
      await loadList();
      const next = list.items.find((i) => i.id === file.id);
      if (next) await open({ id: next.id, origin: next.origin }, { force: true });
      else {
        selected = null;
        file = null;
        renderFiche();
      }
    } catch (error) {
      message = error.message;
      renderFiche();
    }
  }

  newButton.addEventListener('click', () => create());

  return {
    async show() {
      shown = true;
      listRoot.hidden = false;
      ficheRoot.hidden = false;
      await loadList();
      renderFiche();
    },
    hide() {
      shown = false;
      listRoot.hidden = true;
      ficheRoot.hidden = true;
    },
    /** Rechargement à chaud : la liste suit, la fiche aussi si rien n'est en cours. */
    async reloaded() {
      if (!shown || busy) return;
      await loadList();
      if (selected && !dirty) await open(selected, { force: true });
    },
    count: loadList,
    get dirty() {
      return dirty;
    },
  };
}

