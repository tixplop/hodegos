# ADR 0006 — Remarques supprimées avec le fichier supprimé depuis Hodegos

Statut : accepté · 31 octobre 2026

## Contexte

Jusqu'ici, un fichier supprimé ne détache pas ses remarques (`docs/cadrage.md`,
format d'une remarque) : une suppression suivie d'une recréation, comme un
changement de branche, les aurait perdues. Tous les fichiers étaient supprimés
hors de Hodegos.

La phase 8 permet de supprimer un fichier ou un dossier depuis l'arbre. Dans ce
cas, l'intention est claire : la personne ne veut plus de ce fichier.

## Décision

- Une suppression faite **depuis Hodegos** (`delete_entry`) retire, pour chaque
  fichier supprimé, ses remarques (`remarks.json`), ses choix forcés
  (`overrides.json`) et sa passe d'ouverture (`opened.json`).
- Si le fichier revient ensuite (git, terminal, autre éditeur), il repart sans
  remarque. C'est accepté.
- Une suppression faite **hors de Hodegos** garde la règle actuelle : remarques
  conservées, pour survivre aux changements de branche.

## Alternatives écartées

- Tout conserver : des remarques orphelines s'accumulent pour des fichiers
  abandonnés, et reviennent si un fichier de même nom est créé plus tard.
- Archiver les remarques pour les restaurer : complexité sans demande.

## Conséquences

- `docs/cadrage.md` précise la règle (suppression depuis Hodegos ou hors de
  Hodegos).
- Le dialogue de suppression n'a pas à parler des remarques : leur perte suit
  celle du fichier.
