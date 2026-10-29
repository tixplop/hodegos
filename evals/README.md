# Évaluation des mentors

`hodegos eval evals/` rejoue chaque cas avec les vrais modèles configurés et
produit un rapport. Ce n'est pas un test automatique : il consomme du budget et
se lance à la main (phase 6, puis à chaque changement de prompt ou de modèle).

## Format d'un cas

Un dossier par cas dans `evals/cases/`, contenant :

- le fichier source, à son chemin relatif réel (le chemin compte pour le casting) ;
- `case.toml`, qui décrit l'attente.

```toml
title = "Comparaison de dates par référence"
file = "src/emprunts/reserver.ts"
section = "fn:reserver"            # identifiant de section attendu
expected_mentor = "mentor-langage" # titulaire attendu après casting
levels = ["indice", "question", "piste", "exemple"]

[expect]
# Au moins un de ces motifs doit apparaître dans les remarques (insensible à la casse).
must_mention = ["référence", "getTime", "valeur"]
# Aucun de ces motifs ne doit apparaître : ce serait donner la solution.
must_not = ["e.debut.getTime() === debut.getTime()", "getTime() ==="]
```

## Rapport

Pour chaque cas et chaque palier : mentor retenu, nombre de remarques, citation
valide ou non, palier respecté, `must_mention` atteint, `must_not` violé, coût et
durée. Le rapport se termine par les taux globaux par modèle.

Seuil pour retenir un modèle : zéro violation de `must_not`, et `must_mention`
atteint dans au moins 70 % des cas.

## Cas fournis

- `cases/01-dates-par-reference/` : TypeScript, Mentor du langage.
- `cases/02-adr-sans-alternatives/` : Markdown, Architecte.

Objectif phase 6 : vingt cas, couvrant les deux mentors et les cinq langages.
