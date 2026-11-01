# Phase 9 — Des mentors qui voient le projet

1er novembre 2026 · spécification unique, livrée en six sous-phases (9a à 9f)

## Objectif

Aujourd'hui, un mentor ne voit qu'une section, 20 lignes autour et le diff. Il
ne sait pas qu'une décision d'architecture interdit ce qu'il relit, ni qu'un
autre fichier fait déjà la même chose. La phase 9 donne aux mentors une
**connaissance du projet**, tenue à jour à chaque modification, et ajoute des
**mentors de projet** qui relisent l'ensemble.

Le principe ne change pas : les mentors font constater, posent des questions,
montent d'un palier à votre demande ; ils n'écrivent jamais votre code ou votre
texte, dans aucun fichier.

Maquettes et cas d'usage : `docs/maquettes/phase9/` (UC-9.1 à UC-9.14 dans
`README.md`). Décisions : ADR 0009 (connaissance du projet calculée sans
modèle), ADR 0010 (mentors de projet, revues automatiques et réévaluation).

## Décisions prises (1er novembre 2026)

1. Un même mentor couvre une portée ou les deux : `scopes: [section]`,
   `[project]` ou `[section, project]`.
2. Les mentors de projet se lancent d'eux-mêmes : à la création du projet ; à
   son ouverture, si des fichiers ont changé depuis la dernière revue ; après des
   modifications. La connaissance globale est rétablie à chaque modification.
   Tout cela se règle.
3. Les remarques de projet s'affichent d'abord dans la vue Projet, puis dans le
   fichier cité quand on l'ouvre.
4. Passage cité disparu : une remarque liée à ce seul passage disparaît ; une
   remarque liée au projet est réévaluée par son mentor à l'échelle du projet.
5. Les plafonds de contexte se règlent.

Lecture retenue du point 2 : la **connaissance globale** est la carte du
projet (9a). Elle est recalculée à chaque modification enregistrée, sans appel
au modèle, donc sans coût : chaque passe et chaque revue part de l'état courant.
Les **revues** des mentors de projet, elles, coûtent : après des modifications,
elles partent quand l'édition est au calme, avec un intervalle minimal et un
plafond quotidien (9d).

## Précisions retenues à l'intégration (1er novembre 2026)

Questions relevées en confrontant la spécification au code et aux documents, et
tranchées avant 9a.

1. **« Fichiers suivis »** veut dire *non ignorés* : tout fichier de l'arbre de
   Hodegos (hors `.gitignore`, fichiers de secrets et exclusions du projet),
   commité ou non. Carte, diff d'une revue et empreinte fonctionnent donc sans
   git et dès la création par `hodegos new` (`git init` sans commit).
2. **Diff depuis la dernière revue** : à chaque revue, les textes relus sont
   gardés dans un instantané personnel,
   `~/.local/share/hodegos/<id-projet>/snapshots/` : un magasin par contenu
   (empreinte SHA-256 du texte **masqué**, partagé entre mentors) et un
   manifeste par mentor (chemin → empreinte) noté dans `reviews.json`. Aucun
   secret n'y est copié ; les contenus qu'aucun manifeste ne cite plus sont
   supprimés après chaque revue. Le diff ne dépend pas de git.
3. **Vocabulaire** : « mentor de projet » désigne désormais la portée `project`.
   Un mentor rangé dans `<projet>/.hodegos/mentors/` (origine `project`, phase
   8e) devient un **mentor du dépôt** dans les documents et l'interface (libellé
   « du dépôt » dans Réglages · Mentors, changé avec la vue Projet en 9a) ; la
   valeur `origin: project` du protocole ne change pas. Les textes déjà écrits
   de la phase 8 (`docs/phase8.md`, critères 8e) gardent l'ancien sens.
4. **Citations `see_also`** : comme toute citation, elles doivent figurer dans
   le texte **effectivement envoyé** pour ce fichier (signatures, section ou
   fichier entier), en plus d'exister mot pour mot dans le fichier enregistré ;
   sinon la citation est retirée et le lien ouvre le fichier sans endroit
   précis.

Points réglés par défaut, à revoir si l'usage le demande :

5. **`create` et `open`** : `create` est la première ouverture d'un projet pour
   lequel aucune revue n'est notée (`reviews.json` absent), qu'il vienne de
   `hodegos new` ou non (la création n'écrit rien sous `~/.local/share`) ;
   ensuite, c'est `open`.
6. **Mentor `strong-signal` de portée `project`** : pas de revue `create` ;
   revue automatique (`open`, `change`, `commit`) seulement si l'un de ses
   `triggers.keywords` figure dans le diff depuis sa dernière revue. La revue
   demandée reste possible, comme pour un mentor `on-demand`.

## Découpage et ordre

| Sous-phase | Contenu | Dépend de |
| --- | --- | --- |
| 9a | Carte du projet : calcul, plafond, exclusions, onglet « Carte » | — |
| 9b | Contexte des mentors de section : voisinage, carte, « Voir aussi », « Contexte de cette passe » | 9a |
| 9c | Mentors de projet : portées, revue demandée, remarques sur plusieurs fichiers, vue Projet | 9a |
| 9d | Revues automatiques, ancres dans les fichiers, réévaluation | 9c |
| 9e | Réglages : revues du projet, portées dans l'éditeur de mentors, mentors livrés | 9c, 9d |
| 9f | Évaluation sur plusieurs fichiers ; activation par défaut des mentors de projet livrés | 9c à 9e |

Une sous-phase à la fois, avec ses critères de fin (`docs/plan.md`). Tant que
9f n'est pas passée, les mentors livrés n'ont pas la portée `project` (on
l'essaie avec un mentor personnel) et le Mentor du langage garde le contexte
`section`.

## Règles inchangées

- La garde « pas de solution », le cadre commun et ses règles absolues valent
  pour les deux portées ; un fichier de mentor ne peut pas les lever.
- Rien de ce qui est ignoré par git, aucun fichier de secrets, aucune valeur
  masquée n'entre dans la carte, le voisinage ou une revue.
- Tout ce qui est calculé ou réglé est personnel : `~/.config/hodegos`,
  `~/.local/share/hodegos/<id-projet>/`. Rien n'est écrit dans le projet.
- Toute action passe par la WebSocket existante ; tout nouveau message met à
  jour `docs/protocole.md` dans le même commit.
- Le budget mensuel fait foi : au plafond, plus rien d'automatique ; une demande
  explicite reste possible, avec confirmation.
- Aucun appel réseau dans les tests.

---

## 9a — Carte du projet (UC-9.8 à UC-9.10)

Un résumé du projet, calculé par Hodegos sans appel au modèle, envoyé tel quel
aux mentors qui en ont besoin.

### Contenu, par blocs et par ordre de priorité

| Bloc | Contenu | Source |
| --- | --- | --- |
| En-tête | nom du projet, langages et nombre de fichiers, outil de test détecté | arbre, manifestes |
| Vision | `docs/vision.md` sans les commentaires HTML | fichier |
| Décisions | pour chaque ADR (`**/adr/*.md`, `docs/architecture/**`) : numéro, titre, statut, texte de la section Décision | Markdown |
| Arborescence | fichiers non ignorés (précision 1), regroupés par dossier | arbre |
| README | titres et premier paragraphe | Markdown |
| Dépendances | noms et versions des manifestes (`Cargo.toml`, `package.json`, `pyproject.toml`, `requirements.txt`) | manifestes |
| Symboles | par fichier de code : signatures des éléments exportés ou publics, jamais les corps | tree-sitter |
| Tests | fichiers de test et noms des tests | tree-sitter |

- Chaque bloc s'active ou se désactive par projet. Par défaut, tous sont actifs
  sauf Tests.
- Plafond par défaut : **8 000 jetons**, réglable (global dans
  `config.toml`, remplaçable par projet). Au-delà, les blocs sont raccourcis du
  bas vers le haut de la liste (Tests, puis Symboles…), jamais coupés au milieu
  d'une ligne ; un bloc raccourci le dit (« … 12 fichiers non listés »).
- Jetons **estimés** à 4 caractères par jeton (pas de tokenizer local) ; l'écran
  dit « estimés ».
- Exclusions : en plus de `.gitignore` et des fichiers de secrets, une liste de
  motifs par projet (`data/**`…), personnelle.
- Le texte part masqué (`mask_secrets`), comme une section.

### Mise à jour

- Recalcul incrémental à chaque modification enregistrée, dans Hodegos ou
  ailleurs (observateur existant) : seuls les blocs touchés sont recalculés ;
  regroupement de 500 ms.
- Persistée dans `~/.local/share/hodegos/<id-projet>/map.json` (blocs, date,
  empreinte de chaque fichier source) pour ne pas tout recalculer à l'ouverture.
- `map_updated { tokens, cap, updated_at, reason }` est poussé après chaque
  recalcul.

### Écran

Vue Projet, onglet « Carte du projet » (maquette `09-3`) : poids et plafond,
blocs avec cases et jetons, exclusions, aperçu du texte **tel qu'envoyé**.

### Configuration

```toml
[context]
map_cap = 8000             # jetons estimés
neighborhood_cap = 3000    # 9b
key_files_cap = 12000      # 9c
```

Par projet, dans `~/.local/share/hodegos/<id-projet>/context.json` :
`{ map_cap, disabled_blocks, exclude }`.

### Messages

`project_map_get` → `project_map { blocks: [{ id, enabled, tokens, truncated,
detail }], tokens, cap, updated_at, text }` ; `project_map_set { map_cap,
disabled_blocks, exclude }` → `project_map` ; poussé : `map_updated`.

## 9b — Contexte des mentors de section (UC-9.1 à UC-9.3)

### Champ `context`

```yaml
context: section     # section | neighborhood | project ; défaut section
```

- `section` : comportement actuel (section, 20 lignes autour, diff).
- `neighborhood` : en plus, le **voisinage** du fichier.
- `project` : en plus, le voisinage et la carte.

### Voisinage, calculé sans modèle

Pour le fichier relu, par ordre de priorité, jusqu'au plafond (3 000 jetons
par défaut, réglable) :

1. les fichiers qu'il importe, dans le projet (imports résolus par tree-sitter :
   `use`/`mod` en Rust, `import`/`require` relatifs en TypeScript et
   JavaScript, imports relatifs en Python ; liens relatifs en Markdown) ;
2. les fichiers qui l'importent ;
3. son fichier de test (convention du langage : `x.test.ts`, `test_x.py`,
   `#[cfg(test)]` du même fichier, `tests/`) ;
4. les documents Markdown qui citent son chemin ou son dossier : la section qui
   le cite ;
5. les autres fichiers du même dossier.

Un fichier de code voisin est envoyé **en signatures** (pas les corps) ; un
document voisin, par la section concernée. Mêmes exclusions et même masquage
que la carte.

### « Voir aussi »

Le format de sortie d'une remarque de section gagne un champ facultatif :

```json
"see_also": [ { "path": "docs/architecture/0001-stockage.md", "quote": "Un seul module" } ]
```

- 3 entrées au plus ; `path` doit faire partie du contexte envoyé ; `quote`
  (facultative, 3 à 120 caractères) doit exister mot pour mot dans ce fichier
  enregistré. Une entrée invalide est retirée ; la remarque reste.
- Le serveur ajoute `section_id` et `line_hint` ; l'interface en fait des liens
  qui ouvrent le fichier à cet endroit.
- La remarque reste ancrée dans le fichier relu ; `see_also` n'ancre rien.
- La garde « pas de solution » ne change pas.

### « Contexte de cette passe »

Chaque remarque garde la liste de ce qui est parti (`context: [{ block,
detail, tokens }]`, jamais les contenus) ; le panneau l'affiche dans un bloc
repliable (maquette `09-1`), avec un lien vers la carte.

### Prompt

Le contexte supplémentaire est ajouté au message utilisateur, après la section
et avant le diff, sous des titres fixes (« Voisinage », « Carte du projet ») ; le
cadre commun ajoute une règle : « Le voisinage et la carte servent à comprendre ;
vos citations viennent de la section, vos renvois (`see_also`) des fichiers
reçus. »

## 9c — Mentors de projet et vue Projet (UC-9.4, UC-9.6, UC-9.11)

### Portées d'un mentor

```yaml
scopes: [section, project]   # défaut [section]
project:
  key_files: ["docs/architecture/**", "docs/vision.md"]
  max_remarks: 3             # 1 à 3, par revue
  triggers: [create, open, change]   # 9d ; sous-ensemble de create | open | change | commit
```

- `scopes` vide ou inconnu : erreur de validation. `project` est ignoré sans la
  portée `project`.
- Avec la portée `section`, tout fonctionne comme aujourd'hui (casting,
  `triggers`, `on_open`, `context`).
- Avec la portée `project`, le mentor est un **mentor de projet** : il ne passe
  pas par le casting et relit le projet lors d'une **revue**.
- La posture est commune aux deux portées ; `{scope}` y est remplacé par
  `section` ou `projet`.
- Un mentor `on-demand` n'a pas de revue automatique, seulement demandée.

### Revue

Ce que reçoit un mentor de projet, dans cet ordre :

1. la carte (9a) ;
2. ses fichiers clés, en entier, masqués, dans la limite de `key_files_cap`
   (12 000 jetons par défaut ; au-delà, les fichiers les moins récemment
   modifiés sont réduits à leurs signatures ou à leurs titres) ;
3. le diff depuis **sa** dernière revue (fichiers non ignorés, précisions 1 et 2,
   20 fichiers au plus, les plus modifiés) ; à la première revue, aucun diff ;
4. ses remarques de projet encore ouvertes (message et citations), pour ne pas
   se répéter.

Un appel par mentor, en parallèle, modèle de son niveau. Une seule revue à la
fois par mentor ; une revue en cours devient caduque si une nouvelle revue du
même mentor est demandée (coût compté, résultat ignoré).

### Remarques de projet

Ce que le modèle produit :

```json
{
  "remarks": [
    {
      "about": "project",
      "quotes": [
        { "path": "src/emprunts/annuler.ts", "quote": "readFileSync(\"data/emprunts.json\"" },
        { "path": "src/notifications/email.ts", "quote": "writeFileSync(" }
      ],
      "level": "indice",
      "kind": "question",
      "message": "Comptez les fichiers qui écrivent dans data/emprunts.json…",
      "confidence": 0.8
    }
  ]
}
```

- `about` : `passage` (la remarque ne porte que sur un passage : exactement une
  citation) ou `project` (elle porte sur le projet : 0 à 3 citations, dans des
  fichiers différents).
- Chaque citation : 3 à 120 caractères, mot pour mot dans le fichier
  enregistré, et dans un texte **effectivement envoyé** (fichier clé, diff ou
  carte). Une citation invalide est retirée ; une remarque `passage` sans sa
  citation est écartée ; une remarque `project` garde les citations valides.
- `kind: next_step` permis, sans citation (comme en phase 7).
- Garde « pas de solution » : règles de code inchangées aux trois premiers
  paliers ; au palier exemple, les identifiants comptés sont ceux de **toutes**
  les sections citées, et un exemple ne doit recopier aucun fragment de l'une
  d'elles. Sans citation, l'exemple est refusé (palier maximal : piste).
- `max_remarks` du bloc `project` (3 au plus), les plus confiantes.

Ce que le serveur ajoute :

```json
{
  "id": "rp_01J…",
  "scope": "project",
  "mentor_id": "architecte",
  "about": "project",
  "anchors": [ { "path": "src/emprunts/annuler.ts", "section_id": "fn:annuler",
                 "quote": "…", "line_hint": 6, "status": "ok" } ],
  "level": "indice", "levels_given": ["indice"],
  "status": "open", "thread": [], "context": [ … ],
  "review_id": "rv_01J…", "created_at": "…"
}
```

- Persistées dans `~/.local/share/hodegos/<id-projet>/project_remarks.json`.
- Palier suivant, réponse, ignorer, résolue : mêmes messages qu'une remarque
  de section (`hint_next`, `remark_reply`, `remark_dismiss`), qui acceptent un
  identifiant `rp_`. Le palier suivant et la relance reçoivent la carte, les
  fichiers cités et le fil.
- Statuts : ceux d'une remarque, plus `to_review` (9d).

### Vue Projet (maquette `09-2`)

- Bouton « Projet » dans l'en-tête ; l'écran remplace l'éditeur, comme les
  réglages. Onglets : Remarques, Carte du projet (9a), Historique des revues
  (date, déclencheur, mentors, coût, issue).
- Remarques : filtres par mentor, cartes avec citations cliquables (ouvrent le
  fichier à l'endroit), paliers, réponse, ignorer, résolue.
- Panneau : mentors de projet, déclencheurs, dernière revue, état en cours
  (« Relit le projet… »), et « Demander une revue… ».
- **Revue demandée** : dialogue avec les mentors (cochables), ce qui sera
  envoyé, coût et durée estimés (d'après les dernières revues, sinon prix du
  modèle × jetons estimés), budget restant ; « Lancer la revue ».

### Messages

`review_estimate { mentor_ids }` → `review_estimate { mentors: [{ id,
tokens, cost, duration_ms }], total_cost }` ; `review_request { mentor_ids,
confirm_over_budget }` → `review_started { review_id, trigger, mentor_ids }`,
puis `project_remarks { review_id, items }` et `review_done { review_id,
outcomes, cost, duration_ms }` ; `project_remarks_get` → `project_remarks
{ items }` ; `reviews_get` → `reviews { items }` ; `remark_update` porte aussi
les remarques de projet.

## 9d — Revues automatiques, ancres, réévaluation (UC-9.5, UC-9.7, UC-9.12 à UC-9.14)

### Déclencheurs

| Déclencheur | Quand | Pour qui |
| --- | --- | --- |
| `create` | après `hodegos new`, à la première ouverture du projet | mentors de projet `proactive` |
| `open` | au lancement de Hodegos sur le projet, si des fichiers ont changé depuis la dernière revue de ce mentor | idem |
| `change` | après des modifications enregistrées (dans Hodegos ou ailleurs), quand l'édition est au calme | idem, si un fichier modifié est dans le périmètre du mentor (fichiers clés, ou code et documents non exclus) |
| `commit` | après un changement de `HEAD` (désactivé par défaut) | idem |

- Changements détectés par une empreinte des fichiers non ignorés (précision 1),
  notée à chaque revue (`reviews.json`).
- Réglages personnels (maquette `09-5`) : revues automatiques oui ou non ;
  déclencheurs actifs ; **calme** après la dernière modification (3 min par
  défaut) ; **intervalle minimal** entre deux revues d'un mentor (20 min) ;
  **revues automatiques par jour et par mentor** (12). Un mentor peut renoncer à
  un déclencheur (`project.triggers`, éditable en 9e). Valeurs par défaut à
  confirmer à l'usage.
- Les revues automatiques partent sans dialogue, comptent dans le budget, sont
  signalées (barre d'état, panneau) et s'arrêtent au plafond du mois ou au
  plafond quotidien ; la revue demandée reste possible.
- Une revue automatique attend la fin de la revue précédente du même mentor ;
  plusieurs déclencheurs proches ne font qu'une revue.

### Où s'affichent les remarques de projet

- **D'abord dans la vue Projet** ; un compteur sur le bouton « Projet » et,
  dans l'arbre, un repère « P » sur chaque fichier cité.
- **Puis dans le fichier**, une fois ouvert : passage surligné et marqueur de
  projet (forme distincte des remarques de section), et, dans le panneau, une
  section repliée « Remarques de projet sur ce fichier » avec des cartes courtes
  qui renvoient à la vue Projet. Le fil se traite dans la vue Projet.

### Ancres et réévaluation

Les ancres d'une remarque de projet sont vérifiées comme celles d'une remarque
de section (à l'enregistrement, après une modification externe, à l'ouverture
du projet), passage par passage.

- Remarque `about: passage` dont le passage disparaît : `detached`, retirée de
  la vue Projet et du fichier (elle ne s'affiche plus nulle part ; elle reste
  dans `project_remarks.json`).
- Remarque `about: project` dont un passage disparaît : le passage est marqué
  `gone` et la remarque passe en `to_review`. Son mentor la **réévalue** à
  l'échelle du projet, au plus tôt quand l'édition est au calme (mêmes règles de
  rythme et de budget qu'une revue automatique, mais hors plafond quotidien) :
  - il reçoit la carte, ses fichiers clés, les fichiers encore cités, le fil et
    le message ;
  - il répond `{ "verdict": "keep", "quotes": [...] }` (la remarque reste, avec
    0 à 3 citations valides, message et palier inchangés) ou `{ "verdict":
    "withdraw", "reason": "…" }` (raison de 200 caractères au plus, soumise à la
    garde ; la remarque passe en `resolved`, la raison est ajoutée au fil) ;
  - JSON invalide relancé une fois ; sinon, ou sans fournisseur, la remarque
    reste `to_review` et sera réévaluée à la revue suivante.
- Une remarque répondue (`answered`) est réévaluée de la même façon ; votre
  réponse fait partie du fil envoyé.
- Fichier supprimé ou déplacé depuis Hodegos : mêmes règles que les ADR 0006
  et 0008, passage par passage (un passage d'un fichier supprimé est `gone`).

### Messages

Poussés : `review_started` (avec `trigger`), `review_done`,
`project_remarks`, `remark_update` (`to_review`, réévaluée) ;
`project_status { pending: [{ mentor_id, trigger, at }], open, to_review }`.
`file` ajoute `project_remarks` (celles qui citent ce fichier).

## 9e — Réglages et mentors livrés (UC-9.11, UC-9.12)

- **Réglages · Revues du projet** (maquette `09-5`) : déclencheurs, rythme,
  plafonds de contexte (carte, voisinage, fichiers clés), dépense du mois des
  revues automatiques.
  ```toml
  [project_review]
  enabled = true
  triggers = ["create", "open", "change"]
  calm_secs = 180
  min_interval_secs = 1200
  daily_max = 12
  ```
- **Réglages · Mentors** (maquette `09-4`) : portées (cases « Section » et
  « Projet »), « Comme mentor de section » (contexte, déclencheurs du casting,
  remarques par passe), « Comme mentor de projet » (déclencheurs, fichiers clés,
  remarques par revue) ; validation par le chargeur, comme en 8e.
- `hodegos mentors list` affiche les portées et le contexte.
- **Mentors livrés** (activés en 9f, après évaluation) :
  - Mentor du langage : `scopes: [section]`, `context: neighborhood` ;
  - Architecte : `scopes: [section, project]`, `context: project`, fichiers
    clés `docs/architecture/**`, `**/adr/*.md` ; posture : écarts entre décisions
    et code, doublons, dépendances non annoncées ;
  - Mentor produit : `scopes: [section, project]`, `context: project`, fichier
    clé `docs/vision.md` ; posture : écarts entre la vision et ce qui est
    construit, questions ouvertes de la vision ;
  - Testeur (nouveau) : `scopes: [project]`, `model: fast`, déclencheur
    `change` seulement ; posture : comportements sans test, tests qui ne
    vérifient rien.

## 9f — Évaluation sur plusieurs fichiers

- `evals/projects/` : 3 petits projets d'exemple (TypeScript, Rust, Python),
  20 à 40 fichiers chacun, avec vision et ADR, et des écarts plantés (décision
  contournée, doublon, module sans test, vision dépassée).
- Cas de revue (`evals/cases/`, nouveau type `review`) : projet, mentor,
  diff appliqué ; attendus : notions (`must_mention`), fichiers cités, aucun
  fragment de solution **dans aucun des fichiers** (motifs interdits par
  fichier), `about` attendu.
- Cas de voisinage : une section dont la bonne remarque dépend d'un fichier
  voisin ; on compare `context: section` et `neighborhood`.
- Cas de réévaluation : passage supprimé, attendu `keep` ou `withdraw`.
- Seuils de l'ADR 0003 (aucune divulgation, notion ≥ 70 %), plus : citations
  valides ≥ 80 %. Les cas sont intégrés au binaire (ADR 0007) ; un changement de
  niveau les rejoue.
- Coût d'une revue mesuré et noté dans le rapport ; s'il dépasse ce qui tient
  dans 10 $ par mois avec les réglages par défaut (usage d'une journée type
  décrit dans le rapport), on réduit les plafonds par défaut avant d'activer les
  mentors livrés.

---

## Protocole : récapitulatif des ajouts

| Sens | Messages |
| --- | --- |
| Front → backend | `project_map_get`, `project_map_set`, `review_estimate`, `review_request`, `project_remarks_get`, `reviews_get` |
| Backend → front | `project_map`, `map_updated`, `review_estimate`, `review_started`, `review_done`, `project_remarks`, `reviews`, `project_status` |
| Modifiés | `hint_next`, `remark_reply`, `remark_dismiss` (identifiants `rp_`) ; `remark_update` (remarques de projet, `to_review`) ; `remarks` et `remark_update` (champ `see_also`, `context`) ; `file` (`project_remarks`) ; `settings` (`[context]`, `[project_review]`) ; mentors (`scopes`, `context`, `project`) |

## Tests

- 9a : carte d'un projet d'exemple (blocs, ordre, plafond, raccourcis,
  exclusions, `.gitignore`, secrets masqués) ; recalcul incrémental après un
  enregistrement et après une modification externe.
- 9b : voisinage par langage (imports, importateurs, test, document qui cite) ;
  plafond ; `see_also` invalide retiré ; prompt construit avec `FakeLlmClient`.
- 9c : validation de `scopes` et `project` ; revue avec carte, fichiers clés et
  diff ; citations sur plusieurs fichiers (valide, introuvable, hors du texte
  envoyé, même fichier deux fois) ; garde au palier exemple sur plusieurs
  sections ; budget dépassé.
- 9d : chaque déclencheur (création, ouverture avec et sans changement,
  modification avec calme et intervalle, plafond quotidien) avec une horloge
  simulée ; ancres `passage` détachées et masquées ; `to_review` puis `keep` et
  `withdraw` ; JSON invalide.
- 9e : réécriture de `config.toml` qui garde les commentaires ; validation des
  portées dans l'éditeur.
- Front : vérifications manuelles par sous-phase, deux thèmes.

## Critères de fin (à reporter dans `docs/plan.md`)

Pour chaque sous-phase : `cargo test --workspace`, clippy et fmt propres,
`docs/protocole.md` à jour, vérification manuelle dans les deux thèmes.

**9a**
- [ ] La carte d'un projet réel s'affiche telle qu'envoyée, sous son plafond,
  sans fichier ignoré ni secret ; désactiver un bloc ou exclure un chemin la
  change aussitôt.
- [ ] Un enregistrement, dans Hodegos ou ailleurs, met la carte à jour.

**9b**
- [ ] Un mentor `context: neighborhood` reçoit le voisinage ; une remarque peut
  renvoyer à un autre fichier, et le lien l'ouvre à l'endroit cité.
- [ ] « Contexte de cette passe » liste ce qui est parti, sans contenu.

**9c**
- [ ] Un mentor personnel `scopes: [section, project]` fonctionne dans les deux
  portées.
- [ ] « Demander une revue… » annonce coût et durée, puis produit des remarques
  qui citent plusieurs fichiers ; paliers, réponse et fermeture fonctionnent dans
  la vue Projet.

**9d**
- [ ] Revues automatiques à la création, à l'ouverture après un changement fait
  hors de Hodegos, et après des modifications une fois au calme ; intervalle et
  plafond quotidien respectés ; arrêt au plafond du budget.
- [ ] Remarques de projet dans la vue Projet, puis dans le fichier ouvert.
- [ ] Passage supprimé : remarque `passage` disparue ; remarque `project`
  réévaluée, puis maintenue ou levée avec sa raison.

**9e**
- [ ] Revues du projet et portées réglables depuis l'interface ;
  `hodegos mentors list` affiche portées et contexte.

**9f**
- [ ] Cas de revue, de voisinage et de réévaluation au seuil ; rapport dans
  `evals/reports/` avec le coût d'une journée type ; mentors livrés activés dans
  leurs portées.
- [ ] Coût mesuré présenté avant l'activation des mentors livrés ; rythme
  (calme, intervalle, revues par jour) et plafonds de contexte confirmés ou
  réduits.

## Hors phase 9

Index vectoriel et recherche sémantique ; lecture de fichiers à la demande du
modèle (outils) ; fiche projet rédigée par le modèle ; remarques qui proposent
un plan de travail ; revue sur une branche ou une demande de fusion ; contexte
partagé entre projets.
