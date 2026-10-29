// Point d'entrée du paquet ui/vendor/codemirror.js : seulement ce dont l'éditeur se sert.
export { EditorState, Compartment } from '@codemirror/state';
export {
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
} from '@codemirror/view';
export {
  HighlightStyle,
  syntaxHighlighting,
  indentOnInput,
  bracketMatching,
  indentUnit,
} from '@codemirror/language';
export { defaultKeymap, history, historyKeymap, indentWithTab } from '@codemirror/commands';
export { searchKeymap, highlightSelectionMatches } from '@codemirror/search';
export { tags } from '@lezer/highlight';
export { rust } from '@codemirror/lang-rust';
export { javascript } from '@codemirror/lang-javascript';
export { python } from '@codemirror/lang-python';
export { markdown } from '@codemirror/lang-markdown';
