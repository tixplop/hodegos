# ADR 0005 — Terminal intégré et confiné au projet

Statut : accepté, confirmé par le prototype de la sous-phase 8f · 31 octobre 2026

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

## Résultat du prototype (31 octobre 2026)

Prototype jetable, hors du dépôt : un binaire Rust (`portable-pty` et `bwrap`), une
page servie avec l'en-tête CSP exact de Hodegos, Chromium sans écran (Playwright).
Machine : WSL2, noyau 6.6, Ubuntu 26.04, `bwrap` 0.11.1, zsh avec oh-my-zsh.

**1. Confinement sous WSL2 : il fonctionne.** Espaces de noms utilisateur
autorisés sans réglage. Dans le shell confiné : `cd /` ne montre que `bin dev etc
home lib lib64 mnt proc sbin tmp usr`, `ls ~` que les montages ; `/usr`, `/etc`
et `~` refusent l'écriture, le projet et `/tmp` l'acceptent ; `git commit` dans le
projet, `cargo build` (dépendance téléchargée), `npm view` et `npm install` passent ;
Ctrl+C, Ctrl+Z, `jobs` et `fg` fonctionnent ; `OPENROUTER_API_KEY` est absente.
Arrêt : tuer `bwrap` tue tout l'espace de noms, y compris un processus détaché par
`setsid` et un autre qui ignore HUP et TERM ; un SIGKILL du parent (arrêt brutal de
Hodegos) fait de même grâce à `--die-with-parent`. Le groupe de processus n'a pas à
être tué à part : pas de `rustix` ni de `nix` dans notre code.

Ajustements à la ligne de commande décrite plus haut :

- `--remount-ro /` après les montages : sans lui, écrire dans `~` réussit (dans la
  racine en mémoire de bwrap, perdue ensuite), contre la règle « écrire hors du
  projet et des montages `rw` échoue ».
- **Pas de `--new-session`** : il retire au shell son terminal de contrôle (Ctrl+Z
  sans effet, Ctrl+C tue tout le terminal). Il protège d'une injection `TIOCSTI`
  dans le terminal *parent* ; ici le parent est un pseudo-terminal propre à ce
  terminal, et le noyau a `dev.tty.legacy_tiocsti = 0`.
- DNS : sous WSL2, `/etc/resolv.conf` est un lien vers `/mnt/wsl/resolv.conf`. La
  cible d'un lien de `/etc/resolv.conf` hors de `/etc` est montée en lecture seule,
  au même chemin.
- Node installé par nvm vit dans `~/.nvm` : sans ce montage, `npm` est introuvable.

**2. xterm.js 5.5.0 sous la CSP : oui, avec le rendu canvas et une cale.** La CSP
n'est pas assouplie.

- Le rendu DOM, par défaut, est inutilisable : il crée deux `<style>` (thème,
  dimensions) et pose un attribut `style` sur les cellules colorées, que la CSP
  refuse (texte sans couleurs ni dimensions).
- Le rendu canvas (`@xterm/addon-canvas` 0.7.0) dessine les cellules sans styles en
  ligne. Les deux `<style>` restants passent par une cale : `createElement('style')`
  rend un élément inerte `hodegos-style`, dont le texte alimente une feuille
  `adoptedStyleSheets` du shadow root qui porte le terminal (comme CodeMirror).
  Résultat : gras, 16, 256 et 16 millions de couleurs, saisie, redimensionnement
  (`@xterm/addon-fit` 0.10.0), **aucun message dans la console**.
- La feuille de xterm.js est intégrée au paquet JavaScript : `fetch` est refusé par
  `connect-src`.
- Le rendu WebGL n'a rien affiché dans Chromium sans écran (rendu logiciel) : non
  retenu, à réessayer plus tard dans un vrai navigateur.

**3. Débit : fluide, et la WebSocket reste disponible.**

- Navigateur, rendu canvas : 10 Mo écrits par lots de 64 Kio en 1,4 s, 18 ms entre
  deux images au 95ᵉ centile (21 ms au plus).
- Serveur : `cat` de 10 Mo dans le terminal confiné, lu par lots de 16 ms au plus et
  64 Kio, encodé en base64, servi dans la même boucle qu'un autre message toutes les
  10 ms. Lecteur rapide : 10 Mo en 0,24 s, aucun retard pour l'autre message.
  Lecteur lent (20 ms par message) : canal borné, la lecture du pseudo-terminal est
  suspendue, au plus 70 Kio en attente ; l'autre message attend 22 ms au pire.

**Points tranchés** (proposés par le plan de 8f, confirmés par le prototype) :

- Configuration du shell : sans elle, zsh lance son assistant de premier démarrage.
  Montés en lecture seule, s'ils existent : `.zshrc`, `.zprofile`, `.zshenv`,
  `.zlogin`, `.bashrc`, `.bash_profile`, `.profile`, `.inputrc`, `.oh-my-zsh`.
  L'historique va dans le `/tmp` du terminal (`HISTFILE`), le cache d'oh-my-zsh et
  `XDG_CACHE_HOME` aussi : perdus à la fermeture.
- `OPENROUTER_API_KEY` et les variables `HODEGOS_*` sont retirées de
  l'environnement du terminal.
- `~/.cargo` en lecture seule, sauf ses caches `registry` et `git` en lecture-
  écriture : `cargo build` télécharge, mais rien n'est déposé dans `~/.cargo/bin`.
  `~/.npm` reste en lecture-écriture (cache) : **limite connue**, son dossier `_npx`
  contient du code que `npx` exécute ensuite, y compris hors du terminal.
- Montages par défaut : `~/.gitconfig` (ro), `~/.cargo` (ro, caches rw),
  `~/.rustup` (ro), `~/.npm` (rw), `~/.nvm` (ro), `~/.local/bin` (ro), et les
  fichiers du shell ci-dessus ; `~/.ssh` jamais par défaut. Un chemin absent est
  ignoré. `[terminal] binds` de `config.toml` remplace cette liste.

**Dépendances retenues**, versions épinglées : `portable-pty` 0.9.0 (19 crates au
total, dont `nix` et `libc` ; il fait le `setsid` et le `TIOCSCTTY` qu'un pseudo-
terminal exige, que `unsafe_code = forbid` nous interdit), `base64` 0.22.1 (déjà
dans `Cargo.lock`) ; côté front, `@xterm/xterm` 5.5.0, `@xterm/addon-canvas` 0.7.0
et `@xterm/addon-fit` 0.10.0 (MIT), empaquetés par `scripts/vendor-codemirror.sh`.

## Conséquences

- Le terminal dépend d'un paquet système (`bubblewrap`) : `README.md` et le
  message d'erreur l'indiquent.
- Les outils hors des montages ne sont pas visibles dans le terminal ; la liste
  `[terminal] binds` de `config.toml` le règle.
- Les tests du confinement ne tournent que si `bwrap` fonctionne (la CI GitHub
  restreint les espaces de noms) ; sinon ils sont ignorés avec un message.
