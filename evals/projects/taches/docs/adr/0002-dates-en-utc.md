# 0002 — Dates en UTC

Statut : acceptée

## Contexte

Les échéances et les dates de réalisation sont comparées, triées et
regroupées par semaine. Avec des heures locales, le passage à l'heure d'été
crée des doublons et des trous, et une base copiée sur une autre machine ne
dit plus la même chose.

## Décision

Toutes les dates sont stockées en UTC, au format ISO 8601 avec décalage
(`2026-11-03T07:00:00+00:00`). `Depot` ne renvoie que des `datetime` munis du
fuseau UTC. La conversion en heure locale se fait uniquement à l'affichage,
dans `taches/affichage.py`.

## Conséquences

- Les comparaisons entre dates stockées sont sûres.
- Tout code qui compare une date stockée à « aujourd'hui » doit d'abord
  savoir de quel aujourd'hui il parle.
- Les tests fixent des dates UTC explicites.

## Alternatives

- Heure locale naïve : écartée, ambiguë au changement d'heure.
- Horodatage entier (secondes) : écarté, illisible dans la base.
