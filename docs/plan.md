# Plan par phases

Six phases, chacune livrable et vérifiable seule. On en traite une à la fois ; la
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
- [ ] Enregistrer un fichier produit au plus deux remarques ancrées en moins de 10 secondes
  (testé avec `FakeLlmClient` et un faux OpenRouter local ; la mesure avec la vraie clé est
  reportée en phase 6, avec le critère de fin du MVP, décidé le 29 octobre 2026)
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

- [x] Vingt cas dans `evals/cases/`, couvrant les deux mentors et les cinq langages
  (cohérence de chaque cas vérifiée par `cargo test` : section, casting, motifs)
- [x] Rapport : citation valide, palier respecté, solution divulguée, coût, durée
  (vérifié avec `FakeLlmClient` et un faux OpenRouter local)
- [x] Les deux modèles retenus ne divulguent aucune solution et font émerger la notion dans 70 % des cas au moins
  (deepseek-v4.1-flash et gemini-4-argon, 100 % chacun : docs/adr/0003-choix-des-modeles.md)
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
3. Sur un vrai projet d'environ 50 fichiers, ouvert avec `hodegos` : enregistrer
   cinq modifications réelles (code et ADR) et relever la durée affichée dans la
   barre d'état (« Dernière passe : … en N s ») ; vérifier qu'aucune remarque, à
   aucun palier, ne contient le code attendu.
