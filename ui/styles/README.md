# Design system Hodegos

Hodegos (du grec ὁδηγός, « celui qui montre le chemin ») est un environnement de développement où des mentors IA vous guident sans écrire à votre place. Ils lisent ce que vous rédigez, code ou documentation, et interviennent par paliers : un indice, puis une question, puis une piste, et un exemple seulement si vous le demandez. L'interface sert l'écriture : sobre, dense, lisible pendant des heures. Votre texte est la vedette ; les mentors se tiennent à côté, jamais devant.

Le front-end est en HTML, CSS et JavaScript sans framework. Les composants sont des classes CSS préfixées `hd-` dans `ui/styles/components.css`, les variables dans `ui/styles/tokens.css` ; `ui/js/hodegos.js` n'ajoute que de petits utilitaires (`window.Hodegos`). Exemples d'assemblage complet : `docs/maquettes/`.

## Principes

- **Votre texte d'abord.** L'éditeur ou le document occupe la colonne centrale sur `surface-0`. Une remarque ne recouvre jamais ce que vous écrivez : elle s'ancre dessus et s'affiche dans le panneau.
- **Guider sans faire.** Chaque remarque montre son palier (Indice, Question, Piste, Exemple). Le palier suivant se demande d'un clic ; aucun code complet n'apparaît sans demande explicite.
- **Le bon interlocuteur, visible.** Le mentor qui suit une section ou un fichier est toujours affiché, avec la raison de sa présence (« Chemin `docs/architecture/**` »).
- **Une couleur, un rôle.** `accent` = action et sélection. `mentor` = la voix des mentors, et rien d'autre. `highlight` = un passage visé par une remarque, et rien d'autre.
- **Un statut se lit en mots.** Un badge porte toujours un mot (« Question », « Acquis »), jamais une couleur seule.
- **Peu, mais juste.** Deux remarques au plus par passe ; une section a un mentor titulaire et deux consultants au plus.

## Ton et rédaction

- Vouvoiement pour l'interface et pour les mentors.
- Un mentor parle à la première personne, pose des questions plutôt qu'il n'affirme, et propose sans imposer : « Je peux créer le squelette du test. L'assertion reste à vous. »
- Il ne qualifie pas votre travail (« bien », « mauvais ») ; il décrit ce qu'il voit et demande.
- Chaque remarque est signée par son mentor (`hd-msg__author--mentor`).
- Phrases courtes, verbes d'action pour les boutons : « Passer à la piste », « Changer d'interlocuteur ».
- Majuscule en début de libellé seulement. Pas d'emoji. Pas de point d'exclamation.
- Fichiers, fonctions et lignes en `text-code` ou `text-id` : `reserver.ts`, `l.7`.
- Une donnée inconnue est un espace réservé explicite : « [budget] », jamais un chiffre inventé.

## Logo

Le repère est un sentier pointillé qui part d'un point (vous) et mène à un repère d'arrivée (l'objectif), sur une tuile `ink` de 28px aux coins `radius-logo`. Le sentier et le point de départ sont en `surface-0`, l'arrivée en `accent` cerclée de `surface-0`. Il se compose en SVG en ligne (composant Logo) pour suivre le thème ; une version fixe pour le thème clair est dans `ui/hodegos-mark.svg` (icône d'onglet).

- Le nom s'écrit « Hodegos » dans le texte, et « hodegos » en minuscules (`text-wordmark`) à côté du repère.
- Le repère seul sert d'icône (onglet, extension). Ne pas le recolorer, ne pas l'animer, ne pas ajouter de texte dans la tuile.

## Couleurs

Deux thèmes, `light` (par défaut) et `dark`, posés sur `data-theme="…"`.

- Fond d'application `surface-50` ; éditeur, panneaux, cartes `surface-0` ; remplissages discrets `surface-100`.
- Texte `ink` ; métadonnées et numéros de ligne `ink-muted`. Les deux restent au-dessus de 4.5:1 sur `surface-0`, `surface-50` et `surface-100` dans les deux thèmes.
- Séparateurs `border` ; contours de champs et boutons secondaires `border-control` (3:1 minimum).
- Bouton principal : fond `accent`, texte `on-accent`. Ligne courante de l'éditeur et fichier ouvert : `accent-soft`, texte `accent-ink`.
- Mentors : nom et avatar en `mentor` sur `mentor-soft` ou `surface-0` ; encarts de proposition sur `mentor-soft`.
- Statuts : `success`, `warning`, `danger`, chacun avec son fond `-soft`. Le mot du badge est obligatoire.
- Passage visé : fond `highlight`, soulignement `highlight-edge`, texte `ink`.
- Coloration du code : `syntax-keyword`, `syntax-string`, `syntax-number`, `syntax-comment`, `syntax-type`, `syntax-function`, dans l'éditeur seulement. Chacune reste au-dessus de 4.5:1 sur `surface-0`, `surface-50` et `accent-soft` (ligne courante) dans les deux thèmes. Elles n'empruntent ni `accent`, ni `mentor`, ni `highlight`, réservées à leur rôle.

## Typographie

Inter pour l'interface (famille `sans`), JetBrains Mono pour le code (famille `mono`). Elles sont servies par le binaire depuis `ui/fonts/` (woff2 du sous-ensemble latin, copiés par `scripts/vendor-codemirror.sh`, déclarés par `@font-face` dans `ui/styles/fonts.css`) ; jamais depuis Google Fonts, à cause de la CSP. Les piles de repli s'appliquent tant qu'elles se chargent.

- `text-display` : titre du document ouvert, un par écran.
- `text-title` / `text-subtitle` : sections et panneaux.
- `text-body` / `text-body-strong` : texte courant, remarques, libellés de boutons.
- `text-small`, `text-caption`, `text-overline` : métadonnées, badges, intitulés de groupe.
- `text-code` / `text-id` : code (interligne 24px dans l'éditeur), fichiers, lignes.
- `text-wordmark` : le nom à côté du repère, et nulle part ailleurs.

## Espacement, rayons, ombres

- Grille de 4px : `space-1` à `space-10`. Padding de carte `space-3`/`space-4`, de panneau `space-4`, de document `space-8`/`space-10`.
- `radius-sm` pour les identifiants, `radius-md` pour boutons et champs, `radius-logo` pour la tuile du repère, `radius-lg` pour cartes et document, `radius-pill` pour badges, paliers et avatars.
- Les surfaces se séparent par des contours, pas par des ombres. `shadow-popover` est réservé à ce qui flotte.

## Mise en page

L'écran type de Hodegos :

- en-tête `.hd-header` : logo, chemin du fichier, une action principale au plus ;
- navigation `.hd-sidebar` (fichiers, mémoire, remarques à traiter) ;
- zone principale `.hd-main` : onglets de fichiers, puis `.hd-editor` (code) ou `.hd-document` (doc, limité à 860px) ;
- panneau `.hd-panel` (360–440px) : casting des mentors (`hd-mentors`), puis les remarques ;
- barre d'état `.hd-statusbar` : section ou ligne courante, titulaire et consultants, déclencheur de la dernière passe, connexion au fournisseur de modèles.

Les colonnes passent les unes sous les autres sur écran étroit.

## États et accessibilité

- Focus clavier : anneau `focus` de 2px plein, décalé de 2px (`:focus-visible`).
- Survol : `surface-100` sous les éléments neutres, `accent-hover` sous le bouton principal.
- Désactivé : opacité 0.5 et `aria-disabled`.
- Vrais éléments natifs : `<button>`, `<a href>`, `<input>` avec `<label>`. Onglets en `role="tab"` (`Hodegos.initTabs`), choix exclusifs en `aria-pressed` (`Hodegos.initChoices`).
- Les paliers d'aide sont une liste ordonnée avec `aria-current="step"` sur le palier courant.
- Cible tactile de 32px minimum (`hd-btn--sm`), 36–40px par défaut.

## Iconographie

Aucune bibliothèque d'icônes n'est retenue. Icônes SVG en ligne, trait de 2px, 16px, `stroke="currentColor"`, `aria-hidden="true"` quand un libellé les accompagne. Pas d'emoji. Les glyphes `→` et `✓` sont admis dans les étapes et les paliers. Les mentors sont représentés par un avatar de deux lettres en `text-code` (AR, TS, UX), jamais par un visage.

## Utilisation

```html
<link rel="stylesheet" href="/styles/tokens.css">
<link rel="stylesheet" href="/styles/fonts.css">
<link rel="stylesheet" href="/styles/components.css">
<script src="/js/hodegos.js"></script>

<body class="hd-app">
  <button class="hd-btn hd-btn--primary" type="button">Démarrer avec ce point de départ</button>
  <span class="hd-badge hd-badge--info">Question</span>
</body>
```

`Hodegos.setTheme('dark')` change de thème ; `Hodegos.nextHint('indice')` renvoie le palier suivant ; `Hodegos.getAnchor(root)` renvoie le passage sélectionné et l'identifiant stable (`data-anchor-id`) le plus proche.
