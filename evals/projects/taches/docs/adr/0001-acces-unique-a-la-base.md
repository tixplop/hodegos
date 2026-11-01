# 0001 — Accès unique à la base

Statut : acceptée

## Contexte

Les tâches sont stockées dans une base SQLite locale. Plusieurs modules en ont
besoin : la ligne de commande, le planning, les rappels, les statistiques,
l'export. Si chacun ouvre sa connexion et écrit ses requêtes, le schéma se
retrouve dispersé, et chaque changement de table oblige à relire tout le code.

## Décision

Tout accès à la base SQLite passe par `taches/depot.py` (classe `Depot`).
Aucun autre module n'ouvre de connexion ni n'écrit de SQL : ils demandent au
dépôt des objets `Tache` et lui confient leurs modifications.

## Conséquences

- Le schéma et les requêtes sont en un seul endroit, testés par
  `tests/test_depot.py`.
- Les autres modules se testent avec des listes de `Tache`, sans base.
- Le dépôt grossit à chaque nouveau besoin de lecture : il faut accepter d'y
  ajouter des méthodes plutôt que de contourner.
- Si le dépôt dépasse quelques centaines de lignes, on le découpera par usage.

## Alternatives

- Un ORM (SQLAlchemy) : écarté, trop lourd pour quatre tables et un seul
  utilisateur de la base.
- Connexion libre dans chaque module : écartée pour les raisons du contexte.
