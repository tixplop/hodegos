# Maquettes de la phase 9

Écrans de référence de la phase 9 (`docs/phase9.md`), en HTML statique, avec le
design system de `ui/styles/`. Mêmes règles que `docs/maquettes/README.md`.
La spécification prime en cas d'écart.

| Fichier | Écran | Sous-phase |
| --- | --- | --- |
| 09-1-contexte-section.html | Remarque de section éclairée par le voisinage, « Voir aussi », contexte de la passe | 9b |
| 09-2-vue-projet.html | Vue Projet : remarques de projet, mentors, revue demandée, remarque à réévaluer | 9c, 9d |
| 09-3-carte-projet.html | Carte du projet telle qu'envoyée, blocs, plafond, exclusions | 9a |
| 09-4-mentor-portees.html | Éditer un mentor : une portée ou les deux | 9e |
| 09-5-reglages-revues.html | Réglages : revues du projet, rythme, plafonds | 9d, 9e |

Les valeurs affichées (jetons, coûts, durées) sont des exemples.

## Cas d'usage

### Mentor de section avec contexte

- **UC-9.1** Remarque éclairée par le voisinage : le Mentor du langage voit que
  `reserver.ts` lit les emprunts par le dépôt et qu'`annuler.ts` ouvre le fichier
  lui-même ; il pose la question, sans dire quoi changer.
- **UC-9.2** « Voir aussi » : une remarque de section renvoie à 3 autres
  fichiers au plus ; elle reste ancrée dans le fichier courant.
- **UC-9.3** « Contexte de cette passe » : ce qui est parti vers le modèle,
  bloc par bloc, en jetons estimés ; lien vers la carte.

### Mentors de projet

- **UC-9.4** Demander une revue : mentors cochables, contexte envoyé, coût et
  durée estimés, budget restant.
- **UC-9.5** Revue automatique : à la création du projet, à l'ouverture si des
  fichiers ont changé depuis la dernière revue, après des modifications une fois
  l'édition au calme. Sans dialogue, comptée dans le budget.
- **UC-9.6** Traiter une remarque de projet : filtres, paliers, réponse,
  ignorer, résolue ; « Étape suivante » sans citation.
- **UC-9.7** Vue Projet d'abord (compteur, repère P dans l'arbre), puis le
  fichier une fois ouvert : passage surligné, carte courte qui renvoie à la vue
  Projet.

### Carte du projet

- **UC-9.8** Voir la carte telle qu'envoyée, son poids et sa date ; recalculée à
  chaque modification enregistrée, dans Hodegos ou ailleurs, sans modèle.
- **UC-9.9** Retirer un bloc ou exclure un chemin (réglage personnel).
- **UC-9.10** Régler le plafond (8 000 jetons par défaut, par projet).

### Portées et réglages

- **UC-9.11** Un mentor couvre la section, le projet ou les deux ; posture
  commune, réglages propres à chaque portée.
- **UC-9.12** Régler les revues du projet : déclencheurs, calme, intervalle,
  revues par jour, plafonds de contexte.

### Passages disparus

- **UC-9.13** Remarque liée à un passage : le passage disparaît, la remarque
  aussi.
- **UC-9.14** Remarque liée au projet : « À réévaluer » ; le mentor la relit à
  l'échelle du projet, la maintient avec de nouvelles citations ou la lève en
  disant pourquoi.
