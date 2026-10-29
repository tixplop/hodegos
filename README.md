# Hodegos

IDE personnel où des mentors IA vous font progresser sans écrire à votre place.

Ce dépôt contient pour l'instant le kit de démarrage : cadrage, plan, protocole,
design system, maquettes, mentors livrés et cas d'évaluation. Le code arrive phase
par phase (voir docs/plan.md).

## Prérequis (WSL2)

- Rust stable : `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- Outils de compilation : `sudo apt install build-essential pkg-config libssl-dev git`
- Node.js et npm, uniquement pour empaqueter CodeMirror en phase 2
- Un navigateur côté Windows (Edge ou Chrome)
- Une clé OpenRouter, à partir de la phase 4 : `~/.config/hodegos/credentials.toml`
  (droits 600) ou variable `OPENROUTER_API_KEY`

Le dépôt et les projets ouverts dans Hodegos doivent vivre dans le système de
fichiers Linux (`~/…`), pas sous `/mnt/c`.

## Démarrer avec Claude Code

```sh
cd ~/projets/hodegos
git init && git add . && git commit -m "Ajouter le kit de démarrage"
claude
```

Puis coller le contenu de PROMPT.md.

## Contenu

| Chemin | Rôle |
| --- | --- |
| CLAUDE.md | Instructions permanentes pour Claude Code |
| PROMPT.md | Prompt de la première session |
| docs/cadrage.md | Dossier de cadrage complet |
| docs/plan.md | Six phases et critères de fin |
| docs/protocole.md | Messages WebSocket |
| docs/adr/ | Décisions d'architecture |
| docs/maquettes/ | Écrans en HTML statique, à ouvrir dans un navigateur |
| ui/styles/, ui/js/ | Design system Hodegos |
| mentors/ | Mentors livrés par défaut |
| evals/ | Cas d'évaluation des mentors |
