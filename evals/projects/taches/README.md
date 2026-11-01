# Tâches

Un petit outil en ligne de commande pour noter les tâches ménagères du foyer,
les planifier sur la semaine et recevoir chaque matin le rappel de ce qui est à
faire. Les données restent sur la machine, dans une base SQLite
(`~/.local/share/taches/taches.db` par défaut).

## Utilisation

```sh
taches ajouter "Sortir les poubelles" --le 2026-11-03 --recurrence hebdomadaire
taches semaine
taches rappels
```
