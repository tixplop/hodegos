# Recherche dans le catalogue

## Objectif

Permettre aux voisins de trouver un objet par son nom.

## Solution

Un cluster Elasticsearch indexe le catalogue en temps réel. Chaque ajout
d'objet déclenche une réindexation, et la recherche passe par une API
dédiée.
