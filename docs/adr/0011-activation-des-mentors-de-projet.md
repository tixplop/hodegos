# ADR 0011 — Activation des mentors de la phase 9 et rythme par défaut

Statut : accepté · 1er novembre 2026

## Contexte

Les mentors livrés de la phase 9 (Architecte et Mentor produit en portées
section et projet, Mentor du langage avec le voisinage, Testeur nouveau)
attendaient l'évaluation 9f : seuils de l'ADR 0003, plus 80 % de citations
valides, et un coût qui tienne dans 10 $ par mois avec les réglages par défaut.

## Évaluation

Trente-sept cas, dont quinze sur trois projets d'exemple (`evals/projects/`) :
neuf revues, trois cas de voisinage, trois réévaluations. Rapports :
`evals/reports/2026-11-01-…-gemini-4-argon.md` (complet), `…-2.md` (niveau fast
rejoué), `…-3.md` (cas 26 relancé après une panne du fournisseur).

- `strong` · `google/gemini-4-argon` : notion 14/16 (88 %), citations valides
  84 %, aucune solution proposée. Retenu.
- `fast` · `deepseek/deepseek-v4.1-flash-1017` : au premier passage, notion
  19/21 mais une solution au cas 07 (palier exemple, « cas voisin » qui
  contenait le comparateur attendu) : non retenu. Après la précision du cadre
  commun ci-dessous : notion 18/21 (86 %), citations valides 100 %, aucune
  solution proposée. Retenu.
- Voisinage : 3/3 ; au cas 36, seule la passe avec le voisinage fait émerger la
  notion. Réévaluations : 3/3.
- Coût d'une revue : 0,0143 $ (Architecte), 0,0103 $ (Mentor produit), 0,0005 $
  (Testeur) ; réévaluation 0,0085 $.
- Journée type (40 enregistrements sur six heures, deux ouvertures, quatre
  commits, deux réévaluations, rejouée avec le planificateur) : 9, 9 et 8 revues,
  0,26 $ par jour, **5,80 $ par mois** sur 22 jours ouvrés.

## Décision

- Les brouillons de `evals/mentors/` sont installés dans `mentors/` : les
  mentors de projet livrés relisent d'eux-mêmes (création, ouverture,
  modifications) ; le Mentor du langage reçoit le voisinage.
- Le cadre commun précise le palier exemple : le cas voisin illustre la notion
  sans contenir l'appel, l'argument ou l'expression qui corrigerait la section.
  La règle « pas de solution » est renforcée, jamais levée.
- Les réglages par défaut sont **confirmés** : calme 3 min, intervalle 20 min,
  12 revues par jour et par mentor, déclencheurs création, ouverture et
  modifications ; plafonds de la carte (8 000), du voisinage (3 000) et des
  fichiers clés (12 000) inchangés.
- Deux cas ont été corrigés après avoir vu les résultats, parce que les remarques
  étaient justes et l'attente trop étroite : le cas 35 n'exige plus
  `about = "project"`, le cas 25 accepte une citation de la vision et la tournure
  « aucun nom » de test.

## Conséquences

- Un projet ouvert pour la première fois déclenche une revue de chaque mentor de
  projet (environ 0,025 $) ; le budget mensuel fait toujours foi.
- Le niveau strong a été évalué avant la précision du palier exemple ; elle ne
  fait que restreindre ce qui est permis. À la prochaine réévaluation du niveau
  (changement de modèle ou d'effort), il sera rejoué avec le cadre actuel.
- Une remarque liée au projet dont la citation est écartée reste affichée sans
  citation ; la garde ne voit alors aucun identifiant à comparer. Aucun cas ne
  l'a fait divulguer, mais c'est un point à surveiller.
- Le Testeur, rapide et peu coûteux, ne trouve pas toujours (cas 30) : sa posture
  est à affiner après usage.
