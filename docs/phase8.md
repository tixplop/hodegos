# Phase 8 — Confort de l'IDE

31 octobre 2026 · spécification unique, livrée en six sous-phases (8a à 8f)

## Objectif

Rendre Hodegos utilisable comme IDE au quotidien, sans changer son principe :
gérer les fichiers du projet et voir leur état git, lire une sélection, activer
un mentor quand on le choisit, écrire du Markdown comme dans Obsidian, régler
modèles, effort, clé et mentors depuis l'interface, et ouvrir un terminal confiné
au projet.

Maquettes et cas d'usage : `docs/maquettes/phase8/` (UC-8.1 à UC-8.24 dans
`README.md`). Décisions : ADR 0005 (terminal intégré confiné), ADR 0006
(remarques supprimées avec le fichier), ADR 0007 (évaluation depuis l'IDE).

## Découpage et ordre

| Sous-phase | Contenu | Dépend de |
| --- | --- | --- |
| 8a | Fichiers : créer, supprimer ; statut git dans l'arbre | — |
| 8b | Éditeur : sélection visible ; mentor activé par le choix | — |
| 8c | Markdown : source, aperçu en direct, lecture | — |
| 8d | Réglages : modèles, effort, clé d'API, budget ; réévaluation | — |
| 8e | Réglages : édition des mentors | 8d (écran Réglages) |
| 8f | Terminal intégré et confiné | prototype en tête de sous-phase |

Une sous-phase à la fois, avec ses critères de fin (`docs/plan.md`). La
sous-phase 8f commence par un prototype jetable qui lève ses deux risques
(confinement sous WSL2, xterm.js sous la CSP) avant toute intégration.

## Règles inchangées

- Rien de ce qui est ajouté ne change la règle « pas de solution », le cadre
  commun des prompts ni ce qui part vers le modèle.
- Toute action passe par la WebSocket existante, authentifiée par le cookie et
  contrôlée par `Origin` et `Host` ; aucun nouveau port, aucun nouveau service.
- Réglages et données restent personnels : `~/.config/hodegos`,
  `~/.local/share/hodegos`. Seule exception : un mentor que vous choisissez
  d'enregistrer dans le projet (`.hodegos/mentors/`, 8e).
- Tout nouveau message met à jour `docs/protocole.md` dans le même commit.

---

## 8a — Fichiers et statut git

### Créer (UC-8.1, UC-8.2)

- Entrées : boutons « Nouveau fichier » et « Nouveau dossier » en tête de
  l'arbre (racine), et menu contextuel d'un dossier (dans ce dossier).
- Le nom se saisit dans l'arbre ; Entrée crée, Échap annule.
- Refus (`error`, message lisible à côté du champ) : nom vide, contenant `/`,
  `\0`, `.` ou `..` seuls ; nom déjà pris ; chemin sous `.git` ; chemin exclu
  par `.gitignore` ; sortie de la racine. Pas de création de dossiers
  intermédiaires.
- Un fichier est créé vide, avec les droits par défaut de l'utilisateur, puis
  ouvert dans un onglet ; sa ligne `dir` porte ses mentors.
- Message `create_entry { path, kind: "file" | "dir" }` → `entry_created
  { path, kind }`, puis `dir` du dossier parent.

### Supprimer (UC-8.3)

- Entrée : menu contextuel d'un fichier ou d'un dossier, « Supprimer… ».
- Avant la confirmation, le front demande `delete_preview { path }` →
  `delete_preview { path, kind, files, untracked, modified, ignored, restorable }` :
  nombre de fichiers concernés, dont non suivis et modifiés ; `restorable` vaut
  `true` si git peut tout restaurer (fichiers suivis, sans modification).
- Confirmation (maquette `08-1b-suppression.html`) :
  - suivi, sans modification : « Supprimer », récupérable par `git restore` ;
  - suivi, modifié : « Supprimer », avec l'avertissement que les modifications
    non commitées sont perdues ;
  - non suivi : « Supprimer définitivement » ;
  - dossier : nombre de fichiers, non suivis à part.
- `delete_entry { path }` supprime réellement (pas de corbeille), puis
  `entry_deleted { path, kind }` et `dir` du parent ; les onglets des fichiers
  supprimés se ferment.
- **Remarques** (ADR 0006) : une suppression faite **depuis Hodegos** retire,
  pour chaque fichier supprimé, ses remarques (`remarks.json`), ses choix forcés
  (`overrides.json`) et sa passe d'ouverture (`opened.json`). Si le fichier
  revient ensuite (git, terminal), il repart sans remarque : c'est accepté.
  Une suppression faite **hors de Hodegos** garde la règle actuelle (remarques
  conservées), pour survivre aux changements de branche.
- Refus : racine du projet, `.git`, chemin exclu, sortie de la racine.
- Dossier qui contient des fichiers ignorés par git (`target/`, `node_modules/`),
  invisibles dans l'arbre : tout est supprimé ; `delete_preview` les compte à part
  (`ignored`) et la confirmation le dit (« dont N ignorés par git »). Un lien
  symbolique est supprimé lui-même, jamais suivi (décidé le 31 octobre 2026).

### Statut git (UC-8.4)

- Calcul : `git status --porcelain=v2 -z --untracked-files=all` lancé à la
  racine, avec `GIT_OPTIONAL_LOCKS=0` (ne jamais prendre `index.lock`). Le
  binaire `git` de l'utilisateur, donc sa configuration.
- Statuts affichés : `M` modifié (index ou arbre de travail), `A` ajouté,
  `U` non suivi, `C` en conflit, `R` renommé. Un fichier suivi supprimé n'est pas
  dans l'arbre ; il compte dans la barre d'état. Lettre **et** couleur
  (`warning` pour M et R, `success` pour A et U, `danger` pour C) : le statut se
  lit en mots ou en lettres, jamais par la couleur seule.
- Dossiers : un point `warning` si un descendant a un statut ; calculé par le
  front à partir de la liste.
- Barre d'état : branche (ou `HEAD détachée`), nombre de changements, de
  conflits. Projet sans dépôt, ou git absent : « Pas de dépôt git », aucune
  lettre.
- Rafraîchissement, avec un délai de regroupement de 300 ms : à la connexion,
  après `save`, `create_entry`, `delete_entry`, `file_changed`, et quand
  `.git/index`, `.git/HEAD` ou `.git/refs/` changent (commit, checkout fait
  dans le terminal ou ailleurs). Le dossier `.git` est observé sans récursion
  dans les objets.
- Arbre (décidé le 31 octobre 2026) : à chaque `git_status` reçu, le front
  redemande `list_dir` pour les dossiers dépliés, ce qui fait apparaître ou
  disparaître les fichiers créés ou supprimés par un checkout ou une commande.
  Toujours aucune observation récursive du projet.
- Projet dans un sous-dossier d'un dépôt, ou worktree (`.git` est un fichier) :
  pris en charge (décidé le 31 octobre 2026). `git rev-parse --git-dir
  --show-prefix` donne le dossier git à observer et le préfixe du projet ; les
  chemins de `git status` sont ramenés à la racine du projet, ceux qui en sortent
  sont ignorés.
- Règles globales d'exclusion de git (`core.excludesFile`) : Hodegos ne les lit
  pas pour l'arbre (`docs/cadrage.md`, sécurité), alors que `git status` les
  applique. Un fichier exclu seulement par elles reste dans l'arbre, sans lettre.
- Message serveur `git_status { branch, detached, entries: [{ path, status }],
  changes, conflicts }`, poussé à chaque changement ; `null` sans dépôt.
- Hors phase : renommer et déplacer (les remarques sont rattachées au chemin),
  afficher les fichiers ignorés, diff dans la gouttière.

## 8b — Sélection et activation d'un mentor

### Sélection visible (UC-8.5)

- La ligne active n'a plus de fond : liseré gauche `accent` de 3 px et numéro
  de ligne en `accent-ink` gras. La sélection garde
  `color-mix(in srgb, var(--accent) 25%, transparent)`.
- Vérifier le contraste du texte sur la sélection (4,5:1) dans les deux thèmes,
  sur `surface-0` ; ajuster le pourcentage si besoin et le consigner dans
  `ui/styles/README.md`.

### Activer un mentor (UC-8.6 à UC-8.8)

- Le panneau renomme « Choisir X » en « Activer X ». Activer = choix forcé
  (comme aujourd'hui) **et** passe immédiate sur la section du curseur avec ce
  mentor.
- `cast_override` reçoit `activate: bool` (défaut `false`, compatibilité) et
  `confirm_over_budget: bool`. Avec `activate`, après le `casting`, le serveur
  lance une **passe demandée** :
  - section : `section_id` du message, sinon celle du dernier `cursor` ;
  - version enregistrée, diff vide (le prompt le dit déjà pour les reprises) ;
  - `no_change` ne s'applique pas ; exclusions, masquage, plafond de 2
    remarques et fournisseur comme une passe ;
  - c'est une demande explicite : au-delà du budget, `budget_exceeded` sans
    `confirm_over_budget`, comme `hint_next`.
- Le front annonce la passe (« Relit la section… ») dès `pass_started`.
- Fichier modifié non enregistré : dialogue « Enregistrer et demander » /
  « Version enregistrée » / « Annuler ». « Enregistrer et demander » envoie
  `save { …, skip_pass: true }` puis `cast_override { activate: true }` : une
  seule passe, avec le mentor activé.
- « Revenir au choix automatique » (`mentor_id: null`) ne lance rien ;
  `activate` y est ignoré.

## 8c — Markdown : source, aperçu en direct, lecture (UC-8.9 à UC-8.11)

Front uniquement, sans nouvelle bibliothèque : l'arbre syntaxique de
`@codemirror/lang-markdown` (`@lezer/markdown`, déjà empaqueté) suffit.

- Sélecteur de mode dans la barre de l'éditeur, pour les fichiers Markdown
  seulement : Source, Aperçu, Lecture. Ctrl+E bascule entre le dernier mode
  d'édition et Lecture. Mode mémorisé dans `localStorage` (global, pas par
  fichier), comme le thème.
- **Source** : comportement actuel.
- **Aperçu en direct** : extension CodeMirror (décorations calculées sur l'arbre
  syntaxique, sur les lignes visibles) :
  - marques masquées (`#`, `*`, `_`, `` ` ``, crochets et URL des liens), titres
    en taille de titre, gras, italique, code en ligne, puces rendues ;
  - cases `- [ ]` / `- [x]` en cases à cocher ; cocher modifie le texte
    (`[x]`), comme une frappe ;
  - commentaires HTML masqués ;
  - toute ligne qui touche la sélection ou le curseur montre sa syntaxe brute.
- **Lecture** : document non éditable, construit en éléments DOM à partir de
  l'arbre (jamais `innerHTML`). Le HTML brut du document s'affiche comme du
  texte ; les images ne sont pas chargées (texte alternatif et chemin) ; un lien
  relatif vers un fichier du projet ouvre un onglet ; un lien externe s'ouvre
  dans un nouvel onglet (`noopener noreferrer`). Double-clic : retour en aperçu
  sur ce bloc.
- Ancres des remarques : surlignage et marqueurs dans les trois modes ; en
  lecture, le passage cité est retrouvé dans le texte rendu de sa section.
- CSP : aucune règle assouplie ; styles par classes ou CSSOM, jamais d'attribut
  `style` posé en HTML.

## 8d — Réglages : modèles, effort, clé, budget (UC-8.12 à UC-8.15)

### Écran

- Bouton « Réglages » dans l'en-tête ; l'écran remplace l'éditeur (pas
  d'onglet de fichier), navigation à gauche : Modèles et effort, Clé d'API,
  Budget, Mentors (8e). Rappel permanent : réglages personnels, écrits dans
  `~/.config/hodegos`.
- Changements en attente jusqu'à « Enregistrer » ; « Annuler les changements »
  revient aux valeurs lues.

### Modèles et effort

- Par niveau (`fast`, `strong`) : identifiant OpenRouter et effort de
  raisonnement (`Par défaut`, `Faible`, `Moyen`, `Élevé`).
- `model_check { id }` → `model_info { id, exists, structured_outputs,
  reasoning, prompt_price, completion_price }`, d'après l'API publique des
  modèles d'OpenRouter (`/api/v1/models`, `supported_parameters`). Un modèle sans
  sorties structurées est refusé. L'effort n'est proposé que si `reasoning` est
  accepté.
- Envoi : `reasoning: { effort: "low" | "medium" | "high" }` dans la requête
  OpenRouter, absent pour « Par défaut » ; jamais envoyé à un modèle qui ne
  l'accepte pas.
- Configuration :
  ```toml
  [llm]
  fast = "deepseek/deepseek-v4.1-flash-1017"
  strong = "google/gemini-4-argon"
  fast_effort = "default"      # default | low | medium | high
  strong_effort = "medium"
  ```
  Écriture avec `toml_edit` (commentaires et ordre conservés), atomique ; effet
  à la passe suivante, sans redémarrage.

### Réévaluation (ADR 0007)

- Tout changement d'identifiant ou d'effort d'un niveau **déclenche** une
  évaluation de ce niveau, comme l'exige l'ADR 0003. L'utilisateur est prévenu
  **avant** la dépense : dialogue avec le niveau, le nombre de cas, le coût
  estimé et la durée estimée, puis « Enregistrer et évaluer » ou « Annuler »
  (rien n'est écrit).
- Estimation : coût et durée du dernier passage du même niveau avec le même
  modèle, sinon prix du modèle (`model_info`) × jetons du dernier passage du
  niveau ; affichée comme une estimation. Plafond : `--max-cost` de l'évaluation
  (3 USD par défaut), rappelé dans le dialogue.
- L'évaluation tourne dans le serveur, sur les cas **intégrés au binaire**
  (`evals/cases/` par `rust-embed`, comme les mentors livrés) ; `hodegos eval`
  garde son argument de dossier, par défaut les cas intégrés.
- Progression : `eval_progress { level, done, total, cost }` ; fin :
  `eval_done { level, model, effort, passed, notion, divulged, cost,
  duration_ms, report }`. Rapport écrit dans
  `~/.local/share/hodegos/evals/<date>-<modèle>-<effort>.md`, jamais dans le
  projet. La dépense compte dans `usage.json`.
- Résultat : sous le seuil (aucune divulgation, notion ≥ 70 %), le réglage
  reste appliqué mais l'écran et la barre d'état l'indiquent en `warning`, avec
  « Revenir à l'ancien réglage ». Pendant l'évaluation, les passes continuent
  avec le nouveau réglage.
- Résultats connus (décidé le 31 octobre 2026) : ceux des évaluations faites
  depuis l'IDE (`~/.local/share/hodegos/evals/`), plus un court résumé des choix
  livrés intégré au binaire (ADR 0003 : niveau, modèle, effort, notion, coût,
  durée, jetons). Il sert au rappel du dernier résultat (UC-8.12) et à
  l'estimation ; un réglage absent des deux est « jamais évalué ».
- Une évaluation à la fois ; elle s'arrête avec Hodegos (rapport partiel marqué
  comme tel).

### Clé d'API

- `settings` donne `key_source` (`env`, `file`, `none`) et `key_hint` (quatre
  derniers caractères) ; la clé n'est **jamais** renvoyée au front.
- `api_key_set { key }` écrit `credentials.toml` (droits 600, atomique) ; refusé
  si `OPENROUTER_API_KEY` est définie (la variable prime, le champ est en lecture
  seule et le dit). La clé reçue n'est jamais journalisée.
- `provider_test` → `provider_test_result { ok, message }` (appel léger,
  sans coût de génération).

### Budget

- Plafond, seuil d'avertissement, dépense du mois (UTC) et barre de
  progression ; mêmes règles qu'aujourd'hui.

### Messages

`settings_get` → `settings { fast, strong, fast_effort, strong_effort,
budget, key_source, key_hint, last_eval }` ; `settings_set { … }` → `settings`
(ou `eval_required { level, cases, estimated_cost, estimated_duration_ms }` si un
niveau change, puis `settings_set { …, confirm_eval: true }`) ; `model_check`,
`api_key_set`, `provider_test`, `eval_progress`, `eval_done`.

## 8e — Réglages : mentors (UC-8.16 à UC-8.20)

- Liste : tous les mentors connus, avec origine (livré, personnel, projet),
  surcharge (« remplace le livré »), mode, niveau de modèle, état (actif,
  désactivé, en erreur avec la raison du chargeur).
- Un mentor livré se lit mais ne s'édite pas : « Personnaliser » (comme
  `mentors eject`) le copie dans les mentors personnels, ou du projet, puis
  l'ouvre.
- Édition : formulaire du frontmatter (nom, initiales, mode, priorité, niveau de
  modèle, palier d'entrée, remarques par passe, `on_open`, déclencheurs) et
  posture dans un éditeur Markdown. Le serveur valide avec les **mêmes règles
  que le chargeur** avant d'écrire ; refus avec la liste des erreurs, champ par
  champ. Les commentaires YAML du frontmatter ne sont pas conservés : l'écran le
  dit. Bascule « Modifier le fichier brut » pour éditer le fichier tel quel.
- « Prompt assemblé » : cadre commun en lecture seule, puis posture (substitution
  de `{language}` sur un exemple).
- Créer (vide ou par duplication), désactiver (`enabled: false`, dans la copie
  existante ou dans une surcharge minimale), supprimer une surcharge (revenir au
  mentor livré, confirmation).
- Enregistrer dans le projet crée `.hodegos/mentors/` si besoin : c'est la seule
  écriture de Hodegos dans le projet hors fichiers de l'utilisateur, faite à sa
  demande.
- Le rechargement à chaud existant applique le résultat (`mentors_changed`,
  `casting` recalculé).
- Pas d'effort par mentor : l'effort reste un réglage de niveau.
- Messages : `mentors_list` → `mentors { items }` ; `mentor_get { id, origin }`
  → `mentor_file { id, origin, path, content, editable }` ; `mentor_save
  { origin, id, content }` ; `mentor_eject { id, to }` ; `mentor_delete
  { id, origin }` ; `mentor_prompt { id }` → `mentor_prompt { text }`.

## 8f — Terminal intégré et confiné (UC-8.21 à UC-8.24, ADR 0005)

### Prototype (début de 8f, jetable)

1. Confinement sous WSL2 : `bwrap` disponible, espaces de noms utilisateur
   autorisés, shell interactif fonctionnel (git, cargo, npm) avec les montages
   ci-dessous.
2. xterm.js sous la CSP stricte : rendu, couleurs, redimensionnement, sans
   assouplir la CSP (piste : montage dans un shadow root avec
   `adoptedStyleSheets`, comme CodeMirror).
3. Débit : une sortie de 10 Mo (`cat`) reste fluide et ne bloque pas la
   WebSocket des autres messages.

Résultat consigné dans l'ADR 0005 avant l'intégration.

Points relevés à l'intégration de la phase 8, à trancher avec le prototype :

- Fichiers de démarrage du shell (`~/.bashrc`, `~/.zshrc`, `~/.oh-my-zsh`…) :
  absents par défaut, le shell démarre sans sa configuration ; les ajouter aux
  montages `ro` par défaut ?
- `OPENROUTER_API_KEY` : variable « propre à Hodegos », donc retirée de
  l'environnement du terminal ?
- Montages `rw` (`~/.cargo`, `~/.npm`) : une commande confinée peut y déposer un
  exécutable lancé ensuite hors du confinement (`~/.cargo/bin`). À dire dans
  l'ADR 0005 comme limite connue, ou à monter en `ro` sauf le cache ?

### Processus

- Un pseudo-terminal par terminal, ouvert par le serveur ; le shell est celui de
  l'utilisateur (`$SHELL`, sinon `/bin/bash`), avec son UID, lancé dans la
  racine du projet.
- **Confinement obligatoire** par bubblewrap (espaces de noms, sans droits
  root) :
  - racine du projet en lecture-écriture, au même chemin ;
  - `/usr`, `/bin`, `/lib*`, `/etc` en lecture seule ; `/dev` minimal, `/proc`
    neuf, `/tmp` vide en mémoire ;
  - le reste de `$HOME` absent, sauf une liste réglable
    (`[terminal] binds`, chemin et mode `ro` ou `rw`). Par défaut :
    `~/.gitconfig` (ro), `~/.cargo` (rw), `~/.rustup` (ro), `~/.npm` (rw),
    `~/.local/bin` (ro) ; `~/.ssh` n'est pas monté par défaut ;
  - `--unshare-pid`, `--die-with-parent`, `--new-session` ; réseau conservé
    (git, cargo et npm en ont besoin).
- Le dossier de travail ne peut pas quitter l'arborescence montée : `cd /home`
  ne montre que ce qui est monté.
- `bwrap` absent, ou espaces de noms refusés : **pas de terminal**, avec un
  message qui dit pourquoi et comment l'installer ; jamais de repli sans
  confinement.
- Variables d'environnement : celles de l'utilisateur, moins celles propres à
  Hodegos ; `TERM=xterm-256color`.

### Flux

- Même WebSocket : `term_open { cols, rows }` → `term_opened { term_id }` ;
  `term_input { term_id, data }` ; `term_output { term_id, data }` (sorties
  regroupées toutes les 16 ms au plus) ; `term_resize { term_id, cols, rows }` ;
  `term_close { term_id }` ; `term_exited { term_id, code }`. `data` en base64
  (octets bruts, pas forcément de l'UTF-8).
- Limites : 4 terminaux par fenêtre ; `term_input` de 64 Kio au plus ; si la
  WebSocket ne suit pas, la lecture du pseudo-terminal est suspendue (pas de
  mémoire sans borne).
- Fin : fermer l'onglet tue le processus et ses descendants (le groupe) ; la
  fermeture de la fenêtre (WebSocket) et l'arrêt de Hodegos tuent tous les
  terminaux.
- Rien de ce qui passe dans un terminal n'est journalisé au-dessus de `trace`,
  ni envoyé à un mentor.

### Interface

- Bouton « Terminal » dans l'en-tête et Ctrl+` : panneau sous l'éditeur,
  redimensionnable ; Ctrl+` replie sans tuer. Onglets « bash · 1 »… , « + »,
  « Fermer ». Rappel : utilisateur, racine du projet, confiné.
- Les effets passent par l'existant : `git_status` (8a) et `file_changed`.

---

## Protocole : récapitulatif des ajouts

| Sens | Messages |
| --- | --- |
| Front → backend | `create_entry`, `delete_preview`, `delete_entry`, `settings_get`, `settings_set`, `model_check`, `api_key_set`, `provider_test`, `mentors_list`, `mentor_get`, `mentor_save`, `mentor_eject`, `mentor_delete`, `mentor_prompt`, `term_open`, `term_input`, `term_resize`, `term_close` |
| Backend → front | `entry_created`, `delete_preview`, `entry_deleted`, `git_status`, `settings`, `eval_required`, `model_info`, `provider_test_result`, `eval_progress`, `eval_done`, `mentors`, `mentor_file`, `mentor_prompt`, `term_opened`, `term_output`, `term_exited` |
| Modifiés | `save` (+ `skip_pass`), `cast_override` (+ `activate`, `confirm_over_budget`) |

## Tests

- Aucun appel réseau : `model_check` et `provider_test` contre le faux serveur
  local ; l'évaluation intégrée avec `FakeLlmClient`.
- 8a : création et refus, suppression d'un fichier et d'un dossier avec retrait
  des remarques, choix forcés et passes d'ouverture ; suppression externe qui
  garde les remarques ; analyse de `git status --porcelain=v2 -z` (renommages,
  conflits, noms avec espaces) ; dépôt absent.
- 8b : `activate` lance une passe sur la section du curseur, sans `no_change` ;
  `skip_pass` ; budget dépassé sans confirmation.
- 8d : écriture de `config.toml` qui conserve les commentaires ; clé jamais
  renvoyée ni journalisée ; refus de `api_key_set` avec la variable définie ;
  `eval_required` à chaque changement de niveau ; `reasoning` absent pour
  « Par défaut » et pour un modèle qui ne l'accepte pas.
- 8e : mêmes erreurs de validation que le chargeur ; un mentor livré n'est
  jamais écrit ; création de `.hodegos/mentors/` seulement à l'enregistrement.
- 8f : tests du confinement **seulement si `bwrap` fonctionne** (sinon ignorés
  avec un message ; la CI GitHub sous Ubuntu 24.04 restreint les espaces de
  noms) : `cd /` puis `ls $HOME` ne montre que les montages ; écrire hors du
  projet échoue ; le processus meurt avec la WebSocket. Dans tous les cas :
  limites, base64, regroupement des sorties.
- Front : vérifications manuelles par sous-phase dans `docs/plan.md`, deux
  thèmes.

## Critères de fin (à reporter dans `docs/plan.md`)

Pour chaque sous-phase : `cargo test --workspace`, clippy et fmt propres,
`docs/protocole.md` à jour, vérification manuelle dans les deux thèmes.

**8a**
- [ ] Créer un fichier et un dossier depuis l'arbre ; les refus s'affichent à
  côté du champ.
- [ ] Supprimer un fichier suivi, un fichier non suivi et un dossier : la
  confirmation dit ce qui est récupérable ; remarques, choix forcés et passe
  d'ouverture des fichiers supprimés sont retirés.
- [ ] Lettres et couleurs git à jour après un enregistrement, un commit et un
  checkout faits hors de Hodegos ; « Pas de dépôt git » sans dépôt.

**8b**
- [ ] La sélection reste lisible sur la ligne active, contraste vérifié dans les
  deux thèmes.
- [ ] « Activer X » rend X titulaire et lance une passe sur la section du
  curseur ; fichier non enregistré : une seule passe après « Enregistrer et
  demander » ; « Revenir au choix automatique » ne lance rien.

**8c**
- [ ] Source, Aperçu, Lecture et Ctrl+E sur `docs/vision.md` et un ADR ; la
  ligne du curseur montre sa syntaxe ; cocher une case modifie le texte.
- [ ] Lecture sans `innerHTML` : du HTML brut s'affiche comme du texte ; un lien
  relatif ouvre un onglet ; CSP inchangée, aucune erreur dans la console.
- [ ] Ancres des remarques visibles dans les trois modes.

**8d**
- [ ] Modèle et effort modifiables ; un modèle sans sorties structurées est
  refusé ; `reasoning` n'est envoyé qu'aux modèles qui l'acceptent.
- [ ] Tout changement de niveau annonce coût et durée estimés, puis lance
  l'évaluation sur les cas intégrés ; rapport sous `~/.local/share/hodegos/evals/`.
- [ ] `config.toml` garde ses commentaires ; la clé n'est jamais renvoyée au
  front ni journalisée ; la variable d'environnement prime et le dit.

**8e**
- [ ] Personnaliser, éditer, créer, dupliquer, désactiver, revenir au livré ;
  erreurs de validation identiques au chargeur ; rechargement à chaud.
- [ ] Un mentor livré n'est jamais écrit ; `.hodegos/mentors/` n'est créé qu'à
  l'enregistrement d'un mentor de projet.

**8f**
- [ ] Prototype consigné dans l'ADR 0005 (confinement sous WSL2, xterm.js sous la
  CSP, débit).
- [ ] Terminal dans la racine du projet, avec l'UID de l'utilisateur ; `cd /`
  puis `ls ~` ne montre que les montages ; écrire hors du projet et des montages
  `rw` échoue.
- [ ] Sans `bwrap` utilisable : pas de terminal, message explicite.
- [ ] Fermer l'onglet, la fenêtre ou Hodegos tue le shell et ses descendants ;
  un commit fait dans le terminal met l'arbre à jour.

## Hors phase 8

Renommer et déplacer ; afficher les fichiers ignorés ; diff git dans la
gouttière ; effort par mentor ; images dans la lecture Markdown ; terminal non
confiné ; réglages par projet (les réglages restent personnels).
