# 0001 — Stockage dans un fichier JSON, derrière un seul module

Statut : acceptée

## Contexte

La première version tourne sur un petit serveur partagé, pour un seul
immeuble. Les volumes sont faibles (quelques centaines d'objets et
d'emprunts). Nous voulons pouvoir passer à une base de données plus tard
sans réécrire l'application.

## Décision

Les données sont stockées dans `data/troc.json`. Seul le module
`src/stockage/depot.ts` lit et écrit ce fichier : les autres modules passent
par la classe `Depot` et n'importent jamais `node:fs`.

## Conséquences

- Changer de stockage ne touche qu'un module.
- Les écritures sont sérialisées par `Depot`, ce qui évite d'écraser une
  modification concurrente.
- Le fichier entier est relu à chaque requête : acceptable aux volumes visés.

## Alternatives

- SQLite : plus robuste, mais une dépendance native de plus à installer sur le
  serveur partagé.
- Un fichier par type de donnée : plus simple à lire à la main, mais les
  écritures croisées (objet et emprunt) ne seraient plus atomiques.
