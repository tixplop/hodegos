# ADR 0001 — Erreurs typées

Statut : accepté

## Contexte

Les relevés viennent de banques différentes et sont souvent abîmés : séparateur
inattendu, date au mauvais format, montant avec une espace insécable. La vision
exige qu'une ligne mal formée ne fasse pas perdre l'import des autres. Un
programme qui s'arrête sur la première anomalie ne répond pas à ce besoin.

## Décision

Toutes les fonctions faillibles renvoient `Result<_, BudgetError>`, le type
d'erreur unique défini dans `src/erreur.rs`. Aucun `unwrap` ni `expect` hors
des tests : l'import d'un relevé mal formé n'arrête jamais le programme, il
signale la ligne fautive et continue.

## Conséquences

- L'appelant décide quoi faire d'une ligne refusée (l'ignorer, la compter).
- Chaque nouvelle cause d'échec ajoute une variante à `BudgetError`.
- Le code de parsing est un peu plus long qu'avec des `unwrap`.

## Alternatives

- `anyhow` partout : plus court, mais l'import ne peut plus distinguer une
  ligne fautive d'un fichier illisible.
- Paniquer et relancer : inacceptable, on perd tout l'import en cours.
