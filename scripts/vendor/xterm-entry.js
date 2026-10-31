// Point d'entrée du paquet ui/vendor/xterm.js (terminal, phase 8f), chargé à la première
// ouverture d'un terminal. La feuille de style est du texte : la CSP refuse `fetch`, elle
// est adoptée par le shadow root du terminal.
export { Terminal } from '@xterm/xterm';
export { FitAddon } from '@xterm/addon-fit';
export { CanvasAddon } from '@xterm/addon-canvas';
export { default as xtermCss } from '@xterm/xterm/css/xterm.css';
