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

`levels` doit suivre l'ordre indice, question, piste, exemple sans sauter de
palier : le premier est joué comme une passe, les suivants comme « Palier
suivant ». `must_mention` et `must_not` ne peuvent pas être vides, et aucun motif
`must_not` ne doit déjà figurer dans la section (vérifié par `cargo test`, avec
la section et le casting de chaque cas).

## Lancer l'évaluation

```sh
cargo run --release -p hodegos-cli -- eval evals/
cargo run --release -p hodegos-cli -- eval evals/ --fast fournisseur/modele-a --strong fournisseur/modele-b
cargo run --release -p hodegos-cli -- eval evals/ --model fournisseur/modele-c --case adr --max-cost 1
```

- La clé et `config.toml` sont ceux de l'IDE (`~/.config/hodegos/`). `--fast`,
  `--strong` et `--model` (les deux niveaux) remplacent les modèles configurés,
  pour comparer des candidats sans toucher à la configuration.
- Seuls les mentors livrés sont utilisés : vos surcharges personnelles n'influent
  pas sur les résultats.
- Le Mentor du langage utilise le modèle `fast`, l'Architecte et le Mentor produit
  le modèle `strong` : chaque modèle est jugé sur les cas de son niveau (15 et 7 cas).
- Coût : chaque appel compte dans le budget du mois (`usage.json`), sans être
  bloqué par son plafond. `--max-cost` (3 USD par défaut) arrête l'évaluation
  proprement ; les paliers non joués sont marqués « arrêt (--max-cost) ».
- Les appels se font l'un après l'autre. Une panne du fournisseur arrête le cas,
  sans nouvelle tentative.
- Code de sortie : 0 si chaque modèle évalué est retenu, 1 sinon.

## Déroulé d'un cas

1. Le fichier est découpé en sections ; la section `section` doit exister.
2. Casting comme dans l'IDE (chemin, langage, titres, frontmatter `mentors:`) ;
   un titulaire différent de `expected_mentor` est signalé, et le cas est joué
   avec le titulaire réel.
3. Premier palier : passe sur la section, comme si le fichier venait d'être créé
   (toute la section en ajout dans le diff).
4. Paliers suivants : « Palier suivant » sur la remarque la plus confiante, avec
   le fil réel (remarque d'origine, messages précédents). Si un palier ne donne
   rien, les suivants sont « non atteints ».

Prompts, masquage des secrets, exclusions et validation sont ceux de l'IDE.

## Rapport

Écrit dans `evals/reports/AAAA-MM-JJ-<modèles>.md` (ou `--out`), sans écraser un
rapport du même jour, et résumé dans le terminal. Les rapports sont commités :
le choix des modèles (ADR) s'appuie sur eux.

- Par modèle et par niveau : cas, notion émergée (un cas compte si un palier au
  moins la fait émerger), solution proposée (brouillons) et affichée, JSON
  invalides, pannes, castings différents, coût, durées médiane et au 90ᵉ centile,
  verdict.
- Par cas et par palier : issue, citations valides, palier respecté, notion,
  motifs `must_not` trouvés, coût, durée, puis chaque message proposé par le
  modèle, affiché ou écarté (avec le motif).

Les motifs sont cherchés sans tenir compte de la casse.

Verdict (décidé en phase 6) :

- **retenu** : aucun motif `must_not` dans ce que le modèle propose, **brouillons
  écartés par la garde compris** (la garde est un filet, pas une qualité du
  modèle), et `must_mention` atteint dans 70 % des cas au moins ;
- **non retenu** : sinon ;
- **non concluant** : panne, cas non joué ou arrêt sur `--max-cost` : à relancer.

## Cas fournis

Vingt-deux cas, couvrant les trois mentors et les cinq langages :

| Langage | Mentor | Cas |
| --- | --- | --- |
| TypeScript | Mentor du langage | 01 dates comparées par référence, 03 `forEach` avec `async`, 04 `find` et assertion non nulle, 05 `sort` sur place |
| JavaScript | Mentor du langage | 06 `var` capturée dans une boucle, 07 `sort` sans comparateur, 08 montants en flottants |
| Python | Mentor du langage | 09 valeur par défaut mutable, 10 liaison tardive, 11 liste modifiée pendant le parcours, 12 `is` au lieu de `==` |
| Rust | Mentor du langage | 13 `unwrap` sur une saisie, 14 borne de fenêtre glissante, 15 `clone` pour l'emprunteur, 16 itérateur jamais consommé |
| Markdown | Architecte | 02 ADR sans alternatives, 17 conséquences sans inconvénient, 18 décision disproportionnée, 19 invalidation « plus tard », 20 conception sans contraintes |
| Markdown | Mentor produit | 21 vision neuve (gabarit vide), 22 vision au public vague et à la première version trop large |
