# Plan par phases

Huit phases, chacune livrable et vérifiable seule (la phase 8 en six sous-phases). On en traite une à la fois ; la
suivante ne commence qu'une fois les critères de fin de la précédente cochés.

## Phase 1 — Squelette

Workspace Cargo et ses cinq crates ; `hodegos ~/projet` démarre axum sur
`127.0.0.1` (port 7420 par défaut, `--port` pour en changer), sert une page vide intégrée au binaire et affiche l'URL
d'accès dans le terminal ; jeton, cookie, contrôles `Origin` et `Host`, redirection
de `localhost` ; WebSocket qui répond à un `ping`.

- [x] `cargo run -p hodegos-cli -- ~/projets/exemple` affiche `http://127.0.0.1:PORT/?token=…`
- [x] Un Ctrl+clic sur l'URL ouvre la page dans le navigateur Windows ; le jeton disparaît de l'URL
- [x] `http://localhost:PORT/…` est redirigé vers `http://127.0.0.1:PORT/…`
- [x] Tests : refus sans jeton, refus d'une `Origin` étrangère, refus d'un `Host` étranger, `ping`/`pong`
- [x] Un chemin de projet sous `/mnt/` est refusé au lancement avec un message clair
- [x] `cargo test`, `clippy -D warnings` et `fmt --check` passent

## Phase 2 — Éditeur

Arborescence, ouverture et enregistrement par le protocole, CodeMirror 6 avec
coloration des cinq langages, mise en page et thèmes du design system, polices
servies par le binaire, détection des modifications externes.

- [x] `scripts/vendor-codemirror.sh` produit `ui/vendor/codemirror.js` (versions épinglées)
- [x] L'arborescence n'affiche ni `.git` ni les fichiers ignorés
- [x] On ouvre, modifie et enregistre un fichier ; `saved` renvoie la nouvelle version
- [x] Une modification hors de Hodegos produit `file_changed` et un bandeau dans l'éditeur
- [x] Conflit de version : `version_conflict` et choix « Recharger » ou « Écraser »
- [x] Thèmes clair et sombre conformes à `docs/maquettes/`, aucune ressource externe chargée
- [x] Vérification manuelle : un fichier de 5 000 lignes reste fluide

## Phase 3 — Sections et casting

Découpage tree-sitter, registre des mentors (trois niveaux, `enabled: false`,
rechargement à chaud), score et casting, message `casting`, panneau des
interlocuteurs, barre d'état, commandes `mentors list` et `mentors eject`.
Pas encore d'appel au modèle.

- [x] Tests de découpage pour Rust, TypeScript, JavaScript, Python et Markdown ; identifiants stables
- [x] Tests de casting : scores, départage, seuils, « aucun mentor », choix forcé, frontmatter `mentors:`
- [x] Déplacer le curseur d'une fonction à un titre d'ADR change le titulaire affiché, avec ses raisons
- [x] Modifier un fichier de mentor s'applique sans redémarrage ; un fichier invalide est signalé
- [x] `mentors list` et `mentors eject architecte` fonctionnent

## Phase 4 — Première passe

Client OpenRouter derrière `LlmClient`, exclusions et masquage des secrets, prompt
en trois couches, validation des remarques, ancrage, persistance dans
`~/.local/share/hodegos/`, affichage des ancres et des cartes, budget.

- [x] Tests avec `FakeLlmClient` : JSON invalide, relance unique, citation introuvable, garde « pas de solution », plafond
- [x] Test : un fichier `.env` ou ignoré par git n'est jamais envoyé ; un secret dans une section est masqué
- [x] Enregistrer un fichier produit au plus deux remarques ancrées en moins de 10 secondes
  (testé avec `FakeLlmClient` et un faux OpenRouter local ; la mesure avec la vraie clé est
  reportée en phase 6, avec le critère de fin du MVP, décidé le 29 octobre 2026 ; délai et
  plafond de deux remarques vérifiés avec la vraie clé le 30 octobre 2026, voir phase 6)
- [x] Budget : avertissement à 80 %, arrêt des passes proactives à 100 %
- [x] Panne du fournisseur : passe abandonnée, état visible, aucune nouvelle tentative

## Phase 5 — Paliers et conversation

`hint_next`, `remark_reply`, `remark_dismiss`, remarques détachées, choix forcé de
l'interlocuteur depuis le panneau (livré en phase 3 ; la phase 5 vérifie que le mentor
choisi tient aussi le fil).

- [x] Une remarque monte de l'indice à l'exemple ; le bouton disparaît au dernier palier
- [x] L'exemple passe la garde « pas de solution » (test avec réponses enregistrées)
- [x] Une remarque dont la citation disparaît passe en `detached` et reste lisible
- [x] Une réponse de l'utilisateur relance le mentor au même palier

Vérifications manuelles (navigateur, faux OpenRouter local via `base_url`) :

- [x] Paliers affichés en `hd-stepper` ; « Palier suivant » absent au palier exemple
- [x] « Répondre » ouvre le champ ; la réponse s'affiche signée « Vous », puis le message du mentor
- [x] Les blocs de code d'un exemple s'affichent en police à chasse fixe, sans HTML venu du modèle
- [x] Carte détachée : badge « Détachée », sans surlignage ni paliers, bouton « Fermer »
- [x] « Résolu », « Ignorer » et « Fermer » retirent la carte ; thèmes clair et sombre lisibles

## Phase 6 — Évaluation et durcissement

Commande `hodegos eval`, vingt cas, rapport, choix des deux modèles,
journalisation propre, documentation d'installation dans le README.

- [x] Vingt cas dans `evals/cases/` (22 depuis la phase 7), couvrant les deux mentors et les cinq langages
  (cohérence de chaque cas vérifiée par `cargo test` : section, casting, motifs)
- [x] Rapport : citation valide, palier respecté, solution divulguée, coût, durée
  (vérifié avec `FakeLlmClient` et un faux OpenRouter local)
- [x] Les deux modèles retenus ne divulguent aucune solution et font émerger la notion dans 70 % des cas au moins
  (deepseek-v4.1-flash et gemini-4-argon, 100 % chacun en phase 6 ; confirmé après la phase 7
  sur 22 cas : 93 % pour `fast`, 86 % pour `strong`, voir docs/adr/0003-choix-des-modeles.md)
- [x] Critère de fin du MVP tenu sur un vrai projet de 50 fichiers
  (vérifié par l'utilisateur le 30 octobre 2026 sur un projet réel : le MVP répond au besoin)
- [x] Avec la vraie clé, un enregistrement produit ses remarques en moins de 10 secondes
  (critère reporté de la phase 4 ; mesuré le 30 octobre 2026 par la WebSocket, de
  `save` à `status` : 1,4 à 5,1 s sur cinq enregistrements TypeScript, Python, Rust
  et deux ADR, avec deepseek-v4.1-flash et gemini-4-argon)
- [x] Journaux : une ligne par passe et par reprise, ni contenu ni clé au niveau par défaut
  (test de la clé au niveau `trace`)
- [x] README : installation, configuration, lancement, mentors, évaluation, journaux, dépannage

Reste à faire avec la vraie clé (par l'utilisateur) :

1. ~~`hodegos eval evals/ --fast <candidat>` et `--strong <candidat>` pour chaque
   candidat ; garder les rapports de `evals/reports/`.~~ Fait le 30 octobre 2026.
2. ~~Consigner le choix dans `docs/adr/0003-choix-des-modeles.md` (candidats, chiffres
   des rapports, coût) et dans `config.toml`.~~ Fait le 30 octobre 2026.
3. ~~Sur un vrai projet d'environ 50 fichiers, ouvert avec `hodegos` : enregistrer
   cinq modifications réelles (code et ADR) et relever la durée affichée dans la
   barre d'état (« Dernière passe : … en N s ») ; vérifier qu'aucune remarque, à
   aucun palier, ne contient le code attendu.~~ Fait le 30 octobre 2026.

## Phase 7 — Kick-off d'un projet

`hodegos new`, gabarit `docs/vision.md`, Mentor produit, passe à l'ouverture
(`on_open`), remarques `next_step` sans citation, commentaires HTML retirés avant
envoi, deux cas d'évaluation (21 et 22). Spécification : `docs/phase7-kickoff.md` ;
décision : `docs/adr/0004-kickoff-par-document.md`.

- [x] `hodegos new` : refus de `/mnt/…` et d'un dossier non vide, création du gabarit avec substitutions, `git init` facultatif, tests à l'appui
  (`crates/hodegos-cli/tests/new.rs`, `Project::create` dans core, lien symbolique vers `/mnt` compris)
- [x] Les commentaires HTML sont retirés avant envoi au modèle (test)
  (section, contexte et diff : core `comments`, `section_prompt`, requêtes du faux client dans `tests/open_pass.rs`)
- [x] `on_open` : passe complète à la première ouverture seulement (test avec `FakeLlmClient`)
  (`crates/hodegos-server/tests/open_pass.rs` : réouverture, redémarrage, panne puis nouvelle tentative, budget, mentor sans `on_open`)
- [x] `next_step` à citation vide accepté et ancré sur le titre ; citation vide refusée pour les autres types (tests)
- [x] Casting : Mentor produit titulaire sur `docs/vision.md`, absent ailleurs (test)
- [x] Vérification manuelle : `hodegos new ~/projets/essai` ouvre `vision.md` et le Mentor produit pose au plus deux questions de cadrage en moins de 10 secondes
  (vérifié par l'utilisateur le 30 octobre 2026)
- [x] `hodegos eval` sur les cas 21 et 22 : aucune violation de `must_not`
  (30 octobre 2026, gemini-4-argon : aucune solution, brouillons compris ; notion émergée
  dans le cas 21 seulement : evals/reports/2026-10-30-deepseek-deepseek-v4.1-flash-1017-google-gemini-4-argon-2.md)
- [x] docs/cadrage.md, docs/plan.md, docs/protocole.md et CLAUDE.md intègrent la phase 7

## Phase 8 — Confort de l'IDE

Fichiers et statut git, sélection et activation d'un mentor, Markdown en aperçu et
en lecture, réglages (modèles, effort, clé, budget, mentors), terminal intégré et
confiné. Spécification : `docs/phase8.md` ; décisions : `docs/adr/0005-terminal-integre-confine.md`,
`docs/adr/0006-suppression-des-remarques.md`, `docs/adr/0007-evaluation-depuis-l-ide.md` ;
maquettes et cas d'usage : `docs/maquettes/phase8/`.

Une sous-phase à la fois, dans l'ordre 8a → 8f. Pour chacune : `cargo test --workspace`,
clippy et fmt propres, `docs/protocole.md` à jour, vérification manuelle dans les deux thèmes.

### 8a — Fichiers et statut git

- [x] Créer un fichier et un dossier depuis l'arbre ; les refus s'affichent à côté du champ
  (core `entries`, `crates/hodegos-server/tests/entries.rs`)
- [x] Supprimer un fichier suivi, un fichier non suivi et un dossier : la confirmation dit ce qui est
  récupérable ; remarques, choix forcés et passe d'ouverture des fichiers supprimés sont retirés
  (`tests/entries.rs` : aperçu avec un vrai dépôt, oubli des données, suppression externe qui les garde)
- [x] Lettres et couleurs git à jour après un enregistrement, un commit et un checkout faits hors de
  Hodegos ; « Pas de dépôt git » sans dépôt
  (core `git`, `crates/hodegos-server/tests/git.rs` : sous-dossier de dépôt, HEAD détachée)

Vérification de l'interface, le 31 octobre 2026, par un parcours scripté dans Chromium sans écran
(Playwright, projet local) dans les deux thèmes : création et refus à côté du champ, menu contextuel,
lettres et point des dossiers, commit et checkout faits hors de Hodegos, trois confirmations de
suppression, Échap, « Pas de dépôt git », aucune erreur dans la console. Confirmé à la main dans le
navigateur Windows le 31 octobre 2026.

### 8b — Sélection et activation d'un mentor

- [x] La sélection reste lisible sur la ligne active, contraste vérifié dans les deux thèmes
  (ligne active sans fond ; sélection ramenée de 25 % à 18 % d'`accent`, minimum 4,54:1 en clair,
  consigné dans `ui/styles/README.md`)
- [x] « Activer X » rend X titulaire et lance une passe sur la section du curseur ; fichier non
  enregistré : une seule passe après « Enregistrer et demander » ; « Revenir au choix automatique »
  ne lance rien
  (`crates/hodegos-server/tests/activate.rs` : section du curseur ou du message, sans `no_change`,
  `skip_pass`, budget dépassé sans confirmation, fichier sensible)

Vérification de l'interface, le 31 octobre 2026, par un parcours scripté dans Chromium sans écran
(Playwright, faux fournisseur local) dans les deux thèmes : sélection en accent à 18 % sur la ligne
active avec liseré et numéro en gras, badge « Relit la section… » puis remarque, « Revenir au choix
automatique » sans appel, dialogue d'un fichier non enregistré (Échap annule, « Enregistrer et
demander » ne fait qu'un appel), aucune erreur dans la console. Reste à confirmer à la main dans le
navigateur Windows.

### 8c — Markdown : source, aperçu en direct, lecture

- [x] Source, Aperçu, Lecture et Ctrl+E sur `docs/vision.md` et un ADR ; la ligne du curseur montre
  sa syntaxe ; cocher une case modifie le texte
- [x] Lecture sans `innerHTML` : du HTML brut s'affiche comme du texte ; un lien relatif ouvre un
  onglet ; CSP inchangée, aucune erreur dans la console
- [x] Ancres des remarques visibles dans les trois modes

Vérification de l'interface, le 31 octobre 2026, par un parcours scripté dans Chromium sans écran
(Playwright, faux fournisseur local) dans les deux thèmes, sur la vision d'un projet créé par
`hodegos new`, un ADR et un document piège (balises `<script>`, `<img onerror>`, bloc HTML, lien
`javascript:`, image distante) : trois modes et Ctrl+E, mode retrouvé après rechargement, syntaxe
brute sur la ligne du curseur, commentaires masqués et atteints aux flèches, case cochée puis
annulée par Ctrl+Z, ancres et marqueurs dans les trois modes, aucun élément `script`, `img` ni
attribut `style` ou `onclick` en lecture, lien relatif ouvert dans un onglet, double-clic vers
l'aperçu, aucune erreur dans la console. `tests/ui.rs` refuse `innerHTML` dans le front. Reste à
confirmer à la main dans le navigateur Windows.

### 8d — Réglages : modèles, effort, clé, budget

- [x] Modèle et effort modifiables ; un modèle sans sorties structurées est refusé ; `reasoning`
  n'est envoyé qu'aux modèles qui l'acceptent
- [x] Tout changement de niveau annonce coût et durée estimés, puis lance l'évaluation sur les cas
  intégrés ; rapport sous `~/.local/share/hodegos/evals/`
- [x] `config.toml` garde ses commentaires ; la clé n'est jamais renvoyée au front ni journalisée ;
  la variable d'environnement prime et le dit

Vérification le 31 octobre 2026 : `tests/settings.rs` et les tests de `hodegos-llm` (faux
OpenRouter local, `FakeLlmClient`, journal capturé), puis un parcours scripté dans Chromium sans
écran dans les deux thèmes : modèle sans sorties structurées refusé, effort « Moyen » du niveau
fort, dialogue d'estimation, évaluation de 7 cas (requêtes avec `reasoning.effort`, rapport et
`results.json` sous `~/.local/share/hodegos/evals/`), résultat sous le seuil signalé dans l'écran
et la barre d'état, retour à l'ancien réglage sans évaluation, clé remplacée (600, jamais dans la
page), test de connexion, budget enregistré, commentaires de `config.toml` conservés, aucune
erreur dans la console. Reste à confirmer à la main dans le navigateur Windows, et une évaluation
réelle avec une vraie clé.

### 8e — Réglages : mentors

- [x] Personnaliser, éditer, créer, dupliquer, désactiver, revenir au livré ; erreurs de validation
  identiques au chargeur ; rechargement à chaud
- [x] Un mentor livré n'est jamais écrit ; `.hodegos/mentors/` n'est créé qu'à l'enregistrement
  d'un mentor de projet

Vérification le 31 octobre 2026 : `tests/mentor_settings.rs` et les tests de `hodegos-mentors`,
puis un parcours scripté dans Chromium sans écran dans les deux thèmes : mentor livré en lecture,
personnalisé, renommé avec un chemin ajouté (fichier réécrit dans le style des livrés), initiales
et priorité refusées sous leurs champs sans écriture, prompt assemblé, fichier brut, mentor de
projet créé (`.hodegos/mentors/` absent jusqu'à l'enregistrement), mentor livré désactivé puis
réactivé, retour au mentor livré après confirmation, aucune erreur dans la console. Reste à
confirmer à la main dans le navigateur Windows.

### 8f — Terminal intégré et confiné

- [x] Prototype consigné dans l'ADR 0005 (confinement sous WSL2, xterm.js sous la CSP, débit)
- [ ] Terminal dans la racine du projet, avec l'UID de l'utilisateur ; `cd /` puis `ls ~` ne montre
  que les montages ; écrire hors du projet et des montages `rw` échoue
- [ ] Sans `bwrap` utilisable : pas de terminal, message explicite
- [ ] Fermer l'onglet, la fenêtre ou Hodegos tue le shell et ses descendants ; un commit fait dans
  le terminal met l'arbre à jour
