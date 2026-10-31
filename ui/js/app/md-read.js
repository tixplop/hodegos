/* Lecture du Markdown (phase 8c) : document non éditable, construit en éléments DOM à partir
   de l'arbre syntaxique, jamais par `innerHTML`. Le HTML du document reste du texte, les
   images ne sont pas chargées. */
import { ensureSyntaxTree, syntaxTree } from '/vendor/codemirror.js';

/** Marques de syntaxe : jamais rendues. */
const MARKS = new Set([
  'HeaderMark', 'EmphasisMark', 'CodeMark', 'LinkMark', 'QuoteMark', 'ListMark',
  'StrikethroughMark', 'TaskMarker', 'TableDelimiter', 'LinkTitle', 'LinkLabel', 'CodeInfo',
]);

/** Liens ouverts dans un nouvel onglet du navigateur ; tout autre schéma reste du texte. */
const EXTERNAL = /^(https?:|mailto:)/i;
const SCHEME = /^[a-z][a-z0-9+.-]*:/i;

const ENTITIES = { amp: '&', lt: '<', gt: '>', quot: '"', apos: "'", nbsp: ' ' };

/** Temps accordé à l'analyse complète d'un long document avant de rendre ce qui est prêt. */
const PARSE_BUDGET_MS = 200;

function element(tag, className) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  return node;
}

function entity(text) {
  const named = /^&([a-z]+);$/i.exec(text);
  if (named) return ENTITIES[named[1].toLowerCase()] ?? text;
  const numeric = /^&#(x[0-9a-f]+|\d+);$/i.exec(text);
  if (!numeric) return text;
  const code = numeric[1][0].toLowerCase() === 'x' ? parseInt(numeric[1].slice(1), 16) : Number(numeric[1]);
  return code > 0 && code <= 0x10ffff ? String.fromCodePoint(code) : text;
}

/**
 * Chemin dans le projet d'un lien relatif, depuis le dossier du document ; `#…` et `?…`
 * retirés. `null` s'il sort du projet ou ne désigne pas de fichier.
 */
export function resolveLink(docPath, url) {
  let target = url.replace(/[#?].*$/, '');
  if (!target) return null;
  try {
    target = decodeURIComponent(target);
  } catch {
    return null;
  }
  const parts = target.startsWith('/') ? [] : docPath.split('/').slice(0, -1);
  for (const part of target.split('/')) {
    if (part === '' || part === '.') continue;
    if (part === '..') {
      if (!parts.length) return null;
      parts.pop();
    } else {
      parts.push(part);
    }
  }
  return parts.length ? parts.join('/') : null;
}

function slug(text) {
  return text.trim().toLowerCase().replace(/[^\p{L}\p{N}\s-]/gu, '').replace(/\s+/g, '-');
}

/**
 * Rendu du contenu en ligne d'un document. `segments` reçoit chaque nœud texte copié tel
 * quel de la source, avec sa position, pour y poser les ancres des remarques.
 */
function inlineRenderer(doc, docPath, segments, onLink) {
  const slice = (from, to) => doc.sliceString(from, to);

  const text = (parent, from, to) => {
    if (from >= to) return;
    const node = document.createTextNode(slice(from, to));
    segments.push({ node, from });
    parent.append(node);
  };

  /** Contenu en ligne de `node` entre `from` et `to` : texte et éléments enfants. */
  const inline = (parent, node, from = node.from, to = node.to) => {
    let pos = from;
    for (let child = node.firstChild; child; child = child.nextSibling) {
      if (child.to <= from || child.from >= to) continue;
      text(parent, pos, child.from);
      pos = child.to;
      const rendered = inlineNode(child);
      if (rendered) parent.append(rendered);
    }
    text(parent, pos, to);
  };

  const inner = (node, tag, className) => {
    const marks = node.getChildren(node.name === 'InlineCode' ? 'CodeMark' : node.name === 'Strikethrough' ? 'StrikethroughMark' : 'EmphasisMark');
    const out = element(tag, className);
    const from = marks.length ? marks[0].to : node.from;
    const to = marks.length > 1 ? marks[marks.length - 1].from : node.to;
    inline(out, node, from, to);
    return out;
  };

  const link = (label, url) => {
    if (EXTERNAL.test(url)) {
      const a = element('a', 'hd-link');
      a.href = url;
      a.target = '_blank';
      a.rel = 'noopener noreferrer';
      a.append(...label);
      return a;
    }
    const local = url.startsWith('#');
    const path = !local && !SCHEME.test(url) && !url.startsWith('//') ? resolveLink(docPath, url) : null;
    const anchor = local ? slug(decodeSafe(url.slice(1))) : null;
    if (!path && !anchor) {
      const span = element('span');
      span.title = url;
      span.append(...label);
      return span;
    }
    // Lien interne : pas d'adresse, qui mènerait hors de l'IDE ; Entrée l'ouvre aussi.
    const a = element('a', 'hd-link app-read__internal');
    a.tabIndex = 0;
    a.setAttribute('role', 'link');
    a.title = path ?? url;
    a.append(...label);
    const open = (event) => {
      event.preventDefault();
      if (path) onLink(path);
      else a.closest('.app-read')?.querySelector(`[data-slug="${CSS.escape(anchor)}"]`)?.scrollIntoView({ block: 'start' });
    };
    a.addEventListener('click', open);
    a.addEventListener('keydown', (event) => {
      if (event.key === 'Enter') open(event);
    });
    return a;
  };

  const inlineNode = (node) => {
    switch (node.name) {
      case 'Emphasis': return inner(node, 'em');
      case 'StrongEmphasis': return inner(node, 'strong');
      case 'Strikethrough': return inner(node, 'del');
      case 'InlineCode': return inner(node, 'code', 'hd-mono app-read__code');
      case 'Link': {
        const marks = node.getChildren('LinkMark');
        const url = node.getChild('URL');
        const label = element('span');
        if (marks.length >= 2) inline(label, node, marks[0].to, marks[1].from);
        else text(label, node.from, node.to);
        return link([...label.childNodes], url ? slice(url.from, url.to) : '');
      }
      case 'Autolink':
      case 'URL': {
        const url = node.name === 'URL' ? node : node.getChild('URL');
        if (!url) return document.createTextNode(slice(node.from, node.to));
        const label = element('span');
        text(label, url.from, url.to);
        return link([...label.childNodes], slice(url.from, url.to));
      }
      case 'Image': {
        // Jamais chargée : texte alternatif et chemin.
        const marks = node.getChildren('LinkMark');
        const url = node.getChild('URL');
        const figure = element('span', 'app-read__image');
        const alt = marks.length >= 2 ? slice(marks[0].to, marks[1].from) : '';
        figure.textContent = `Image : ${alt || 'sans texte alternatif'}${url ? ` (${slice(url.from, url.to)})` : ''}`;
        return figure;
      }
      case 'Escape': return document.createTextNode(slice(node.from + 1, node.to));
      case 'Entity': return document.createTextNode(entity(slice(node.from, node.to)));
      case 'HardBreak': return element('br');
      case 'Comment': return null;
      case 'HTMLTag':
      case 'ProcessingInstruction': {
        const span = element('span', 'hd-mono app-read__html');
        text(span, node.from, node.to);
        return span;
      }
      default: {
        if (MARKS.has(node.name)) return null;
        const span = element('span');
        inline(span, node);
        return span;
      }
    }
  };

  return { text, inline };
}

/** Alignement de chaque colonne, lu dans la ligne de délimitation (`:--`, `:-:`, `--:`). */
function alignments(delimiter) {
  return delimiter.trim().replace(/^\|/, '').replace(/\|$/, '').split('|').map((spec) => {
    const cell = spec.trim();
    if (cell.startsWith(':') && cell.endsWith(':') && cell.length > 1) return 'center';
    if (cell.endsWith(':')) return 'right';
    if (cell.startsWith(':')) return 'left';
    return null;
  });
}

/**
 * Tableau GFM : cellules rangées par colonne d'après les `|` (une cellule vide n'a pas de
 * nœud), comblées ou tronquées au nombre de colonnes de la ligne de délimitation.
 */
function table(node, doc, inline) {
  const delimiter = node.getChildren('TableDelimiter')[0];
  const aligns = delimiter ? alignments(doc.sliceString(delimiter.from, delimiter.to)) : [];
  const wrap = element('div', 'app-read__table-wrap');
  const out = element('table', 'app-read__table');
  const body = element('tbody');
  for (let row = node.firstChild; row; row = row.nextSibling) {
    if (row.name !== 'TableHeader' && row.name !== 'TableRow') continue;
    const header = row.name === 'TableHeader';
    const cells = [];
    let column = row.firstChild?.name === 'TableDelimiter' ? -1 : 0;
    for (let child = row.firstChild; child; child = child.nextSibling) {
      if (child.name === 'TableDelimiter') column += 1;
      else if (child.name === 'TableCell') cells[column] = child;
    }
    const count = aligns.length || cells.length;
    const tr = element('tr');
    for (let index = 0; index < count; index += 1) {
      const td = element(header ? 'th' : 'td', aligns[index] ? `app-read__cell--${aligns[index]}` : '');
      const cell = cells[index];
      if (cell) {
        td.dataset.from = String(cell.from);
        inline(td, cell);
      }
      tr.append(td);
    }
    if (header) {
      const head = element('thead');
      head.append(tr);
      out.append(head);
    } else {
      body.append(tr);
    }
  }
  out.append(body);
  wrap.append(out);
  return wrap;
}

/**
 * Tableau GFM `node` de `state` rendu comme en lecture, pour l'aperçu en direct. Chaque
 * cellule porte sa position source dans `data-from`.
 */
export function renderTable(state, node, docPath, onLink) {
  const { inline } = inlineRenderer(state.doc, docPath, [], onLink);
  return table(node, state.doc, inline);
}

/** Construit la lecture d'un document ; `segments` comme pour `inlineRenderer`. */
function build(state, docPath, segments, onLink) {
  const { doc } = state;
  const tree = ensureSyntaxTree(state, doc.length, PARSE_BUDGET_MS) ?? syntaxTree(state);
  const slice = (from, to) => doc.sliceString(from, to);
  const { text, inline } = inlineRenderer(doc, docPath, segments, onLink);

  const blocks = (parent, node) => {
    for (let child = node.firstChild; child; child = child.nextSibling) {
      const rendered = block(child);
      if (rendered) parent.append(rendered);
    }
  };

  const located = (out, node) => {
    out.dataset.from = String(node.from);
    return out;
  };

  const code = (node, className) => {
    const pre = element('pre', className);
    const content = element('code', 'hd-mono');
    const parts = node.getChildren('CodeText');
    parts.forEach((part, index) => {
      if (index) content.append('\n');
      text(content, part.from, part.to);
    });
    pre.append(content);
    return pre;
  };

  const block = (node) => {
    const heading = /^(?:ATX|Setext)Heading(\d)$/.exec(node.name);
    if (heading) {
      const out = element(`h${heading[1]}`);
      inline(out, node);
      out.dataset.slug = slug(out.textContent);
      return located(out, node);
    }
    switch (node.name) {
      case 'Paragraph': {
        const out = element('p');
        inline(out, node);
        return located(out, node);
      }
      case 'BulletList':
      case 'OrderedList': {
        const out = element(node.name === 'BulletList' ? 'ul' : 'ol');
        const first = node.firstChild?.getChild('ListMark');
        const start = first ? parseInt(slice(first.from, first.to), 10) : 1;
        if (node.name === 'OrderedList' && start !== 1 && Number.isFinite(start)) out.start = start;
        blocks(out, node);
        return located(out, node);
      }
      case 'ListItem': {
        const out = element('li');
        const task = node.getChild('Task');
        if (task) out.className = 'app-read__task';
        blocks(out, node);
        return located(out, node);
      }
      case 'Task': {
        const marker = node.getChild('TaskMarker');
        const label = element('span');
        const box = element('input', 'app-read__check');
        box.type = 'checkbox';
        box.disabled = true;
        box.checked = Boolean(marker) && /x/i.test(slice(marker.from, marker.to));
        inline(label, node, marker ? marker.to : node.from);
        box.setAttribute('aria-label', label.textContent.trim());
        const out = element('span', 'app-read__task-line');
        out.append(box, label);
        return out;
      }
      case 'Blockquote': {
        const out = element('blockquote');
        blocks(out, node);
        return located(out, node);
      }
      case 'FencedCode':
      case 'CodeBlock':
        return located(code(node, 'app-read__pre'), node);
      case 'HorizontalRule':
        return located(element('hr'), node);
      case 'HTMLBlock':
      case 'ProcessingInstructionBlock': {
        // Le HTML du document s'affiche comme du texte.
        const out = element('pre', 'app-read__pre app-read__html');
        const content = element('code', 'hd-mono');
        text(content, node.from, node.to);
        out.append(content);
        return located(out, node);
      }
      case 'Table':
        return located(table(node, doc, inline), node);
      case 'CommentBlock':
      case 'LinkReference':
        return null;
      default: {
        if (MARKS.has(node.name)) return null;
        const out = element('p');
        text(out, node.from, node.to);
        return located(out, node);
      }
    }
  };

  const root = element('article', 'app-read__doc');
  blocks(root, tree.topNode);
  return root;
}

function decodeSafe(text) {
  try {
    return decodeURIComponent(text);
  } catch {
    return text;
  }
}

/**
 * Entoure de `wrap()` le texte rendu entre les positions source `from` et `to`, morceau
 * par morceau (une citation peut chevaucher du gras). Renvoie le dernier morceau.
 */
function wrapRange(segments, from, to, wrap) {
  let last = null;
  for (let i = 0; i < segments.length; i += 1) {
    const segment = segments[i];
    const end = segment.from + segment.node.length;
    if (end <= from || segment.from >= to) continue;
    let { node } = segment;
    if (from > segment.from) {
      node = node.splitText(from - segment.from);
      segments.splice(i + 1, 0, { node, from });
      i += 1;
    }
    const start = segments[i].from;
    if (to < end) {
      const rest = node.splitText(to - start);
      segments.splice(i + 1, 0, { node: rest, from: to });
    }
    const wrapper = wrap();
    node.replaceWith(wrapper);
    wrapper.append(node);
    last = wrapper;
  }
  return last;
}

function marker(number, active) {
  const span = element('span', active ? 'hd-marker hd-marker--active' : 'hd-marker');
  span.setAttribute('aria-hidden', 'true');
  span.textContent = String(number);
  return span;
}

/**
 * Vue de lecture dans `container`. `onLink(path)` pour un lien vers un fichier du projet,
 * `onBlock(pos)` au double-clic sur un bloc, `onAnchor(remarkId)` au clic sur un passage cité.
 */
export function createReader(container, { onLink, onBlock, onAnchor }) {
  let shown = null;

  container.addEventListener('dblclick', (event) => {
    const target = event.target.closest?.('[data-from]');
    if (!target || !container.contains(target)) return;
    window.getSelection()?.removeAllRanges();
    onBlock(Number(target.dataset.from));
  });
  container.addEventListener('click', (event) => {
    const anchor = event.target.closest?.('[data-remark]');
    if (anchor) onAnchor(anchor.dataset.remark);
  });

  return {
    /**
     * Affiche `state` (document de `path`) avec ses ancres
     * (`[{ id, number, from, to, active }]`) ; rien n'est refait si rien n'a changé.
     */
    render(state, path, anchors) {
      const key = JSON.stringify(anchors);
      if (shown && shown.doc === state.doc && shown.path === path && shown.key === key) return;
      const segments = [];
      const root = build(state, path, segments, onLink);
      for (const anchor of anchors) {
        const last = anchor.to > anchor.from
          ? wrapRange(segments, anchor.from, anchor.to, () => {
            const mark = element('mark', anchor.active ? 'hd-anchor hd-anchor--active' : 'hd-anchor');
            mark.dataset.remark = anchor.id;
            return mark;
          })
          : null;
        const badge = marker(anchor.number, anchor.active);
        if (last) {
          last.after(badge);
        } else {
          // Passage absent du texte rendu (commentaire, adresse) : marqueur en fin de bloc.
          const blocks = [...root.querySelectorAll('[data-from]')].filter((b) => Number(b.dataset.from) <= anchor.from);
          (blocks.pop() ?? root).append(badge);
        }
      }
      const { scrollTop } = container;
      container.replaceChildren(root);
      container.scrollTop = shown?.path === path ? scrollTop : 0;
      shown = { doc: state.doc, path, key };
    },
    /** Amène à l'écran le bloc qui contient `pos`. */
    scrollTo(pos) {
      const blocks = [...container.querySelectorAll('[data-from]')].filter((b) => Number(b.dataset.from) <= pos);
      blocks.pop()?.scrollIntoView({ block: 'center' });
    },
    /** Amène à l'écran le passage cité de la remarque `id`. */
    reveal(id) {
      container.querySelector(`[data-remark="${CSS.escape(id)}"]`)?.scrollIntoView({ block: 'center' });
    },
  };
}
