# ADR 0010 — Mentors de projet, revues automatiques et réévaluation

Statut : accepté · 1er novembre 2026

## Contexte

Certaines remarques ne tiennent dans aucune section : une décision contournée
par deux fichiers, une vision dépassée par ce qui est construit, un module sans
test. Il faut des mentors qui relisent le projet entier, sans perdre la règle
« pas de solution » ni le contrôle du budget.

## Décision

- Un mentor déclare ses **portées** : `scopes: [section]`, `[project]` ou
  `[section, project]`. Un seul fichier, une posture commune ; le cadre commun
  reste hors d'atteinte pour les deux portées.
- Un mentor de projet relit lors d'une **revue** : carte, fichiers clés, diff
  depuis sa dernière revue, ses remarques ouvertes.
- Les revues partent **d'elles-mêmes** : à la création du projet, à son
  ouverture si des fichiers ont changé depuis la dernière revue, après des
  modifications une fois l'édition au calme (et, si on l'active, après un
  commit). Calme, intervalle minimal, plafond quotidien et déclencheurs se
  règlent ; les revues automatiques comptent dans le budget et s'arrêtent à son
  plafond. Une revue demandée annonce son coût estimé.
- Une remarque de projet est liée à un **passage** (une citation) ou au
  **projet** (0 à 3 citations dans des fichiers différents). La garde « pas de
  solution » s'applique à chaque section citée.
- Elles s'affichent d'abord dans la vue Projet, puis dans le fichier cité une
  fois ouvert.
- Passage disparu : une remarque liée au passage disparaît ; une remarque liée
  au projet passe « à réévaluer », et son mentor la maintient (nouvelles
  citations) ou la lève (raison dans le fil).
- Les mentors livrés ne reçoivent la portée `project` qu'après une évaluation
  sur des projets d'exemple (sous-phase 9f).

## Alternatives écartées

- **Deux fichiers par mentor** (Architecte et Architecte du projet) : postures
  qui divergent, deux mentors à personnaliser pour un même rôle.
- **Revues seulement à la demande** : la connaissance des mentors se
  périmerait entre deux demandes ; ce n'est pas l'usage voulu.
- **Revue à chaque enregistrement** : coût proportionnel à la frappe, et
  remarques sur un état transitoire.
- **Détacher toute remarque dont un passage disparaît** : une remarque sur le
  projet reste souvent vraie quand un seul fichier change.

## Conséquences

- Nouvelles données personnelles par projet : `map.json`, `context.json`,
  `project_remarks.json`, `reviews.json`.
- Le coût des revues automatiques est la principale incertitude : l'évaluation
  (9f) le mesure sur une journée type, et les valeurs par défaut sont revues
  avant d'activer les mentors livrés.
- Le contrat des remarques s'étend (`about`, `quotes`, `see_also`,
  `to_review`) ; `docs/cadrage.md` le décrit.
