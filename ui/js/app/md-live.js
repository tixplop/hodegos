/* Aperçu en direct du Markdown (phase 8c) : décorations calculées sur l'arbre syntaxique.
   Marques masquées, titres, puces et cases rendus ; toute ligne qui touche la sélection
   montre sa syntaxe brute. */
import {
  Decoration,
  EditorView,
  Prec,
  StateField,
  ViewPlugin,
  WidgetType,
  keymap,
  syntaxTree,
} from '/vendor/codemirror.js';

const hidden = Decoration.replace({});
const rawMark = Decoration.mark({ class: 'app-md-mark' });
const linkText = Decoration.mark({ class: 'app-md-link' });
const inlineCode = Decoration.mark({ class: 'app-md-code' });
const lineClass = (name) => Decoration.line({ class: name });

class BulletWidget extends WidgetType {
  eq() {
    return true;
  }

  toDOM() {
    const span = document.createElement('span');
    span.className = 'app-md-bullet';
    span.textContent = '•';
    return span;
  }
}

class RuleWidget extends WidgetType {
  eq() {
    return true;
  }

  toDOM() {
    const span = document.createElement('span');
    span.className = 'app-md-hr';
    span.setAttribute('aria-hidden', 'true');
    return span;
  }
}

/** Case d'une tâche : la cocher remplace `[ ]` par `[x]` comme une frappe (Ctrl+Z l'annule). */
class TaskWidget extends WidgetType {
  constructor(checked, label) {
    super();
    this.checked = checked;
    this.label = label;
  }

  eq(other) {
    return other.checked === this.checked && other.label === this.label;
  }

  toDOM(view) {
    const input = document.createElement('input');
    input.type = 'checkbox';
    input.className = 'app-md-task';
    input.checked = this.checked;
    if (this.label) input.setAttribute('aria-label', this.label);
    // Le focus reste dans l'éditeur : la ligne ne passe pas en syntaxe brute au clic.
    input.addEventListener('mousedown', (event) => event.preventDefault());
    input.addEventListener('change', () => {
      const pos = view.posAtDOM(input);
      if (!/^\[[ xX]\]$/.test(view.state.sliceDoc(pos, pos + 3))) return;
      view.dispatch({
        changes: { from: pos + 1, to: pos + 2, insert: input.checked ? 'x' : ' ' },
        userEvent: 'input',
      });
    });
    return input;
  }
}

const bullet = Decoration.replace({ widget: new BulletWidget() });
const rule = Decoration.replace({ widget: new RuleWidget() });

/** Lignes qui touchent la sélection ou le curseur : elles montrent leur syntaxe brute. */
function touches(state, from, to) {
  const start = state.doc.lineAt(from).from;
  const end = state.doc.lineAt(to).to;
  return state.selection.ranges.some((range) => range.from <= end && range.to >= start);
}

/** Un passage cité entièrement dans une marque masquée garde cette marque visible. */
function pinned(anchors, from, to) {
  return anchors.some((a) => a.to > a.from && a.from >= from && a.to <= to);
}

function inlineDecorations(view, anchorRanges) {
  const { state } = view;
  const { doc } = state;
  const anchors = anchorRanges(state);
  const ranges = [];
  const seen = new Set();
  const once = (key) => !seen.has(key) && seen.add(key);

  const hide = (from, to) => {
    if (from >= to) return;
    if (touches(state, from, to) || pinned(anchors, from, to)) {
      ranges.push(rawMark.range(from, to));
      return;
    }
    // Coupée au marqueur d'un passage cité qui finit dedans : il reste affiché.
    const cuts = anchors.map((a) => a.to).filter((pos) => pos > from && pos < to).sort((a, b) => a - b);
    let start = from;
    for (const cut of [...cuts, to]) {
      if (cut > start) ranges.push(hidden.range(start, cut));
      start = cut;
    }
  };
  const replace = (decoration, from, to) => {
    if (touches(state, from, to) || pinned(anchors, from, to)) ranges.push(rawMark.range(from, to));
    else ranges.push(decoration.range(from, to));
  };
  const lines = (from, to, name) => {
    for (let pos = from; pos <= to;) {
      const line = doc.lineAt(pos);
      if (once(`${name}:${line.from}`)) ranges.push(lineClass(name).range(line.from));
      pos = line.to + 1;
    }
  };
  const spaceAfter = (pos) => (doc.sliceString(pos, pos + 1) === ' ' ? pos + 1 : pos);

  // Toute la zone affichée d'un coup : les commentaires masqués découpent `visibleRanges`.
  const { from, to } = view.viewport;
  syntaxTree(state).iterate({
    from,
    to,
    enter(node) {
      const { name } = node;
      const heading = /^(ATX|Setext)Heading(\d)$/.exec(name);
      if (heading) {
        const end = heading[1] === 'Setext' ? doc.lineAt(node.to).from - 1 : node.from;
        lines(node.from, Math.max(node.from, end), `app-md-h${heading[2]}`);
        return true;
      }
      switch (name) {
        case 'HeaderMark': {
          const parent = node.node.parent;
          if (parent?.name.startsWith('Setext')) hide(node.from, node.to);
          else if (node.from === parent?.from) hide(node.from, spaceAfter(node.to));
          else hide(doc.sliceString(node.from - 1, node.from) === ' ' ? node.from - 1 : node.from, node.to);
          return false;
        }
        case 'EmphasisMark':
        case 'StrikethroughMark':
          hide(node.from, node.to);
          return false;
        case 'InlineCode':
          ranges.push(inlineCode.range(node.from, node.to));
          for (const mark of node.node.getChildren('CodeMark')) hide(mark.from, mark.to);
          return false;
        case 'FencedCode':
        case 'CodeBlock':
          lines(node.from, node.to, 'app-md-codeblock');
          return false;
        case 'Link': {
          const marks = node.node.getChildren('LinkMark');
          if (marks.length < 2) return true;
          hide(marks[0].from, marks[0].to);
          if (marks[1].from > marks[0].to) ranges.push(linkText.range(marks[0].to, marks[1].from));
          hide(marks[1].from, node.to);
          return true;
        }
        case 'Autolink':
          for (const mark of node.node.getChildren('LinkMark')) hide(mark.from, mark.to);
          return false;
        case 'LinkMark':
        case 'URL':
        case 'LinkTitle':
        case 'LinkLabel':
        case 'Image':
        case 'CommentBlock':
          return false;
        case 'Comment':
          if (doc.lineAt(node.from).number === doc.lineAt(node.to).number) hide(node.from, node.to);
          return false;
        case 'Escape':
          hide(node.from, node.from + 1);
          return false;
        case 'ListMark': {
          const item = node.node.parent;
          if (item?.parent?.name !== 'BulletList') return false;
          if (item.getChild('Task')) hide(node.from, spaceAfter(node.to));
          else replace(bullet, node.from, node.to);
          return false;
        }
        case 'TaskMarker': {
          const text = doc.sliceString(node.from, node.to);
          const label = doc.sliceString(node.to, doc.lineAt(node.to).to).trim();
          const task = Decoration.replace({ widget: new TaskWidget(/x/i.test(text), label) });
          replace(task, node.from, node.to);
          return false;
        }
        case 'QuoteMark':
          lines(node.from, node.from, 'app-md-quote');
          hide(node.from, spaceAfter(node.to));
          return false;
        case 'HorizontalRule':
          replace(rule, node.from, node.to);
          return false;
        default:
          return true;
      }
    },
  });
  return Decoration.set(ranges, true);
}

/**
 * Commentaires HTML sur leurs propres lignes (consignes du gabarit de vision) : masqués
 * avec leurs lignes. Un plugin ne peut pas remplacer un saut de ligne, d'où un champ d'état.
 */
function commentBlocks(state, anchorRanges) {
  const anchors = anchorRanges(state);
  const ranges = [];
  syntaxTree(state).iterate({
    enter(node) {
      if (node.name === 'CommentBlock') {
        const from = state.doc.lineAt(node.from).from;
        const to = state.doc.lineAt(node.to).to;
        if (!touches(state, from, to) && !pinned(anchors, from, to)) {
          ranges.push(Decoration.replace({ block: true }).range(from, to));
        }
        return false;
      }
      // Les commentaires en bloc ne vivent que dans les conteneurs.
      return node.name === 'Document' || node.name === 'Blockquote' || node.name.endsWith('List') || node.name === 'ListItem';
    },
  });
  return Decoration.set(ranges);
}

/**
 * Flèches haut et bas : un commentaire masqué compte comme une ligne, le curseur s'y arrête
 * et il réapparaît. Sans cela, la flèche passe par-dessus et il reste inaccessible.
 */
function stepIntoComment(field, dir) {
  return (view) => {
    const { state } = view;
    const range = state.selection.main;
    if (state.selection.ranges.length > 1 || !range.empty) return false;
    const next = state.doc.lineAt(range.head).number + dir;
    if (next < 1 || next > state.doc.lines) return false;
    const target = state.doc.line(next);
    let hit = false;
    state.field(field).between(target.from, target.from, () => {
      hit = true;
      return false;
    });
    if (!hit) return false;
    view.dispatch({ selection: { anchor: target.from }, scrollIntoView: true, userEvent: 'select' });
    return true;
  };
}

const theme = EditorView.theme({
  '&.app-md-live': { fontSize: '15px' },
  '&.app-md-live .cm-scroller': { fontFamily: 'var(--font-sans)', fontVariantLigatures: 'normal', fontFeatureSettings: 'normal', lineHeight: '24px' },
  '&.app-md-live .cm-gutters': { display: 'none' },
  '&.app-md-live .cm-content': { boxSizing: 'border-box', maxWidth: '820px', padding: '28px 56px 24px' },
  // Ligne active : le liseré de la gouttière, masquée en aperçu, passe sur la ligne.
  '&.app-md-live .cm-activeLine': { boxShadow: 'inset 3px 0 0 var(--accent)' },
  '.app-md-h1': { fontSize: '28px', lineHeight: '34px', fontWeight: '700' },
  '.app-md-h2': { fontSize: '20px', lineHeight: '28px', fontWeight: '600', paddingTop: '8px' },
  '.app-md-h3': { fontSize: '17px', lineHeight: '26px', fontWeight: '600', paddingTop: '6px' },
  '.app-md-h4, .app-md-h5, .app-md-h6': { fontWeight: '600' },
  '.app-md-mark': { color: 'var(--ink-muted)' },
  '.app-md-link': { color: 'var(--accent)' },
  '.app-md-code': { fontFamily: 'var(--font-mono)', fontVariantLigatures: 'none', fontSize: '13px', backgroundColor: 'var(--surface-100)', borderRadius: 'var(--radius-sm)', padding: '1px 3px' },
  '.app-md-codeblock': { fontFamily: 'var(--font-mono)', fontVariantLigatures: 'none', fontSize: '13px', backgroundColor: 'var(--surface-50)' },
  '.app-md-quote': { color: 'var(--ink-muted)', borderLeft: '3px solid var(--border-control)', paddingLeft: '12px' },
  '.app-md-bullet': { display: 'inline-block', minWidth: '0.8em', color: 'var(--ink-muted)' },
  '.app-md-task': { margin: '0 6px 0 0', verticalAlign: 'middle', accentColor: 'var(--accent)', cursor: 'pointer' },
  '.app-md-hr': { display: 'inline-block', width: '100%', verticalAlign: 'middle', borderTop: '1px solid var(--border-control)' },
});

/**
 * Extension de l'aperçu. `anchorRanges(state)` renvoie les passages cités affichés
 * (`[{ from, to }]`) : ils ne disparaissent pas dans une marque masquée.
 */
export function livePreview(anchorRanges) {
  const plugin = ViewPlugin.fromClass(class {
    constructor(view) {
      this.decorations = inlineDecorations(view, anchorRanges);
    }

    update(update) {
      if (update.docChanged || update.viewportChanged || update.selectionSet
        || update.transactions.some((tr) => tr.effects.length)
        || syntaxTree(update.startState) !== syntaxTree(update.state)) {
        this.decorations = inlineDecorations(update.view, anchorRanges);
      }
    }
  }, { decorations: (value) => value.decorations });

  const comments = StateField.define({
    create: (state) => commentBlocks(state, anchorRanges),
    update(value, tr) {
      if (tr.docChanged || tr.selection || tr.effects.length
        || syntaxTree(tr.startState) !== syntaxTree(tr.state)) {
        return commentBlocks(tr.state, anchorRanges);
      }
      return value;
    },
    provide: (field) => EditorView.decorations.from(field),
  });

  const arrows = Prec.high(keymap.of([
    { key: 'ArrowDown', run: stepIntoComment(comments, 1) },
    { key: 'ArrowUp', run: stepIntoComment(comments, -1) },
  ]));

  return [EditorView.editorAttributes.of({ class: 'app-md-live' }), theme, plugin, comments, arrows];
}
