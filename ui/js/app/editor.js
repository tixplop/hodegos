/* Éditeur CodeMirror 6 : langages du MVP, thème branché sur les tokens du design system,
   aperçu en direct du Markdown (md-live.js). */
import {
  EditorState,
  Compartment,
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
  markdownLanguage,
} from '/vendor/codemirror.js';
import { livePreview } from './md-live.js';

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
    // Base GFM : cases à cocher, tableaux, texte barré.
    case 'markdown': return markdown({ base: markdownLanguage });
    default: return [];
  }
}

const highlight = HighlightStyle.define([
  { tag: [t.keyword, t.controlKeyword, t.operatorKeyword, t.definitionKeyword, t.moduleKeyword, t.modifier, t.self], color: 'var(--syntax-keyword)' },
  { tag: [t.string, t.special(t.string), t.regexp, t.character, t.monospace], color: 'var(--syntax-string)' },
  { tag: [t.number, t.bool, t.null, t.atom, t.escape], color: 'var(--syntax-number)' },
  { tag: [t.comment, t.lineComment, t.blockComment, t.docComment, t.meta], color: 'var(--syntax-comment)', fontStyle: 'italic' },
  // Marques du Markdown (`#`, `*`, `|` d'un tableau…) : droites, l'italique les rend illisibles.
  { tag: t.processingInstruction, color: 'var(--syntax-comment)' },
  { tag: [t.typeName, t.className, t.namespace, t.standard(t.typeName)], color: 'var(--syntax-type)' },
  { tag: [t.function(t.variableName), t.function(t.propertyName), t.macroName], color: 'var(--syntax-function)' },
  { tag: t.heading, fontWeight: '700' },
  { tag: t.strong, fontWeight: '700' },
  { tag: t.emphasis, fontStyle: 'italic' },
  { tag: t.strikethrough, textDecoration: 'line-through' },
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
  // Ligne active sans fond, qui masquerait la sélection : liseré et numéro seulement.
  '.cm-activeLine': { backgroundColor: 'transparent' },
  '.cm-activeLineGutter': { backgroundColor: 'transparent', boxShadow: 'inset 3px 0 0 var(--accent)', color: 'var(--accent-ink)', fontWeight: '600' },
  // Sélection : accent à 18 % avec le focus (contraste vérifié, voir ui/styles/README.md),
  // surface-100 sans. Même sélecteur que le thème de base de CodeMirror pour l'emporter.
  '.cm-selectionBackground, ::selection': { backgroundColor: 'var(--surface-100)' },
  '&.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground': { backgroundColor: 'color-mix(in srgb, var(--accent) 18%, transparent)' },
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
  // Remarque de projet : marqueur carré cerclé, surlignage en pointillés (phase 9d).
  '.app-marker--project': { borderRadius: '3px', backgroundColor: 'var(--surface-0)', color: 'var(--ink)', boxShadow: 'inset 0 0 0 1.5px var(--ink)' },
  '.app-marker--project.app-marker--active': { backgroundColor: 'var(--accent)', color: 'var(--on-accent)', boxShadow: 'none' },
  '.app-anchor--project': { borderBottomStyle: 'dashed' },
  '.cm-textfield': { font: 'inherit', border: '1px solid var(--border-control)', borderRadius: 'var(--radius-sm)', backgroundColor: 'var(--surface-0)', color: 'var(--ink)' },
  '.cm-button': { font: 'inherit', border: '1px solid var(--border-control)', borderRadius: 'var(--radius-sm)', backgroundImage: 'none', backgroundColor: 'var(--surface-0)', color: 'var(--ink)' },
});

/* ---------- Ancres des remarques ---------- */

/** Remplace les ancres affichées : `[{ id, number, from, to, active }]`. */
const setAnchors = StateEffect.define();

/** Numéro de la remarque, juste après le passage cité (hd-marker). */
class MarkerWidget extends WidgetType {
  /** `project` : remarque de projet, marqueur de forme distincte (phase 9d). */
  constructor(number, active, project = false) {
    super();
    this.number = number;
    this.active = active;
    this.project = project;
  }

  eq(other) {
    return other.number === this.number && other.active === this.active && other.project === this.project;
  }

  toDOM() {
    const span = document.createElement('span');
    span.className = `app-marker${this.active ? ' app-marker--active' : ''}${this.project ? ' app-marker--project' : ''}`;
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
        class: `app-anchor${anchor.active ? ' app-anchor--active' : ''}${anchor.project ? ' app-anchor--project' : ''}`,
        attributes: { 'data-remark': anchor.id },
      }).range(anchor.from, anchor.to));
    }
    ranges.push(Decoration.widget({ widget: new MarkerWidget(anchor.number, anchor.active, anchor.project), side: 1 }).range(anchor.to));
  }
  return Decoration.set(ranges, true);
}

/** Les ancres suivent le texte pendant la frappe, jusqu'au prochain remplacement. */
const anchorsField = StateField.define({
  create: () => [],
  update(anchors, tr) {
    for (const effect of tr.effects) {
      if (effect.is(setAnchors)) return effect.value;
    }
    if (!tr.docChanged) return anchors;
    return anchors.map((anchor) => {
      const from = tr.changes.mapPos(anchor.from, 1);
      return { ...anchor, from, to: Math.max(from, tr.changes.mapPos(anchor.to, -1)) };
    });
  },
  provide: (field) => EditorView.decorations.from(field, anchorDecorations),
});

/** Ancres affichées (`[{ id, number, from, to, active }]`) ; `from === to` : marqueur seul. */
function anchorRanges(state) {
  return state.field(anchorsField, false) ?? [];
}

/** Nouveau chemin du document : son fichier a été déplacé depuis l'arbre. */
const setPath = StateEffect.define();

/** Chemin du document dans le projet, posé à la création de l'état d'un onglet. */
const pathField = StateField.define({
  create: () => '',
  update: (path, tr) => tr.effects.find((e) => e.is(setPath))?.value ?? path,
});

/** Aperçu en direct du Markdown : présent ou non dans l'état, sans toucher à l'historique. */
const previewSlot = new Compartment();

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
 * Petit éditeur de texte (posture d'un mentor, fichier brut), avec le même thème que
 * l'éditeur principal, monté lui aussi dans un shadow root. `onChange()` à chaque frappe.
 */
export function createTextEditor(parent, { onChange, label }) {
  const root = parent.attachShadow({ mode: 'open' });
  const view = new EditorView({ parent: root, root });
  return {
    /** Remplace le texte ; `readOnly` pour une lecture seule. */
    set(text, { language = 'markdown', readOnly = false } = {}) {
      view.setState(EditorState.create({
        doc: text,
        extensions: [
          lineNumbers(),
          highlightActiveLineGutter(),
          history(),
          drawSelection(),
          highlightActiveLine(),
          syntaxHighlighting(highlight),
          keymap.of([...defaultKeymap, ...historyKeymap, indentWithTab]),
          theme,
          EditorView.lineWrapping,
          EditorState.readOnly.of(readOnly),
          EditorView.contentAttributes.of({ 'aria-label': label }),
          EditorView.domEventHandlers({ beforeinput: replaceMultilineSelection }),
          languageSupport(language, ''),
          EditorView.updateListener.of((update) => {
            if (update.docChanged) onChange();
          }),
        ],
      }));
    },
    get value() {
      return view.state.doc.toString();
    },
    focus: () => view.focus(),
  };
}

/**
 * Une seule vue, montée dans un shadow root : CodeMirror y pose ses styles par
 * `adoptedStyleSheets`, que la CSP stricte autorise, au lieu de balises `<style>`
 * qu'elle bloque. Les variables du design system traversent le shadow root.
 *
 * Chaque onglet garde son propre état (document, historique, sélection).
 * `onChange()` à chaque modification, `onCursor({ line, col })` à chaque déplacement,
 * `onAnchor(remarkId)` au clic sur un passage cité, `onLink(path)` au clic sur un lien
 * relatif d'un tableau en aperçu.
 * Ctrl+S est géré par la page, pour fonctionner aussi hors de l'éditeur.
 */
export function createEditor(parent, { onChange, onCursor, onAnchor, onLink }) {
  const root = parent.attachShadow({ mode: 'open' });
  const view = new EditorView({ parent: root, root });
  const preview = livePreview({ anchorRanges, docPath: (state) => state.field(pathField, false) ?? '', onLink, onAnchor });

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
        pathField.init(() => path),
        previewSlot.of([]),
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
    /** `state` avec un nouveau chemin, historique et modifications gardés. */
    retarget(state, path) {
      const spec = { effects: setPath.of(path) };
      if (state !== view.state) return state.update(spec).state;
      view.dispatch(spec);
      return view.state;
    },
    /** Aperçu en direct (Markdown) ou source, pour l'état affiché. */
    setPreview(on) {
      if ((previewSlot.get(view.state) === preview) === on) return;
      view.dispatch({ effects: previewSlot.reconfigure(on ? preview : []) });
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
