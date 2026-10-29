# Protocole WebSocket

Une seule connexion WebSocket par fenêtre, sur `/ws`, authentifiée par le cookie
de session posé après l'échange du jeton. Elle transporte des messages JSON
versionnés ; le front ne parle jamais directement au disque ni au modèle.

Ce document fait référence : tout ajout ou changement de message le met à jour
dans le même commit.

## Enveloppe

```json
{ "v": 1, "type": "save", "id": "c-42", "payload": { } }
```

- `v` : version du protocole, `1` pour le MVP.
- `type` : nom du message, en `snake_case`.
- `id` : fourni par l'émetteur quand une réponse est attendue ; la réponse reprend
  la même valeur dans `re`, au niveau de l'enveloppe (y compris pour `error`).
- `payload` : objet propre au type ; absent ou `null` pour un message sans charge
  utile (`ping`, `pong`).

Une enveloppe illisible, un type inconnu, une charge utile incorrecte ou une
version `v` différente de `1` reçoivent `error` avec le code `bad_message` ; la
connexion reste ouverte.

```json
{ "v": 1, "type": "ping", "id": "c-1" }
{ "v": 1, "type": "pong", "re": "c-1" }
```

## Du front vers le backend

| Type | Charge utile | Effet |
| --- | --- | --- |
| `ping` | — | Renvoie `pong` (phase 1) |
| `list_dir` | `path` (`""` pour la racine) | Renvoie `dir` (entrées, en excluant `.git` et les fichiers ignorés) |
| `open_file` | `path` | Renvoie `file` (contenu, `version`, langage, sections, remarques ouvertes), puis toujours un `casting` (curseur en ligne 1) |
| `save` | `path`, `content`, `base_version` | Écrit sur disque un fichier existant, renvoie `saved`, puis déclenche une passe : `pass_started` puis `remarks` et `status`, ou directement `status` si aucun appel n'a lieu |
| `cursor` | `path`, `line`, `col` (à partir de 1) | Met à jour la section courante, envoie `casting` s'il change ; sans `id`, aucune autre réponse |
| `hint_next` | `remark_id`, `confirm_over_budget` (facultatif, `false`) | Demande le palier suivant au mentor de la remarque ; renvoie `remark_update` quand il a répondu, puis `status` |
| `remark_reply` | `remark_id`, `text` (2 000 caractères au plus), `confirm_over_budget` (facultatif) | Ajoute votre réponse et renvoie `remark_update` ; le mentor est relancé au même palier, ce qui pousse un second `remark_update`, puis `status` |
| `remark_dismiss` | `remark_id`, `reason` (`ignored` ou `resolved`) | Ferme la remarque (le statut prend la valeur de `reason`) ; renvoie `remark_update` |
| `cast_override` | `path`, `section_id` (facultatif), `mentor_id` (ou `null` pour annuler) | Force un interlocuteur pour la section ou le fichier ; renvoie `casting` (ou `not_found` si le mentor n'est pas actif) |

## Du backend vers le front

| Type | Charge utile | Quand |
| --- | --- | --- |
| `pong` | — | réponse à `ping` |
| `dir` | `path`, `entries` (`name`, `kind` : `dir` ou `file`, `mentors`) | réponse à `list_dir` |
| `file` | `path`, `content`, `version`, `language`, `sections`, `remarks` (ouvertes, répondues ou détachées) | réponse à `open_file` |
| `saved` | `path`, `version` | réponse à `save` |
| `casting` | `path`, `section_id`, `section_name`, `titular`, `consultants`, `available`, `reasons`, `forced` | ouverture, changement de section ou de casting (curseur, enregistrement, modification externe, rechargement des mentors), choix forcé |
| `pass_started` | `path`, `sections` (identifiants), `mentors` (titulaires appelés) | dès qu'une passe appelle le modèle |
| `remarks` | `path`, `items` (nouvelles remarques complètes, au plus 2) | à la fin d'une passe |
| `remark_update` | une remarque complète | réponse à `hint_next`, `remark_reply` et `remark_dismiss` ; relance après une réponse ; détachement après un enregistrement ou une modification externe |
| `file_changed` | `path`, `version` (`null` si supprimé) | fichier ouvert dans la fenêtre, modifié ou supprimé hors de Hodegos |
| `mentors_changed` | `warnings` (`file`, `message`) | rechargement à chaud des mentors ; aussi à la connexion s'il y a des fichiers en erreur |
| `status` | `provider_ok`, `provider_message`, `budget_used`, `budget_cap`, `budget_level`, `last_pass` | à la connexion, puis après chaque enregistrement |
| `error` | `code`, `message`, `version` (seulement pour `version_conflict`) ; `re` dans l'enveloppe si l'erreur répond à une demande | erreur liée ou non à une demande |

Détails des fichiers :

- `path` est relatif à la racine du projet, séparé par `/`.
- `version` est l'empreinte SHA-256 du contenu (16 caractères hexadécimaux) : elle
  change dès que le contenu change, quel qu'en soit l'auteur.
- `language` vaut `rust`, `typescript`, `javascript`, `python`, `markdown` ou `text`.
- Les entrées de `dir` viennent dossiers d'abord, puis par ordre alphabétique.
- Une section : `{ id, kind, name, start_line, end_line, parents }`, lignes à
  partir de 1 et bornes incluses ; `parents` liste les titres parents (Markdown).
  Identifiants : `kind:nom` (`fn:reserver`, `impl:Display-for-Point`, `h:decision`),
  rang ajouté aux homonymes (`fn:new-2`), `top:N` pour le reste.
- `mentors` (dans `dir`) : titulaire et consultant du fichier, calculés sur son
  chemin et son langage seulement ; vide pour un dossier.
- `remarks` (dans `file`) : remarques du fichier au statut `open`, `answered` ou
  `detached`, persistées entre deux lancements.

Détails des passes :

- Une remarque suit le contrat de `docs/cadrage.md`, avec en plus `kind`
  (`observation`, `question` ou `next_step`). `anchor.quote` est vide pour une
  étape suivante sans citation, ancrée au début de la section.
- `status.provider_message` : raison de l'indisponibilité du fournisseur (clé
  absente, modèles non configurés, erreur de la dernière passe), `null` sinon.
- `status.budget_level` : `ok`, `warning` (seuil `warn_at` atteint) ou `exceeded`
  (plafond atteint, plus de passe proactive) ; montants en dollars, mois en UTC.
- `status.last_pass` : `{ at, path, outcome, remarks, duration_ms }`, avec
  `outcome` parmi `ok`, `no_change`, `no_mentor`, `excluded`, `budget_exceeded`,
  `provider_error`, `invalid_response`.
- Les codes `budget_exceeded` et `provider_error` servent aux demandes explicites
  (`hint_next`, `remark_reply`) ; une passe proactive les signale par `status`,
  jamais par `error`.

Détails des remarques (phase 5) :

- `hint_next` : le palier suivant est calculé par le serveur (indice → question →
  piste → exemple). Le message du mentor s'ajoute au fil ; `level` et
  `levels_given` avancent. Si le mentor n'ajoute rien ou si sa réponse est écartée
  (garde « pas de solution », JSON invalide après la relance unique), la remarque
  revient inchangée. Refus : `bad_message` au palier exemple, pour une remarque
  fermée ou détachée, ou si une demande est déjà en cours pour elle ; `not_found`
  pour une remarque ou un mentor inconnus ; `budget_exceeded` au-delà du budget
  sans `confirm_over_budget: true` ; `provider_error` sans fournisseur ou en cas
  de panne, sans nouvelle tentative.
- `remark_reply` : le premier `remark_update` (qui porte `re`) contient votre
  message (`author: "user"`) et le statut `answered`. Le second, spontané, arrive
  quand la relance est finie : avec le message du mentor au même palier, ou
  inchangé si le mentor n'ajoute rien ou ne peut pas être joint. Au-delà du
  budget sans confirmation : `budget_exceeded`, et rien n'est enregistré. Une
  réponse vide, trop longue, ou sur une remarque fermée ou détachée : `bad_message`.
- Détachement : à l'ouverture, à l'enregistrement et après une modification
  externe, chaque remarque active est recalée sur la version enregistrée. Si sa
  section ou sa citation n'existe plus, elle passe en `detached` (définitivement,
  même si la citation revient) ; `remark_update` est poussé, sauf à l'ouverture
  où `file` la contient déjà. Sinon, `anchor.line_hint` est mis à jour sans
  message. Une remarque détachée reste lisible et peut être fermée ; elle
  n'accepte ni palier suivant ni réponse.

Détails du casting :

- `titular` est `null` quand aucun mentor n'atteint le seuil (« Aucun mentor pour
  cette section ») ; `section_id` est `null` pour un fichier vide.
- `available` : mentors actifs ni titulaires ni consultants, `on-demand` compris.
- `reasons` : `{ "<id>": ["chemin docs/architecture/**", "langage markdown",
  "titre « Décision »"] }` pour le titulaire et les consultants.
- `forced` : `"user"` (choix fait par `cast_override`), `"frontmatter"`
  (`mentors:` en tête du document) ou `null`.
- Les sections et le casting portent sur la version enregistrée du fichier.
- `file_changed` ne concerne que les fichiers ouverts ou enregistrés par la fenêtre,
  et seulement si la version sur disque diffère de la dernière connue : un
  enregistrement fait par Hodegos ne le déclenche pas.

Le format complet d'une remarque est décrit dans `docs/cadrage.md`, section
« Contrat : format d'une remarque ». Un mentor dans `titular`, `consultants` et
`available` est décrit par `id`, `name`, `initials`, `role`.

## Codes d'erreur

| Code | Sens |
| --- | --- |
| `bad_message` | JSON invalide, type inconnu ou charge utile incorrecte |
| `path_forbidden` | chemin absolu, avec `..`, hors de la racine (lien symbolique compris), sous `.git` ou exclu par `.gitignore` |
| `not_found` | fichier ou remarque introuvable |
| `version_conflict` | `base_version` ne correspond plus à la version sur disque |
| `not_text` | fichier qui n'est pas un texte UTF-8 de 2 Mo au plus |
| `budget_exceeded` | budget mensuel atteint : demande explicite non confirmée |
| `provider_error` | fournisseur non configuré, erreur ou délai dépassé chez le fournisseur de modèles |
| `internal` | erreur inattendue (détail dans le journal du serveur) |

Conflit : si `base_version` ne correspond plus à la version sur disque, le serveur
refuse `save` avec `version_conflict` et la version sur disque dans `version` ; le
front propose de recharger (`open_file`) ou d'écraser (un nouvel envoi de `save`
avec cette version comme `base_version`).

```json
{ "v": 1, "type": "error", "re": "c-7",
  "payload": { "code": "version_conflict", "message": "…", "version": "3f2a9c0d1e4b5a67" } }
```
