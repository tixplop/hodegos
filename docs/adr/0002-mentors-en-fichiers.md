# ADR 0002 — Mentors décrits en fichiers Markdown, à trois niveaux

Statut : accepté · 29 octobre 2026

## Contexte

Les mentors doivent s'adapter à la nature du contenu, être livrés avec le produit
et rester modifiables sans recompiler.

## Décision

Un mentor est un fichier Markdown (frontmatter YAML + posture en texte libre). Les
fichiers sont lus à trois niveaux : livrés dans le binaire, personnels
(`~/.config/hodegos/mentors/`), projet (`.hodegos/mentors/`) ; à `id` égal, le plus
précis l'emporte. Le casting est déterministe (score sur chemins, langages, titres,
mots-clés), sans appel au modèle.

## Alternatives écartées

- Mentors codés en Rust : toute modification exige une recompilation.
- Casting par le modèle : plus lent, plus coûteux, non reproductible et difficile à tester.

## Conséquences

- Le cadre commun des prompts reste dans le code et n'est pas modifiable par
  fichier : un mentor personnalisé ne peut pas lever la règle « pas de solution ».
- Commandes `mentors list` et `mentors eject`, rechargement à chaud et
  `enabled: false` sont nécessaires pour que la personnalisation soit confortable.
