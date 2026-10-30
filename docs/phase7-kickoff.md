# Phase 7 — Kick-off d'un projet

30 octobre 2026 · complément au cadrage, à intégrer dans docs/cadrage.md et docs/plan.md

## Objectif

Un projet démarre comme un document : `hodegos new` crée le dossier et un
`docs/vision.md` à remplir, et un Mentor produit pose les questions de cadrage
pendant que vous l'écrivez. Aucun écran spécifique : on réutilise le casting par
chemin, les remarques ancrées et les paliers (voir docs/adr/0004-kickoff-par-document.md).

## Dans la phase 7

- La commande `hodegos new <chemin>`.
- Le gabarit `templates/vision.md`, intégré au binaire.
- Le mentor livré `mentors/mentor-produit.md`.
- Une passe à l'ouverture (`on_open`), pour que le mentor se manifeste sur un document neuf.
- Les remarques `next_step` sans citation, ancrées sur le titre de la section.
- Deux cas d'évaluation (`evals/cases/21-*`, `22-*`, numérotés 03 et 04 au départ).

## Hors phase 7

- Les **trois points de départ** de la maquette 01 : nouveau type de sortie, avec
  son propre contrat. Phase ultérieure.
- Tout écran dédié au kick-off : on reste dans l'éditeur et le panneau des mentors.
- Toute génération de fichiers de code, d'arborescence ou de configuration du
  projet par un mentor.

## Commande `hodegos new`

```text
hodegos new <chemin> [--no-git] [--no-open]
```

1. Refuse un chemin sous `/mnt/` (même message que le lancement) et un dossier
   existant non vide. Un dossier existant vide est accepté.
2. Crée le dossier, puis `docs/vision.md` à partir du gabarit, en remplaçant
   `{project_name}` par le nom du dossier et `{date}` par la date du jour (AAAA-MM-JJ).
3. Sauf `--no-git` : `git init` si `git` est disponible, sans commit. Absence de
   `git` : avertissement, pas d'échec.
4. Sauf `--no-open` : lance l'IDE sur le dossier comme `hodegos <chemin>`, et le
   front ouvre `docs/vision.md` à la connexion (champ `initial_file` du message
   `status` initial, à ajouter au protocole).

Rien n'est écrit dans `~/.config` ni `~/.local/share` par cette commande.

## Gabarit `docs/vision.md`

Cinq sections fixes ; les consignes sont des commentaires HTML. Avant tout envoi
au modèle, les commentaires HTML sont retirés du texte de la section (règle valable
pour tous les fichiers Markdown, pas seulement le gabarit).

## Passe à l'ouverture (`on_open`)

Nouveau champ facultatif du frontmatter d'un mentor : `on_open: true` (défaut
`false`). Quand un fichier est ouvert et que le titulaire d'au moins une section a
`on_open: true` :

- si le projet n'a **aucune remarque enregistrée** pour ce fichier, une passe a
  lieu sur toutes les sections du fichier, pas seulement les sections modifiées ;
- sinon, rien : les passes suivantes restent déclenchées par l'enregistrement.

La passe d'ouverture respecte le budget et `max_remarks` comme une autre passe.

## Remarques `next_step` sans citation

Le cadrage autorise déjà `kind: next_step` sans citation. Précisions :

- `quote` peut être une chaîne vide **uniquement** pour `next_step` ;
- l'ancre porte alors le `section_id` et la citation vide ; l'interface l'affiche
  sur le titre de la section ;
- la règle de validation 2 (« citation introuvable ») ne s'applique pas à une
  citation vide d'un `next_step` ; elle reste valable pour tous les autres cas.

## Identifiants de section des titres

Les cas d'évaluation s'appuient sur des identifiants comme `h:idee` ou
`h:premiere-version`. Règle, si le code ne la fixe pas déjà : `h:` suivi du titre en
minuscules, accents retirés, toute suite de caractères non alphanumériques remplacée
par `-`, sans `-` en début ni en fin. Si le code existant suit une autre règle, garder
la sienne et adapter les `case.toml`.

## Casting attendu

Sur `docs/vision.md`, le Mentor produit doit être titulaire de chaque section
(chemin 50 + langage 30 + titre 20 à 40, priorité 90). Sur tout autre fichier
Markdown, il ne doit pas l'être : l'Architecte et les autres mentors gardent leur
place. Un test de casting le vérifie.

## Décisions prises au démarrage de la phase (30 octobre 2026)

- Casting : le Mentor produit n'a ni `languages` ni `headings`, seulement le chemin
  `docs/vision.md` (50 points). Avec `languages: [markdown]` (40 points depuis la
  phase 3, et non 30) et sa priorité de 90, il devenait titulaire de tout fichier
  Markdown.
- Cadre commun inchangé : il demande toujours une citation. La posture fait citer
  le titre d'une section vide ; la citation vide reste acceptée pour `next_step`.
- Passe d'ouverture : sections dont le titulaire a `on_open`, dans l'ordre du
  fichier, 8 au plus (pas de limite à 2 sections) ; toujours 2 remarques au plus.
- « Première ouverture seulement » : en plus de l'absence de remarque, un
  marqueur `opened.json` (données de l'utilisateur) est posé quand la passe
  d'ouverture aboutit, même sans remarque ; après une panne ou un budget atteint,
  elle est retentée à l'ouverture suivante.
- Commentaires HTML retirés aussi du diff et du contexte, pas seulement de la
  section.
- Cas d'évaluation renumérotés 21 et 22 (03 et 04 existaient déjà) ; le cas 22
  démarre au palier `question` (`entry_level` du mentor) et le cas 21 n'accepte
  plus « ? » comme notion émergée.
- `status.initial_file` n'est non nul que dans le premier `status` d'une connexion.

## Critères de fin

- [x] `hodegos new` : refus de `/mnt/…` et d'un dossier non vide, création du gabarit avec substitutions, `git init` facultatif, tests à l'appui
- [x] Les commentaires HTML sont retirés avant envoi au modèle (test)
- [x] `on_open` : passe complète à la première ouverture seulement (test avec `FakeLlmClient`)
- [x] `next_step` à citation vide accepté et ancré sur le titre ; citation vide refusée pour les autres types (tests)
- [x] Casting : Mentor produit titulaire sur `docs/vision.md`, absent ailleurs (test)
- [x] Vérification manuelle : `hodegos new ~/projets/essai` ouvre `vision.md` et le Mentor produit pose au plus deux questions de cadrage en moins de 10 secondes
  (vérifié par l'utilisateur le 30 octobre 2026)
- [x] `hodegos eval` sur les cas 21 et 22 : aucune violation de `must_not`
  (vérifié par l'utilisateur le 30 octobre 2026)
- [x] docs/cadrage.md, docs/plan.md, docs/protocole.md et CLAUDE.md intègrent la phase 7
