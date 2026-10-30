# ADR 0003 — Choix des modèles rapide et fort

Statut : proposé · 30 octobre 2026 · en attente des rapports d'évaluation

## Contexte

Hodegos appelle deux modèles via OpenRouter : le modèle `fast` pour le Mentor du
langage, qui répond à chaque enregistrement, et le modèle `strong` pour
l'Architecte, plus rare et plus exigeant (ADR, documents de conception).

Contraintes :

- sorties structurées (`structured_outputs`) acceptées, pour le contrat JSON des
  remarques ;
- aucune solution divulguée : le modèle est jugé sur ce qu'il propose, brouillons
  écartés par la garde compris (la garde est un filet, pas une qualité du modèle) ;
- notion attendue (`must_mention`) atteinte dans 70 % des cas au moins ;
- remarques affichées en moins de 10 secondes après un enregistrement ;
- dépense compatible avec un plafond de 10 USD par mois.

Méthode : `hodegos eval evals/` sur les vingt cas (15 pour le modèle `fast`,
5 pour le modèle `strong`), un rapport par candidat dans `evals/reports/`.

## Candidats et résultats

À remplir à partir des rapports. Un candidat « non concluant » (panne, arrêt sur
`--max-cost`) est relancé avant de figurer ici.

| Niveau | Modèle | Notion | Solution (brouillons) | Solution (affichée) | JSON invalides | Durée médiane / p90 | Coût des cas | Verdict | Rapport |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| fast | `deepseek/deepseek-v4.1-flash-1017` (configuré aujourd'hui) | | | | | | | | |
| fast | | | | | | | | | |
| strong | `deepseek/deepseek-v4.1-flash-1017` (configuré aujourd'hui) | | | | | | | | |
| strong | | | | | | | | | |

## Décision

À trancher : pour chaque niveau, le moins cher des candidats retenus dont la
durée au 90ᵉ centile reste sous 10 secondes ; à égalité, celui qui fait émerger
la notion le plus souvent.

- `fast` : …
- `strong` : …

## Alternatives écartées

- Un seul modèle pour les deux niveaux : à retenir seulement si le modèle rapide
  est aussi retenu sur les cas de l'Architecte.
- Modèles sans sorties structurées : le contrat JSON des remarques n'est pas tenu
  de façon fiable.
- …(candidats non retenus, avec leur motif tiré des rapports)

## Conséquences

- `config.toml` d'exemple (README) et configuration locale mis à jour avec les
  identifiants retenus.
- Tout changement de modèle ou de prompt relance `hodegos eval` et met cet ADR à
  jour (ou le remplace par un nouvel ADR).
- Les identifiants OpenRouter datés peuvent disparaître : un modèle retiré par le
  fournisseur impose une nouvelle évaluation.
