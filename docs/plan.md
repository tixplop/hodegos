# Plan par phases

Neuf phases, chacune livrable et vérifiable seule (la phase 8 en sept sous-phases, la phase 9 en six). On en traite une à la fois ; la
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
`docs/adr/0006-suppression-des-remarques.md`, `docs/adr/0007-evaluation-depuis-l-ide.md`,
`docs/adr/0008-remarques-suivent-un-deplacement.md` ;
maquettes et cas d'usage : `docs/maquettes/phase8/`.

Une sous-phase à la fois, dans l'ordre 8a → 8g. Pour chacune : `cargo test --workspace`,
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
demander » ne fait qu'un appel), aucune erreur dans la console. Confirmé à la main dans le
navigateur Windows le 31 octobre 2026.

### 8c — Markdown : source, aperçu en direct, lecture

- [x] Source, Aperçu, Lecture et Ctrl+E sur `docs/vision.md` et un ADR ; la ligne du curseur montre
  sa syntaxe ; cocher une case modifie le texte
- [x] Lecture sans `innerHTML` : du HTML brut s'affiche comme du texte ; un lien relatif ouvre un
  onglet ; CSP inchangée, aucune erreur dans la console
- [x] Ancres des remarques visibles dans les trois modes
- [x] Tableaux rendus en aperçu (alignement, tableau large) ; source dès que la sélection y entre,
  au clavier comme à la souris ; une remarque ancrée dans une cellule reste rendue, surlignée
  dans sa cellule

Vérification de l'interface, le 31 octobre 2026, par un parcours scripté dans Chromium sans écran
(Playwright, faux fournisseur local) dans les deux thèmes, sur la vision d'un projet créé par
`hodegos new`, un ADR et un document piège (balises `<script>`, `<img onerror>`, bloc HTML, lien
`javascript:`, image distante) : trois modes et Ctrl+E, mode retrouvé après rechargement, syntaxe
brute sur la ligne du curseur, commentaires masqués et atteints aux flèches, case cochée puis
annulée par Ctrl+Z, ancres et marqueurs dans les trois modes, aucun élément `script`, `img` ni
attribut `style` ou `onclick` en lecture, lien relatif ouvert dans un onglet, double-clic vers
l'aperçu, aucune erreur dans la console. `tests/ui.rs` refuse `innerHTML` dans le front. Confirmé
à la main dans le navigateur Windows le 31 octobre 2026.

Tableaux vérifiés le 1er novembre 2026 par un parcours scripté dans Chromium sans écran (Playwright,
projet local, remarques posées dans une cellule et sur deux cellules) dans les deux thèmes :
alignement gauche, centre et droite, cellule vide à sa place, tableau large qui défile dans son
cadre sans élargir l'éditeur (le texte de l'aperçu revient désormais à la ligne), flèches haut et
bas qui entrent dans le tableau et le montrent en source, clic qui place le curseur dans la
cellule visée, passages cités surlignés dans leurs cellules avec leur marqueur, clic sur un
passage qui met sa remarque en avant sans quitter le rendu, `|` droits en source, ancres
inchangées en source, aucune erreur dans la console. Confirmé à la main dans le navigateur Windows
le 1er novembre 2026.

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
erreur dans la console. Confirmé à la main dans le navigateur Windows le 31 octobre 2026 ; une
évaluation réelle avec une vraie clé reste à faire.

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
réactivé, retour au mentor livré après confirmation, aucune erreur dans la console. Confirmé
à la main dans le navigateur Windows le 31 octobre 2026.

### 8f — Terminal intégré et confiné

- [x] Prototype consigné dans l'ADR 0005 (confinement sous WSL2, xterm.js sous la CSP, débit)
- [x] Terminal dans la racine du projet, avec l'UID de l'utilisateur ; `cd /` puis `ls ~` ne montre
  que les montages ; écrire hors du projet et des montages `rw` échoue
- [x] Sans `bwrap` utilisable : pas de terminal, message explicite
- [x] Fermer l'onglet, la fenêtre ou Hodegos tue le shell et ses descendants ; un commit fait dans
  le terminal met l'arbre à jour

Vérification le 31 octobre 2026 : `tests/terminal.rs` (vrai `bwrap` : `ls ~` réduit aux montages,
écriture refusée dans `~` et `/usr`, fermeture de l'onglet et de la fenêtre sans processus
restant, commit qui pousse `git_status` ; faux `bwrap` : protocole et limites ; `bwrap` absent :
message), puis un parcours scripté dans Chromium sans écran dans les deux thèmes : shell dans le
projet, `OPENROUTER_API_KEY` absente, couleurs, redimensionnement par la poignée, Ctrl+` qui
replie sans tuer, deux onglets, fermeture qui tue les descendants, arbre mis à jour après un
commit, aucune erreur dans la console. Confirmé à la main dans le navigateur Windows le 31 octobre
2026, ainsi que l'arbre qui suit les fichiers créés dans le terminal sans commit (`tests/tree.rs`).

### 8g — Déplacer un fichier ou un dossier

- [x] Déplacer un fichier et un dossier par glisser-déposer, et au clavier par « Déplacer vers… » ;
  cibles interdites signalées ; refus du serveur affichés près de l'arbre
  (core `entries` et `tree`, `crates/hodegos-server/tests/moves.rs` : refus, `list_dirs`)
- [x] Remarques, choix forcés et passes d'ouverture suivent le déplacement (ADR 0008) ; échec de leur
  mise à jour : renommage annulé (`tests/moves.rs` : `overrides.json` impossible à écrire)
- [x] Onglets ouverts, même modifiés, et fil d'Ariane suivent le nouveau chemin ; aucun bandeau
  « fichier supprimé » (`tests/moves.rs` : aucun `file_changed` après le déplacement)

Vérification de l'interface, le 31 octobre 2026, par un parcours scripté dans Chromium sans écran
(Playwright, projet local) dans les deux thèmes : surbrillance d'un dossier et de l'en-tête, cible
interdite sans surbrillance, Échap qui annule, dossier replié ouvert après 600 ms, refus « nom déjà
pris » sous l'arbre, dépôt à la racine, « Déplacer vers… » au clavier (filtre, flèches, Entrée),
onglet modifié qui suit puis s'enregistre au nouveau chemin, fil d'Ariane, remarque ancrée à sa
place, casting au nouveau chemin (choix forcé gardé), aucune erreur dans la console. Confirmé à la
main dans le navigateur Windows le 31 octobre 2026.

## Phase 9 — Des mentors qui voient le projet

Carte du projet calculée sans modèle, voisinage et « Voir aussi » pour les mentors de section,
mentors de projet qui relisent l'ensemble (revues demandées puis automatiques), remarques sur
plusieurs fichiers et leur réévaluation, réglages, évaluation sur plusieurs fichiers.
Spécification : `docs/phase9.md` (précisions retenues à l'intégration comprises) ; décisions :
`docs/adr/0009-connaissance-du-projet-sans-modele.md`, `docs/adr/0010-mentors-de-projet.md` ;
maquettes et cas d'usage : `docs/maquettes/phase9/`.

Une sous-phase à la fois, dans l'ordre 9a → 9f. Pour chacune : `cargo test --workspace`,
clippy et fmt propres, `docs/protocole.md` à jour, vérification manuelle dans les deux thèmes.
Jusqu'à 9f, les mentors livrés gardent la portée `section` (et le Mentor du langage le contexte
`section`) : la portée `project` s'essaie avec un mentor personnel.

### 9a — Carte du projet

- [x] La carte d'un projet réel s'affiche telle qu'envoyée, sous son plafond, sans fichier ignoré
  ni secret ; désactiver un bloc ou exclure un chemin la change aussitôt
  (core `map`, `crates/hodegos-server/tests/project_map.rs` : blocs, plafond, `.gitignore`,
  secrets par nom et par contenu, valeur masquée, exclusions, `context.json` hors du projet)
- [x] Un enregistrement, dans Hodegos ou ailleurs, met la carte à jour
  (`tests/project_map.rs` : `save`, dossier jamais ouvert, dossier créé ensuite, fichier ignoré
  sans effet, carte relue depuis `map.json` ; core : seuls les fichiers touchés sont relus)

Vérification de l'interface, le 1er novembre 2026, par un parcours scripté dans Chromium sans écran
(Playwright) dans les deux thèmes, sur un clone du dépôt Hodegos (232 fichiers, 10 ADR) avec un
`.env.local` non ignoré : carte de 7 900 jetons sous 8 000, symboles raccourcis et signalés, ni
`target/` ni `.env.local` ni clé ; Tests activé, `evals/**` exclu, plafond à 3 000 appliqués
aussitôt ; plafond à 50 et motif `../x` refusés sous le poids ; fichier créé hors de Hodegos dans
un dossier jamais ouvert, puis enregistrement de `README.md` depuis l'éditeur, repris par la carte
(« après un changement sur le disque », « après un enregistrement ») ; réglages écrits dans
`~/.local/share/hodegos/<id>/context.json`, rien dans le projet ; aucune erreur dans la console.
Confirmé à la main dans le navigateur Windows le 1er novembre 2026.

### 9b — Contexte des mentors de section

- [x] Un mentor `context: neighborhood` reçoit le voisinage ; une remarque peut renvoyer à un autre
  fichier, et le lien l'ouvre à l'endroit cité
  (core `map::neighbors` et `map::imports`, mentors `prompt`, llm `filter_see_also`,
  `crates/hodegos-server/tests/context.rs` : voisinage en signatures sans ignoré ni secret,
  carte pour `project`, prompt inchangé pour `section`, renvois filtrés et complétés, même
  contexte au palier suivant)
- [x] « Contexte de cette passe » liste ce qui est parti, sans contenu
  (`tests/context.rs` : `block`, `detail`, `tokens` seulement, persistés dans `remarks.json`)

Vérification de l'interface, le 1er novembre 2026, par un parcours scripté dans Chromium sans écran
(Playwright, faux fournisseur local qui renvoie aux fichiers reçus) dans les deux thèmes, sur un
projet TypeScript avec un mentor du dépôt `context: neighborhood` : badge « Contexte : voisinage »,
voisinage envoyé (fichier importé, test qui l'importe, ADR qui cite le fichier, même dossier) sans
corps de fonction ni fichier ignoré, « Voir aussi » avec trois liens qui ouvrent `depot.ts` à la
ligne 3 et l'ADR à la section Décision (ligne 5), « Contexte de cette passe » dépliable (section,
voisinage et chaque fichier avec ses jetons, carte non envoyée), resté ouvert d'un rendu à l'autre,
lien « Ce que savent les mentors » vers la vue Projet, palier suivant envoyé avec le voisinage,
aucune erreur dans la console.
Confirmé à la main dans le navigateur Windows le 1er novembre 2026.

### 9c — Mentors de projet et vue Projet

- [x] Un mentor personnel `scopes: [section, project]` fonctionne dans les deux portées
  (mentors `mentor` et `casting` : portées, bloc `project`, `{scope}`, formulaire, casting sans
  les mentors de projet seuls ; `crates/hodegos-server/tests/review.rs` : passe de section puis
  revue du même mentor)
- [x] « Demander une revue… » annonce coût et durée, puis produit des remarques qui citent plusieurs
  fichiers ; paliers, réponse et fermeture fonctionnent dans la vue Projet
  (core `map::review` : fichiers clés, instantané, diff ; llm `project` : citations sur plusieurs
  fichiers, garde sur chaque section citée ; `tests/review.rs` : estimation par le prix puis par
  la revue précédente, carte, fichiers clés, diff à la deuxième revue, remarques ouvertes
  rappelées, paliers, réponse, fermeture, budget, panne, revue caduque, redémarrage)

Vérification de l'interface, le 1er novembre 2026, par un parcours scripté dans Chromium sans écran
(Playwright, faux fournisseur et faux catalogue locaux) dans les deux thèmes, avec un mentor du
dépôt `scopes: [section, project]` : onglets Remarques, Carte et Historique, panneau des mentors
de projet, « Demander une revue… » avec mentor à cocher, contenu envoyé, jetons, coût et durée
estimés et budget restant, revue lancée (fichiers clés envoyés en entier), deux remarques dont
une étape suivante, citations qui ouvrent le fichier à la ligne, palier suivant, réponse puis
relance du mentor, « Contexte de cette passe », « Résolu » qui retire la remarque, historique
avec coût et durée, aucune erreur dans la console.
Confirmé à la main dans le navigateur Windows le 1er novembre 2026.

### 9d — Revues automatiques, ancres, réévaluation

- [x] Revues automatiques à la création, à l'ouverture après un changement fait hors de Hodegos, et
  après des modifications une fois au calme ; intervalle et plafond quotidien respectés ; arrêt au
  plafond du budget
  (`scheduler.rs` avec une horloge simulée ; `crates/hodegos-server/tests/auto.rs` : création,
  relance sans changement, ouverture après un changement fait ailleurs, calme, intervalle, plafond
  quotidien puis lendemain, budget, commit désactivé par défaut puis activé)
- [x] Remarques de projet dans la vue Projet, puis dans le fichier ouvert
  (`tests/reevaluate.rs` : `file.project_remarks` ; `project_status` à la connexion et à chaque
  changement)
- [x] Passage supprimé : remarque `passage` disparue ; remarque `project` réévaluée, puis maintenue
  ou levée avec sa raison
  (llm : verdicts `keep` et `withdraw` ; `tests/reevaluate.rs` : détachement à l'enregistrement,
  réévaluation au calme après une modification faite ailleurs, maintenue avec une nouvelle
  citation puis levée avec sa raison, réponse illisible retentée après la revue suivante,
  fichier déplacé puis supprimé depuis Hodegos, déplacement annulé si l'écriture échoue)

Vérification de l'interface, le 1er novembre 2026, par un parcours scripté dans Chromium sans écran
(Playwright, faux fournisseur local, `calm_secs = 5`, `min_interval_secs = 10`) dans les deux
thèmes : compteur sur le bouton « Projet », repère « P » sur les fichiers cités, passage surligné en
pointillés avec marqueurs « P1 »… dans le fichier ouvert, section « Remarques de projet sur ce
fichier » qui renvoie à la remarque dans la vue Projet, « Revue du projet prévue au calme » dans la
barre d'état et le panneau après une modification faite hors de Hodegos, revue « Modifications »
dans l'historique, passage cité retiré puis remarque « À réévaluer » avec son explication, levée au
calme, aucune erreur dans la console.
Confirmé à la main dans le navigateur Windows le 1er novembre 2026.

### 9e — Réglages et mentors livrés

- [ ] Revues du projet et portées réglables depuis l'interface ; `hodegos mentors list` affiche
  portées et contexte

### 9f — Évaluation sur plusieurs fichiers

- [ ] Cas de revue, de voisinage et de réévaluation au seuil ; rapport dans `evals/reports/` avec le
  coût d'une journée type ; mentors livrés activés dans leurs portées
- [ ] Coût mesuré présenté avant l'activation des mentors livrés ; rythme (calme, intervalle, revues
  par jour) et plafonds de contexte confirmés ou réduits
- [ ] docs/cadrage.md, docs/plan.md, docs/protocole.md et CLAUDE.md intègrent la phase 9
