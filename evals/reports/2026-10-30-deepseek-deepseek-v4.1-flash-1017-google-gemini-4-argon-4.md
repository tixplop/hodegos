# Évaluation des mentors — 2026-10-30 09:03

4 cas, coût total 0,0063 $. Un modèle est retenu s'il ne propose aucune solution (motifs `must_not`, brouillons écartés compris) et fait émerger la notion (`must_mention`) dans 70 % des cas au moins.

## Verdict par modèle

| Niveau · modèle | Cas | Notion émergée | Solution (brouillons) | Solution (affichée) | JSON invalides | Pannes | Casting différent | Coût | Durée médiane | Durée p90 | Verdict |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| fast · deepseek/deepseek-v4.1-flash-1017 | 4 | 4/4 (100 %) | 1 | 0 | 0 | 0 | 0 | 0,0063 $ | 3,7 s | 5,9 s | **non retenu** |

## Cas

### 13-rs-unwrap-saisie — unwrap sur une saisie utilisateur : panique au lieu d'une erreur

`src/commande.rs` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0017 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | non | non | 0,0005 $ | 3,1 s |
| Question | affiché | — | 1/1 | non | non | 0,0001 $ | 3,2 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0005 $ | 3,3 s |
| Exemple | rien | — | 1/1 | non | **oui** : `-> Result<u32` | 0,0006 $ | 7,1 s |

**Indice** — affiché

Citation : `io::stdin().read_line(&mut ligne).unwrap();`

> Attardez-vous sur la valeur renvoyée par read_line et sur ce qu'elle vaut quand l'entrée est vide ou interrompue. Demandez-vous ce que votre fonction renvoie dans ce cas.

**Indice** — affiché

Citation : `ligne.trim().parse().unwrap()`

> Regardez la signature de parse et ce qu'elle produit quand le texte saisi n'est pas un entier. Comparez le type attendu par votre fonction avec ce que cette ligne peut fournir.

**Question** — affiché

> Quand l'entrée est vide, que contient `ligne` après l'appel ? Testez le parse sur une chaîne vide dans un petit programme : que renvoie-t-il, et quel est le type de ce résultat ? En quoi cela change-t-il les cas que votre fonction doit distinguer ?

**Piste** — affiché

> La notion en jeu est la propagation d'erreur. Vos deux appels produisent un `Result` ; `unwrap` transforme un échec en arrêt du programme. Voyez le chapitre « Gestion des erreurs » du Rust Book (sections `Result`, opérateur `?`, propagation), la doc de `Result::unwrap` et celle de `str::parse` avec `std::num::ParseIntError`. Ces pages montrent comment un échec remonte vers l'appelant au lieu d'être consommé sur place.

**Exemple** — écarté : garde « pas de solution » : exemple qui reprend le code de la section

> Cas voisin, autre domaine : lire un âge depuis une ligne.
> 
> ```rust
> fn lire_age() -> Result<u32, String> {
>     let mut s = String::new();
>     io::stdin().read_line(&mut s)
>         .map_err(|e| e.to_string())?;
>     s.trim().parse::<u32>()
>         .map_err(|e| e.to_string())
> }
> ```
> 
> Ici les deux échecs deviennent un `Err` que l'appelant traite. Rien n'arrête le programme. Comparez avec le type de retour de votre fonction : le vôtre vaut `u32`.

### 14-rs-fenetre-glissante — Borne de boucle : la dernière fenêtre manque, et soustraction qui déborde

`src/stats.rs` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0021 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | oui | non | 0,0005 $ | 4,7 s |
| Question | affiché | — | 1/1 | oui | non | 0,0002 $ | 5,7 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0002 $ | 3,7 s |
| Exemple | rien | — | 1/1 | non | non | 0,0011 $ | 1,7 s |

**Indice** — affiché

Citation : `0..valeurs.len() - fenetre`

> Vérifiez la borne haute de cette plage : pour des valeurs de longueur 5 et fenetre 2, comptez le nombre de fenêtres qu'elle produit, puis celui que vous attendez.

**Indice** — affiché

Citation : `valeurs.len() - fenetre`

> Cette soustraction s'exécute avant tout contrôle. Que se passe-t-il si fenetre dépasse valeurs.len() ? Et si fenetre vaut 0 ?

**Question** — affiché

> Testez deux appels limites : fenetre = 0, puis fenetre plus grande que valeurs.len(). Que vaut alors l'expression de la borne, sachant que les longueurs sont des usize ? Le résultat correspond-il à ce qu'un appelant peut attendre de votre signature ?

**Piste** — affiché

> Deux notions se croisent ici : le découpage en fenêtres, que la bibliothèque standard expose comme un itérateur dédié (voir la documentation de `slice::windows`, section « Examples », et la différence avec `chunks`) ; et la validité des arguments, que `usize` ne protège pas d'une soustraction qui passe sous zéro. Pour ce second point, lisez la section du livre Rust sur les dépassements d'entiers en mode debug, et repérez la méthode `checked_sub` sur les entiers. La question à trancher avant d'écrire : votre signature accepte-t-elle ces appels, ou les refuse-t-elle ?

**Exemple** — écarté : garde « pas de solution » : exemple qui reprend le code de la section

> Un cas voisin, sur des octets : un garde en tête rend la précondition explicite, et un itérateur dédié supprime l'arithmétique d'indices.
> 
> ```rust
> fn blocs(d: &[u8], t: usize) -> Vec<&[u8]> {
>     if t == 0 || t > d.len() {
>         return Vec::new();
>     }
>     d.chunks(t).collect()
> }
> 
> fn maxima_glissants(v: &[u8], f: usize) -> Vec<u8> {
>     v.windows(f)
>         .map(|w| *w.iter().max().unwrap())
>         .collect()
> }
> ```
> 
> Ici la taille des tranches est validée avant tout calcul, et le nombre de fenêtres vient de l'itérateur, non d'une soustraction.

### 15-rs-clone-pour-l-emprunteur — clone partout pour contourner l'emprunteur, et liste vide qui panique

`src/inventaire.rs` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0015 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0005 $ | 5,9 s |
| Question | affiché | — | 1/1 | non | non | 0,0001 $ | 3,8 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0005 $ | 4,6 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0004 $ | 5,3 s |

**Indice** — affiché

Citation : `let mut meilleur = self.articles[0].clone();`

> Que renvoie cette fonction si `self.articles` est vide ? L'initialisation par indice suppose au moins un élément. Vérifiez ce que la signature promet à l'appelant dans ce cas.

**Question** — affiché

> Une fois le cas vide identifié, que devrait renvoyer la fonction dans cette situation ? Votre signature actuelle, `-> String`, permet-elle d'exprimer ce choix à l'appelant, ou l'oblige-t-elle à deviner ?

**Piste** — affiché

> Deux notions à consulter. L'absence de résultat s'exprime par `Option` : voyez la doc de `std::option::Option`, et les adaptateurs `Iterator::max_by` / `max_by_key`, qui renvoient déjà un `Option` — cela relève de la signature, pas de l'algorithme. Second point : l'itération par emprunt ; le chapitre « Iterators » du Rust Book montre comment parcourir une collection sans la dupliquer. Liez les deux : le parcours fournit des références, `Option` porte le cas sans élément.

**Exemple** — affiché

> Cas voisin, sur une autre collection :
> 
> ```rust
> struct Panier { prix: Vec<u32> }
> 
> impl Panier {
>     fn plus_cher(&self) -> Option<u32> {
>         self.prix.iter().copied().max()
>     }
> }
> ```
> 
> Ce que ce voisinage ajoute aux paliers précédents : la signature annonce l'absence (`Option`), donc l'appelant ne peut pas ignorer le cas vide ; `iter()` parcourt par emprunt, sans dupliquer ; l'adaptateur se charge du parcours. Transposez à des `String` et à une comparaison de longueur : la forme reste la même, seul le critère change.

### 16-rs-iterateur-paresseux — map sans consommation : l'itérateur paresseux ne s'exécute jamais

`src/journal.rs` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0011 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0005 $ | 3,0 s |
| Question | affiché | — | 1/1 | non | non | 0,0001 $ | 2,7 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0002 $ | 3,3 s |
| Exemple | rien | — | 1/1 | non | non | 0,0003 $ | 4,5 s |

**Indice** — affiché

Citation : `lignes.iter().map(|ligne| archive.push(ligne.to_uppercase()));`

> Portez votre attention sur la ligne `lignes.iter().map(...)` : demandez-vous à quel moment la fermeture passée à `map` est réellement exécutée, et ce qu'il advient de l'itérateur produit.

**Question** — affiché

> Si vous appelez `archiver` avec trois lignes puis affichez `archive.len()` juste après, quel nombre attendez-vous, et quel nombre obtenez-vous ? Ce test vous dira si le corps de la fonction a été exécuté.

**Piste** — affiché

> La notion en jeu est la distinction entre adaptateur et consommateur d'itérateur : les adaptateurs comme `map` sont paresseux et ne font rien tant qu'aucun consommateur ne parcourt l'itérateur.
> 
> Pour vous documenter : la page `std::iter::Iterator` de la documentation standard, qui sépare explicitement « Adapters » et « Consumers » ; le chapitre sur les itérateurs du Rust Book traite aussi cette paresse.
> 
> Demandez-vous ensuite lequel des deux rôles votre fonction doit confier à `map`, et lequel est nécessaire pour que les lignes soient réellement ajoutées.

**Exemple** — écarté : garde « pas de solution » : exemple qui reprend le code de la section

> Un cas voisin, sans rapport avec votre code :
> 
> ```rust
> let mots = vec![String::from("a"), String::from("b")];
> // adaptateur seul : rien ne se passe
> mots.iter().map(|m| m.len());
> 
> // consommateur : le travail est fait
> let tailles: Vec<usize> = mots.iter().map(|m| m.len()).collect();
> assert_eq!(tailles.len(), 2);
> 
> // consommateur à effet de bord
> let mut v = Vec::new();
> mots.iter().for_each(|m| v.push(m.clone()));
> assert_eq!(v.len(), 2);
> ```
> 
> Comparez les trois blocs : lequel produit un effet observable ? Lequel ne fait rien ? Transportez cette distinction sur votre fonction.

