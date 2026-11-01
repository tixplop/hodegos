# Budget — vision

Démarré le 4 mars 2026

## Idée

Savoir, à la fin de chaque mois, où est passé mon argent, sans tableur à tenir
à la main ni application qui demande l'accès à mon compte bancaire.

## Pour qui

Une personne seule ou un couple qui suit ses dépenses mensuelles, télécharge
déjà ses relevés en CSV depuis le site de sa banque et veut un rapport lisible
en deux minutes, dans un terminal.

## Problème

Les relevés bancaires sont des listes de libellés obscurs (« PRLV SEPA EDF »,
« CB CARREFOUR 12/03 »). Les regrouper par catégorie à la main prend une soirée
par mois, et on finit par abandonner.

## Contraintes

- Hors ligne, données locales uniquement.
- Un relevé mal formé ne doit jamais faire perdre l'import des autres lignes.
- Rust, un seul binaire.

## Première version

- Import d'un relevé CSV bancaire (date, libellé, montant).
- Catégories attribuées par mots-clés du libellé, avec des règles par défaut
  que l'on peut compléter.
- Rapport mensuel : total par catégorie, dépenses, revenus, solde du mois.

## Hors première version

- Plusieurs comptes et virements entre comptes.
- Budgets prévisionnels et alertes de dépassement.
- Graphiques, export PDF, interface graphique.
- Synchronisation avec la banque.
