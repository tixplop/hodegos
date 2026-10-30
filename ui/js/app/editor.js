/* Éditeur CodeMirror 6 : langages du MVP, thème branché sur les tokens du design system. */
import {
  EditorState,
  EditorView,
  Decoration,
  WidgetType,
  StateField,
  StateEffect,
  keymap,
  lineNumbers,
  highlightActiveLine,
  highlightActiveLineGutter,
  highlightSpecialChars,
  drawSelection,
  dropCursor,
  rectangularSelection,
  crosshairCursor,
  HighlightStyle,
  syntaxHighlighting,
  indentOnInput,
  bracketMatching,
  defaultKeymap,
  history,
  historyKeymap,
  indentWithTab,
  searchKeymap,
  highlightSelectionMatches,
  tags as t,
  rust,
  javascript,
  python,
  markdown,
} from '/vendor/codemirror.js';

export const LANGUAGE_NAMES = {
  rust: 'Rust',
  typescript: 'TypeScript',
  javascript: 'JavaScript',
  python: 'Python',
  markdown: 'Markdown',
  text: 'Texte',
};

function languageSupport(language, path) {
  const jsx = /x$/i.test(path);
  switch (language) {
    case 'rust': return rust();
    case 'typescript': return javascript({ typescript: true, jsx });
    case 'javascript': return javascript({ jsx });
    case 'python': return python();
    case 'markdown': return markdown();
    default: return [];
  }
}

const highlight = HighlightStyle.define([
  { tag: [t.keyword, t.controlKeyword, t.operatorKeyword, t.definitionKeyword, t.moduleKeyword, t.modifier, t.self], color: 'var(--syntax-keyword)' },
  { tag: [t.string, t.special(t.string), t.regexp, t.character, t.monospace], color: 'var(--syntax-string)' },
  { tag: [t.number, t.bool, t.null, t.atom, t.escape], color: 'var(--syntax-number)' },
  { tag: [t.comment, t.lineComment, t.blockComment, t.docComment, t.meta], color: 'var(--syntax-comment)', fontStyle: 'italic' },
  { tag: [t.typeName, t.className, t.namespace, t.standard(t.typeName)], color: 'var(--syntax-type)' },
  { tag: [t.function(t.variableName), t.function(t.propertyName), t.macroName], color: 'var(--syntax-function)' },
  { tag: t.heading, fontWeight: '700' },
  { tag: t.strong, fontWeight: '700' },
  { tag: t.emphasis, fontStyle: 'italic' },
  { tag: [t.link, t.url], textDecoration: 'underline' },
  { tag: t.invalid, color: 'var(--danger)' },
]);

const theme = EditorView.theme({
  '&': { flex: '1 1 auto', minHeight: '0', backgroundColor: 'var(--surface-0)', color: 'var(--ink)', fontSize: '13px' },
  '&.cm-focused': { outline: 'none' },
  // Sans ligatures : « === » et « => » doivent se lire tels qu'ils sont écrits.
  '.cm-scroller': { fontFamily: 'var(--font-mono)', fontVariantLigatures: 'none', fontFeatureSettings: '"calt" 0, "liga" 0', lineHeight: '24px', overflow: 'auto' },
  '.cm-content': { padding: 'var(--space-4) 0', caretColor: 'var(--ink)', tabSize: '4' },
  '.cm-cursor, .cm-dropCursor': { borderLeftColor: 'var(--ink)', borderLeftWidth: '2px' },
  '.cm-gutters': { backgroundColor: 'var(--surface-0)', color: 'var(--ink-muted)', border: 'none' },
  '.cm-lineNumbers .cm-gutterElement': { minWidth: '52px', padding: '0 var(--space-4) 0 0', textAlign: 'right' },
  '.cm-activeLine': { backgroundColor: 'var(--accent-soft)' },
  '.cm-activeLineGutter': { backgroundColor: 'var(--accent-soft)', color: 'var(--accent-ink)', fontWeight: '600' },
  '&.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground, .cm-selectionBackground, ::selection': { backgroundColor: 'var(--surface-100)' },
  '&.cm-focused .cm-selectionBackground': { backgroundColor: 'color-mix(in srgb, var(--accent) 25%, transparent)' },
  '.cm-selectionMatch': { backgroundColor: 'var(--surface-100)' },
  '&.cm-focused .cm-matchingBracket': { backgroundColor: 'var(--surface-100)', outline: '1px solid var(--border-control)' },
  '.cm-panels': { backgroundColor: 'var(--surface-50)', color: 'var(--ink)' },
  '.cm-panels.cm-panels-top': { borderBottom: '1px solid var(--border)' },
  '.cm-panels.cm-panels-bottom': { borderTop: '1px solid var(--border)' },
  '.cm-searchMatch': { backgroundColor: 'var(--surface-100)', outline: '1px solid var(--border-control)' },
  '.cm-searchMatch.cm-searchMatch-selected': { backgroundColor: 'var(--accent-soft)' },
  '.app-anchor': { backgroundColor: 'var(--highlight)', borderBottom: '2px solid var(--highlight-edge)', color: 'var(--ink)', cursor: 'pointer' },
  '.app-anchor--active': { outline: '2px solid var(--accent)', outlineOffset: '1px' },
  '.app-marker': { display: 'inline-block', marginLeft: '3px', padding: '0 5px', borderRadius: '999px', backgroundColor: 'var(--ink)', color: 'var(--surface-0)', fontFamily: 'var(--font-sans)', fontSize: '10px', lineHeight: '15px', fontWeight: '600', verticalAlign: 'super' },
  '.app-marker--active': { backgroundColor: 'var(--accent)', color: 'var(--on-accent)' },
  '.cm-textfield': { font: 'inherit', border: '1px solid var(--border-control)', borderRadius: 'var(--radius-sm)', backgroundColor: 'var(--surface-0)', color: 'var(--ink)' },
  '.cm-button': { font: 'inherit', border: '1px solid var(--border-control)', borderRadius: 'var(--radius-sm)', backgroundImage: 'none', backgroundColor: 'var(--surface-0)', color: 'var(--ink)' },
});

/* ---------- Ancres des remarques ---------- */

/** Remplace les ancres affichées : `[{ id, number, from, to, active }]`. */
const setAnchors = StateEffect.define();

/** Numéro de la remarque, juste après le passage cité (hd-marker). */
class MarkerWidget extends WidgetType {
  constructor(number, active) {
    super();
    this.number = number;
    this.active = active;
  }

  eq(other) {
    return other.number === this.number && other.active === this.active;
  }

  toDOM() {
    const span = document.createElement('span');
    span.className = this.active ? 'app-marker app-marker--active' : 'app-marker';
    span.setAttribute('aria-hidden', 'true');
    span.textContent = String(this.number);
    return span;
  }
}

function anchorDecorations(anchors) {
  const ranges = [];
  for (const anchor of anchors) {
    if (anchor.to > anchor.from) {
      ranges.push(Decoration.mark({
        class: anchor.active ? 'app-anchor app-anchor--active' : 'app-anchor',
        attributes: { 'data-remark': anchor.id },
      }).range(anchor.from, anchor.to));
    }
    ranges.push(Decoration.widget({ widget: new MarkerWidget(anchor.number, anchor.active), side: 1 }).range(anchor.to));
  }
  return Decoration.set(ranges, true);
}

/** Les ancres suivent le texte pendant la frappe, jusqu'au prochain remplacement. */
const anchorsField = StateField.define({
  create: () => Decoration.none,
  update(decorations, tr) {
    for (const effect of tr.effects) {
      if (effect.is(setAnchors)) return anchorDecorations(effect.value);
    }
    return decorations.map(tr.changes);
  },
  provide: (field) => EditorView.decorations.from(field),
});

/**
 * Position d'une citation dans le document : l'occurrence la plus proche de `lineHint`.
 * Sans citation (étape suivante), toute la ligne de `lineHint`, c'est-à-dire le titre
 * de la section en Markdown. `null` si la citation a disparu.
 */
export function locate(doc, quote, lineHint) {
  const hint = Math.min(Math.max(lineHint, 1), doc.lines);
  if (!quote) {
    const line = doc.line(hint);
    return { from: line.from, to: line.to };
  }
  const text = doc.toString();
  let best = null;
  for (let index = text.indexOf(quote); index !== -1; index = text.indexOf(quote, index + 1)) {
    const distance = Math.abs(doc.lineAt(index).number - hint);
    if (!best || distance < best.distance) best = { from: index, to: index + quote.length, distance };
  }
  return best && { from: best.from, to: best.to };
}

/**
 * Frappe par-dessus une sélection de plusieurs lignes : on remplace la sélection
 * nous-mêmes. Laissé au navigateur, Chrome fusionne les lignes en posant des
 * attributs `style` pour garder la coloration, que la CSP stricte bloque.
 */
function replaceMultilineSelection(event, view) {
  if (event.inputType !== 'insertText' || event.isComposing || !event.data) return false;
  const { state } = view;
  const crossesLines = state.selection.ranges.some(
    (range) => !range.empty && state.doc.lineAt(range.from).number !== state.doc.lineAt(range.to).number,
  );
  if (!crossesLines) return false;
  event.preventDefault();
  view.dispatch(state.update(state.replaceSelection(event.data), { userEvent: 'input.type', scrollIntoView: true }));
  return true;
}

function position(state) {
  const head = state.selection.main.head;
  const line = state.doc.lineAt(head);
  return { line: line.number, col: head - line.from + 1 };
}

/**
 * Une seule vue, montée dans un shadow root : CodeMirror y pose ses styles par
 * `adoptedStyleSheets`, que la CSP stricte autorise, au lieu de balises `<style>`
 * qu'elle bloque. Les variables du design system traversent le shadow root.
 *
 * Chaque onglet garde son propre état (document, historique, sélection).
 * `onChange()` à chaque modification, `onCursor({ line, col })` à chaque déplacement,
 * `onAnchor(remarkId)` au clic sur un passage cité.
 * Ctrl+S est géré par la page, pour fonctionner aussi hors de l'éditeur.
 */
export function createEditor(parent, { onChange, onCursor, onAnchor }) {
  const root = parent.attachShadow({ mode: 'open' });
  const view = new EditorView({ parent: root, root });

  function createState(doc, language, path) {
    return EditorState.create({
      doc,
      extensions: [
        lineNumbers(),
        highlightActiveLineGutter(),
        highlightSpecialChars(),
        history(),
        drawSelection(),
        dropCursor(),
        EditorState.allowMultipleSelections.of(true),
        indentOnInput(),
        syntaxHighlighting(highlight),
        bracketMatching(),
        rectangularSelection(),
        crosshairCursor(),
        highlightActiveLine(),
        highlightSelectionMatches(),
        keymap.of([
          ...defaultKeymap,
          ...searchKeymap,
          ...historyKeymap,
          indentWithTab,
        ]),
        theme,
        anchorsField,
        EditorView.domEventHandlers({
          click(event) {
            const anchor = event.target.closest?.('[data-remark]');
            if (anchor) onAnchor(anchor.getAttribute('data-remark'));
            return false;
          },
          beforeinput: replaceMultilineSelection,
        }),
        languageSupport(language, path),
        EditorView.updateListener.of((update) => {
          if (update.docChanged) onChange();
          if (update.docChanged || update.selectionSet) onCursor(position(update.state));
        }),
      ],
    });
  }

  return {
    createState,
    /** Affiche l'état d'un onglet. */
    show(state) {
      view.setState(state);
      onCursor(position(state));
    },
    get state() {
      return view.state;
    },
    focus: () => view.focus(),
    /** Affiche les ancres de l'onglet courant. */
    showAnchors(anchors) {
      view.dispatch({ effects: setAnchors.of(anchors) });
    },
    /** Place le curseur sur `pos` et l'amène à l'écran. */
    reveal(pos) {
      view.dispatch({ selection: { anchor: pos }, scrollIntoView: true });
      view.focus();
    },
  };
}
