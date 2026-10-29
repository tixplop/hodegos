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
| `list_dir` | `path` | Renvoie `dir` (entrées, en excluant `.git` et les fichiers ignorés) |
| `open_file` | `path` | Renvoie `file` (contenu, `version`, langage, sections, remarques ouvertes) |
| `save` | `path`, `content`, `base_version` | Écrit sur disque, renvoie `saved`, puis déclenche une passe |
| `cursor` | `path`, `line`, `col` | Met à jour la section courante, renvoie `casting` si elle change |
| `hint_next` | `remark_id` | Demande le palier suivant ; renvoie `remark_update` |
| `remark_reply` | `remark_id`, `text` | Ajoute votre réponse ; le mentor peut relancer au même palier |
| `remark_dismiss` | `remark_id`, `reason` (`ignored` ou `resolved`) | Ferme la remarque |
| `cast_override` | `path`, `section_id` (facultatif), `mentor_id` (ou `null` pour annuler) | Force un interlocuteur pour la section ou le fichier |

## Du backend vers le front

| Type | Charge utile | Quand |
| --- | --- | --- |
| `pong` | — | réponse à `ping` |
| `dir` | `path`, `entries` (`name`, `kind`, `mentors`) | réponse à `list_dir` |
| `file` | `path`, `content`, `version`, `language`, `sections`, `remarks` | réponse à `open_file` |
| `saved` | `path`, `version` | réponse à `save` |
| `casting` | `path`, `section_id`, `titular`, `consultants`, `available`, `reasons` | ouverture, changement de section, choix forcé |
| `pass_started` | `path`, `sections`, `mentors` | dès qu'une passe démarre |
| `remarks` | `path`, `items` (remarques complètes) | à la fin d'une passe |
| `remark_update` | une remarque complète | palier suivant, réponse, fermeture, détachement |
| `file_changed` | `path`, `version` | fichier modifié hors de Hodegos |
| `mentors_changed` | `warnings` (fichiers invalides) | rechargement à chaud des mentors |
| `status` | `provider_ok`, `budget_used`, `budget_cap`, `last_pass` | au démarrage puis à chaque passe |
| `error` | `code`, `message` (`re` dans l'enveloppe si l'erreur répond à une demande) | erreur liée ou non à une demande |

Le format complet d'une remarque est décrit dans `docs/cadrage.md`, section
« Contrat : format d'une remarque ». Un mentor dans `titular`, `consultants` et
`available` est décrit par `id`, `name`, `initials`, `role`.

## Codes d'erreur

| Code | Sens |
| --- | --- |
| `bad_message` | JSON invalide, type inconnu ou charge utile incorrecte |
| `path_forbidden` | chemin hors de la racine du projet ou exclu |
| `not_found` | fichier ou remarque introuvable |
| `version_conflict` | `base_version` ne correspond plus à la version sur disque |
| `budget_exceeded` | budget mensuel atteint, passe proactive refusée |
| `provider_error` | erreur ou délai dépassé chez le fournisseur de modèles |
| `internal` | erreur inattendue (détail dans le journal du serveur) |

Conflit : si `base_version` ne correspond plus à la version sur disque, le serveur
refuse `save` avec `version_conflict` ; le front propose de recharger ou d'écraser
(un nouvel envoi de `save` avec la version courante).
