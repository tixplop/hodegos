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

- [ ] `scripts/vendor-codemirror.sh` produit `ui/vendor/codemirror.js` (versions épinglées)
- [ ] L'arborescence n'affiche ni `.git` ni les fichiers ignorés
- [ ] On ouvre, modifie et enregistre un fichier ; `saved` renvoie la nouvelle version
- [ ] Une modification hors de Hodegos produit `file_changed` et un bandeau dans l'éditeur
- [ ] Conflit de version : `version_conflict` et choix « Recharger » ou « Écraser »
- [ ] Thèmes clair et sombre conformes à `docs/maquettes/`, aucune ressource externe chargée
- [ ] Vérification manuelle : un fichier de 5 000 lignes reste fluide

## Phase 3 — Sections et casting

Découpage tree-sitter, registre des mentors (trois niveaux, `enabled: false`,
rechargement à chaud), score et casting, message `casting`, panneau des
interlocuteurs, barre d'état, commandes `mentors list` et `mentors eject`.
Pas encore d'appel au modèle.

- [ ] Tests de découpage pour Rust, TypeScript, JavaScript, Python et Markdown ; identifiants stables
- [ ] Tests de casting : scores, départage, seuils, « aucun mentor », choix forcé, frontmatter `mentors:`
- [ ] Déplacer le curseur d'une fonction à un titre d'ADR change le titulaire affiché, avec ses raisons
- [ ] Modifier un fichier de mentor s'applique sans redémarrage ; un fichier invalide est signalé
- [ ] `mentors list` et `mentors eject architecte` fonctionnent

## Phase 4 — Première passe

Client OpenRouter derrière `LlmClient`, exclusions et masquage des secrets, prompt
en trois couches, validation des remarques, ancrage, persistance dans
`~/.local/share/hodegos/`, affichage des ancres et des cartes, budget.

- [ ] Tests avec `FakeLlmClient` : JSON invalide, relance unique, citation introuvable, garde « pas de solution », plafond
- [ ] Test : un fichier `.env` ou ignoré par git n'est jamais envoyé ; un secret dans une section est masqué
- [ ] Enregistrer un fichier produit au plus deux remarques ancrées en moins de 10 secondes
- [ ] Budget : avertissement à 80 %, arrêt des passes proactives à 100 %
- [ ] Panne du fournisseur : passe abandonnée, état visible, aucune nouvelle tentative

## Phase 5 — Paliers et conversation

`hint_next`, `remark_reply`, `remark_dismiss`, remarques détachées, choix forcé de
l'interlocuteur depuis le panneau.

- [ ] Une remarque monte de l'indice à l'exemple ; le bouton disparaît au dernier palier
- [ ] L'exemple passe la garde « pas de solution » (test avec réponses enregistrées)
- [ ] Une remarque dont la citation disparaît passe en `detached` et reste lisible
- [ ] Une réponse de l'utilisateur relance le mentor au même palier

## Phase 6 — Évaluation et durcissement

Commande `hodegos eval`, vingt cas, rapport, choix des deux modèles,
journalisation propre, documentation d'installation dans le README.

- [ ] Vingt cas dans `evals/cases/`, couvrant les deux mentors et les cinq langages
- [ ] Rapport : citation valide, palier respecté, solution divulguée, coût, durée
- [ ] Les deux modèles retenus ne divulguent aucune solution et font émerger la notion dans 70 % des cas au moins
- [ ] Critère de fin du MVP tenu sur un vrai projet de 50 fichiers
