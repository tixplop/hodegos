# ADR 0008 — Les remarques suivent un déplacement fait depuis Hodegos

Statut : accepté · 31 octobre 2026

## Contexte

Les remarques, les choix forcés et les passes d'ouverture sont rattachés au
chemin du fichier (`docs/cadrage.md`, format d'une remarque). Jusqu'ici, aucun
fichier ne changeait de chemin depuis Hodegos : un déplacement fait ailleurs
(terminal, autre éditeur, git) apparaît comme une suppression suivie d'une
création.

La sous-phase 8g permet de déplacer un fichier ou un dossier par
glisser-déposer dans l'arbre. Dans ce cas, l'intention est claire : c'est le
même fichier, à un autre endroit.

## Décision

- Un déplacement fait **depuis Hodegos** (`move_entry`) fait suivre, pour
  chaque fichier déplacé (descendants d'un dossier compris), ses remarques
  (`remarks.json`), ses choix forcés (`overrides.json`) et sa passe
  d'ouverture (`opened.json`) au nouveau chemin.
- Seuls les fichiers présents sur le disque au moment du déplacement sont
  concernés. Les remarques d'un fichier absent (supprimé hors de Hodegos, donc
  gardées par l'ADR 0006) restent sous l'ancien chemin.
- La mise à jour des trois fichiers de données suit le renommage ; si elle
  échoue, le renommage est annulé et les données reviennent à leur état
  d'avant.
- Le déplacement est un simple renommage sur le disque (`fs::rename`), sans
  `git mv` : rien n'est indexé. Git montre l'ancien chemin supprimé et le
  nouveau non suivi jusqu'au prochain `git add`.
- Un déplacement fait **hors de Hodegos** reste une suppression suivie d'une
  création : remarques gardées sous l'ancien chemin (ADR 0006).

## Alternatives écartées

- `git mv` : indexerait à la place de la personne, et échouerait pour un
  fichier non suivi.
- Faire suivre aussi les remarques des fichiers absents (par préfixe de
  chemin) : elles accompagneraient un fichier qui, s'il revient par git,
  reviendra à l'ancien chemin.
- Détecter les déplacements externes (même contenu disparu et apparu) :
  heuristique fragile, sans demande.

## Conséquences

- `docs/cadrage.md` renvoie à cet ADR depuis le format d'une remarque.
- Le déplacement se défait en glissant en sens inverse : pas de confirmation.
