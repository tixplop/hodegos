# Évaluation des mentors

`hodegos eval evals/` rejoue chaque cas avec les vrais modèles configurés et
produit un rapport. Ce n'est pas un test automatique : il consomme du budget et
se lance à la main (phase 6, puis à chaque changement de prompt ou de modèle).

Les cas de `evals/cases/` sont intégrés au binaire (phase 8d, ADR 0007) :
`hodegos eval` sans dossier les joue, et l'IDE aussi quand un modèle ou un effort
change dans les réglages. `cargo test` vérifie qu'ils sont identiques à ceux du
dépôt.

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

## Cas sur un projet d'exemple (phase 9f)

`evals/projects/` contient trois petits projets (`troc` en TypeScript, `budget`
en Rust, `taches` en Python), avec vision, ADR, tests et des écarts plantés
(décision contournée, doublon, module sans test, vision dépassée). Un cas qui
porte `kind` les désigne par `project` ; son dossier `after/` (mêmes chemins
relatifs) contient les fichiers qui remplacent ou s'ajoutent à ceux du projet.
Le projet est recopié dans un dossier temporaire ; carte, fichiers clés, diff,
voisinage, prompts et validation sont ceux de l'IDE.

**Revue** (`kind = "review"`) : revue du mentor de projet `mentor`. Sans
`after/`, première revue (carte et fichiers clés seulement : jamais les corps de
fonction) ; avec `after/`, revue suivante, le diff part aussi.

```toml
kind = "review"
title = "Revue : un module relit un fichier de données sans passer par Depot"
project = "troc"
mentor = "architecte"

[expect]
must_mention = ["0001", "Depot", "un seul module"]
cited_files = ["src/emprunts/historique.ts"] # la bonne remarque cite l'un d'eux
about = "project"                            # facultatif : project ou passage

[expect.must_not]                            # par fichier, cherchés dans tous les messages
"src/emprunts/historique.ts" = ["depot.lire().archives"]
```

Le cas réussit si une remarque affichée fait émerger la notion, cite un des
`cited_files` et porte le `about` attendu.

**Voisinage** (`kind = "neighborhood"`) : une section d'un projet dont la bonne
remarque dépend d'un fichier voisin. Elle est jouée deux fois avec le titulaire
du casting, en contexte `section` puis `neighborhood` ; le cas réussit si la
passe avec le voisinage fait émerger la notion. Champs : `project`, `file`,
`section`, `expected_mentor`, `level`, `[expect]` comme une passe.

**Réévaluation** (`kind = "reevaluation"`) : une remarque de projet (`mentor`,
`level`, `about`, `message`, `quotes = [{ path, quote }]`) dont `after/` fait
disparaître au moins un passage cité ; le cas réussit si le verdict est
`verdict` (`keep` avec au moins une citation valide, ou `withdraw`).
`expect.must_mention` est facultatif, `expect.must_not` vise la raison donnée.

`cargo test` vérifie chaque cas sur projet sans modèle : citations présentes puis
disparues, `after/` qui change le diff, voisinage qui ajoute du contexte, aucun
motif `must_not` déjà écrit dans le projet ni dans `after/`.

## Lancer l'évaluation

```sh
cargo run --release -p hodegos-cli -- eval evals/
cargo run --release -p hodegos-cli -- eval            # cas intégrés, rapport personnel
cargo run --release -p hodegos-cli -- eval evals/ --fast fournisseur/modele-a --strong fournisseur/modele-b
cargo run --release -p hodegos-cli -- eval evals/ --model fournisseur/modele-c --case adr --max-cost 1
cargo run --release -p hodegos-cli -- eval evals/ --mentors ~/brouillons-de-mentors/ --case troc
```

- La clé et `config.toml` sont ceux de l'IDE (`~/.config/hodegos/`). `--fast`,
  `--strong` et `--model` (les deux niveaux) remplacent les modèles configurés,
  pour comparer des candidats sans toucher à la configuration.
- Seuls les mentors livrés sont utilisés : vos surcharges personnelles n'influent
  pas sur les résultats. `--mentors <dossier>` place un dossier de mentors
  par-dessus, comme des mentors personnels, pour évaluer des mentors avant de les
  livrer (la 9f l'a fait pour ceux de la phase 9). Sans mentor de projet, un cas de
  revue ou de réévaluation est signalé « non joué ».
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

Écrit dans `<dossier>/reports/AAAA-MM-JJ-<modèles>.md` (ou `--out`), sans écraser
un rapport du même jour, et résumé dans le terminal. Les rapports de `evals/reports/`
sont commités : le choix des modèles (ADR) s'appuie sur eux. Sans dossier, le
rapport va dans `~/.local/share/hodegos/evals/`, jamais dans un projet.

Le rapport est réécrit après chaque cas et marqué « Rapport partiel » jusqu'à la
fin : une évaluation interrompue laisse ce qu'elle a joué.

- Par modèle et par niveau : cas, notion émergée (un cas compte si un palier au
  moins la fait émerger), solution proposée (brouillons) et affichée, JSON
  invalides, pannes, castings différents, coût, jetons (entrée et sortie), durées
  médiane et au 90ᵉ centile, verdict.
- Par sorte de cas (passe, revue, voisinage, réévaluation) : cas réussis,
  citations valides, solutions proposées, coût ; voisinage : notion avec la
  section seule et avec le voisinage ; coût moyen d'une revue par mentor.
- Coût d'une journée type : 40 enregistrements sur six heures, deux ouvertures,
  quatre commits, deux réévaluations, rejoués avec le planificateur des revues
  automatiques et ses réglages par défaut, multipliés par les coûts mesurés ;
  par mois sur 22 jours ouvrés, comparé à 10 $.
- Par cas et par palier : issue, citations valides, palier respecté, notion,
  motifs `must_not` trouvés, coût, durée, puis chaque message proposé par le
  modèle, affiché ou écarté (avec le motif).

Les motifs sont cherchés sans tenir compte de la casse.

Verdict (décidé en phase 6) :

- **retenu** : aucun motif `must_not` dans ce que le modèle propose, **brouillons
  écartés par la garde compris** (la garde est un filet, pas une qualité du
  modèle), `must_mention` atteint dans 70 % des cas au moins, et 80 % des
  citations valides au moins (phase 9f : une citation est valide si elle figure
  dans un texte envoyé et dans le fichier) ;
- **non retenu** : sinon ;
- **non concluant** : panne, cas non joué ou arrêt sur `--max-cost` : à relancer.

## Cas fournis

Trente-sept cas. Les vingt-deux premiers, des passes, couvrent les trois mentors de
section et les cinq langages :

| Langage | Mentor | Cas |
| --- | --- | --- |
| TypeScript | Mentor du langage | 01 dates comparées par référence, 03 `forEach` avec `async`, 04 `find` et assertion non nulle, 05 `sort` sur place |
| JavaScript | Mentor du langage | 06 `var` capturée dans une boucle, 07 `sort` sans comparateur, 08 montants en flottants |
| Python | Mentor du langage | 09 valeur par défaut mutable, 10 liaison tardive, 11 liste modifiée pendant le parcours, 12 `is` au lieu de `==` |
| Rust | Mentor du langage | 13 `unwrap` sur une saisie, 14 borne de fenêtre glissante, 15 `clone` pour l'emprunteur, 16 itérateur jamais consommé |
| Markdown | Architecte | 02 ADR sans alternatives, 17 conséquences sans inconvénient, 18 décision disproportionnée, 19 invalidation « plus tard », 20 conception sans contraintes |
| Markdown | Mentor produit | 21 vision neuve (gabarit vide), 22 vision au public vague et à la première version trop large |

Les quinze suivants (phase 9f) se jouent sur les projets d'exemple :

| Projet | Revue | Voisinage | Réévaluation |
| --- | --- | --- | --- |
| `troc` (TypeScript) | 23 stockage contourné (Architecte), 24 rappels par e-mail hors première version (Mentor produit), 25 annulation sans test (Testeur) | 26 arguments inversés | 27 vision élargie : `withdraw` |
| `budget` (Rust) | 28 `unwrap` dans l'import (Architecte), 29 formatage des montants en double (Architecte), 30 catégories sans test (Testeur) | 31 période inversée | 32 import corrigé à moitié : `keep` |
| `taches` (Python) | 33 SQL hors du dépôt (Architecte), 34 vision pour les familles (Mentor produit), 35 test sans assertion (Testeur) | 36 dates en UTC | 37 export passé par le dépôt : `withdraw` |
