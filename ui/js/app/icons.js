/* Icônes SVG en ligne : 16px, trait de 2px, couleur du texte. */
const SVG = 'http://www.w3.org/2000/svg';

const PATHS = {
  chevron: 'M6 4l4 4-4 4',
  close: 'M4 4l8 8M12 4l-8 8',
  'file-plus': 'M9 2H4.5A1.5 1.5 0 0 0 3 3.5v9A1.5 1.5 0 0 0 4.5 14h7a1.5 1.5 0 0 0 1.5-1.5V6zM9 2v4h4M8 8v4M6 10h4',
  'folder-plus': 'M2 4.5A1.5 1.5 0 0 1 3.5 3H6l1.5 1.5h5A1.5 1.5 0 0 1 14 6v5.5a1.5 1.5 0 0 1-1.5 1.5h-9A1.5 1.5 0 0 1 2 11.5zM8 7v4M6 9h4',
  branch: 'M5 3v7M5 14a1.5 1.5 0 1 0 0-3 1.5 1.5 0 0 0 0 3zM11 6a1.5 1.5 0 1 0 0-3 1.5 1.5 0 0 0 0 3zM11 6c0 3-6 2.5-6 5',
};

export function icon(name) {
  const svg = document.createElementNS(SVG, 'svg');
  svg.setAttribute('width', '16');
  svg.setAttribute('height', '16');
  svg.setAttribute('viewBox', '0 0 16 16');
  svg.setAttribute('fill', 'none');
  svg.setAttribute('stroke', 'currentColor');
  svg.setAttribute('stroke-width', '2');
  svg.setAttribute('stroke-linecap', 'round');
  svg.setAttribute('stroke-linejoin', 'round');
  svg.setAttribute('aria-hidden', 'true');
  const path = document.createElementNS(SVG, 'path');
  path.setAttribute('d', PATHS[name]);
  svg.append(path);
  return svg;
}
