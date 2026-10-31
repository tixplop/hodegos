/* Confirmation d'une suppression (maquette 08-1b) : dit ce que git peut restaurer. */
import { confirmDialog } from './dialog.js';

function plural(count, one, many) {
  return count === 1 ? `1 ${one}` : `${count} ${many}`;
}

function paragraph(...parts) {
  const p = document.createElement('p');
  p.append(...parts);
  return p;
}

function muted(text) {
  const p = paragraph(text);
  p.className = 'hd-muted';
  return p;
}

function badge(text, variant) {
  const span = document.createElement('span');
  span.className = `hd-badge hd-badge--${variant}`;
  span.textContent = text;
  return span;
}

function code(text) {
  const span = document.createElement('span');
  span.className = 'hd-mono';
  span.textContent = text;
  return span;
}

/**
 * `preview` : charge de `delete_preview` ; `repo` : le projet a un dépôt git ;
 * `tabs` et `dirty` : onglets ouverts concernés, dont non enregistrés.
 */
export function confirmDeletion(preview, { repo, tabs, dirty, returnFocus }) {
  const name = preview.path.split('/').pop();
  const isDir = preview.kind === 'dir';
  const lost = !repo || preview.untracked > 0 || preview.ignored > 0;
  const body = [];

  if (!isDir) {
    if (!repo) {
      body.push(paragraph(badge('Définitif', 'danger'), " Ce projet n'a pas de dépôt git : rien ne permettra de récupérer ce fichier."));
    } else if (preview.untracked > 0) {
      body.push(paragraph(badge('Définitif', 'danger'), " Ce fichier n'est pas suivi par git : rien ne permettra de le récupérer."));
    } else if (preview.modified > 0) {
      body.push(paragraph(badge('Modifications perdues', 'warning'), ' Ce fichier est suivi par git : ', code('git restore'), ' récupère la dernière version commitée, mais les modifications non commitées sont perdues.'));
    } else {
      body.push(paragraph(badge('Récupérable', 'success'), ' Ce fichier est suivi par git, sans modification : ', code('git restore'), ' le récupère.'));
    }
  } else {
    const head = preview.files === 0 ? 'Ce dossier est vide.' : `Ce dossier contient ${plural(preview.files, 'fichier', 'fichiers')}.`;
    if (!repo && preview.files > 0) {
      body.push(paragraph(badge('Définitif', 'danger'), ` ${head} Ce projet n'a pas de dépôt git : rien ne permettra de les récupérer.`));
    } else if (lost) {
      body.push(paragraph(badge('Définitif', 'danger'), ` ${head}`));
    } else if (preview.modified > 0) {
      body.push(paragraph(badge('Modifications perdues', 'warning'), ` ${head}`));
    } else {
      body.push(paragraph(badge('Récupérable', 'success'), ` ${head} Tous sont suivis par git, sans modification : `, code('git restore'), ' les récupère.'));
    }
    if (repo && preview.untracked > 0) {
      body.push(paragraph(`Dont ${plural(preview.untracked, 'non suivi', 'non suivis')} : rien ne permettra de les récupérer.`));
    }
    if (repo && preview.modified > 0) {
      body.push(paragraph(`Dont ${plural(preview.modified, 'modifié', 'modifiés')} : `, code('git restore'), ' récupère la dernière version commitée, les modifications non commitées sont perdues.'));
    }
    if (preview.ignored === 1) {
      body.push(paragraph("Dont 1 fichier ignoré par git, absent de l'arbre : il est supprimé aussi, sans retour."));
    } else if (preview.ignored > 1) {
      body.push(paragraph(`Dont ${preview.ignored} fichiers ignorés par git, absents de l'arbre : ils sont supprimés aussi, sans retour.`));
    }
  }

  if (tabs > 0) {
    let text = 'Il est ouvert dans un onglet, qui sera fermé.';
    if (isDir) {
      text = tabs === 1
        ? '1 fichier est ouvert dans un onglet, qui sera fermé.'
        : `${tabs} fichiers sont ouverts dans des onglets, qui seront fermés.`;
    }
    body.push(muted(text));
  }
  if (dirty > 0) {
    body.push(muted(dirty === 1 && !isDir
      ? 'Vos modifications non enregistrées dans cet onglet seront perdues.'
      : `Les modifications non enregistrées de ${plural(dirty, 'onglet', 'onglets')} seront perdues.`));
  }

  return confirmDialog({
    title: [isDir ? 'Supprimer le dossier ' : 'Supprimer ', code(name), ' ?'],
    body,
    confirmLabel: lost ? 'Supprimer définitivement' : 'Supprimer',
    returnFocus,
  });
}
