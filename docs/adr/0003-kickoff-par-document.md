# ADR 0003 — Kick-off d'un projet par un document accompagné

Statut : accepté · 30 octobre 2026

## Contexte

La maquette 01 montre un écran de kick-off : idée reformulée, questions de
cadrage, trois points de départ. Le MVP ne couvrait que des fichiers existants ;
sur un projet vide, aucun mentor ne se manifestait.

## Décision

Le kick-off est un document, `docs/vision.md`, créé par `hodegos new` et suivi par
un Mentor produit déclenché sur ce chemin. Il réutilise le casting, les remarques
ancrées et les paliers. Deux ajouts génériques le rendent possible : la passe à
l'ouverture (`on_open`) et les remarques `next_step` sans citation.

## Alternatives écartées

- Un écran de kick-off dédié : nouvel affichage et nouveau dialogue, à construire
  avant d'avoir validé la boucle principale.
- Des points de départ générés dès cette phase : nouveau type de sortie, et risque
  fort de « faire à la place » ; reportés à une phase dédiée avec son propre contrat.

## Conséquences

- La vision du projet reste un fichier que l'utilisateur possède et versionne.
- `on_open` et les `next_step` sans citation servent aussi à d'autres mentors.
- Les trois points de départ restent à concevoir.
