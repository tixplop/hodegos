# Évaluation des mentors — 2026-10-30 06:09

20 cas, coût total 0,0390 $. Un modèle est retenu s'il ne propose aucune solution (motifs `must_not`, brouillons écartés compris) et fait émerger la notion (`must_mention`) dans 70 % des cas au moins.

## Verdict par modèle

| Niveau · modèle | Cas | Notion émergée | Solution (brouillons) | Solution (affichée) | JSON invalides | Pannes | Casting différent | Coût | Durée médiane | Durée p90 | Verdict |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| fast · openai/gpt-6-luna | 15 | 14/15 (93 %) | 1 | 1 | 0 | 0 | 0 | 0,0124 $ | 3,1 s | 5,1 s | **non retenu** |
| strong · openai/gpt-6.1-terra | 5 | 4/5 (80 %) | 0 | 0 | 0 | 0 | 0 | 0,0265 $ | 3,4 s | 5,1 s | **retenu** |

## Cas

### 01-dates-par-reference — Comparaison de dates par référence et chevauchement non détecté

`src/emprunts/reserver.ts` · titulaire `mentor-langage` · modèle `openai/gpt-6-luna` · notion émergée · coût 0,0010 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | non | non | 0,0003 $ | 6,0 s |
| Question | affiché | — | 1/1 | non | non | 0,0002 $ | 3,4 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0002 $ | 2,9 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0003 $ | 4,5 s |

**Indice** — affiché

Citation : `e.debut === debut`

> Regardez ce que `===` compare lorsque ses opérandes sont des objets `Date`, et comment comparer les dates elles-mêmes.

**Indice** — affiché

Citation : `e.objetId === objet.id && e.debut === debut`

> Examinez les réservations dont le début diffère, mais dont la période recoupe celle demandée.

**Question** — affiché

> Pouvez-vous vérifier avec deux objets `Date` distincts représentant le même instant si `===` les considère égaux, puis déterminer comment comparer leurs instants ?

**Piste** — affiché

> La notion en jeu est la comparaison de valeurs temporelles plutôt que de références d’objets. Consultez la documentation de `Date.prototype.getTime()` pour comparer les instants représentés.

**Exemple** — affiché

> Dans un autre contexte, deux dates distinctes peuvent représenter le même instant :
> ```ts
> const a = new Date("2025-01-01T00:00:00Z");
> const b = new Date("2025-01-01T00:00:00Z");
> a === b; // false
>  a.getTime() === b.getTime(); // true
> ```

### 02-adr-sans-alternatives — ADR incohérent : hors ligne exigé, base hébergée choisie, aucune alternative

`docs/architecture/0001-stockage.md` · titulaire `architecte` · modèle `openai/gpt-6.1-terra` · notion émergée · coût 0,0050 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | oui | non | 0,0024 $ | 4,6 s |
| Question | affiché | — | 1/1 | non | non | 0,0011 $ | 1,7 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0015 $ | 3,1 s |

**Indice** — affiché

Citation : `Nous stockons toutes les données dans une base PostgreSQL hébergée, interrogée
par une API REST.`

> Relisez cette décision en regard de l’exigence de disponibilité hors ligne du contexte.

**Indice** — affiché

Citation : `une base PostgreSQL hébergée`

> Relisez ce choix en regard des alternatives envisagées et des critères qui permettraient de le réévaluer.

**Question** — affiché

> Comment les voisins pourront-ils consulter les données lorsque leur téléphone n’a pas de connexion à l’API ?

**Piste** — affiché

> La notion en jeu est la conception « offline-first » : elle examine l’accès aux données sans réseau et leur synchronisation au retour de la connexion. Consultez la documentation sur les applications web hors ligne et les stratégies de synchronisation.

### 03-ts-foreach-async — forEach avec une fonction async : le compte est renvoyé avant les enregistrements

`src/import/importer.ts` · titulaire `mentor-langage` · modèle `openai/gpt-6-luna` · notion émergée · coût 0,0008 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0002 $ | 2,6 s |
| Question | affiché | — | 1/1 | non | non | 0,0001 $ | 2,1 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0002 $ | 3,4 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0003 $ | 4,3 s |

**Indice** — affiché

Citation : `fiches.forEach(async (fiche) => {`

> Regardez le lien entre le callback asynchrone de `forEach` et le moment où `importerTout` renvoie sa valeur.

**Question** — affiché

> Quand `importerTout` renvoie sa valeur, les appels à `enregistrer` ont-ils tous terminé ? Quelle valeur de `enregistrees` peut-elle alors contenir ?

**Piste** — affiché

> La notion en jeu est l’attente des promesses dans une itération asynchrone. Consultez la documentation TypeScript ou MDN sur `for...of` avec `await` et `Promise.all` : ces approches correspondent à des exécutions séquentielles ou concurrentes.

**Exemple** — affiché

> Exemple voisin : pour deux lectures indépendantes, `Promise.all` attend les deux résultats avant de poursuivre.
> ```ts
> async function chargerEcran() {
>   const [profil, panier] = await Promise.all([
>     lireProfil(),
>     lirePanier(),
>   ]);
>   return { profil, panier };
> }
> ```

### 04-ts-find-undefined — find peut renvoyer undefined, masqué par une assertion non nulle

`src/panier/total.ts` · titulaire `mentor-langage` · modèle `openai/gpt-6-luna` · notion émergée · coût 0,0008 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0002 $ | 3,7 s |
| Question | affiché | — | 1/1 | non | non | 0,0001 $ | 2,9 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0002 $ | 3,4 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0003 $ | 4,1 s |

**Indice** — affiché

Citation : `return produit!.prix;`

> Examinez le cas où aucune référence ne correspond : que vaut le résultat de `find` et que suppose l’accès juste après ?

**Question** — affiché

> Quel résultat votre fonction doit-elle garantir si la référence est inconnue, et comment ce contrat peut-il être exprimé dans son type de retour ?

**Piste** — affiché

> La notion en jeu est l’assertion de non-nullité (`!`) : elle modifie l’analyse de TypeScript, mais n’ajoute aucune vérification à l’exécution. Consultez la documentation TypeScript sur les assertions de type et `strictNullChecks` pour distinguer garantie statique et comportement réel.

**Exemple** — affiché

> Dans un cas voisin, une recherche peut rendre explicite l’absence :
> ```ts
> function scoreDe(scores: Map<string, number>, nom: string): number | undefined {
>   return scores.get(nom);
> }
> ```
> L’appelant doit alors tenir compte des deux résultats possibles.

### 05-ts-tri-sur-place — sort trie sur place le tableau reçu en paramètre

`src/agenda/tri.ts` · titulaire `mentor-langage` · modèle `openai/gpt-6-luna` · notion émergée · coût 0,0008 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | non | non | 0,0002 $ | 4,7 s |
| Question | affiché | — | 1/1 | non | non | 0,0001 $ | 1,7 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0002 $ | 2,6 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0002 $ | 3,2 s |

**Indice** — affiché

Citation : `.sort((a, b) => a.debut.getTime() - b.debut.getTime())`

> Regardez si le tri peut modifier le tableau reçu : vérifiez le comportement de `sort` sur son tableau d’origine.

**Indice** — affiché

Citation : `.slice(0, n)`

> Vérifiez aussi le résultat attendu lorsque `n` est négatif : le comportement de `slice` mérite attention dans ce cas.

**Question** — affiché

> Les appelants de cette fonction peuvent-ils réutiliser `creneaux` en supposant qu’il conserve son ordre initial après l’appel ?

**Piste** — affiché

> La distinction entre méthodes mutantes et non mutantes des tableaux est en jeu. Consultez la documentation de `Array.prototype.sort` et `Array.prototype.toSorted` sur MDN, en vérifiant la compatibilité de votre environnement.

**Exemple** — affiché

> Exemple voisin : avec `[3, 1, 2]`, `sort` réordonne le tableau lui-même. `toSorted` renvoie un tableau trié distinct et laisse l’original dans son ordre initial.

### 06-js-var-boucle — var dans une boucle : toutes les fonctions voient la même variable

`src/ui/boutons.js` · titulaire `mentor-langage` · modèle `openai/gpt-6-luna` · notion émergée · coût 0,0008 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0002 $ | 2,6 s |
| Question | affiché | — | 1/1 | non | non | 0,0001 $ | 2,7 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0002 $ | 3,0 s |
| Exemple | affiché | — | 1/1 | non | non | 0,0003 $ | 6,0 s |

**Indice** — affiché

Citation : `afficher("Bouton " + (i + 1));`

> Regardez quand la fonction du clic lit `i` : comparez sa valeur à ce moment-là avec celle qu’elle avait lors de l’enregistrement du gestionnaire.

**Question** — affiché

> Quand un clic survient après la fin de la boucle, quelle valeur `i` contient-elle, et que vont donc afficher les différents boutons ?

**Piste** — affiché

> La notion en jeu est la fermeture (closure) : chaque gestionnaire conserve une référence à la variable `i`, et `var` partage cette variable dans toute la fonction. Consultez la documentation MDN sur les fermetures et la portée de `var` et `let`.

**Exemple** — affiché

> Dans un cas voisin, des fonctions mémorisent le numéro de chaque jour :
> ```js
> const callbacks = [];
> for (let n = 0; n < 3; n++) {
>   callbacks.push(() => console.log("Jour", n));
> }
> callbacks.forEach(callback => callback());
> ```
> Ici, chaque fonction conserve sa propre valeur de `n`.

### 07-js-tri-numerique — sort sans comparateur trie des nombres comme du texte

`src/stats/mediane.js` · titulaire `mentor-langage` · modèle `openai/gpt-6-luna` · notion émergée · coût 0,0009 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | oui | non | 0,0003 $ | 4,5 s |
| Question | affiché | — | 1/1 | oui | non | 0,0002 $ | 3,0 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0002 $ | 3,5 s |
| Exemple | affiché | — | 1/1 | non | non | 0,0002 $ | 3,8 s |

**Indice** — affiché

Citation : `.sort();`

> Regardez l’ordre produit par `sort()` sans comparateur lorsque les valeurs sont numériques.

**Indice** — affiché

Citation : `if (triees.length % 2 === 0) {`

> Examinez les indices utilisés dans cette branche lorsque `valeurs` ne contient aucun élément.

**Question** — affiché

> Quel ordre `sort()` sans comparateur applique-t-il aux nombres, et la médiane reste-t-elle correcte lorsque certaines valeurs ont plusieurs chiffres ?

**Piste** — affiché

> La notion en jeu est le comparateur par défaut de `Array.prototype.sort` : son fonctionnement avec des nombres est documenté sur MDN, dans la référence de `Array.prototype.sort`.

**Exemple** — affiché

> Exemple voisin :
> ```js
> const nombres = [2, 10, 1];
> console.log([...nombres].sort()); // [1, 10, 2]
> ```
> L’ordre obtenu n’est pas l’ordre numérique.

### 08-js-prix-flottants — Montants en euros flottants comparés avec ===

`src/caisse/reste.js` · titulaire `mentor-langage` · modèle `openai/gpt-6-luna` · notion émergée · coût 0,0008 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0002 $ | 3,5 s |
| Question | affiché | — | 1/1 | non | non | 0,0001 $ | 2,4 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0002 $ | 2,8 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0003 $ | 5,7 s |

**Indice** — affiché

Citation : `return total - dejaVerse === 0 ? "payé" : "à payer";`

> Regardez la comparaison exacte du solde après les multiplications par des prix en euros décimaux.

**Question** — affiché

> Quels montants décimaux pourriez-vous tester pour vérifier si le solde vaut réellement zéro après les multiplications et les additions ?

**Piste** — affiché

> La notion en jeu est la précision des nombres à virgule flottante (IEEE 754) : certains montants décimaux ne sont pas représentés exactement en JavaScript. Consultez la documentation MDN sur le type `Number` et `Number.EPSILON`, ainsi que les pratiques de représentation des montants monétaires en unités entières.

**Exemple** — affiché

> Exemple voisin : représenter des prix en centimes entiers évite les fractions binaires.
> ```js
> const prixEnCentimes = [199, 250];
> const total = prixEnCentimes.reduce((somme, prix) => somme + prix, 0);
> console.log(total === 449); // true
> ```
> À l’inverse, l’addition de nombres comme 0.1 et 0.2 peut produire un résultat qui n’est pas exactement 0.3.

### 09-py-defaut-mutable — Liste mutable comme valeur par défaut d'un paramètre

`inventaire/panier.py` · titulaire `mentor-langage` · modèle `openai/gpt-6-luna` · notion émergée · coût 0,0007 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | oui | non | 0,0002 $ | 3,2 s |
| Question | affiché | — | 1/1 | non | non | 0,0001 $ | 2,3 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0002 $ | 3,1 s |
| Exemple | affiché | — | 1/1 | non | non | 0,0002 $ | 3,7 s |

**Indice** — affiché

Citation : `def ajouter(article, panier=[]):`

> Regardez la valeur par défaut dans la signature : à quel moment Python l’évalue-t-il, et que se passe-t-il si plusieurs appels omettent `panier` ?

**Question** — affiché

> Quel comportement attendez-vous si deux appels sans `panier` doivent créer des paniers indépendants ?

**Piste** — affiché

> La notion en jeu est celle des valeurs par défaut mutables : Python évalue les valeurs par défaut à la définition de la fonction et conserve l’objet entre les appels. Consultez la documentation Python sur les valeurs par défaut des arguments et les objets mutables.

**Exemple** — affiché

> Cas voisin :
> ```python
> def ajouter_tag(tag, tags=None):
>     if tags is None:
>         tags = []
>     tags.append(tag)
>     return tags
> ```
> Chaque appel sans `tags` obtient ainsi une nouvelle liste.

### 10-py-liaison-tardive — Fermetures créées dans une boucle : liaison tardive de la variable

`rapports/filtres.py` · titulaire `mentor-langage` · modèle `openai/gpt-6-luna` · notion émergée · coût 0,0009 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0002 $ | 3,2 s |
| Question | affiché | — | 1/1 | non | non | 0,0002 $ | 2,2 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0002 $ | 2,9 s |
| Exemple | affiché | — | 1/1 | non | non | 0,0004 $ | 7,7 s |

**Indice** — affiché

Citation : `filtres.append(lambda note: note >= limite)`

> Regardez ce que chaque fonction conserve de la variable `limite` une fois la boucle terminée.

**Question** — affiché

> Après la boucle, que compare chacun des filtres si vous les appelez avec la même note : la limite de son tour ou la dernière limite parcourue ?

**Piste** — affiché

> La notion en jeu est la fermeture lexicale : ces fonctions conservent une référence à la variable `limite`, plutôt que sa valeur à chaque tour. Consultez la FAQ Python sur les fonctions lambda dans une boucle et la liaison tardive.

**Exemple** — affiché

> Exemple voisin : conserver la couleur propre à chaque fonction.
> ```python
> fonctions = []
> for couleur in ("bleu", "vert"):
>     fonctions.append(lambda c=couleur: c)
> print([f() for f in fonctions])  # ['bleu', 'vert']
> ```
> Ici, l’argument par défaut mémorise la valeur au moment de la création de chaque fonction.

### 11-py-liste-modifiee-en-parcours — Liste modifiée pendant qu'on la parcourt : des éléments sont sautés

`taches/nettoyage.py` · titulaire `mentor-langage` · modèle `openai/gpt-6-luna` · notion émergée · coût 0,0008 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0002 $ | 3,1 s |
| Question | affiché | — | 1/1 | non | non | 0,0001 $ | 3,1 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0002 $ | 2,5 s |
| Exemple | affiché | — | 1/1 | non | non | 0,0003 $ | 4,4 s |

**Indice** — affiché

Citation : `for tache in taches:
        if tache["terminee"]:
            taches.remove(tache)`

> Examinez le parcours lorsque deux tâches terminées se suivent : que voit la boucle après le retrait de la première ?

**Question** — affiché

> Comment pourriez-vous parcourir les tâches sans que leur retrait modifie les éléments que la boucle doit encore examiner ?

**Piste** — affiché

> La notion en jeu est la modification d’une liste pendant son itération. Consultez la documentation Python sur les itérateurs de listes et les méthodes de filtrage des séquences.

**Exemple** — affiché

> Dans un cas voisin, pour garder les nombres positifs sans modifier la liste pendant son parcours :
> ```python
> nombres = [3, -1, 2]
> positifs = [n for n in nombres if n > 0]
> ```

### 12-py-is-au-lieu-de-egal — is compare l'identité des objets, pas leur valeur

`comptes/verification.py` · titulaire `mentor-langage` · modèle `openai/gpt-6-luna` · notion émergée · coût 0,0007 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | oui | non | 0,0002 $ | 2,6 s |
| Question | affiché | — | 1/1 | non | non | 0,0001 $ | 1,3 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0001 $ | 2,3 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0003 $ | 5,1 s |

**Indice** — affiché

Citation : `saisi.strip() is attendu`

> Regardez la comparaison finale : vérifiez si elle porte sur le contenu des chaînes ou sur leur identité en mémoire.

**Question** — affiché

> Comment pouvez-vous vérifier par vous-même si deux chaînes distinctes mais contenant les mêmes caractères produisent le résultat attendu ici ?

**Piste** — affiché

> La notion en jeu est la différence entre égalité de valeur et identité d’objet en Python. Consultez la documentation sur les comparaisons, notamment les opérateurs `==` et `is`.

**Exemple** — affiché

> Exemple voisin avec des listes : `a = [1, 2]` et `b = [1, 2]`. `a == b` vaut `True` car leur contenu est égal ; `a is b` vaut `False` car ce sont deux objets distincts.

### 13-rs-unwrap-saisie — unwrap sur une saisie utilisateur : panique au lieu d'une erreur

`src/commande.rs` · titulaire `mentor-langage` · modèle `openai/gpt-6-luna` · notion émergée · coût 0,0009 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0002 $ | 3,9 s |
| Question | affiché | — | 1/1 | oui | non | 0,0002 $ | 2,7 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0002 $ | 3,8 s |
| Exemple | affiché | — | 1/1 | oui | **oui** : `.parse()?` | 0,0003 $ | 7,6 s |

**Indice** — affiché

Citation : `io::stdin().read_line(&mut ligne).unwrap();
    ligne.trim().parse().unwrap()`

> Examinez le parcours de lecture et de conversion lorsque l’entrée est vide ou ne représente pas un entier de type `u32`.

**Question** — affiché

> Quel comportement souhaitez-vous garantir à l’appelant si la lecture ou la conversion échoue, plutôt que de faire terminer le programme par une panique ?

**Piste** — affiché

> La notion à explorer est la propagation des erreurs avec `Result` et l’opérateur `?`. La lecture et l’analyse peuvent produire des types d’erreur différents ; consultez le chapitre « Gestion des erreurs » du Livre Rust et la documentation de `str::parse`.

**Exemple** — affiché

> Dans un cas voisin, une fonction qui lit un fichier puis convertit son contenu peut propager deux types d’erreurs avec `?` :
> ```rust
> fn lire_age(fichier: &str) -> Result<u8, Box<dyn std::error::Error>> {
>     let contenu = std::fs::read_to_string(fichier)?;
>     let age = contenu.trim().parse()?;
>     Ok(age)
> }
> ```
> Le type d’erreur englobant permet ici de couvrir la lecture et la conversion.

### 14-rs-fenetre-glissante — Borne de boucle : la dernière fenêtre manque, et soustraction qui déborde

`src/stats.rs` · titulaire `mentor-langage` · modèle `openai/gpt-6-luna` · notion émergée · coût 0,0009 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | oui | non | 0,0002 $ | 2,4 s |
| Question | affiché | — | 1/1 | non | non | 0,0002 $ | 2,7 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0002 $ | 4,0 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0003 $ | 6,4 s |

**Indice** — affiché

Citation : `for debut in 0..valeurs.len() - fenetre {`

> Vérifiez les bornes de cette boucle pour le dernier départ valide, puis pour les cas où la fenêtre dépasse la longueur des valeurs.

**Question** — affiché

> Que devrait retourner la fonction lorsque `fenetre` vaut zéro, et quel résultat produit alors la division par `fenetre` ?

**Piste** — affiché

> La notion en jeu est la borne exclusive des plages `Range` en Rust, ainsi que la soustraction non vérifiée de valeurs `usize`. Consultez la documentation de `std::ops::Range` et des tranches, notamment leurs règles de bornes et d’indexation.

**Exemple** — affiché

> Dans un cas indépendant, `0..3` parcourt les valeurs `0`, `1` et `2` : la borne finale est exclue. À vous de déterminer combien de départs sont valides pour une séquence de longueur 5 et une fenêtre de longueur 2.

### 15-rs-clone-pour-l-emprunteur — clone partout pour contourner l'emprunteur, et liste vide qui panique

`src/inventaire.rs` · titulaire `mentor-langage` · modèle `openai/gpt-6-luna` · notion **non émergée** · coût 0,0008 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0002 $ | 3,1 s |
| Question | affiché | — | 1/1 | non | non | 0,0001 $ | 2,4 s |
| Piste | affiché | — | 1/1 | non | non | 0,0002 $ | 3,1 s |
| Exemple | affiché | — | 1/1 | non | non | 0,0003 $ | 4,6 s |

**Indice** — affiché

Citation : `let mut meilleur = self.articles[0].clone();`

> Regardez la précondition de cet accès et vérifiez si tous les inventaires possibles la respectent.

**Question** — affiché

> Quel résultat souhaitez-vous obtenir lorsque `articles` est vide, et comment votre fonction pourrait-elle le représenter ?

**Piste** — affiché

> La notion à explorer est la représentation d’un résultat éventuellement absent avec `Option`, ainsi que l’accès conditionnel aux éléments d’un `Vec`. Consultez la documentation standard de `Option` et de `Vec::first`.

**Exemple** — affiché

> Dans un cas voisin, une fonction qui cherche le premier nombre d’une tranche peut représenter l’absence ainsi :
> ```rust
> fn premier_nombre(nombres: &[i32]) -> Option<i32> {
>     nombres.first().copied()
> }
> ```
> Elle renvoie `None` si la tranche est vide, et `Some(nombre)` sinon.

### 16-rs-iterateur-paresseux — map sans consommation : l'itérateur paresseux ne s'exécute jamais

`src/journal.rs` · titulaire `mentor-langage` · modèle `openai/gpt-6-luna` · notion émergée · coût 0,0008 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0002 $ | 3,0 s |
| Question | affiché | — | 1/1 | oui | non | 0,0002 $ | 2,7 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0002 $ | 3,6 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0002 $ | 4,5 s |

**Indice** — affiché

Citation : `lignes.iter().map(|ligne| archive.push(ligne.to_uppercase()));`

> Regardez si l’itérateur créé ici est parcouru avant la fin de la fonction, et à quel moment l’effet de bord de la closure peut se produire.

**Question** — affiché

> Quelle différence faites-vous entre une opération qui transforme un itérateur et une opération qui le consomme ?

**Piste** — affiché

> La notion en jeu est la paresse des adaptateurs d’itérateurs : `map` ne déclenche pas sa closure tant qu’un consommateur ne parcourt pas l’itérateur. Consultez la documentation de `Iterator::map` et des méthodes consommatrices.

**Exemple** — affiché

> Dans un cas voisin, `collect` parcourt l’itérateur et déclenche la transformation :
> ```rust
> let nombres = [1, 2, 3];
> let doubles: Vec<_> = nombres.iter().map(|n| n * 2).collect();
> ```

### 17-adr-consequences-positives — ADR dont les conséquences ne listent aucun inconvénient

`docs/architecture/0002-connexion.md` · titulaire `architecte` · modèle `openai/gpt-6.1-terra` · notion **non émergée** · coût 0,0062 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | non | non | 0,0025 $ | 5,7 s |
| Question | affiché | — | 1/1 | non | non | 0,0018 $ | 3,9 s |
| Piste | affiché | — | 1/1 | non | non | 0,0018 $ | 3,6 s |

**Indice** — affiché

Citation : `- Plus de mots de passe à gérer.`

> Regardez aussi les conséquences du lien par e-mail pour les membres, notamment autour de son accès et de sa durée de validité.

**Indice** — affiché

Citation : `- Le gardien n'a plus de réinitialisations à faire.`

> Regardez les situations qui pourraient conduire à réexaminer la décision, notamment du point de vue du gardien.

**Question** — affiché

> Que se passe-t-il pour un membre qui ne peut pas accéder à son e-mail ou dont le lien a expiré ? Quelles conséquences cela entraîne-t-il pour lui et pour le gardien ?

**Piste** — affiché

> La notion en jeu est la dépendance à un service externe. Consultez la documentation sur l’analyse des modes de défaillance et sur les procédures d’assistance à la connexion.

### 18-adr-decision-disproportionnee — Décision disproportionnée au regard des contraintes du contexte

`docs/architecture/0003-notifications.md` · titulaire `architecte` · modèle `openai/gpt-6.1-terra` · notion émergée · coût 0,0047 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | oui | non | 0,0019 $ | 3,7 s |
| Question | affiché | — | 1/1 | oui | non | 0,0014 $ | 2,9 s |
| Piste | affiché | — | 1/1 | non | non | 0,0014 $ | 3,3 s |

**Indice** — affiché

Citation : `Nous déployons un cluster Kafka de trois nœuds`

> Rapprochez ce choix des contraintes d’hébergement décrites dans le contexte.

**Indice** — affiché

Citation : `avec un service consommateur par type de message.`

> Regardez les alternatives envisagées et les conséquences de cette organisation dans l’ADR.

**Question** — affiché

> Comment les trois nœuds seront-ils hébergés, et leurs besoins en ressources sont-ils compatibles avec le Raspberry Pi et l’absence de budget ?

**Piste** — affiché

> La notion en jeu est le dimensionnement des ressources et des coûts d’exploitation. Consultez la documentation de déploiement de Kafka pour estimer les besoins des nœuds, puis confrontez-les aux ressources disponibles.

### 19-adr-nous-verrons — Cache sans stratégie d'invalidation, renvoyée à plus tard

`docs/architecture/0004-cache.md` · titulaire `architecte` · modèle `openai/gpt-6.1-terra` · notion émergée · coût 0,0051 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0019 $ | 3,8 s |
| Question | affiché | — | 1/1 | non | non | 0,0015 $ | 3,4 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0018 $ | 3,4 s |

**Indice** — affiché

Citation : `Nous verrons plus tard
s'il faut l'invalider.`

> Rapprochez ce passage de la fréquence des ajouts et retraits d’objets décrite dans le contexte.

**Question** — affiché

> Que verra un utilisateur si un objet est ajouté ou retiré après sa mise en cache ? Pendant combien de temps ce décalage serait-il acceptable ?

**Piste** — affiché

> Explorez la cohérence du cache et la fraîcheur des données. Consultez la documentation Redis sur l’expiration des clés et les stratégies d’invalidation, ainsi que leurs effets sur la durée pendant laquelle une donnée peut rester périmée.

### 20-conception-sans-contraintes — Document de conception qui choisit une solution sans dire les contraintes

`docs/design/recherche.md` · titulaire `architecte` · modèle `openai/gpt-6.1-terra` · notion émergée · coût 0,0055 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | oui | non | 0,0026 $ | 5,1 s |
| Question | affiché | — | 1/1 | non | non | 0,0014 $ | 2,6 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0016 $ | 2,9 s |

**Indice** — affiché

Citation : `Un cluster Elasticsearch indexe le catalogue en temps réel.`

> Regardez le lien entre ce choix, l’objectif de recherche par nom et les autres options envisagées.

**Indice** — affiché

Citation : `Chaque ajout
d'objet déclenche une réindexation`

> Regardez ce qui se passe lorsque la taille du catalogue et la fréquence des ajouts évoluent.

**Question** — affiché

> Quelles autres approches avez-vous envisagées pour la recherche par nom, et sur quels critères les avez-vous écartées ?

**Piste** — affiché

> Explorez la notion de compromis architectural : coût d’exploitation, pertinence des résultats et volumes attendus. Consultez les guides de rédaction d’ADR sur la comparaison des alternatives.

