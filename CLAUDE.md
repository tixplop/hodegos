# Hodegos

IDE personnel où des mentors IA guident sans écrire à la place de l'utilisateur.
Backend Rust dans WSL2, interface web servie en local, modèles via OpenRouter.

Lire avant toute tâche :
- docs/cadrage.md   : vision, périmètre du MVP, architecture, contrats, prompts
- docs/plan.md      : les sept phases, leurs critères de fin (à cocher)
- docs/protocole.md : messages WebSocket entre le front et le backend
- docs/maquettes/   : maquettes HTML statiques des écrans (référence visuelle)
- ui/styles/README.md : design system Hodegos (règles d'usage)

## Règles non négociables
- Un mentor ne produit jamais le code ou le texte de l'utilisateur. Toute
  modification des prompts ou de la validation garde la règle « pas de solution ».
- Le cadre commun des prompts (règles absolues, paliers, format JSON) n'est pas
  modifiable par un fichier de mentor.
- Le serveur n'écoute que sur 127.0.0.1 et exige le jeton de lancement.
  Adresse canonique 127.0.0.1 ; localhost est redirigé vers elle.
- Rien de personnel dans le dépôt de l'utilisateur : données utilisateur sous
  ~/.config/hodegos et ~/.local/share/hodegos.
- Aucun appel réseau dans les tests. Le LLM passe par le trait LlmClient ;
  les tests utilisent FakeLlmClient.
- Ne jamais envoyer au modèle un fichier ignoré par git ni un fichier de secrets.
- Le front ne charge aucune ressource externe (CSP stricte) : polices et
  CodeMirror sont servis par le binaire.

## Stack
- Rust stable, workspace Cargo : crates/hodegos-{core,mentors,llm,server,cli}.
- tokio, axum, notify, similar, tree-sitter, reqwest, serde, tracing, rust-embed.
- Gabarit de vision : templates/vision.md, intégré au binaire par include_str!.
- Front : ui/ en HTML, CSS, JS sans framework ni étape de build ;
  CodeMirror 6 pré-empaqueté dans ui/vendor/ par scripts/vendor-codemirror.sh
  (script à écrire en phase 2 ; c'est la seule étape npm du projet).
- Design system Hodegos : ui/styles/tokens.css, ui/styles/components.css (classes hd-*),
  ui/js/hodegos.js (utilitaires). Ne pas réinventer de composants qui existent.
- Cible unique : x86_64-unknown-linux-gnu (WSL2). Pas d'empaquetage.

## Dépendances entre crates
core ← mentors ; llm indépendant ; server assemble tout ; cli lance server.
Seul server connaît HTTP et la WebSocket.

## Commandes
- cargo build --release
- cargo test --workspace
- cargo clippy --workspace --all-targets -- -D warnings
- cargo fmt --all --check
- cargo run -p hodegos-cli -- ~/projets/exemple   (lance l'IDE, affiche l'URL d'accès)
- cargo run -p hodegos-cli -- new ~/projets/essai (crée un projet et docs/vision.md, puis l'ouvre)
- cargo run -p hodegos-cli -- eval evals/          (évaluation, vraie clé requise)
- cargo run -p hodegos-cli -- mentors list         (mentors actifs et leur origine)

## Conventions
- Erreurs : thiserror dans les bibliothèques, anyhow seulement dans cli.
- Pas d'unwrap ni d'expect hors des tests ; journaux avec tracing, jamais la clé.
- Chaque appel du cœur prend une Session { user, project } ; aucun état global.
- Messages du protocole : enum serde, tag "type", version v: 1 ; tout ajout
  de message met à jour docs/protocole.md dans le même commit.
- Une décision structurante = un ADR court dans docs/adr/NNNN-titre.md.
- Textes de l'interface en français, vouvoiement (voir ui/styles/README.md).
- Messages de commit : Conventional Commits, description en français à l'impératif
  (`type(portée): description`, types feat, fix, docs, test, refactor, chore ;
  portées : core, mentors, llm, server, cli, ui, editor, eval, adr, plan).

## Façon de travailler
- Suivre docs/plan.md, une phase à la fois ; ne pas anticiper les phases suivantes
  (phase 7 : kick-off par document ; les trois points de départ viendront après).
- Avant de coder une phase : proposer un plan court (fichiers, étapes, tests) et
  attendre validation.
- Une phase est terminée quand ses critères de fin sont vérifiés : tests verts,
  clippy propre, critères cochés dans docs/plan.md.
- En cas d'ambiguïté du cadrage : poser la question plutôt que deviner, et
  consigner la réponse dans le document concerné.
