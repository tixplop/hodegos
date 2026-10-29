# Maquettes

Écrans de référence en HTML statique, construits avec le design system de
`ui/styles/`. À ouvrir directement dans un navigateur depuis ce dossier. Pour voir
le thème sombre, remplacer `data-theme="light"` par `data-theme="dark"` sur le
premier `<div class="hd-app">`.

| Fichier | Écran | Dans le MVP |
| --- | --- | --- |
| 01-kickoff.html | Kick-off d'un projet avec le Mentor produit | Non (vision cible) |
| 02-doc-architecture.html | Doc d'architecture : l'Architecte titulaire, remarques ancrées, paliers | Oui, sans le mentor Sécurité |
| 03-code.html | Code : Mentor du langage, ancres dans l'éditeur, étape suivante | Oui, sans le Testeur (le nom affiché « Mentor TypeScript » correspond au Mentor du langage) |
| 04-mentors-progression.html | Liste des mentors, proactivité, progression | En partie : la liste des mentors oui, la progression non |

Ce sont des maquettes, pas du code à réutiliser : les styles en ligne servent la
mise en page de la maquette. L'implémentation s'appuie sur les classes `hd-*` et
sur le protocole, pas sur ce HTML. Seule source extérieure chargée ici : les
polices Google Fonts, pour le confort d'affichage ; l'application, elle, ne charge
rien d'extérieur.
