/* Vue Projet (phase 9a, maquette 09-3) : carte du projet telle qu'envoyée aux mentors, son
   poids estimé et son plafond, ses blocs, les exclusions propres au projet. Les réglages
   s'appliquent aussitôt ; ils sont écrits dans ~/.local/share/hodegos, jamais dans le dépôt. */

const BLOCKS = {
  header: 'En-tête',
  vision: 'Vision',
  decisions: 'Décisions',
  tree: 'Arborescence',
  readme: 'README',
  dependencies: 'Dépendances',
  symbols: 'Symboles',
  tests: 'Tests',
};

const REASONS = {
  open: 'au lancement',
  save: 'après un enregistrement',
  external: 'après un changement sur le disque',
  settings: 'après un changement de réglage',
};

const number = new Intl.NumberFormat('fr-FR');
const percent = new Intl.NumberFormat('fr-FR', { style: 'percent', maximumFractionDigits: 0 });
const time = new Intl.DateTimeFormat('fr-FR', { hour: '2-digit', minute: '2-digit' });

function element(tag, className, text) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

/**
 * Onglet « Carte du projet » dans `root`. `request(type, payload)` envoie une demande et
 * renvoie la réponse.
 */
export function createProjectView(root, { request }) {
  const $ = (id) => root.querySelector(`#${id}`);
  const ui = {
    weight: $('carte-poids'),
    cap: $('carte-plafond'),
    updated: $('carte-maj'),
    bar: $('carte-barre'),
    error: $('carte-erreur'),
    blocks: $('carte-blocs'),
    excludes: $('carte-exclusions'),
    excludeForm: $('carte-exclure'),
    pattern: $('carte-motif'),
    text: $('carte-texte'),
    copy: $('carte-copier'),
  };
  /** Dernière carte reçue. */
  let map = null;
  let busy = false;

  function showError(message) {
    ui.error.textContent = message ?? '';
    ui.error.hidden = !message;
  }

  function render() {
    if (!map) return;
    ui.weight.textContent = `${number.format(map.tokens)} jetons estimés`;
    if (document.activeElement !== ui.cap) ui.cap.value = String(map.cap);
    const share = map.cap ? map.tokens / map.cap : 0;
    ui.bar.firstElementChild.style.width = `${Math.min(100, share * 100)}%`;
    ui.bar.setAttribute('aria-label', `Carte : ${percent.format(share)} du plafond`);
    const at = new Date(map.updated_at);
    const when = Number.isNaN(at.getTime()) ? '' : ` · mise à jour à ${time.format(at)}, ${REASONS[map.reason] ?? ''}`;
    const fallback = map.cap === map.default_cap ? '' : ` · plafond propre à ce projet (${number.format(map.default_cap)} par défaut)`;
    ui.updated.textContent = `estimés à 4 caractères par jeton${when}${fallback}`;

    ui.blocks.replaceChildren(...map.blocks.map(blockRow));
    ui.excludes.replaceChildren(...map.exclude.map(excludeItem));
    if (map.exclude.length === 0) ui.excludes.append(element('li', 'app-project__note', 'Aucune.'));
    ui.text.textContent = map.text;
  }

  function blockRow(block) {
    const row = element('tr');
    if (!block.enabled) row.className = 'app-project__off';
    const name = element('td');
    const label = element('label', 'hd-row app-project__block');
    const box = element('input');
    box.type = 'checkbox';
    box.checked = block.enabled;
    box.disabled = block.id === 'header';
    if (box.disabled) box.title = "L'en-tête est toujours envoyé.";
    box.addEventListener('change', () => toggle(block.id, box.checked));
    label.append(box, element('strong', '', BLOCKS[block.id] ?? block.id));
    name.append(label);
    const detail = element('td', 'app-project__detail', block.detail);
    if (block.truncated) detail.append(' ', element('span', 'hd-badge hd-badge--warning', 'raccourci'));
    const tokens = element('td', 'hd-mono app-project__num', block.enabled && block.tokens ? number.format(block.tokens) : '—');
    row.append(name, detail, tokens);
    return row;
  }

  function excludeItem(pattern) {
    const item = element('li', 'app-project__exclude');
    const remove = element('button', 'hd-btn hd-btn--ghost hd-btn--sm', 'Retirer');
    remove.type = 'button';
    remove.setAttribute('aria-label', `Ne plus exclure ${pattern}`);
    remove.addEventListener('click', () => apply({ exclude: map.exclude.filter((p) => p !== pattern) }));
    item.append(element('span', 'hd-id', pattern), remove);
    return item;
  }

  /** Envoie les réglages modifiés ; la carte recalculée revient dans la réponse. */
  async function apply(change) {
    if (!map || busy) return false;
    const next = {
      map_cap: map.cap,
      disabled_blocks: map.blocks.filter((b) => !b.enabled).map((b) => b.id),
      exclude: map.exclude,
      ...change,
    };
    busy = true;
    try {
      const reply = await request('project_map_set', next);
      showError(null);
      map = reply.payload;
      render();
      return true;
    } catch (error) {
      // Refus (`invalid_settings`) : rien n'a changé, l'écran revient aux valeurs en vigueur.
      showError(error.message);
      ui.cap.value = String(map.cap);
      render();
      return false;
    } finally {
      busy = false;
    }
  }

  function toggle(id, enabled) {
    const disabled = map.blocks.filter((b) => (b.id === id ? !enabled : !b.enabled)).map((b) => b.id);
    apply({ disabled_blocks: disabled });
  }

  ui.cap.addEventListener('change', () => {
    const value = Number(ui.cap.value);
    if (!Number.isInteger(value) || value === map?.cap) return;
    apply({ map_cap: value });
  });

  ui.excludeForm.addEventListener('submit', async (event) => {
    event.preventDefault();
    const pattern = ui.pattern.value.trim();
    if (!pattern || !map) return;
    if (map.exclude.includes(pattern)) {
      ui.pattern.value = '';
      return;
    }
    if (await apply({ exclude: [...map.exclude, pattern] })) ui.pattern.value = '';
  });

  ui.copy.addEventListener('click', async () => {
    try {
      await navigator.clipboard.writeText(map?.text ?? '');
      ui.copy.textContent = 'Copié';
    } catch {
      ui.copy.textContent = 'Copie impossible';
    }
    setTimeout(() => { ui.copy.textContent = 'Copier'; }, 1500);
  });

  return {
    /** Demande la carte et l'affiche. */
    async load() {
      try {
        const reply = await request('project_map_get', null);
        showError(null);
        map = reply.payload;
        render();
      } catch (error) {
        showError(error.message);
      }
    },
    /** `map_updated` : la carte a changé ; elle est redemandée si la vue est affichée. */
    updated(visible) {
      if (visible && !busy) this.load();
    },
  };
}
