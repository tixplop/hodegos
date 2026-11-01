# ADR 0009 — Connaissance du projet calculée sans modèle

Statut : accepté · 1er novembre 2026

## Contexte

Un mentor ne voit qu'une section, 20 lignes autour et le diff. Il ne peut pas
voir qu'une décision d'architecture interdit ce qu'il relit, ni qu'un autre
fichier fait déjà la même chose. Il faut une connaissance globale du projet,
tenue à jour à chaque modification, sur laquelle tous les mentors s'appuient.

Contraintes : budget de 10 $ par mois ; maîtrise de ce qui part vers le modèle
(rien d'ignoré par git, aucun secret) ; aucun appel réseau dans les tests.

## Décision

- Une **carte du projet** est calculée par Hodegos, sans appel au modèle :
  vision, décisions (ADR), arborescence, README, dépendances, signatures des
  éléments exportés (tree-sitter, jamais les corps), tests. Elle est plafonnée
  en jetons estimés (8 000 par défaut, réglable), raccourcie par priorité, et
  recalculée à chaque modification enregistrée.
- Un **voisinage** par fichier est calculé de la même façon : imports,
  importateurs, test associé, documents qui citent le fichier, même dossier ;
  les fichiers de code voisins sont envoyés en signatures.
- Chaque mentor de section choisit son contexte : `section`, `neighborhood`
  ou `project`. Les mentors de projet reçoivent toujours la carte.
- La carte s'affiche telle qu'envoyée ; chaque remarque garde la liste des
  blocs envoyés (sans contenu).

## Alternatives écartées

- **Index vectoriel** (embeddings et recherche sémantique) : appels réseau et
  coût à chaque modification, stockage en plus, et sélection difficile à
  expliquer.
- **Lecture de fichiers par le modèle** (outils) : nombre d'appels et coût
  imprévisibles ; ce qui part n'est plus décidé par Hodegos.
- **Fiche projet rédigée par le modèle** : coût à chaque modification, et une
  erreur du résumé serait reprise par tous les mentors jusqu'à la suivante.

## Conséquences

- La carte ne contient ni corps de fonction ni texte complet, sauf la vision
  et les décisions : un mentor qui a besoin d'un fichier entier doit l'avoir
  dans ses fichiers clés (mentors de projet) ou dans son voisinage.
- Les jetons sont estimés (4 caractères par jeton) : le coût affiché avant une
  revue reste une estimation.
- Le prompt grossit pour les mentors qui choisissent le voisinage ou le
  projet : leur passe coûte plus cher, ce que l'évaluation (9f) mesure.
