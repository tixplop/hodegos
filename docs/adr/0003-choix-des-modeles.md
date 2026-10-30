# ADR 0003 — Choix des modèles rapide et fort

Statut : accepté · 30 octobre 2026

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
5 pour le modèle `strong`), un rapport par paire de candidats dans `evals/reports/`.

Un premier passage (`2026-10-30-deepseek-deepseek-v4.1-flash-1017.md`) a montré
14 pistes et exemples utiles écartés par la limite de 400 caractères. Avant de
comparer les candidats :

- la limite passe à 800 caractères aux paliers piste et exemple, et le cadre
  commun l'annonce au modèle (docs/cadrage.md) ;
- `functools.partial` sort des motifs interdits du cas 10 : nommer un mécanisme
  et sa documentation est le rôle du palier piste.

## Résultats

Rapports du 30 octobre 2026, tous produits avec ces règles. Durées par appel ;
coût des cas du niveau.

| Niveau | Modèle | Notion | Solution (brouillons / affichée) | JSON invalides | Durée médiane / p90 | Coût | Verdict |
| --- | --- | --- | --- | --- | --- | --- | --- |
| fast | `deepseek/deepseek-v4.1-flash-1017` | 15/15 (100 %) | 0 / 0 | 0 | 2,9 s / 5,7 s | 0,021 $ | retenu |
| fast | `anthropic/claude-haiku-5.5` | 14/15 (93 %) | 0 / 0 | 0 | 4,0 s / 6,3 s | 0,027 $ | retenu |
| fast | `openai/gpt-6-luna` | 14/15 (93 %) | 1 / 1 | 0 | 3,1 s / 5,1 s | 0,012 $ | non retenu |
| strong | `google/gemini-4-argon` | 5/5 (100 %) | 0 / 0 | 0 | 4,7 s / 6,4 s | 0,053 $ | retenu |
| strong | `anthropic/claude-sonnet-5.5` | 4/5 (80 %) | 0 / 0 | 0 | 4,6 s / 6,8 s | 0,113 $ | retenu |
| strong | `openai/gpt-6.1-terra` | 4/5 (80 %) | 0 / 0 | 0 | 3,4 s / 5,1 s | 0,027 $ | retenu |

Rapports :

- `evals/reports/2026-10-30-deepseek-deepseek-v4.1-flash-1017-google-gemini-4-argon.md`
- `evals/reports/2026-10-30-anthropic-claude-haiku-5.5-anthropic-claude-sonnet-5.5.md`
- `evals/reports/2026-10-30-openai-gpt-6-luna-openai-gpt-6.1-terra.md`

## Décision

Parmi les candidats retenus, le meilleur taux de notion émergée, puis le coût :

- `fast` : `deepseek/deepseek-v4.1-flash-1017` ;
- `strong` : `google/gemini-4-argon`.

## Alternatives écartées

- `openai/gpt-6-luna` (fast) : au cas 13, son exemple « voisin » contient
  `contenu.trim().parse()?`, le geste exact de la solution attendue.
- `anthropic/claude-haiku-5.5` (fast) : retenu, mais une notion manquée de plus
  et plus cher que deepseek.
- `openai/gpt-6.1-terra` (strong) : deux fois moins cher que gemini, mais une
  notion manquée sur 5. L'écart de coût, un centime environ par passe de
  l'Architecte, ne compense pas sur un échantillon aussi petit.
- `anthropic/claude-sonnet-5.5` (strong) : une notion manquée et le plus cher.
- Un seul modèle pour les deux niveaux : deepseek ne fait émerger la notion que
  dans 3 cas sur 5 de l'Architecte au premier passage (limite à 400 caractères,
  non rejoué depuis).

## Conséquences

- `config.toml` : `fast = "deepseek/deepseek-v4.1-flash-1017"`,
  `strong = "google/gemini-4-argon"`.
- Cinq cas seulement départagent les modèles forts : à revoir si l'Architecte
  déçoit à l'usage, en ajoutant des cas Markdown.
- Le délai de 10 secondes est mesuré par appel : le critère sur un vrai
  enregistrement reste à vérifier dans l'IDE (docs/plan.md, phase 6).
- Tout changement de modèle ou de prompt relance `hodegos eval` et met cet ADR à
  jour (ou le remplace par un nouvel ADR).
- Les identifiants OpenRouter datés peuvent disparaître : un modèle retiré par le
  fournisseur impose une nouvelle évaluation.

## Après la phase 7 (30 octobre 2026)

Évaluation complète relancée avec les 22 cas, dont les deux cas du Mentor produit
(niveau `strong`, désormais 7 cas) :

- `strong` · `google/gemini-4-argon` : notion 5/7 (71 %), aucune solution,
  retenu, avec une marge d'un cas (19 et 22 manqués) ;
- `fast` · `deepseek/deepseek-v4.1-flash-1017` : notion 14/15, mais au cas 13,
  le brouillon d'exemple, écarté par la garde, reprend `-> Result<u32` ; même
  résultat sur les cas Rust relancés seuls. Non retenu selon la règle.

Rapports : `evals/reports/2026-10-30-deepseek-deepseek-v4.1-flash-1017-google-gemini-4-argon-2.md`
(cas 21 et 22), `…-3.md` (22 cas), `…-4.md` (cas Rust).

**Décision : le cas 13 s'arrête au palier piste.** Pour une fonction qui lit une
ligne et la convertit en entier, tout cas voisin de 12 lignes montre la même
lecture, la même conversion et le même type de retour : un exemple honnête livre
la solution. Ce cas ne teste donc plus le palier exemple. Les autres règles et
motifs ne changent pas.

Conséquences :

- Cette décision est prise après avoir vu le résultat ; elle vaut pour tous les
  candidats. `openai/gpt-6-luna` avait été écarté sur le palier exemple de ce même
  cas (« Alternatives écartées ») : son exclusion n'est plus fondée et serait à
  rejouer s'il redevenait candidat.
- Le palier exemple reste testé sur les autres cas du Mentor du langage.
- Le choix de deepseek pour `fast` et de gemini pour `strong` est maintenu, sous
  réserve de la prochaine évaluation complète avec le cas 13 modifié (levée,
  voir ci-dessous).

**Réserve levée (30 octobre 2026).** Évaluation complète des 22 cas avec le cas 13
arrêté au palier piste :

- `fast` · `deepseek/deepseek-v4.1-flash-1017` : notion 14/15 (93 %), aucune
  solution, brouillons compris ; retenu (03 manqué : question écartée pour confiance
  insuffisante) ;
- `strong` · `google/gemini-4-argon` : notion 6/7 (86 %), aucune solution ; retenu
  (22 manqué, comme aux évaluations précédentes).

Coût total 0,0837 $ ; durée médiane 3,3 s (`fast`) et 5,2 s (`strong`). Le cas 13
seul, relancé auparavant, donnait déjà deepseek retenu.

Rapports : `…-5.md` (cas 13), `…-6.md` (22 cas).

Le choix de deepseek pour `fast` et de gemini pour `strong` est confirmé. Reste
ouvert : le cas 22 n'a jamais fait émerger la notion ; élargir ses motifs
`must_mention` ou faire évoluer le Mentor produit reste à décider.
