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
| `save` | `path`, `content`, `base_version` | Écrit sur disque un fichier existant, renvoie `saved`, puis déclenche une passe (à partir de la phase 4) |
| `cursor` | `path`, `line`, `col` (à partir de 1) | Met à jour la section courante, envoie `casting` s'il change ; sans `id`, aucune autre réponse |
| `hint_next` | `remark_id` | Demande le palier suivant ; renvoie `remark_update` |
| `remark_reply` | `remark_id`, `text` | Ajoute votre réponse ; le mentor peut relancer au même palier |
| `remark_dismiss` | `remark_id`, `reason` (`ignored` ou `resolved`) | Ferme la remarque |
| `cast_override` | `path`, `section_id` (facultatif), `mentor_id` (ou `null` pour annuler) | Force un interlocuteur pour la section ou le fichier ; renvoie `casting` (ou `not_found` si le mentor n'est pas actif) |

## Du backend vers le front

| Type | Charge utile | Quand |
| --- | --- | --- |
| `pong` | — | réponse à `ping` |
| `dir` | `path`, `entries` (`name`, `kind` : `dir` ou `file`, `mentors`) | réponse à `list_dir` |
| `file` | `path`, `content`, `version`, `language`, `sections`, `remarks` | réponse à `open_file` |
| `saved` | `path`, `version` | réponse à `save` |
| `casting` | `path`, `section_id`, `section_name`, `titular`, `consultants`, `available`, `reasons`, `forced` | ouverture, changement de section ou de casting (curseur, enregistrement, modification externe, rechargement des mentors), choix forcé |
| `pass_started` | `path`, `sections`, `mentors` | dès qu'une passe démarre |
| `remarks` | `path`, `items` (remarques complètes) | à la fin d'une passe |
| `remark_update` | une remarque complète | palier suivant, réponse, fermeture, détachement |
| `file_changed` | `path`, `version` (`null` si supprimé) | fichier ouvert dans la fenêtre, modifié ou supprimé hors de Hodegos |
| `mentors_changed` | `warnings` (`file`, `message`) | rechargement à chaud des mentors ; aussi à la connexion s'il y a des fichiers en erreur |
| `status` | `provider_ok`, `budget_used`, `budget_cap`, `last_pass` | au démarrage puis à chaque passe |
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
- `remarks` (dans `file`) reste vide jusqu'à la phase 4.

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
| `budget_exceeded` | budget mensuel atteint, passe proactive refusée |
| `provider_error` | erreur ou délai dépassé chez le fournisseur de modèles |
| `internal` | erreur inattendue (détail dans le journal du serveur) |

Conflit : si `base_version` ne correspond plus à la version sur disque, le serveur
refuse `save` avec `version_conflict` et la version sur disque dans `version` ; le
front propose de recharger (`open_file`) ou d'écraser (un nouvel envoi de `save`
avec cette version comme `base_version`).

```json
{ "v": 1, "type": "error", "re": "c-7",
  "payload": { "code": "version_conflict", "message": "…", "version": "3f2a9c0d1e4b5a67" } }
```
