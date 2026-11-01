# 0002 — Périodes de réservation en jours entiers

Statut : acceptée

## Contexte

Les voisins réservent « du samedi au lundi », pas à l'heure près.

## Décision

Une période va d'un jour de début à un jour de fin inclus, stockés à minuit
UTC. La validation d'une période (fin après le début, durée maximale) est
faite par `src/emprunts/validation.ts`, utilisé par tous les modules qui
manipulent des périodes.

## Conséquences

- Deux réservations se chevauchent si l'une commence avant la fin de l'autre.
- Les heures saisies sont ignorées.

## Alternatives

- Créneaux horaires : plus fin, mais personne ne prête une raclette pour deux heures.
