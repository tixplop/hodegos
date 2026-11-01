# ADR 0002 — Montants en centimes

Statut : accepté

## Contexte

Les flottants arrondissent : 0,1 + 0,2 ne vaut pas 0,3. Sur un mois de
dépenses, les écarts finissent par se voir dans le solde. L'affichage d'un
montant (virgule décimale, symbole €, signe) doit être le même partout.

## Décision

Les montants sont des entiers `i64` exprimés en centimes, enveloppés dans le
type `Montant` de `src/montant.rs`. Ce module est le seul à convertir un
montant en texte, par sa fonction `formater` ; les autres modules l'appellent.

## Conséquences

- Les sommes sont exactes.
- Changer le format d'affichage (espace avant €, séparateur des milliers) ne
  touche qu'un fichier.
- La lecture d'un relevé doit convertir « 12,34 » en 1234 centimes.

## Alternatives

- `f64` : rejeté pour les arrondis.
- Une crate décimale (`rust_decimal`) : rejetée, une dépendance de plus pour
  des opérations limitées à l'addition.
