# ADR 0004 — Cache du catalogue

Statut : proposé

## Contexte

La page du catalogue met deux secondes à s'afficher aux heures de pointe. Les
voisins ajoutent et retirent des objets toute la journée.

## Décision

Nous ajoutons un cache Redis devant l'API du catalogue. Nous verrons plus tard
s'il faut l'invalider.

## Conséquences

- La page du catalogue s'affiche plus vite.
