# ADR 0003 — Notifications

Statut : proposé

## Contexte

Troc tourne sur un Raspberry Pi dans la loge du gardien, sans budget
d'hébergement. On envoie au plus une centaine de notifications par jour
(objet rendu, demande d'emprunt).

## Décision

Nous déployons un cluster Kafka de trois nœuds pour acheminer les
notifications, avec un service consommateur par type de message.

## Conséquences

- Les notifications sont découplées du reste de l'application.
