# Hodegos

IDE personnel où des mentors IA vous font progresser sans écrire à votre place.

Vous écrivez votre code ou vos documents ; à chaque enregistrement, le mentor
adapté à la section modifiée (Mentor du langage, Architecte) laisse au plus deux
remarques ancrées sur votre texte. Vous pouvez les faire monter d'un palier
(indice, question, piste, exemple), y répondre ou les fermer. Aucun mentor
n'écrit la solution à votre place.

Hodegos tourne dans WSL2 et s'ouvre dans le navigateur de Windows ; seuls
l'extrait modifié et son contexte partent vers OpenRouter.

## Prérequis (WSL2)

- Rust stable 1.85 ou plus : `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- Un compilateur C pour les grammaires tree-sitter : `sudo apt install build-essential git`
- Un navigateur côté Windows (Edge ou Chrome)
- Une clé OpenRouter
- Node.js et npm, seulement pour régénérer `ui/vendor/` et `ui/fonts/`
  (`scripts/vendor-codemirror.sh`) ; les fichiers produits sont commités

Le dépôt Hodegos et les projets que vous ouvrez doivent vivre dans le système de
fichiers Linux (`~/…`), pas sous `/mnt/c` : les modifications de fichiers n'y
remontent pas, et Hodegos refuse de s'y lancer.

## Installer

```sh
git clone <adresse du dépôt> ~/projets/hodegos
cd ~/projets/hodegos
cargo install --path crates/hodegos-cli
```

Le binaire `hodegos` est installé dans `~/.cargo/bin/` ; il contient l'interface,
les polices, CodeMirror et les mentors livrés. Pour mettre à jour : `git pull`
puis la même commande `cargo install`.

## Configurer OpenRouter

```toml
# ~/.config/hodegos/config.toml
[llm]
provider = "openrouter"
fast   = "[identifiant OpenRouter du modèle rapide]"   # Mentor du langage
strong = "[identifiant OpenRouter du modèle fort]"     # Architecte, Mentor produit
timeout_secs = 30

[budget]
monthly_cap_usd = 10.0
warn_at = 0.8
```

Les modèles retenus et leurs résultats d'évaluation sont dans `docs/adr/`.
Ils doivent accepter les sorties structurées (`structured_outputs` sur la page
du modèle chez OpenRouter).

La clé, dans un fichier lisible par vous seul :

```sh
mkdir -p ~/.config/hodegos
printf 'openrouter_api_key = "sk-or-…"\n' > ~/.config/hodegos/credentials.toml
chmod 600 ~/.config/hodegos/credentials.toml
```

ou dans la variable `OPENROUTER_API_KEY`, qui l'emporte. La clé n'apparaît
jamais dans les journaux ni dans l'interface.

Sans clé ou sans modèle, Hodegos démarre quand même et l'indique dans la barre
d'état ; aucune remarque n'est alors produite.

## Lancer

```sh
hodegos ~/projets/mon-projet
hodegos new ~/projets/nouveau      # crée le projet et docs/vision.md, puis l'ouvre
```

`hodegos new` refuse un dossier sous `/mnt/` ou déjà rempli, lance `git init`
(sans commit ; `--no-git` pour s'en passer) et ouvre l'IDE sur `docs/vision.md`
(`--no-open` pour seulement créer le projet). Le Mentor produit y pose ses
questions de cadrage dès l'ouverture, puis à chaque enregistrement.

Le terminal affiche une adresse `http://127.0.0.1:7420/?token=…` : un Ctrl+clic
l'ouvre dans le navigateur Windows. Le jeton disparaît de l'adresse après le
chargement ; il reste valable tant que `hodegos` tourne (Ctrl+C pour arrêter).
`--port N` change de port.

Dans l'interface :

- Ctrl+S enregistre et déclenche une passe ; les remarques apparaissent à droite,
  ancrées sur le passage cité.
- « Palier suivant » fait avancer une remarque ; « Répondre » relance le mentor
  au même palier ; « Résolu » et « Ignorer » la ferment.
- Le panneau indique qui vous accompagne pour la section sous le curseur, et
  pourquoi ; « Choisir… » force un autre mentor pour cette section.
- La barre d'état montre la dernière passe, l'état d'OpenRouter et le budget.

Ce qui part vers OpenRouter : la section modifiée, son diff, 20 lignes de
contexte de part et d'autre et le chemin du fichier, secrets reconnaissables
masqués. Les fichiers ignorés par git, `.env*`, `*.pem`, `*.key`, les clés SSH
et les fichiers contenant une clé privée ne partent jamais.

## Mentors

```sh
hodegos mentors list                 # mentors actifs, origine, fichiers en erreur
hodegos mentors eject architecte     # copie dans ~/.config/hodegos/mentors/, à modifier
hodegos mentors eject architecte --project   # copie dans .hodegos/mentors/ du projet
```

Ces commandes portent sur le dossier courant. Un mentor est un fichier Markdown
(format dans `docs/cadrage.md`, « Contrat : format d'un mentor »). Les
modifications s'appliquent sans redémarrage. Pour désactiver un mentor livré :
une surcharge avec le même `id` et `enabled: false`.

## Données

- `~/.config/hodegos/` : `config.toml`, `credentials.toml`, mentors personnels.
- `~/.local/share/hodegos/usage.json` : coût cumulé par mois.
- `~/.local/share/hodegos/<id-projet>/` : remarques, choix de mentor et fichiers déjà
  vus par une passe d'ouverture, par projet.

Rien n'est écrit dans vos projets. Seul `.hodegos/mentors/`, si vous le créez, y
contient des mentors propres au projet.

## Évaluer les mentors

```sh
cargo run --release -p hodegos-cli -- eval evals/ --max-cost 3
```

Rejoue vingt-deux cas avec les vrais modèles, compte la dépense dans le budget du mois
et écrit un rapport dans `evals/reports/`. Options, déroulé et critères de choix
d'un modèle : `evals/README.md`.

## Journaux

Les journaux vont sur la sortie d'erreur du terminal : une ligne par passe et
par demande au mentor (fichier, issue, durée, coût), les avertissements et les
erreurs. Ni le contenu de vos fichiers ni la clé n'y figurent au niveau par
défaut. Pour plus de détail :

```sh
RUST_LOG=debug hodegos ~/projets/mon-projet          # tout Hodegos en debug
RUST_LOG=warn,hodegos_server=debug hodegos ~/projets/mon-projet
```

Au niveau `debug`, les motifs de rejet des remarques et les extraits cités
apparaissent.

## Dépannage

- **La page ne s'ouvre pas (`ERR_CONNECTION_REFUSED`)** : le port est peut-être
  réservé par Windows (Hyper-V, WinNAT). Vérifiez avec
  `netsh interface ipv4 show excludedportrange protocol=tcp` côté Windows, puis
  relancez avec `--port` sur un port libre.
- **« Le port 7420 est déjà utilisé »** : une autre instance tourne ; arrêtez-la
  ou passez `--port`.
- **Page « non autorisé » (401)** : ouvrez l'adresse affichée dans le terminal,
  avec son jeton ; un nouveau lancement change le jeton.
- **« OpenRouter indisponible »** : survolez le badge pour la raison (clé
  absente, modèle non configuré, panne). Une passe abandonnée n'est jamais
  relancée seule : enregistrez à nouveau.
- **« Budget du mois atteint »** : plus de passe automatique jusqu'au mois
  suivant (UTC) ou jusqu'à un relèvement de `monthly_cap_usd` ; « Palier
  suivant » et « Répondre » demandent confirmation.
- **« le projet … est sur un disque Windows (/mnt/) »** : déplacez le projet dans `~/`.

## Développer

```sh
cargo build --release
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```

Aucun test n'appelle le réseau. Avant toute contribution, lire `CLAUDE.md`,
`docs/cadrage.md`, `docs/plan.md` et `docs/protocole.md`.

| Chemin | Rôle |
| --- | --- |
| crates/ | Code Rust : core, mentors, llm, server, cli |
| ui/ | Interface (HTML, CSS, JS sans framework), design system Hodegos |
| mentors/ | Mentors livrés par défaut |
| evals/ | Cas d'évaluation des mentors et rapports |
| docs/cadrage.md | Dossier de cadrage complet |
| docs/plan.md | Six phases et critères de fin |
| docs/protocole.md | Messages WebSocket |
| docs/adr/ | Décisions d'architecture |
| docs/maquettes/ | Écrans en HTML statique |
| CLAUDE.md, PROMPT.md | Instructions pour Claude Code |
