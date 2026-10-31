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
| `open_file` | `path` | Renvoie `file` (contenu, `version`, langage, sections, remarques ouvertes), puis toujours un `casting` (curseur en ligne 1) ; si une passe d'ouverture est due (phase 7), `pass_started` puis `remarks` et `status`, ou directement `status` si aucun appel n'a lieu |
| `save` | `path`, `content`, `base_version`, `skip_pass` (facultatif, `false`) | Écrit sur disque un fichier existant, renvoie `saved`, puis déclenche une passe : `pass_started` puis `remarks` et `status`, ou directement `status` si aucun appel n'a lieu. Avec `skip_pass: true`, aucune passe (phase 8b : l'activation qui suit s'en charge) |
| `cursor` | `path`, `line`, `col` (à partir de 1) | Met à jour la section courante, envoie `casting` s'il change ; sans `id`, aucune autre réponse |
| `hint_next` | `remark_id`, `confirm_over_budget` (facultatif, `false`) | Demande le palier suivant au mentor de la remarque ; renvoie `remark_update` quand il a répondu, puis `status` |
| `remark_reply` | `remark_id`, `text` (2 000 caractères au plus), `confirm_over_budget` (facultatif) | Ajoute votre réponse et renvoie `remark_update` ; le mentor est relancé au même palier, ce qui pousse un second `remark_update`, puis `status` |
| `remark_dismiss` | `remark_id`, `reason` (`ignored` ou `resolved`) | Ferme la remarque (le statut prend la valeur de `reason`) ; renvoie `remark_update` |
| `cast_override` | `path`, `section_id` (facultatif), `mentor_id` (ou `null` pour annuler), `activate` et `confirm_over_budget` (facultatifs, `false`) | Force un interlocuteur pour la section ou le fichier ; renvoie `casting` (ou `not_found` si le mentor n'est pas actif). Avec `activate: true` et un `mentor_id`, lance ensuite une passe demandée (phase 8b, voir plus bas) |
| `create_entry` | `path`, `kind` (`file` ou `dir`) | Crée un fichier vide ou un dossier (phase 8a) ; renvoie `entry_created`, puis pousse `dir` du dossier parent |
| `delete_preview` | `path` | Renvoie `delete_preview` : ce qu'emporterait la suppression, sans rien supprimer |
| `delete_entry` | `path` | Supprime pour de bon (pas de corbeille) un fichier ou un dossier ; renvoie `entry_deleted`, puis pousse `dir` du dossier parent |
| `settings_get` | — | Renvoie `settings` (phase 8d) |
| `settings_set` | `fast`, `strong`, `fast_effort`, `strong_effort` (`default`, `low`, `medium` ou `high`), `budget` (`monthly_cap_usd`, `warn_at`), `confirm_eval` (facultatif, `false`) | Écrit `config.toml` et applique les réglages ; renvoie `settings`. Si un niveau change de modèle ou d'effort vers un réglage non retenu, renvoie d'abord `eval_required` sans rien écrire ; la même demande avec `confirm_eval: true` écrit, puis lance l'évaluation (voir plus bas) |
| `model_check` | `id` | Renvoie `model_info`, d'après le catalogue public d'OpenRouter |
| `api_key_set` | `key` | Écrit `credentials.toml` (droits 600) et refait le client ; renvoie `settings`, jamais la clé. Refusé (`key_from_env`) si `OPENROUTER_API_KEY` est définie |
| `mentors_list` | — | Renvoie `mentors` (phase 8e) |
| `mentor_get` | `id`, `origin` (`builtin`, `user` ou `project`) | Renvoie `mentor_file` |
| `mentor_save` | `origin` (`user` ou `project`), `id`, puis `content` (fichier brut) ou `form` et `body` (formulaire et posture) | Valide avec les règles du chargeur, écrit `<dossier>/<id>.md` (ou le fichier qui définit déjà cet `id` à ce niveau), recharge les mentors ; renvoie `mentor_file`. Refus : `mentor_invalid` avec `errors` |
| `mentor_eject` | `id`, `to` (`user` ou `project`) | « Personnaliser » : copie le mentor actif, comme `hodegos mentors eject` ; renvoie `mentor_file` de la copie |
| `mentor_delete` | `id`, `origin` (`user` ou `project`) | Supprime le fichier de ce niveau (le mentor revient au niveau inférieur, ou disparaît) ; renvoie `mentors` |
| `mentor_prompt` | `id`, `content` (facultatif : brouillon) | Renvoie `mentor_prompt` |
| `term_open` | `cols`, `rows` | Ouvre un terminal confiné dans la racine du projet (phase 8f) ; renvoie `term_opened`, ou `terminal_unavailable`, ou `terminal_limit` (4 par fenêtre) |
| `term_input` | `term_id`, `data` (base64, 64 Kio au plus après décodage) | Saisie ; sans `id`, aucune réponse si elle passe |
| `term_resize` | `term_id`, `cols`, `rows` | Redimensionne le pseudo-terminal ; aucune réponse |
| `term_close` | `term_id` | Tue le shell et tout ce qu'il a lancé ; `term_exited` suit |
| `provider_test` | — | Vérifie la clé (`GET /key`, sans génération ni coût) ; renvoie `provider_test_result` |

## Du backend vers le front

| Type | Charge utile | Quand |
| --- | --- | --- |
| `pong` | — | réponse à `ping` |
| `dir` | `path`, `entries` (`name`, `kind` : `dir` ou `file`, `mentors`) | réponse à `list_dir` ; poussé aussi quand les entrées d'un dossier déjà listé changent sur le disque (fichier créé, supprimé ou renommé, hors fichiers ignorés), et après `create_entry` et `delete_entry` |
| `file` | `path`, `content`, `version`, `language`, `sections`, `remarks` (ouvertes, répondues ou détachées) | réponse à `open_file` |
| `saved` | `path`, `version` | réponse à `save` |
| `casting` | `path`, `section_id`, `section_name`, `titular`, `consultants`, `available`, `reasons`, `forced` | ouverture, changement de section ou de casting (curseur, enregistrement, modification externe, rechargement des mentors), choix forcé |
| `pass_started` | `path`, `sections` (identifiants), `mentors` (titulaires appelés) | dès qu'une passe appelle le modèle |
| `remarks` | `path`, `items` (nouvelles remarques complètes, au plus 2) | à la fin d'une passe |
| `remark_update` | une remarque complète | réponse à `hint_next`, `remark_reply` et `remark_dismiss` ; relance après une réponse ; détachement après un enregistrement ou une modification externe |
| `file_changed` | `path`, `version` (`null` si supprimé) | fichier ouvert dans la fenêtre, modifié ou supprimé hors de Hodegos |
| `mentors_changed` | `warnings` (`file`, `message`) | rechargement à chaud des mentors ; aussi à la connexion s'il y a des fichiers en erreur |
| `status` | `provider_ok`, `provider_message`, `budget_used`, `budget_cap`, `budget_level`, `last_pass`, `initial_file` | à la connexion, puis après chaque passe (enregistrement ou ouverture) |
| `entry_created` | `path`, `kind` | réponse à `create_entry` |
| `delete_preview` | `path`, `kind`, `files`, `untracked`, `modified`, `ignored`, `restorable` | réponse à `delete_preview` |
| `entry_deleted` | `path`, `kind` | réponse à `delete_entry` |
| `git_status` | `branch`, `detached`, `entries` (`path`, `status`), `changes`, `conflicts` ; `null` sans dépôt ou sans git | à la connexion, juste après le premier `status`, puis à chaque changement du statut |
| `settings` | `fast`, `strong`, `fast_effort`, `strong_effort`, `budget` (`monthly_cap_usd`, `warn_at`, `used_usd`), `key_source` (`env`, `file` ou `none`), `key_hint`, `editable`, `config_dir`, `last_eval` (`fast`, `strong`), `eval_running` (`fast`, `strong` ou `null`) | réponse à `settings_get`, `settings_set` et `api_key_set` ; poussé à toutes les fenêtres après un changement et à la fin d'une évaluation |
| `eval_required` | `level`, `cases`, `estimated_cost`, `estimated_duration_ms`, `max_cost`, `basis` (`same`, `tokens` ou `prompts`) | réponse à `settings_set` non confirmé quand un niveau doit être réévalué |
| `model_info` | `id`, `exists`, `structured_outputs`, `reasoning`, `prompt_price`, `completion_price` (dollars par jeton) | réponse à `model_check` |
| `provider_test_result` | `ok`, `message` | réponse à `provider_test` |
| `eval_progress` | `level`, `done`, `total`, `cost` | poussé à toutes les fenêtres au début et à la fin de chaque cas d'une évaluation |
| `eval_done` | un résultat d'évaluation (voir plus bas) | poussé à toutes les fenêtres à la fin d'une évaluation, suivi de `settings` et `status` |
| `mentors` | `items` (`id`, `name`, `initials`, `origin`, `path`, `overrides`, `enabled`, `mode`, `model`, `on_open`), `errors` (`origin`, `path`, `id`, `errors`) | réponse à `mentors_list` et `mentor_delete` |
| `mentor_file` | `id`, `origin`, `path`, `content`, `editable`, `form`, `body`, `errors` | réponse à `mentor_get`, `mentor_save` et `mentor_eject` |
| `mentor_prompt` | `text` | réponse à `mentor_prompt` |
| `term_opened` | `term_id`, `shell` (nom court, pour l'onglet), `warning` (`[terminal] binds` invalide) | réponse à `term_open` |
| `term_output` | `term_id`, `data` (base64 : octets bruts, pas forcément de l'UTF-8) | sortie du terminal, regroupée toutes les 16 ms au plus, 64 Kio au plus par message |
| `term_exited` | `term_id`, `code` (`null` si tué par un signal) | fin du shell (`exit`, `term_close`) |
| `error` | `code`, `message`, `version` (seulement pour `version_conflict`), `errors` (seulement pour `mentor_invalid`) ; `re` dans l'enveloppe si l'erreur répond à une demande | erreur liée ou non à une demande |

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
  (`observation`, `question` ou `next_step`). `anchor.quote` peut être vide pour
  une étape suivante (`next_step`) seulement : elle est alors ancrée au début de
  la section (`line_hint` = `start_line`), c'est-à-dire sur son titre en Markdown,
  et l'interface marque cette ligne.
- Passe d'ouverture (phase 7) : à l'`open_file` d'un fichier dont le titulaire
  d'au moins une section a `on_open: true`, si aucune remarque n'a encore été faite
  sur ce fichier et qu'aucune passe d'ouverture n'y a abouti. Elle porte sur les
  sections dont le titulaire a `on_open` (8 au plus, dans l'ordre du fichier), le
  fichier étant traité comme créé ; budget, exclusions et plafond de 2 remarques
  comme une autre passe. Une seule à la fois par fichier et par fenêtre.
- `status.initial_file` : chemin relatif du fichier que le front ouvre à la
  connexion (`docs/vision.md` après `hodegos new`), dans le premier `status` de la
  connexion seulement ; `null` partout ailleurs.
- `status.provider_message` : raison de l'indisponibilité du fournisseur (clé
  absente, modèles non configurés, erreur de la dernière passe), `null` sinon.
- `status.budget_level` : `ok`, `warning` (seuil `warn_at` atteint) ou `exceeded`
  (plafond atteint, plus de passe proactive) ; montants en dollars, mois en UTC.
- `status.last_pass` : `{ at, path, outcome, remarks, duration_ms }`, avec
  `outcome` parmi `ok`, `no_change`, `no_mentor`, `excluded`, `budget_exceeded`,
  `provider_error`, `invalid_response`.
- Les codes `budget_exceeded` et `provider_error` servent aux demandes explicites
  (`hint_next`, `remark_reply`) ; une passe proactive les signale par `status`,
  jamais par `error`. Une passe demandée (`cast_override` avec `activate`) reçoit
  `budget_exceeded` par `error`, comme une demande explicite ; une panne du
  fournisseur arrive par `status`, comme pour une passe.

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
- Activer un mentor (phase 8b) : `cast_override` avec `activate: true` applique le
  choix forcé, renvoie `casting`, puis lance une passe demandée avec ce mentor :
  `pass_started`, `remarks` et `status`, ou directement `status` (fichier sensible :
  `excluded` ; fournisseur absent : `provider_error`). La section relue est
  `section_id`, sinon celle du dernier `cursor` ; la passe porte sur la version
  enregistrée, avec un diff vide, même si rien n'a changé (pas de `no_change`).
  Exclusions, masquage des secrets, modèle du palier du mentor et plafond de 2
  remarques s'appliquent comme pour une passe. Au-delà du budget sans
  `confirm_over_budget: true` : `error` `budget_exceeded`, et le choix forcé n'est
  pas appliqué. Avec `mentor_id: null`, `activate` est ignoré et rien n'est relu. Un
  enregistrement plus récent rend la passe caduque.
- Fichier non enregistré : le front envoie d'abord `save` avec `skip_pass: true`,
  puis `cast_override` avec `activate: true` ; il n'y a qu'une passe, celle du
  mentor activé.
- Les sections et le casting portent sur la version enregistrée du fichier.
- `file_changed` ne concerne que les fichiers ouverts ou enregistrés par la fenêtre,
  et seulement si la version sur disque diffère de la dernière connue : un
  enregistrement fait par Hodegos ne le déclenche pas.

Détails de l'arbre (phase 8a) :

- `create_entry` : `path` est le chemin complet de l'entrée nouvelle (dossier parent
  puis nom). Refus : chemin vide, composant vide ou `.` (`bad_message`) ; `..`, `\0`,
  `.git`, chemin exclu par `.gitignore` ou qui sort de la racine (`path_forbidden`) ;
  dossier parent absent (`not_found`, aucun dossier intermédiaire n'est créé) ; nom
  déjà pris, lien symbolique compris (`already_exists`). Un fichier est créé vide, avec
  les droits par défaut de l'utilisateur ; le front l'ouvre ensuite par `open_file`.
- `delete_preview` et `delete_entry` refusent la racine, `.git`, un chemin exclu ou
  hors de la racine (`path_forbidden`) et un chemin absent (`not_found`). Un lien
  symbolique est supprimé lui-même, jamais suivi (`kind: "file"`, même vers un
  dossier). Un dossier est supprimé avec tout son contenu, fichiers ignorés par git
  compris.
- Comptes de `delete_preview` (fichiers seulement, dossiers non compris) : `files` en
  tout ; `ignored`, exclus par `.gitignore` ou sous un `.git` imbriqué, absents de
  l'arbre ; `untracked`, non suivis ou seulement ajoutés à l'index (tous les fichiers
  non ignorés sans dépôt) ; `modified`, suivis avec des modifications non commitées
  (`M`, `R`, `C`). `restorable` vaut `true` s'il y a un dépôt et que les trois
  derniers comptes sont nuls.
- Une suppression faite par `delete_entry` oublie, pour chaque fichier supprimé, ses
  remarques quel que soit leur statut, ses choix forcés et sa passe d'ouverture
  (`docs/adr/0006-suppression-des-remarques.md`) ; une passe en cours sur ce fichier
  devient caduque, et la fenêtre ne reçoit pas de `file_changed` pour lui. Une
  suppression faite hors de Hodegos garde tout.
- `git_status` : calculé par `git status --porcelain=v2 -z --branch
  --untracked-files=all -- .` lancé à la racine du projet, avec `GIT_OPTIONAL_LOCKS=0`
  et la configuration git de l'utilisateur. `status` vaut `M` (modifié dans l'index ou
  l'arbre de travail), `A` (ajouté à l'index, ou copié), `U` (non suivi), `C` (en
  conflit) ou `R` (renommé). Un fichier suivi supprimé n'est pas dans `entries` mais
  compte dans `changes`. `branch` est `null` quand `detached` vaut `true`. Un projet
  dans un sous-dossier d'un dépôt, ou dans un worktree, reçoit des chemins relatifs à
  sa racine ; ce qui est hors du projet n'apparaît pas.
- Recalcul, après 300 ms de regroupement : après `save`, `create_entry`,
  `delete_entry`, toute modification observée dans les dossiers suivis, et quand
  `HEAD`, `index`, `packed-refs` ou `refs/` changent dans le dossier git (commit,
  checkout faits ailleurs). `git_status` n'est poussé que si le statut a changé ; à
  chaque `git_status`, le front recharge (`list_dir`) les dossiers dépliés.

Le format complet d'une remarque est décrit dans `docs/cadrage.md`, section
« Contrat : format d'une remarque ». Un mentor dans `titular`, `consultants` et
`available` est décrit par `id`, `name`, `initials`, `role`.

Détails des réglages (phase 8d, ADR 0007) :

- La clé n'est jamais renvoyée ni journalisée : `settings` ne donne que son
  origine (`key_source`) et ses quatre derniers caractères (`key_hint`).
  `OPENROUTER_API_KEY` prime sur `credentials.toml`.
- `config.toml` est réécrit avec ses commentaires, son ordre et ses autres clés
  (`toml_edit`), de façon atomique ; les réglages valent dès la passe suivante,
  sans redémarrage. `editable: false` : réglages en lecture seule (aucun dossier
  de configuration).
- Un modèle inconnu du catalogue, sans sorties structurées (`structured_outputs`),
  ou un effort autre que `default` pour un modèle sans `reasoning` sont refusés
  (`model_refused`). `reasoning: { effort }` n'est envoyé au modèle que si le
  catalogue confirme qu'il l'accepte ; il est absent pour `default`.
- Un niveau dont le modèle ou l'effort change est réévalué, sauf si ce réglage a
  déjà un résultat retenu (retour à l'ancien réglage, choix livrés) : un seul
  niveau à la fois (`invalid_settings` sinon), une évaluation à la fois
  (`eval_running`), clé requise (`provider_error`).
- Estimation (`basis`) : `same`, coût et durée du dernier passage de ce réglage ;
  `tokens`, prix du modèle × jetons du dernier passage du niveau ; `prompts`,
  prix du modèle × jetons estimés d'après les prompts des cas (aucun passage
  mesuré). `max_cost` : plafond de l'évaluation (3 USD).
- L'évaluation joue les cas intégrés du niveau avec les mentors livrés ; les
  passes continuent pendant ce temps, avec le nouveau réglage. Sa dépense compte
  dans `usage.json`. Rapport : `~/.local/share/hodegos/evals/AAAA-MM-JJ-<modèle>-<effort>.md`,
  réécrit après chaque cas et marqué partiel jusqu'à la fin ; résultats dans
  `evals/results.json` du même dossier.
- Un résultat (`eval_done`, `last_eval.fast`, `last_eval.strong`) :
  `{ level, model, effort, date, passed, notion, cases, divulged, cost,
  duration_ms, median_ms, prompt_tokens, completion_tokens, report, origin,
  partial, previous }`. `passed` : aucune solution divulguée, notion dans 70 %
  des cas au moins, tous les cas joués. `origin` : `builtin` (résumé des choix
  livrés, `evals/livres.toml`) ou `ide`. `previous` : réglage du niveau avant le
  changement, pour « Revenir à l'ancien réglage ». `last_eval` donne le dernier
  résultat du réglage actuel de chaque niveau, `null` s'il n'a jamais été évalué.

Détails des mentors (phase 8e) :

- Validation : exactement celle du chargeur (`validate` dans `hodegos-mentors`),
  qui collecte toutes les erreurs ; chacune porte son champ (`id`, `name`,
  `initials`, `role`, `mode`, `model`, `priority`, `entry_level`, `max_remarks`,
  `triggers.paths[N]`, `frontmatter`, `yaml`). Le message du chargeur est leur
  réunion, séparée par « ; ». S'y ajoutent l'identifiant déjà défini au même
  niveau et l'identifiant qui change à l'enregistrement.
- `items` : un par identifiant, le niveau le plus précis l'emporte ; `overrides`
  liste les niveaux inférieurs qui le définissent aussi. Un mentor désactivé garde
  le nom du niveau qu'il désactive. `errors` : fichiers ignorés par le chargeur.
- `form` (dans `mentor_file`) : `{ id, name, initials, role, enabled, mode, model,
  priority, entry_level, max_remarks, on_open, triggers { paths, languages,
  headings, keywords } }`, absent pour un fichier invalide ou une surcharge
  minimale (`id` et `enabled: false` seuls). Le formulaire réécrit le frontmatter
  dans l'ordre des mentors livrés, sans ses commentaires ; `content` garde le
  fichier tel quel.
- Un mentor livré n'est jamais écrit (`invalid_settings`). Désactiver un mentor
  livré écrit une surcharge minimale dans les mentors personnels (ou du projet) ;
  désactiver sa propre copie y pose `enabled: false`. `.hodegos/mentors/` n'est
  créé qu'à l'enregistrement d'un mentor de projet.
- Après une écriture, les mentors sont rechargés aussitôt : `mentors_changed` et
  `casting` recalculé, comme pour une modification à la main.
- `mentor_prompt` : cadre commun puis posture, au palier d'entrée du mentor,
  `{language}` remplacé par son premier langage déclencheur (sinon `rust`).

Détails du terminal (phase 8f, ADR 0005) :

- Le shell (`$SHELL`, sinon `/bin/bash`) tourne dans bubblewrap avec votre UID,
  dans la racine du projet : système en lecture seule, `/dev`, `/proc` et `/tmp`
  neufs, `$HOME` vide sauf les montages, projet en lecture-écriture, racine remise
  en lecture seule ; `--unshare-pid`, `--die-with-parent`, réseau conservé.
- Environnement : le vôtre, moins `OPENROUTER_API_KEY` et `HODEGOS_*` ;
  `TERM=xterm-256color`, historique et caches dans le `/tmp` du terminal.
- Contre-pression : si la WebSocket ne suit pas, la lecture du pseudo-terminal est
  suspendue (rien ne s'accumule sans borne) ; les autres messages passent entre deux
  sorties. Une saisie que le terminal ne lit plus reçoit `terminal_limit`.
- Les terminaux d'une fenêtre meurent avec elle (WebSocket fermée) et avec Hodegos.
- Rien de ce qui passe dans un terminal n'est journalisé (seulement des tailles, au
  niveau `trace`) ni envoyé à un mentor.

## Codes d'erreur

| Code | Sens |
| --- | --- |
| `bad_message` | JSON invalide, type inconnu ou charge utile incorrecte ; nom invalide pour `create_entry` |
| `path_forbidden` | chemin absolu, avec `..`, hors de la racine (lien symbolique compris), sous `.git` ou exclu par `.gitignore` |
| `not_found` | fichier ou remarque introuvable |
| `already_exists` | `create_entry` : le nom est déjà pris (phase 8a) |
| `version_conflict` | `base_version` ne correspond plus à la version sur disque |
| `not_text` | fichier qui n'est pas un texte UTF-8 de 2 Mo au plus |
| `budget_exceeded` | budget mensuel atteint : demande explicite non confirmée |
| `provider_error` | fournisseur non configuré, erreur ou délai dépassé chez le fournisseur de modèles ; catalogue des modèles injoignable ; évaluation sans clé |
| `internal` | erreur inattendue (détail dans le journal du serveur) |
| `key_from_env` | `api_key_set` : `OPENROUTER_API_KEY` est définie et prime (phase 8d) |
| `model_refused` | `settings_set` : modèle inconnu, sans sorties structurées, ou effort non accepté |
| `eval_running` | `settings_set` : une évaluation tourne déjà |
| `terminal_unavailable` | `term_open` : pas de confinement possible (bwrap absent, espaces de noms refusés) ; le message dit pourquoi et comment y remédier |
| `terminal_limit` | `term_open` : 4 terminaux déjà ouverts ; `term_input` : saisie que le terminal ne lit plus |
| `mentor_invalid` | `mentor_save`, `mentor_prompt` : le chargeur refuse le mentor ; `errors` champ par champ |
| `invalid_settings` | `settings_set`, `api_key_set` : valeur hors limites, deux niveaux à évaluer à la fois, clé vide, ou réglages en lecture seule |

Conflit : si `base_version` ne correspond plus à la version sur disque, le serveur
refuse `save` avec `version_conflict` et la version sur disque dans `version` ; le
front propose de recharger (`open_file`) ou d'écraser (un nouvel envoi de `save`
avec cette version comme `base_version`).

```json
{ "v": 1, "type": "error", "re": "c-7",
  "payload": { "code": "version_conflict", "message": "…", "version": "3f2a9c0d1e4b5a67" } }
```
