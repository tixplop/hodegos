# Maquettes de la phase 8

Écrans de référence de la phase 8 (`docs/phase8.md`), en HTML statique, avec le
design system de `ui/styles/`. Mêmes règles que `docs/maquettes/README.md` :
thème sombre en remplaçant `data-theme="light"`, styles en ligne réservés à la
maquette, polices Google Fonts pour l'affichage seulement.

| Fichier | Écran | Sous-phase |
| --- | --- | --- |
| 08-1-arborescence.html | Arbre avec création, lettres et couleurs git | 8a |
| 08-1b-suppression.html | Confirmation de suppression | 8a |
| 08-2-selection-mentor.html | Sélection sur la ligne active ; « Activer » un mentor | 8b |
| 08-3-markdown.html | Markdown : source, aperçu en direct, lecture | 8c |
| 08-4-reglages-modeles.html | Réglages : modèles, effort, clé, budget | 8d |
| 08-5-reglages-mentors.html | Réglages : liste et édition des mentors | 8e |
| 08-6-terminal.html | Terminal du projet | 8f |

La spécification prime sur les maquettes en cas d'écart (par exemple le statut
`R` renommé, ou le dialogue d'estimation avant une évaluation, décrits dans
`docs/phase8.md` sans écran dédié).

## Cas d'usage

### 8.1 — Arborescence

UC-8.1 Créer un fichier : clic droit sur un dossier → « Nouveau fichier », nom saisi dans l'arbre, Entrée. Le fichier est créé vide, ouvert dans un onglet, et ses mentors s'affichent.

UC-8.2 Créer un dossier : même geste, « Nouveau dossier ».

UC-8.3 Supprimer : « Supprimer… » ouvre une confirmation qui dit si git peut restaurer le fichier (suivi : dernière version commitée, les modifications non commitées sont perdues) ou si c'est définitif (non suivi). L'onglet ouvert se ferme ; les remarques du fichier sont supprimées (ADR 0006).

UC-8.4 Lire l'état git : une lettre et une couleur par fichier (M modifié, A ajouté, U non suivi, C conflit) ; un dossier porte un point s'il contient un changement. Mis à jour après un commit, un checkout ou une commande faite dans le terminal.

Refus : chemin hors du projet, nom déjà pris, fichier ignoré par git, sous .git.

### 8.2 — Sélection et activation

UC-8.5 Sélectionner sur la ligne courante : la sélection reste lisible au-dessus du fond de la ligne active (aujourd'hui, accent-soft la masque). La ligne active ne garde qu'un liseré et son numéro en gras.

UC-8.6 Activer un mentor : « Architecte » dans « Disponibles » devient titulaire (choix forcé, comme aujourd'hui) et lance aussitôt une passe sur la section du curseur. Mêmes règles qu'une passe : version enregistrée, budget, exclusions, 2 remarques au plus.

UC-8.7 Fichier modifié non enregistré : l'activation propose « Enregistrer et demander » ou « Demander sur la version enregistrée ».

UC-8.8 Revenir au choix automatique : aucune passe n'est lancée.

### 8.3 — Markdown

UC-8.9 Changer de mode : sélecteur Source · Aperçu · Lecture dans la barre de l'éditeur ; Ctrl+E bascule entre édition et lecture, comme dans Obsidian. Choix mémorisé dans le navigateur.

UC-8.10 Aperçu en direct : titres, gras, listes, liens, cases à cocher et code rendus pendant la frappe ; la ligne du curseur révèle sa syntaxe Markdown. Les commentaires HTML du gabarit sont masqués, sauf sur la ligne active.

UC-8.11 Lecture : document rendu, non éditable ; un lien relatif vers un fichier du projet l'ouvre dans un onglet ; double-clic → retour en aperçu à cet endroit.

Dans les trois modes, les passages visés par les mentors restent surlignés et numérotés.

### 8.4 — Modèles, effort, clé

UC-8.12 Changer le modèle d'un niveau : identifiant OpenRouter vérifié (existe, accepte les sorties structurées) ; le dernier résultat d'évaluation connu est rappelé ; un modèle jamais évalué est signalé. Effet à la passe suivante.

UC-8.13 Régler l'effort de raisonnement par niveau (Par défaut, Faible, Moyen, Élevé), envoyé aux seuls modèles qui l'acceptent ; tout changement de modèle ou d'effort lance une évaluation du niveau, après un dialogue qui annonce le coût et la durée estimés (ADR 0007).

UC-8.14 Saisir la clé d'API : champ masqué, jamais réaffichée (4 derniers caractères), écrite dans credentials.toml en 600. Si OPENROUTER_API_KEY est définie, elle prime : le champ est en lecture seule et l'interface le dit. « Tester la connexion ».

UC-8.15 Budget : plafond, seuil d'avertissement, dépense du mois.

Tout est personnel : écrit sous ~/.config/hodegos, jamais dans le projet.

### 8.5 — Mentors

UC-8.16 Voir les mentors : origine (livré, personnel, projet), mode, niveau de modèle, état (actif, désactivé, en erreur avec la raison).

UC-8.17 Personnaliser un mentor livré : « Personnaliser » copie le fichier (comme mentors eject) dans vos mentors ou ceux du projet, puis l'ouvre. Le mentor livré n'est jamais modifié.

UC-8.18 Éditer : formulaire du frontmatter, validé comme au chargement, et posture en Markdown. « Voir le prompt assemblé » montre le cadre commun en lecture seule suivi de la posture. Enregistrer écrit le fichier ; le rechargement à chaud fait le reste.

UC-8.19 Créer un mentor, vide ou par duplication.

UC-8.20 Désactiver (enabled: false) ou supprimer une surcharge pour revenir au mentor livré.

### 8.6 — Terminal

UC-8.21 Ouvrir : bouton « Terminal » ou Ctrl+` → panneau sous l'éditeur, avec le shell de l'utilisateur qui a lancé hodegos ($SHELL), dans la racine du projet, confiné par bubblewrap : le dossier de travail ne peut pas quitter le projet (ADR 0005). Plusieurs terminaux en onglets.

UC-8.22 Travailler : saisie, couleurs, redimensionnement, copier-coller. Ctrl+` replie le panneau sans tuer le shell.

UC-8.23 Effets : un git commit ou checkout met à jour la coloration de l'arbre ; un fichier ouvert modifié par une commande déclenche le bandeau habituel.

UC-8.24 Fermer : fermer l'onglet tue le processus ; l'arrêt de hodegos tue tous les shells.

Rien de ce qui passe dans le terminal n'est envoyé aux mentors.

