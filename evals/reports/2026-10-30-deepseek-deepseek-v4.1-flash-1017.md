# Évaluation des mentors — 2026-10-30 05:47

20 cas, coût total 0,0208 $. Un modèle est retenu s'il ne propose aucune solution (motifs `must_not`, brouillons écartés compris) et fait émerger la notion (`must_mention`) dans 70 % des cas au moins.

## Verdict par modèle

| Niveau · modèle | Cas | Notion émergée | Solution (brouillons) | Solution (affichée) | JSON invalides | Pannes | Casting différent | Coût | Durée médiane | Durée p90 | Verdict |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| fast · deepseek/deepseek-v4.1-flash-1017 | 15 | 12/15 (80 %) | 1 | 1 | 0 | 0 | 0 | 0,0159 $ | 2,9 s | 6,3 s | **non retenu** |
| strong · deepseek/deepseek-v4.1-flash-1017 | 5 | 3/5 (60 %) | 0 | 0 | 0 | 0 | 0 | 0,0048 $ | 2,7 s | 7,8 s | **non retenu** |

## Cas

### 01-dates-par-reference — Comparaison de dates par référence et chevauchement non détecté

`src/emprunts/reserver.ts` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0022 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | non | non | 0,0004 $ | 2,9 s |
| Question | affiché | — | 1/1 | non | non | 0,0006 $ | 4,1 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0004 $ | 7,7 s |
| Exemple | rien | — | 1/1 | non | non | 0,0009 $ | 6,3 s |

**Indice** — affiché

Citation : `e.objetId === objet.id && e.debut === debut`

> Portez votre attention sur la comparaison `e.debut === debut` : la bibliothèque standard documente comment deux Date se comparent. Vérifiez ce que `===` évalue réellement ici, et ce que cela implique quand deux appelants passent la même date.

**Indice** — affiché

Citation : `e.objetId === objet.id && e.debut === debut`

> Regardez ce que couvre la recherche : la condition ne mentionne que `debut`. Demandez-vous ce qui se passe si un emprunt existant commence avant la période demandée et se termine après elle, ou l'inverse.

**Question** — affiché

> Deux objets `Date` créés séparément pour le même instant : d'après vous, que renvoie `===` entre eux ? Pouvez-vous écrire un essai de quelques lignes qui vous donne la réponse, puis relire votre `find` en vous demandant ce qu'il doit vraiment comparer pour décider qu'un objet est déjà pris ?

**Piste** — affiché

> Votre `find` ne regarde que l'égalité des débuts. La notion à documenter ici est le recouvrement d'intervalles : deux réservations peuvent se chevaucher sans partager la même date de début, donc la disponibilité se décide sur l'ordre des bornes, pas sur l'identité d'un instant. Voyez la comparaison d'intervalles semi-ouverts, et reliez-la au TODO voisin sur l'ordre entre début et fin.

**Exemple** — écarté : message vide ou trop long

> Un cas voisin, sur des nombres plutôt que des dates :
> 
> ```ts
> const a = { debut: 9, fin: 12 };
> const b = { debut: 9, fin: 12 };
> 
> a === b;              // false : deux références distinctes
> a.debut === b.debut;  // true  : comparaison de valeurs
> 
> // [9, 12) et [11, 13) se recouvrent sans partager leur début :
> const recouvre = a.debut < 13 && 11 < a.fin;
> ```
> 
> Deux idées y figurent : l'identité d'un instant ne se prouve pas par la référence, et le recouvrement se décide sur l'ordre des bornes. Transportez-les ensuite à votre `find`, puis vérifiez le TODO sur l'ordre entre début et fin.

### 02-adr-sans-alternatives — ADR incohérent : hors ligne exigé, base hébergée choisie, aucune alternative

`docs/architecture/0001-stockage.md` · titulaire `architecte` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion **non émergée** · coût 0,0003 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | rien | 0/2 | 2/2 | non | non | 0,0003 $ | 2,0 s |
| Question | non atteint | — | — | non | non | 0,0000 $ | 0,0 s |
| Piste | non atteint | — | — | non | non | 0,0000 $ | 0,0 s |

**Indice** — écarté : citation introuvable dans la section

Citation : `Les données doivent rester disponibles hors ligne.`

> Le contexte pose une contrainte de disponibilité hors ligne. Regardez comment la décision la traite. Si elle n'apparaît nulle part, la contrainte reste sans réponse : cherchez où vous pourriez dire ce qui est stocké localement et ce qui ne l'est pas.

**Indice** — écarté : citation introuvable dans la section

Citation : `Chaque membre a un profil avec son nom et son numéro d'appartement.`

> Vérifiez si cette phrase décrit un effet de la décision ou un fait déjà donné dans le contexte. Une conséquence doit pouvoir être lue comme « à cause de ce choix, il faudra… ».

### 03-ts-foreach-async — forEach avec une fonction async : le compte est renvoyé avant les enregistrements

`src/import/importer.ts` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion **non émergée** · coût 0,0006 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0004 $ | 5,2 s |
| Question | rien | — | — | non | non | 0,0002 $ | 1,3 s |
| Piste | non atteint | — | — | non | non | 0,0000 $ | 0,0 s |
| Exemple | non atteint | — | — | non | non | 0,0000 $ | 0,0 s |

**Indice** — affiché

Citation : `fiches.forEach(async (fiche) => {`

> Portez votre attention sur la ligne `fiches.forEach(async (fiche) => {` : demandez-vous ce que `forEach` fait de la valeur de retour de son callback, et à quel instant précis la ligne `return enregistrees;` s'exécute par rapport aux `await` qu'elle contient.

### 04-ts-find-undefined — find peut renvoyer undefined, masqué par une assertion non nulle

`src/panier/total.ts` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0013 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0006 $ | 4,1 s |
| Question | affiché | — | 1/1 | non | non | 0,0003 $ | 3,9 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0002 $ | 1,2 s |
| Exemple | rien | — | 1/1 | non | non | 0,0002 $ | 1,7 s |

**Indice** — affiché

Citation : `return produit!.prix;`

> Le « ! » affirme au compilateur que le résultat de « find » est toujours un produit. Regardez ce que « find » renvoie lorsqu'aucun élément ne correspond à la référence, puis ce que devient « .prix » dans ce cas.

**Question** — affiché

> Quel comportement voulez-vous pour un appelant qui demande une référence absente du panier : une valeur de repli, une erreur, ou autre chose ? Une fois ce choix fixé, le consommateur de « prixDe » le sait-il d'après sa signature ?

**Piste** — affiché

> La notion en jeu est le type « undefined » et le rétrécissement de type (narrowing) sur un résultat possiblement absent. La signature de « Array.prototype.find » dans la documentation TypeScript indique le type de retour exact ; la page « strictNullChecks » du manuel explique pourquoi ce type vous oblige à traiter le cas avant d'accéder à une propriété.

**Exemple** — écarté : message vide ou trop long

> Cas voisin, hors de votre fichier : une recherche dans une liste de codes produit, où l'absence est traitée explicitement avant usage.
> 
> ```ts
> function libelleDe(codes: string[], cible: string): string {
>   const trouve = codes.find((c) => c === cible);
>   if (trouve === undefined) {
>     return "inconnu";
>   }
>   return trouve.toUpperCase();
> }
> ```
> 
> Après le test, le type de « trouve » ne contient plus « undefined » : l'accès est sûr. Le même test, mais avec une erreur levée ou une valeur de repli, produit le même rétrécissement ; ce qui change est ce que voit l'appelant. Une troisième piste existe quand l'absence est normale : le type de retour lui-même (par exemple « number | undefined ») porte l'information jusqu'à l'appelant, qui doit alors la traiter aussi.

### 05-ts-tri-sur-place — sort trie sur place le tableau reçu en paramètre

`src/agenda/tri.ts` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0010 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | non | non | 0,0002 $ | 1,3 s |
| Question | affiché | — | 1/1 | non | non | 0,0004 $ | 1,8 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0001 $ | 2,2 s |
| Exemple | rien | — | 1/1 | non | non | 0,0003 $ | 1,8 s |

**Indice** — affiché

Citation : `creneaux
    .sort((a, b) => a.debut.getTime() - b.debut.getTime())`

> Regardez ce que fait `Array.prototype.sort` sur le tableau que la fonction reçoit. Demandez-vous ce que voit l'appelant après l'appel, si son tableau était déjà utilisé ailleurs.

**Indice** — affiché

Citation : `.slice(0, n)`

> Regardez la documentation de `slice` pour les bornes négatives ou nulles. Testez votre fonction avec n = 0 et n = -1 pour voir ce qu'elle renvoie.

**Question** — affiché

> Si l'appelant réutilise son tableau après l'appel, dans quel ordre le retrouve-t-il ? Est-ce ce que son contrat avec `prochainsCreneaux` lui promet ?

**Piste** — affiché

> Notion en jeu : les méthodes de tableau qui mutent le récepteur, opposées à leurs variantes non mutantes. La documentation MDN de `Array.prototype.sort` signale explicitement la mutation en place ; celle de `Array.prototype.toSorted` présente l'équivalent qui renvoie un nouveau tableau. Voir aussi la notion d'immuabilité des entrées d'une fonction.

**Exemple** — écarté : message vide ou trop long

> Cas voisin, sur une autre fonction :
> 
> ```ts
> function derniersEnOrdreInverse(pile: number[], n: number): number[] {
>   return [...pile].reverse().slice(0, n);
> }
> 
> const file = [1, 2, 3];
> const r = derniersEnOrdreInverse(file, 2); // [3, 2]
> console.log(file); // [1, 2, 3] : entrée intacte
> ```
> 
> La même intention écrite sans recopier donnerait `pile.reverse()` : `r` serait identique, mais `file` serait `[3, 2, 1]` après l'appel. Comparez avec votre situation en vous demandant lequel des deux contrats vous voulez offrir à l'appelant.

### 06-js-var-boucle — var dans une boucle : toutes les fonctions voient la même variable

`src/ui/boutons.js` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0010 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0001 $ | 0,9 s |
| Question | affiché | — | 1/1 | non | non | 0,0002 $ | 2,2 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0002 $ | 2,5 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0005 $ | 4,1 s |

**Indice** — affiché

Citation : `boutons[i].addEventListener("click", function () {
      afficher("Bouton " + (i + 1));
    });`

> À quel moment le corps de la fonction passée à addEventListener s'exécute-t-il, par rapport à la boucle ? Vous pouvez tester en cliquant plusieurs boutons et en observant la valeur affichée.

**Question** — affiché

> À la fin de la boucle, combien de variables `i` existent : une par bouton, ou une seule partagée par toutes les fonctions créées ? Que faut-il alors pour que chaque fonction garde sa propre valeur au moment du clic ?

**Piste** — affiché

> La notion en jeu est la fermeture (closure) : une fonction conserve l'accès à la variable de son environnement, pas à sa valeur du moment. Documentez-vous sur « closure » et sur la portée de bloc de `let`/`const` (MDN, chapitre « Portée des variables »), puis sur la méthode `bind` d'une fonction. Comparez ce que chacune de ces trois approches capture réellement.

**Exemple** — affiché

> Un cas voisin, sur une autre API que le DOM :
> 
> ```js
> for (var n = 0; n < 3; n++) {
>   taches.push(function () { return n; }); // toutes rendent 3
> }
> for (let n = 0; n < 3; n++) {
>   taches.push(function () { return n; }); // rendent 0, 1, 2
> }
> ```
> 
> Comparez ce que chaque variante capture, puis cherchez l'équivalent avec `bind` : la valeur est alors fixée à la création, sans dépendre de la variable.

### 07-js-tri-numerique — sort sans comparateur trie des nombres comme du texte

`src/stats/mediane.js` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0011 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | non | non | 0,0003 $ | 1,6 s |
| Question | affiché | — | 1/1 | non | non | 0,0002 $ | 3,1 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0001 $ | 1,1 s |
| Exemple | rien | — | 1/1 | non | non | 0,0005 $ | 4,1 s |

**Indice** — affiché

Citation : `const triees = [...valeurs].sort();`

> Regardez ce que fait `sort()` sans argument : quel ordre applique-t-il par défaut sur les éléments ?

**Indice** — affiché

Citation : `if (triees.length % 2 === 0) {`

> Suivez ce que valent `triees[milieu - 1]` et `triees[milieu]` quand `valeurs` est vide.

**Question** — affiché

> Prenez `mediane([1, 2, 10])`. Quelle position occupe 10 après le tri, et quel nombre votre fonction renvoie-t-elle alors comme valeur du milieu ? Vous pouvez le vérifier dans la console.

**Piste** — affiché

> La notion en jeu est le comparateur de tri : `sort()` accepte une fonction de comparaison qui définit l'ordre. Voyez la page MDN de `Array.prototype.sort`, section sur le paramètre `compareFn`, et la notion de soustraction comme comparateur pour les nombres.

**Exemple** — écarté : message vide ou trop long

> Un cas voisin, sur des chaînes et sur des objets :
> 
> ```js
> const mots = ['poire', 'kiwi', 'figue'];
> const parLongueur = [...mots].sort((a, b) => a.length - b.length);
> // ['kiwi', 'figue', 'poire']
> 
> const notes = [{ n: 12 }, { n: 3 }, { n: 7 }];
> const parNote = [...notes].sort((a, b) => a.n - b.n);
> // [{ n: 3 }, { n: 7 }, { n: 12 }]
> ```
> 
> La fonction de comparaison renvoie un nombre : seul son signe compte pour l'ordre. Testez le même jeu de valeurs, une fois sans comparateur, une fois avec.

### 08-js-prix-flottants — Montants en euros flottants comparés avec ===

`src/caisse/reste.js` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0009 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | oui | non | 0,0004 $ | 4,6 s |
| Question | affiché | — | 1/1 | non | non | 0,0003 $ | 3,7 s |
| Piste | rien | — | 1/1 | non | non | 0,0002 $ | 3,9 s |
| Exemple | non atteint | — | — | non | non | 0,0000 $ | 0,0 s |

**Indice** — affiché

Citation : `return total - dejaVerse === 0 ? "payé" : "à payer";`

> Portez votre attention sur la valeur exacte de la différence avant la comparaison : les montants en euros sont des nombres à virgule. Que vaut cette différence quand les centimes ne tombent pas juste ?

**Indice** — affiché

Citation : `"à payer"`

> Regardez ce que renvoie la fonction dans les cas limites du versement : versement égal au total, versement supérieur au total. Voyez-vous un état que le retour actuel ne distingue pas ?

**Question** — affiché

> Dans votre console, que renvoie la comparaison entre la somme de 0,1 et 0,2 d'une part, et 0,3 d'autre part ? Rejouez ensuite votre fonction avec une ligne à 0,1 € et un versement de 0,3 € : la différence obtenue vaut-elle exactement zéro ?

**Piste** — écarté : message vide ou trop long

> La notion en jeu est l'arithmétique en virgule flottante : un montant en euros calculé avec des nombres JavaScript ne tombe presque jamais sur une valeur exacte. Deux pistes de documentation pour la nommer et la cadrer : la page MDN « Nombres et dates » (section sur la précision des nombres à virgule) et la page sur Number.EPSILON. Vous y trouverez les deux familles d'approches usuelles : comparer avec une tolérance, ou convertir les montants en une unité entière. À vous de choisir laquelle convient à un calcul de caisse.

### 09-py-defaut-mutable — Liste mutable comme valeur par défaut d'un paramètre

`inventaire/panier.py` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0010 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | oui | non | 0,0003 $ | 4,6 s |
| Question | affiché | — | 1/1 | non | non | 0,0001 $ | 1,4 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0002 $ | 1,8 s |
| Exemple | rien | — | 1/1 | non | non | 0,0003 $ | 2,1 s |

**Indice** — affiché

Citation : `def ajouter(article, panier=[]):`

> Regardez à quel moment précis la valeur par défaut est évaluée, et combien de fois sur la durée de vie du module. Ce point se vérifie en appelant la fonction deux fois de suite, puis en comparant les identifiants des deux retours.

**Question** — affiché

> Si deux personnes distinctes appellent ajouter sans fournir de panier, obtiennent-elles deux paniers indépendants ? Écrivez un test qui appelle la fonction depuis deux endroits séparés et vérifiez si les emprunts restent cloisonnés.

**Piste** — affiché

> La notion en jeu porte un nom : l'argument par défaut mutable. Elle est décrite dans le tutoriel Python, section « Valeurs par défaut des arguments », et dans la FAQ du même nom (« Pourquoi les arguments par défaut sont-ils partagés entre les objets ? »). La documentation y présente aussi l'idiome de remplacement usuel, à base d'une valeur sentinelle, que vous pourrez confronter à votre cas.

**Exemple** — écarté : message vide ou trop long

> Un cas voisin, dans une autre domaine : une liste par défaut qui sert d'accumulateur.
> 
> ```python
> def releve(temperature, historique=[]):
>     historique.append(temperature)
>     return historique
> 
> print(releve(12.0))
> print(releve(15.5))
> print(releve(9.0, []))
> ```
> 
> Confronté à votre situation, ce cas montre où se situe la surprise : ce n'est pas l'ajout qui est en cause, mais ce que désigne le second paramètre au moment de l'appel. À vous de transposer la lecture à votre propre fonction, puis de vérifier ce que renvoient deux appels successifs sans second argument.

### 10-py-liaison-tardive — Fermetures créées dans une boucle : liaison tardive de la variable

`rapports/filtres.py` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0011 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0003 $ | 5,3 s |
| Question | affiché | — | 1/1 | non | non | 0,0004 $ | 8,9 s |
| Piste | affiché | — | 1/1 | oui | **oui** : `functools.partial` | 0,0002 $ | 6,4 s |
| Exemple | rien | — | 1/1 | non | non | 0,0002 $ | 1,5 s |

**Indice** — affiché

Citation : `filtres.append(lambda note: note >= limite)`

> Regardez le moment où `limite` est lu dans cette expression, et comparez-le au moment où la boucle change de valeur. Un essai avec deux limites distinctes, en appelant ensuite les deux fonctions retournées, rend l'écart visible.

**Question** — affiché

> Quand vous appelez la première fonction retournée, quelle valeur de `limite` utilise-t-elle réellement ? Faites l'essai avec deux limites distinctes, puis demandez-vous ce qui, dans la fonction, devrait être figé pour que chaque appel garde la sienne.

**Piste** — affiché

> La notion en jeu est la liaison tardive des fermetures : la variable libre est résolue à l'appel, pas à la création. La FAQ Python traite ce point sous « late binding » (rubrique programmation fonctionnelle), et la documentation de `functools.partial` montre une autre voie pour figer un argument.

**Exemple** — écarté : message vide ou trop long

> Cas voisin, avec des fonctions qui retiennent une valeur d'un contexte :
> 
> def multiplicateurs(facteurs):
>     resultat = []
>     for f in facteurs:
>         resultat.append(lambda x, f=f: x * f)
>     return resultat
> 
> En appelant multiplicateurs([2, 3]) puis chacune des deux fonctions avec 10, chaque appel garde son propre facteur. Comparez la place de `f` dans la lambda : à gauche de `=`, il est lié à la création ; à droite, il reste résolu à l'appel. Cherchez `lambda x, f=f:` dans la documentation des expressions lambda pour le détail de cette liaison.

### 11-py-liste-modifiee-en-parcours — Liste modifiée pendant qu'on la parcourt : des éléments sont sautés

`taches/nettoyage.py` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0006 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0001 $ | 1,3 s |
| Question | affiché | — | 1/1 | non | non | 0,0001 $ | 0,9 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0001 $ | 1,3 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0003 $ | 1,9 s |

**Indice** — affiché

Citation : `for tache in taches:`

> Regardez la ligne `for tache in taches:` en même temps que `taches.remove(tache)` : la boucle parcourt la liste pendant qu'elle est modifiée. Essayez avec deux tâches terminées consécutives pour voir ce que produit ce voisinage.

**Question** — affiché

> Avec deux tâches terminées consécutives, quelle tâche la boucle examine-t-elle juste après le retrait de la première ? Vous pouvez le vérifier en affichant l'indice ou la tâche à chaque tour.

**Piste** — affiché

> La notion en jeu est la distinction entre modifier une liste en place et construire une nouvelle liste. Voyez la section « Structures de données » du tutoriel Python, où la construction de liste par filtrage est présentée, ainsi que la note sur l'itération pendant une mutation.

**Exemple** — affiché

> Un cas voisin, sur des entiers plutôt que des dictionnaires :
> 
> ```python
> nombres = [3, 8, 5, 8, 1]
> for n in nombres:
>     if n == 8:
>         nombres.remove(n)
> print(nombres)  # [3, 5, 8, 1]
> ```
> 
> Comparez la liste affichée avec celle que vous attendiez en la parcourant à la main, tour par tour.

### 12-py-is-au-lieu-de-egal — is compare l'identité des objets, pas leur valeur

`comptes/verification.py` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0009 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0003 $ | 3,9 s |
| Question | affiché | — | 1/1 | non | non | 0,0004 $ | 3,1 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0002 $ | 2,0 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0001 $ | 1,0 s |

**Indice** — affiché

Citation : `return saisi.strip() is attendu`

> Regardez l'opérateur utilisé sur cette ligne de retour. Demandez-vous ce qu'il vérifie entre deux chaînes, puis testez-le avec deux chaînes identiques mais construites séparément.

**Question** — affiché

> Si deux chaînes ont le même contenu mais viennent de sources différentes, que renvoie `is` entre elles ? Et lequel de vos deux tests répond à la question « le code saisi correspond-il au code attendu » ?

**Piste** — affiché

> La notion en jeu : identité d'objet (`is`) contre égalité de valeur (`==`) en Python, et l'internement des chaînes qui rend `is` parfois vrai par hasard. Documentez-vous sur la section « Comparaisons » du tutoriel Python et sur `id()` dans la bibliothèque standard.

**Exemple** — affiché

> Un cas voisin, sur des listes plutôt que des chaînes :
> 
> ````python
> cli = []
> srv = []
> print(cli == srv)   # True : même contenu
> print(cli is srv)   # False : deux objets distincts
> a = cli
> print(a is cli)     # True : même objet
> ````
> 
> La première comparaison porte sur la valeur, la seconde sur l'objet. Le même partage vaut pour les chaînes.

### 13-rs-unwrap-saisie — unwrap sur une saisie utilisateur : panique au lieu d'une erreur

`src/commande.rs` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion **non émergée** · coût 0,0012 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | non | non | 0,0004 $ | 3,1 s |
| Question | affiché | — | 1/1 | non | non | 0,0006 $ | 4,3 s |
| Piste | rien | — | 1/1 | non | non | 0,0002 $ | 1,5 s |
| Exemple | non atteint | — | — | non | non | 0,0000 $ | 0,0 s |

**Indice** — affiché

Citation : `io::stdin().read_line(&mut ligne).unwrap();`

> Regardez ce que renvoie `read_line` lorsqu'il n'y a rien à lire. Quels cas autres que `Ok(n)` cette fonction peut-elle produire, et que contient alors `ligne` ?

**Indice** — affiché

Citation : `ligne.trim().parse().unwrap()`

> Testez avec une entrée vide (juste Entrée, ou fin de flux). Cherchez comment `parse` réagit à une chaîne vide après `trim`, et ce que cela produit du point de vue de l'appelant.

**Question** — affiché

> Si `ligne` est vide ou ne contient pas un entier, que renvoie `ligne.trim().parse::<u32>()`, et que fait `unwrap` de ce résultat ?

**Piste** — écarté : message vide ou trop long

> La notion en jeu est la propagation d'erreur : le type `Result`, l'opérateur `?` et le choix d'un type de retour qui peut échouer. Voyez le chapitre « Gestion des erreurs » du Rust Book, puis la signature et les exemples des sections « Errors » de `std::io::Stdin::read_line` et de `str::parse` dans la documentation standard. Vous y verrez pourquoi une fonction qui renvoie `u32` ne peut pas transmettre une erreur, et ce que cela implique pour `unwrap`.

### 14-rs-fenetre-glissante — Borne de boucle : la dernière fenêtre manque, et soustraction qui déborde

`src/stats.rs` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0008 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | oui | non | 0,0003 $ | 3,9 s |
| Question | affiché | — | 1/1 | non | non | 0,0003 $ | 3,6 s |
| Piste | rien | — | 1/1 | non | non | 0,0002 $ | 4,8 s |
| Exemple | non atteint | — | — | non | non | 0,0000 $ | 0,0 s |

**Indice** — affiché

Citation : `for debut in 0..valeurs.len() - fenetre {`

> Comparez le nombre de fenêtres réellement calculables pour une tranche de longueur n et une fenêtre de taille k, avec la borne écrite ici. Regardez aussi ce que vaut l'expression de la borne quand `fenetre` est égale à la longueur, ou la dépasse.

**Indice** — affiché

Citation : `moyennes.push(somme / fenetre as f64);`

> Que fait cette division quand l'appelant passe une fenêtre vide ?

**Question** — affiché

> Prenez n = 4 et k = 2 : comptez à la main les fenêtres que vous attendez, puis celles que votre boucle produit. Combien en manque-t-il, et d'où vient exactement cette différence ?

**Piste** — écarté : message vide ou trop long

> La notion en jeu est l'arithmétique des bornes sur des entiers non signés : une soustraction dont le résultat serait négatif n'a pas de représentation, et le comportement diffère entre compilation de mise au point et compilation optimisée. Pour la borne de la boucle comme pour les découpages, documentez-vous sur les opérateurs arithmétiques de `usize` et sur la méthode `windows` de `slice`, dont la documentation décrit le nombre de tranches produites pour une longueur et une taille données. Vérifiez ensuite, sur le papier, comment les deux coïncident quand la fenêtre vaut la longueur, puis quand elle la dépasse.

### 15-rs-clone-pour-l-emprunteur — clone partout pour contourner l'emprunteur, et liste vide qui panique

`src/inventaire.rs` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion **non émergée** · coût 0,0013 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0002 $ | 1,7 s |
| Question | affiché | — | 1/1 | non | non | 0,0003 $ | 6,3 s |
| Piste | affiché | — | 1/1 | non | non | 0,0002 $ | 2,4 s |
| Exemple | rien | — | 1/1 | non | non | 0,0005 $ | 4,7 s |

**Indice** — affiché

Citation : `let mut meilleur = self.articles[0].clone();`

> Regardez la ligne où le premier article est lu. Que se passe-t-il si la collection ne contient aucun article ?

**Question** — affiché

> Le cas du vide écarté, demandez-vous ce que votre signature doit promettre : quelle valeur de retour permettrait à l'appelant de distinguer « aucun article » de « l'article vide » ?

**Piste** — affiché

> La notion en jeu est `Option<T>` : elle permet à la signature de porter l'absence de résultat. Voyez sa documentation dans la bibliothèque standard, ainsi que les méthodes de `slice` qui la renvoient (`get`, `max_by_key`), pour choisir ce que votre fonction promet à l'appelant.

**Exemple** — écarté : message vide ou trop long

> Un cas voisin, dans un autre domaine, où l'absence de résultat est portée par le type :
> 
> ```rust
> fn record(scores: &[u32]) -> Option<u32> {
>     let mut best = None;
>     for &s in scores {
>         if best.map_or(true, |b| s > b) {
>             best = Some(s);
>         }
>     }
>     best
> }
> ```
> 
> L'entrée vide ne panique pas : elle traverse simplement la boucle. La signature `Option<u32>` dit à l'appelant qu'il n'y a pas toujours de résultat. Notez aussi l'itération par référence, qui ne clone pas la collection.

### 16-rs-iterateur-paresseux — map sans consommation : l'itérateur paresseux ne s'exécute jamais

`src/journal.rs` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0008 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | oui | non | 0,0003 $ | 4,5 s |
| Question | affiché | — | 1/1 | non | non | 0,0003 $ | 2,6 s |
| Piste | rien | — | 1/1 | non | non | 0,0002 $ | 6,5 s |
| Exemple | non atteint | — | — | non | non | 0,0000 $ | 0,0 s |

**Indice** — affiché

Citation : `lignes.iter().map(|ligne| archive.push(ligne.to_uppercase()));`

> Regardez le moment où la fermeture passée à map est réellement appelée : cherchez dans la documentation de Iterator::map ce qu'il advient de l'itérateur retourné quand il n'est pas consommé. Demandez-vous ensuite ce qui se passe dans archive après l'appel de archiver.

**Question** — affiché

> Appelez archiver avec trois lignes, puis affichez archive.len() et le contenu d'archive. Que constatez-vous par rapport au nombre de lignes reçues ? Quelle ligne de votre fonction a réellement agi pendant cet appel ?

**Piste** — écarté : message vide ou trop long

> La notion en jeu est la distinction entre adaptateur et consommateur d'itérateur. Consultez la page du module std::iter : elle sépare les adaptateurs (qui renvoient un itérateur) des consommateurs (qui parcourent réellement et renvoient une valeur). Cherchez ensuite lequel des deux votre expression utilise, et lequel vous auriez besoin d'utiliser pour que le corps de la fermeture soit exécuté. Regardez aussi le type de retour de cette fermeture : il vous indique ce que l'itérateur produirait s'il était consommé.

### 17-adr-consequences-positives — ADR dont les conséquences ne listent aucun inconvénient

`docs/architecture/0002-connexion.md` · titulaire `architecte` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion **non émergée** · coût 0,0016 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | non | non | 0,0003 $ | 3,9 s |
| Question | affiché | — | 1/1 | non | non | 0,0007 $ | 7,8 s |
| Piste | rien | — | 1/1 | non | non | 0,0006 $ | 4,5 s |

**Indice** — affiché

Citation : `Plus de mots de passe à gérer.`

> Votre section ne recense que des effets favorables. Regardez ce qu'un lecteur aurait besoin de savoir, dans six mois, pour juger si la décision tient toujours.

**Indice** — affiché

Citation : `Le gardien n'a plus de réinitialisations à faire.`

> Le contexte parle de voisins et de téléphone ; ici, rien n'indique ce que la décision suppose de leur côté. Cherchez l'hypothèse non écrite sur la façon dont un membre reçoit le lien.

**Question** — affiché

> Quel effet défavorable un voisin peut-il subir avec ce lien, et quel signal, observé après la mise en service, vous ferait revenir sur la décision ?

**Piste** — écarté : message vide ou trop long

> Deux rubriques des canevas d'ADR complètent ce que vous avez déjà entendu : « conséquences négatives » et « conditions de révision » (critères qui rendraient la décision caduque). Le modèle de Nygard et le site adr.github.io les décrivent ; comparez-y le plan de votre document. Une troisième notion à situer dans le vôtre : la dépendance à un service externe, ici l'acheminement des messages, avec les hypothèses que cela suppose.

### 18-adr-decision-disproportionnee — Décision disproportionnée au regard des contraintes du contexte

`docs/architecture/0003-notifications.md` · titulaire `architecte` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0011 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | oui | non | 0,0004 $ | 5,9 s |
| Question | affiché | — | 1/1 | oui | non | 0,0005 $ | 7,8 s |
| Piste | rien | — | 1/1 | non | non | 0,0002 $ | 2,5 s |

**Indice** — affiché

Citation : `un cluster Kafka de trois nœuds`

> Relisez la section Contexte juste avant d'écrire la décision : « Raspberry Pi », « sans budget d'hébergement », « une centaine de notifications par jour ». Demandez-vous ce que la phrase citée ci-dessous demande en machines, en administration et en coût.

**Question** — affiché

> Quelle panne précise voulez-vous couvrir avec trois nœuds, et qui la remet en route ? Si cette panne n'apparaît pas dans le contexte, quelle contrainte du contexte la décision doit-elle respecter ?

**Piste** — écarté : message vide ou trop long

> La notion en jeu est la proportionnalité de l'architecture : dimensionner un composant sur la charge et les ressources réelles, pas sur un cas théorique. Pour la documenter, appuyez-vous sur le format ADR (le modèle de Michael Nygard) et sur sa section « Options envisagées » : chaque alternative écartée y est nommée avec le critère de rejet. C'est là que la contrainte de contexte devient vérifiable plus tard.

### 19-adr-nous-verrons — Cache sans stratégie d'invalidation, renvoyée à plus tard

`docs/architecture/0004-cache.md` · titulaire `architecte` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0012 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | non | non | 0,0007 $ | 13,7 s |
| Question | affiché | — | 1/1 | non | non | 0,0002 $ | 2,7 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0002 $ | 2,5 s |

**Indice** — affiché

Citation : `Nous verrons plus tard
s'il faut l'invalider.`

> Relisez cette phrase avec la phrase du contexte sur les objets ajoutés et retirés toute la journée. Demandez-vous ce que devient le cache entre deux modifications des voisins.

**Indice** — affiché

Citation : `Nous ajoutons un cache Redis devant l'API du catalogue.`

> Cette section ne dit pas à quelle condition ce choix serait remis en cause. Le contexte, lui, donne des éléments sur la fréquence des changements : cherchez lesquels pourraient servir de seuil de réexamen.

**Question** — affiché

> Si un voisin retire un objet à 10 h et qu'un visiteur ouvre la page du catalogue à 10 h 05, que voit-il ? Et pendant combien de temps, tant que la phrase « nous verrons plus tard » tient ?

**Piste** — affiché

> La notion en jeu est la fraîcheur du cache : la durée pendant laquelle une copie reste servie après une modification de la source. Documentez-vous sur les stratégies d'expiration (durée de vie fixe) et sur l'invalidation à l'écriture, dans la littérature sur les caches. Votre section Décision gagnera à nommer le choix retenu et la limite de fraîcheur acceptée.

### 20-conception-sans-contraintes — Document de conception qui choisit une solution sans dire les contraintes

`docs/design/recherche.md` · titulaire `architecte` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0006 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | oui | non | 0,0003 $ | 2,2 s |
| Question | affiché | — | 1/1 | non | non | 0,0001 $ | 1,5 s |
| Piste | rien | — | 1/1 | non | non | 0,0003 $ | 1,4 s |

**Indice** — affiché

Citation : `Chaque ajout
d'objet déclenche une réindexation`

> Le contexte nomme un public (« les voisins ») et un volume d'ajouts n'est posé nulle part. Regardez ce que suppose l'expression « Chaque ajout d'objet » : combien, à quelle fréquence ? Cela décide si la décision tient.

**Indice** — affiché

Citation : `Un cluster Elasticsearch indexe le catalogue en temps réel.`

> Aucun autre choix n'est examiné ni écarté. Regardez entre le contexte et cette phrase : qu'est-ce qui rendrait une lecture par nom impossible sans un index dédié ?

**Question** — affiché

> Si le nombre d'ajouts par jour était cent fois celui que vous imaginez, ou si des objets étaient modifiés puis supprimés, qu'est-ce qui, dans votre décision, resterait vrai et qu'est-ce qui devrait être réécrit ?

**Piste** — écarté : message vide ou trop long

> La notion en jeu est la volumétrie : nombre d'écritures par jour et taille du catalogue sont des hypothèses à écrire noir sur blanc dans le document. Pour vous documenter, regardez les modèles d'ADR (sections « Hypothèses » et « Conséquences ») et la documentation Elasticsearch sur la réindexation, qui distingue l'indexation au fil de l'eau et la reconstruction complète. C'est là que se lit le coût d'un « chaque ajout ».

