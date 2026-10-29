# ADR 0001 — Stockage des données

Statut : proposé

## Contexte

Troc sert un seul immeuble, de 40 à 200 logements. Les voisins consultent le
catalogue depuis leur téléphone, souvent dans la cage d'escalier ou à la cave.
Les données doivent rester disponibles hors ligne.

## Décision

Nous stockons toutes les données dans une base PostgreSQL hébergée, interrogée
par une API REST.

## Conséquences

- Une seule base à sauvegarder.
- Chaque membre a un profil avec son nom et son numéro d'appartement.
