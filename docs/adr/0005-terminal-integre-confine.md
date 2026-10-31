# ADR 0005 — Terminal intégré et confiné au projet

Statut : accepté, sous réserve du prototype de la sous-phase 8f · 31 octobre 2026

## Contexte

La phase 8 ajoute un terminal dans l'IDE, ouvert dans la racine du projet avec
l'utilisateur qui a lancé Hodegos. Le serveur n'écoute que sur 127.0.0.1 et
exige le jeton de lancement (ADR 0001). Un terminal dans le navigateur donne à
qui tient la session l'exécution de commandes : il ne doit ni ouvrir de
nouvelle porte, ni donner accès au reste de la machine.

Un pseudo-terminal lancé par le serveur ne suffit pas à garder le dossier de
travail dans le projet : `cd /` reste possible dans n'importe quel shell. Le
confinement demande que le reste du système de fichiers ne soit pas visible.

## Décision

- Le terminal est un service **intégré** au backend : un pseudo-terminal par
  terminal, ses octets passent par la WebSocket existante (`term_*`), sous le
  même cookie et les mêmes contrôles `Origin` et `Host`. Aucun port, aucun
  processus de service en plus.
- Le shell est lancé dans un **espace de noms** par bubblewrap (`bwrap`, sans
  droits root) : racine du projet en lecture-écriture au même chemin, système
  en lecture seule, `/tmp` vide, reste de `$HOME` absent sauf une liste de
  montages réglable (outils de développement ; `~/.ssh` exclu par défaut).
  `--unshare-pid`, `--die-with-parent`, `--new-session` ; réseau conservé.
- Pas de confinement possible (bwrap absent, espaces de noms refusés) : pas de
  terminal, avec un message qui explique ; jamais de repli sans confinement.
- Rien de ce qui passe dans un terminal n'est envoyé à un mentor ni journalisé
  au-dessus de `trace`.

## Alternatives écartées

- Ouvrir Windows Terminal dans WSL (`wt.exe wsl.exe --cd`) : simple, mais hors
  de l'IDE et sans confinement.
- Un pseudo-terminal sans espace de noms, ou un shell restreint (`rbash`) :
  `cd` hors du projet reste possible ou contournable.
- Un conteneur (Docker, Podman) : dépendance lourde, et les outils de
  l'utilisateur n'y sont pas.

## À vérifier par le prototype (début de 8f)

- `bwrap` et les espaces de noms utilisateur sous WSL2, avec git, cargo et npm
  dans le shell confiné.
- xterm.js sous la CSP stricte, sans l'assouplir.
- Débit d'une sortie de 10 Mo sans bloquer les autres messages.

Le résultat est ajouté ici avant l'intégration. Un échec sur le premier point
remet la décision en question : on revient à l'utilisateur avant de continuer.

## Conséquences

- Le terminal dépend d'un paquet système (`bubblewrap`) : `README.md` et le
  message d'erreur l'indiquent.
- Les outils hors des montages ne sont pas visibles dans le terminal ; la liste
  `[terminal] binds` de `config.toml` le règle.
- Les tests du confinement ne tournent que si `bwrap` fonctionne (la CI GitHub
  restreint les espaces de noms) ; sinon ils sont ignorés avec un message.
