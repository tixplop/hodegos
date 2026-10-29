/* Éditeur CodeMirror 6 : langages du MVP, thème branché sur les tokens du design system. */
import {
  EditorState,
  EditorView,
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
  '.cm-textfield': { font: 'inherit', border: '1px solid var(--border-control)', borderRadius: 'var(--radius-sm)', backgroundColor: 'var(--surface-0)', color: 'var(--ink)' },
  '.cm-button': { font: 'inherit', border: '1px solid var(--border-control)', borderRadius: 'var(--radius-sm)', backgroundImage: 'none', backgroundColor: 'var(--surface-0)', color: 'var(--ink)' },
});

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
 * `onChange()` à chaque modification, `onCursor({ line, col })` à chaque déplacement.
 * Ctrl+S est géré par la page, pour fonctionner aussi hors de l'éditeur.
 */
export function createEditor(parent, { onChange, onCursor }) {
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
  };
}
