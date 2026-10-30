# Évaluation des mentors — 2026-10-30 10:19

1 cas, coût total 0,0011 $. Un modèle est retenu s'il ne propose aucune solution (motifs `must_not`, brouillons écartés compris) et fait émerger la notion (`must_mention`) dans 70 % des cas au moins.

## Verdict par modèle

| Niveau · modèle | Cas | Notion émergée | Solution (brouillons) | Solution (affichée) | JSON invalides | Pannes | Casting différent | Coût | Durée médiane | Durée p90 | Verdict |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| fast · deepseek/deepseek-v4.1-flash-1017 | 1 | 1/1 (100 %) | 0 | 0 | 0 | 0 | 0 | 0,0011 $ | 2,4 s | 8,1 s | **retenu** |

## Cas

### 13-rs-unwrap-saisie — unwrap sur une saisie utilisateur : panique au lieu d'une erreur

`src/commande.rs` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0011 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | oui | non | 0,0002 $ | 8,1 s |
| Question | affiché | — | 1/1 | non | non | 0,0005 $ | 0,9 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0005 $ | 2,4 s |

**Indice** — affiché

Citation : `io::stdin().read_line(&mut ligne).unwrap();`

> Regardez la valeur que renvoie `read_line` : elle indique le nombre d'octets lus. Que vaut-elle quand l'entrée est terminée, et que contient `ligne` à ce moment-là ?

**Indice** — affiché

Citation : `ligne.trim().parse().unwrap()`

> Portez votre attention sur le `Result` produit par `parse` : il a deux variantes. Que se passe-t-il pour la variante autre que celle attendue ici ?

**Question** — affiché

> Quand l'entrée est déjà terminée, `ligne` reste vide. Déroulez la suite : que renvoie `parse()` sur une chaîne vide, puis que fait `unwrap()` sur ce résultat ? Où l'appel de `lire_quantite` se retrouve-t-il alors ?

**Piste** — affiché

> Notion en jeu : la propagation des erreurs. Votre signature promet un `u32`, alors que deux étapes peuvent échouer : la lecture et la conversion. `unwrap` transforme chacun de ces échecs en arrêt du programme. Pour choisir un autre contrat, voyez le chapitre 9 du livre Rust (« Gestion des erreurs »), la doc de `Result` et celle de `std::num::ParseIntError`. Vous y trouverez deux voies : renvoyer un `Result` et laisser l'appelant décider, ou traiter le cas sur place. À vous de voir laquelle traduit votre intention.

