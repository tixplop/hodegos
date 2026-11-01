# Évaluation des mentors — 2026-11-01 13:31

21 cas, coût total 0,0394 $. Un modèle est retenu s'il ne propose aucune solution (motifs `must_not`, brouillons écartés compris) et fait émerger la notion (`must_mention`) dans 70 % des cas au moins, avec 80 % de citations valides au moins.

## Verdict par modèle

| Niveau · modèle | Cas | Notion émergée | Citations valides | Solution (brouillons) | Solution (affichée) | JSON invalides | Pannes | Casting différent | Coût | Jetons (entrée / sortie) | Durée médiane | Durée p90 | Verdict |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| fast · deepseek/deepseek-v4.1-flash-1017 | 21 | 18/21 (86 %) | 33/33 (100 %) | 0 | 0 | 0 | 1 | 0 | 0,0394 $ | 79742 / 50972 | 4,2 s | 9,6 s | **non concluant** |

## Par sorte de cas

| Sorte | Cas | Réussis | Citations valides | Solution (brouillons) | Coût |
| --- | --- | --- | --- | --- | --- |
| Passe | 15 | 13 | 20/20 | 0 | 0,0296 $ |
| Revue | 3 | 2 | 7/7 | 0 | 0,0046 $ |
| Voisinage | 3 | 3 | 6/6 | 0 | 0,0052 $ |

Une revue réussit si une remarque affichée fait émerger la notion et cite un fichier attendu ; un cas de voisinage, si la passe avec le voisinage la fait émerger ; une réévaluation, si le verdict est celui attendu.

## Voisinage

| Cas | Notion (section seule) | Notion (voisinage) | Coût (section) | Coût (voisinage) |
| --- | --- | --- | --- | --- |
| 26-troc-voisinage-ordre-des-arguments | non | oui | 0,0000 $ | 0,0006 $ |
| 31-budget-voisinage-periode-inversee | oui | oui | 0,0025 $ | 0,0005 $ |
| 36-taches-voisinage-rappels-utc | non | oui | 0,0002 $ | 0,0013 $ |

## Coût d'une revue

| Mentor | Modèle | Revues | Coût moyen | Jetons moyens (entrée / sortie) | Durée moyenne |
| --- | --- | --- | --- | --- | --- |
| `testeur` | deepseek/deepseek-v4.1-flash-1017 | 3 | 0,0015 $ | 2762 / 1808 | 7,4 s |
## Coût d'une journée type

Usage : 40 enregistrements par rafales de cinq (9 h, 9 h 40, 10 h 20, 11 h, 14 h, 14 h 40, 15 h 20, 16 h), 2 ouvertures du projet (9 h, 14 h), 4 commits, 2 passages cités qui disparaissent. Réglages par défaut : calme 3 min, intervalle 20 min, 12 revues par jour et par mentor, déclencheurs create, open, change. Rejoué avec le planificateur de l'IDE ; coûts mesurés ci-dessus.

| Poste | Nombre par jour | Coût unitaire | Par jour |
| --- | --- | --- | --- |
| Revues `architecte` (coût estimé) | 9 | 0,0015 $ | 0,0139 $ |
| Revues `mentor-produit` (coût estimé) | 9 | 0,0015 $ | 0,0139 $ |
| Revues `testeur` | 8 | 0,0015 $ | 0,0124 $ |
| Réévaluations | 2 | 0,0015 $ | 0,0031 $ |
| Surcoût du voisinage (passes) | 40 | 0,0006 $ | 0,0226 $ |

**Par jour : 0,0659 $ · par mois (22 jours ouvrés) : 1,4489 $** — tient dans 10,0000 $ par mois.


## Cas

### 01-dates-par-reference — Comparaison de dates par référence et chevauchement non détecté

`src/emprunts/reserver.ts` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0032 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | non | non | 0,0008 $ | 4,9 s |
| Question | affiché | — | 1/1 | non | non | 0,0007 $ | 2,0 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0002 $ | 1,4 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0014 $ | 9,0 s |

**Indice** — affiché

Citation : `e.debut === debut`

> Regardez `e.debut === debut` : avec deux instances de `Date` distinctes représentant le même instant, quel résultat donne cet opérateur ? Vérifiez avec deux dates créées séparément.

**Indice** — affiché

Citation : `e.objetId === objet.id && e.debut === debut`

> Le test ne regarde qu'un début identique. Regardez ce qui se passe si un emprunt existant commence au milieu de l'intervalle demandé, ou si le vôtre commence avant une réservation déjà enregistrée.

**Question** — affiché

> Un emprunt existant va du 1 au 10, une demande du 5 au 15 : que renvoie votre `find` dans ce cas ? Et pour une demande du 2 au 3, entièrement à l'intérieur d'un emprunt existant ? Quelle condition sur les deux bornes couvrirait ces deux situations à la fois ?

**Piste** — affiché

> Le TODO en fin de section touche la notion d'invariant d'intervalle : une demande n'a de sens que si la borne basse précède la borne haute. En TypeScript, comparer deux `Date` avec `<` ou `>` les convertit en nombres, mais `getTime()` (ou la comparaison explicite des valeurs) rend l'intention lisible. Voyez la page MDN « Date » pour la conversion en nombre et la notion de valeur de temps, puis demandez-vous où placer ce contrôle par rapport au `find` : avant ou après la détection de conflit change l'ordre des erreurs renvoyées.

**Exemple** — affiché

> Cas voisin, sur des bornes numériques plutôt que des `Date` :
> 
> ```ts
> type Plage = { min: number; max: number };
> 
> function longueur(p: Plage): number {
>   if (p.max < p.min) {
>     throw new RangeError("bornes inversées");
>   }
>   return p.max - p.min;
> }
> ```
> 
> Ici l'ordre des bornes est vérifié avant tout calcul. Chez vous, les bornes sont des `Date` : ce qui reste à trouver, c'est la valeur comparable à extraire de chaque date, puis la condition qui décrit le recouvrement de deux intervalles.

### 03-ts-foreach-async — forEach avec une fonction async : le compte est renvoyé avant les enregistrements

`src/import/importer.ts` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0022 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | oui | non | 0,0005 $ | 9,0 s |
| Question | affiché | — | 1/1 | oui | non | 0,0008 $ | 3,3 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0005 $ | 3,9 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0004 $ | 5,4 s |

**Indice** — affiché

Citation : `fiches.forEach(async (fiche) => {`

> Regardez la ligne `fiches.forEach(async (fiche) => {` : demandez-vous ce que forEach fait de la promesse renvoyée par son callback, et à quel moment la fonction est considérée comme terminée. Situez ensuite l'instant où `return enregistrees;` s'exécute par rapport aux `await enregistrer(fiche);`.

**Question** — affiché

> Appelez `importerTout` avec trois fiches, affichez le nombre reçu à la résolution de la promesse, puis affichez à nouveau le total une seconde plus tard : que constatez-vous ? Quelle valeur l'appelant devrait-il obtenir pour pouvoir s'y fier ?

**Piste** — affiché

> Notion à documenter : `Array.prototype.forEach` ne tient aucun compte de la promesse renvoyée par son callback — la page MDN le signale explicitement dans la section sur les callbacks asynchrones. Côté promesses, comparez ensuite deux familles dans la doc MDN : les combinateurs (`Promise.all`, `Promise.allSettled`), qui attendent un ensemble de promesses déjà lancées, et l'itération qui attend chaque promesse avant de passer à la suivante. Le choix dépend de si l'ordre d'enregistrement doit être garanti — question à trancher avant de réécrire. Voisin utile : `src/import/stockage.ts`, pour savoir si `enregistrer` peut échouer sur une fiche et ce que l'appelant attend alors.

**Exemple** — affiché

> Cas voisin, entièrement synchrone :
> 
> const xs = [1, 2, 3];
> let total = 0;
> xs.forEach((x) => {
>   setTimeout(() => { total += x; }, 10);
> });
> console.log(total);
> 
> Affiche 0 : chaque callback est lancé, forEach rend la main aussitôt. La promesse renvoyée par un callback async subit le même sort, elle est lancée et ignorée. Remplacez setTimeout par votre appel await et observez votre compteur : la valeur qu'il a au moment du return est celle des callbacks déjà terminés, pas des trois fiches.

### 04-ts-find-undefined — find peut renvoyer undefined, masqué par une assertion non nulle

`src/panier/total.ts` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0015 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0005 $ | 2,8 s |
| Question | affiché | — | 1/1 | non | non | 0,0002 $ | 1,6 s |
| Piste | affiché | — | 1/1 | non | non | 0,0002 $ | 1,6 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0006 $ | 9,8 s |

**Indice** — affiché

Citation : `return produit!.prix;`

> Que renvoie `find` quand aucune référence ne correspond ? Regardez ce que devient `produit` à ce moment-là, puis ce que le `!` fait à l'exécution. La signature annonce un `number` : vérifiez si ce contrat couvre ce cas.

**Question** — affiché

> Pour une référence absente, que doit faire `prixDe` du point de vue de l'appelant : lever une erreur, renvoyer une valeur sentinelle, ou autre chose ? Cette décision est-elle compatible avec la signature actuelle, qui promet un `number` dans tous les cas ?

**Piste** — affiché

> La notion en jeu est le rétrécissement de type (narrowing) : `find` renvoie un type qui inclut l'absence de résultat, et TypeScript attend que vous traitiez cette branche avant d'accéder à la valeur. Le `!` supprime cette vérification, donc la garantie disparaît au profit de l'exécution. À documenter : la section « Narrowing » du TypeScript Handbook, la page sur `strictNullChecks` (option `strict`), et la page « Return type void / never » pour les fonctions qui ne rendent jamais la main. Côté conception, regardez aussi comment vos voisins du catalogue expriment l'absence d'un produit : cela fixera la convention à suivre ici. Posez-vous enfin la question avant d'écrire : à quel endroit précis le contrat `number` doit-il être tenu ou abandonné, et qui doit en décider.

**Exemple** — affiché

> Cas voisin, sur d'autres types :
> 
> ```ts
> type Stock = Map<string, number>;
> 
> function quantiteDe(stock: Stock, code: string): number {
>   const q = stock.get(code); // number | undefined
>   if (q === undefined) {
>     throw new Error(`code inconnu : ${code}`);
>   }
>   return q; // ici le type est number
> }
> ```
> 
> Le point à transposer : l'accès rend un type qui inclut l'absence, et c'est un test sur cette valeur qui réduit le type pour la suite. Notez que le `throw` clôt la branche absente ; une autre issue possible serait de renvoyer une valeur de repli, ce qui change le contrat. À vous de choisir l'issue pour votre cas et de voir ce qu'elle implique pour l'appelant.

### 05-ts-tri-sur-place — sort trie sur place le tableau reçu en paramètre

`src/agenda/tri.ts` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0016 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0003 $ | 2,4 s |
| Question | affiché | — | 1/1 | non | non | 0,0004 $ | 2,4 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0006 $ | 11,2 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0002 $ | 6,7 s |

**Indice** — affiché

Citation : `.sort((a, b) => a.debut.getTime() - b.debut.getTime())`

> Portez votre attention sur `.sort(` : demandez-vous ce que l'appelant retrouve dans son propre tableau une fois l'appel terminé.

**Question** — affiché

> Quel contrat voulez-vous donner à cette fonction vis-à-vis du tableau reçu : le modifie-t-elle, ou le laisse-t-elle intact ? Si un appelant réutilise son tableau après l'appel, à quel résultat s'attend-il ?

**Piste** — affiché

> La notion en jeu est le tri en place : la méthode de tri réordonne le tableau reçu et renvoie ce même tableau, pas une copie. Pour tenir un contrat « le tableau de l'appelant reste intact », il faut trier autre chose que la référence reçue. Documentez-vous sur MDN à la page de la méthode de tri, section sur la mutation, et comparez avec les méthodes voisines de cette page qui renvoient un nouveau tableau. Vérifiez ce que devient ce contrat quand la liste est vide ou d'un seul élément. Second point : la borne passée au découpage, quand n vaut 0, dépasse la taille ou est négatif — même source, page de la méthode de découpage, section sur les indices négatifs.

**Exemple** — affiché

> Cas voisin, autre sujet, même famille de pièges :
> 
> ```ts
> const notes = [3, 1, 2];
> const inversees = [...notes].reverse(); // copie : notes = [3, 1, 2]
> notes.reverse();                        // en place : notes = [2, 1, 3]
> 
> // borne de découpage, sur une chaîne
> "abcdef".slice(0, 4);   // "abcd"
> "abcdef".slice(0, -2);  // "abcd"
> ```
> 
> Deux choses à transposer vous-même : quelle forme renvoie une nouveauté sans toucher la valeur reçue, et ce que devient la borne quand elle sort de l'intervalle attendu. Vérifiez ensuite, dans Node, ce que vaut la borne après l'appel dans votre cas.

### 06-js-var-boucle — var dans une boucle : toutes les fonctions voient la même variable

`src/ui/boutons.js` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0037 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0007 $ | 1,5 s |
| Question | affiché | — | 1/1 | non | non | 0,0004 $ | 2,1 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0006 $ | 4,4 s |
| Exemple | affiché | — | 1/1 | non | non | 0,0021 $ | 6,0 s |

**Indice** — affiché

Citation : `afficher("Bouton " + (i + 1));`

> Portez votre attention sur le lien entre la boucle et la fonction de rappel : quand cette fonction lit `i`, la boucle a-t-elle déjà continué d'avancer ? Vous pouvez le vérifier en cliquant plusieurs boutons et en comparant les messages affichés.

**Question** — affiché

> Quand un clic survient, la boucle est terminée depuis longtemps : quelle valeur unique `i` reste alors lisible pour tous les rappels ? Et si vous vouliez une valeur propre à chaque bouton, à quel moment faudrait-il la fixer ?

**Piste** — affiché

> La notion en jeu : la portée d'une déclaration et la capture par fermeture. Un rappel ne conserve pas une copie de la valeur, il conserve la déclaration elle-même ; la question est donc de savoir ce que chaque déclaration lie, et à quel moment. À lire sur MDN : les pages « var » (section sur la portée) et « Closure » (le passage sur les fonctions déclarées dans une boucle), puis les pages comparant les déclarations à portée de fonction et celles à portée de bloc. Vous y verrez pourquoi deux boucles écrites presque à l'identique ne lient pas la même chose à chaque tour.

**Exemple** — affiché

> Un cas voisin, hors de votre boucle :
> 
> let niveau = 1;
> const lireNiveau = () => niveau;
> niveau = 3;
> lireNiveau(); // 3 : la lecture a lieu à l'exécution, pas à la création
> 
> function copier(valeur) {
>   return () => valeur;
> }
> copier(7)(); // 7 : ici la valeur a été fixée avant l'appel
> 
> Comparez la première et la seconde forme : dans l'une, la fonction trouve la déclaration encore modifiable ; dans l'autre, chaque appel crée une liaison distincte. Reste à voir laquelle convient à chacun de vos boutons.

### 07-js-tri-numerique — sort sans comparateur trie des nombres comme du texte

`src/stats/mediane.js` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0015 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | non | non | 0,0004 $ | 3,2 s |
| Question | affiché | — | 1/1 | oui | non | 0,0002 $ | 2,2 s |
| Piste | affiché | — | 1/1 | non | non | 0,0002 $ | 6,8 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0007 $ | 9,6 s |

**Indice** — affiché

Citation : `const triees = [...valeurs].sort();`

> Vérifiez le comportement par défaut de sort() dans la documentation MDN : quelle valeur est comparée quand aucun argument n'est fourni ? Déroulez mentalement avec [10, 9, 100] avant de conclure.

**Indice** — affiché

Citation : `const milieu = Math.floor(triees.length / 2);`

> Déroulez votre fonction à la main pour un tableau sans élément : quelle est la valeur de milieu, et que valent ensuite triees[milieu - 1] et triees[milieu] ?

**Question** — affiché

> Pour quelles entrées l'ordre obtenu par ce tri cesse-t-il de coïncider avec l'ordre numérique ? Déroulez [2, 10] puis [1, 20, 3] : à chaque fois, demandez-vous quelle chaîne est comparée à quelle autre, et si la position finale correspond au plus petit nombre.

**Piste** — affiché

> La notion à aller chercher s'appelle la fonction de comparaison (compareFn) de Array.prototype.sort. Sur MDN, ouvrez Array.prototype.sort et lisez la section sur le tri numérique : la fonction attendue y est décrite par sa valeur de retour, de type nombre et de signe porteur de sens, ce qui diffère d'un prédicat booléen. Comparez cette description avec l'absence d'argument dans votre appel pour décider ce qu'il manque. Vérifiez aussi, une fois ce point réglé, ce que renvoie votre fonction pour un tableau vide, cas où le milieu vaut zéro.

**Exemple** — affiché

> Voici deux compareFn voisines, sur des chaînes, pour voir la forme attendue : un nombre dont seul le signe est lu.
> 
> ```js
> const mots = ['poire', 'kiwi', 'abricot'];
> const copie = () => [...mots];
> 
> // écrite à la main : négatif = a avant b, positif = a après b, 0 = égal
> const decroissant = (a, b) => (a < b ? 1 : a > b ? -1 : 0);
> console.log(copie().sort(decroissant));
> 
> // fournie par la bibliothèque
> console.log(copie().sort((a, b) => a.localeCompare(b)));
> ```
> 
> Repérez ce que produit chaque appel en l'absence d'argument, puis transcrivez la même forme pour un critère qui n'est pas du texte.

### 08-js-prix-flottants — Montants en euros flottants comparés avec ===

`src/caisse/reste.js` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0024 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | oui | non | 0,0007 $ | 4,4 s |
| Question | affiché | — | 1/1 | non | non | 0,0004 $ | 2,1 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0008 $ | 2,2 s |
| Exemple | rien | — | 1/1 | non | non | 0,0005 $ | 4,8 s |

**Indice** — affiché

Citation : `total += ligne.prixEuros * ligne.quantite;`

> Regardez la nature des valeurs qui s'accumulent ici, puis ce que la comparaison stricte à 0 exige d'elles. Essayez la fonction avec deux ou trois lignes dont les prix portent des centimes et observez ce que renvoie la fonction.

**Indice** — affiché

Citation : `return total - dejaVerse === 0 ? "payé" : "à payer";`

> Que renvoie la fonction si dejaVerse dépasse le total ? Ce cas est-il représentable dans votre résultat ? Regardez ce que la soustraction peut produire en dehors de 0.

**Question** — affiché

> Quel texte attendez-vous pour un total de 10,01 € face à un versement de 10 € ? Lancez la fonction sur ce cas et comparez. Question voisine : le résultat doit-il seulement distinguer deux situations, ou aussi indiquer combien il reste ?

**Piste** — affiché

> Notion en jeu : en JavaScript, tous les nombres sont des flottants binaires (IEEE 754) ; additionner plusieurs décimaux accumule une erreur d'arrondi, donc une égalité exacte à 0 devient fragile. Deux notions à documenter : la comparaison à tolérance (voir MDN, page Number.EPSILON) et le calcul en unités entières, par exemple des centimes, plutôt qu'en euros décimaux (MDN, Math.round). Sur le second point, cherchez « contrat de retour » : nommer ce que la valeur doit représenter, un état ou un montant, avant de choisir sa forme.

**Exemple** — écarté : garde « pas de solution » : exemple qui recopie un fragment de la section

> Cas voisin (cumul de masses en kg décimales) :
> 
> let total = 0;
> for (const m of [0.1, 0.2, 0.3]) {
>   total += m;
> }
> console.log(total);          // 0.6000000000000001
> console.log(total === 0.6);  // false
> 
> Le même mécanisme que chez vous produit ici un faux résultat négatif : la valeur affichée n'est pas celle qu'on écrit dans le source. À vous de choisir, pour votre fonction, un test qui ne repose pas sur l'égalité exacte, et de décider quelle forme de retour correspond à l'intention (un état, ou le montant restant).

### 09-py-defaut-mutable — Liste mutable comme valeur par défaut d'un paramètre

`inventaire/panier.py` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0015 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0001 $ | 0,9 s |
| Question | affiché | — | 1/1 | non | non | 0,0002 $ | 2,3 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0004 $ | 4,3 s |
| Exemple | affiché | — | 1/1 | non | non | 0,0006 $ | 4,2 s |

**Indice** — affiché

Citation : `panier=[]`

> Regardez `panier=[]` : à quel moment cette liste est-elle créée, et combien d'appels successifs partagent la même ? Testez deux appels de suite en observant le retour.

**Question** — affiché

> Sans second argument, que veut l'appelant : un panier neuf à chaque fois, ou rejoindre un panier déjà existant ? Écrivez le test qui distingue les deux, puis regardez ce que votre fonction renvoie au deuxième appel. Ce que le test affiche correspond-il à l'intention ?

**Piste** — affiché

> La notion en jeu est l'argument par défaut mutable : l'expression entre parenthèses est évaluée une seule fois, à la définition de la fonction, et non à chaque appel. Documentation : le tutoriel Python, section « Valeurs d'arguments par défaut », et la FAQ « Pourquoi les valeurs par défaut sont-elles partagées entre les appels ? ». Le tutoriel y décrit l'idiome usuel, la sentinelle immuable (None) : elle signale « pas de panier fourni » et laisse construire un conteneur neuf à l'appel, sans retirer la possibilité de rejoindre un panier existant. Deux notions à lire : moment d'évaluation, puis sentinelle.

**Exemple** — affiché

> Cas voisin, avec un autre conteneur :
> 
> ```python
> def ranger(cle, cache={}):
>     cache[cle] = True
>     return cache
> 
> print(ranger("a"))
> print(ranger("b"))
> ```
> 
> Les deux affichages montrent le même dictionnaire : le second appel retrouve l'entrée laissée par le premier. Le conteneur n'a pas été recréé entre les deux.
> 
> Reproduisez ce scénario à l'identique sur votre fonction, puis notez à quel moment l'objet vu par chaque appel a été créé. Comparez avec un objet transmis explicitement par l'appelant.

### 10-py-liaison-tardive — Fermetures créées dans une boucle : liaison tardive de la variable

`rapports/filtres.py` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0022 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0004 $ | 5,0 s |
| Question | affiché | — | 1/1 | non | non | 0,0003 $ | 2,2 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0001 $ | 1,1 s |
| Exemple | affiché | — | 1/1 | non | non | 0,0013 $ | 6,4 s |

**Indice** — affiché

Citation : `filtres.append(lambda note: note >= limite)`

> Regardez `limite` : la boucle la réaffecte à chaque tour, alors que les lambdas ne seront exécutées qu'après le `return`. Où chaque lambda ira-t-elle lire sa valeur au moment de son appel ?

**Question** — affiché

> Après le `return`, appelez mentalement chaque filtre avec la plus petite des limites reçues : que renvoie le premier ? Et le dernier ? Combien de valeurs différentes l'ensemble des lambdas peut-il lire, sachant qu'il n'existe qu'une seule variable `limite` ?

**Piste** — affiché

> La notion en jeu est la liaison tardive des variables capturées par une fermeture : une lambda lit la variable au moment de son appel, pas à sa création. Pour creuser, la documentation Python sur les fermetures et la section « Late binding closures » de la FAQ programmation montrent comment une variable de boucle partagée produit ce comportement. Vous pouvez y comparer deux formes de capture et vérifier laquelle fige la valeur au tour courant.

**Exemple** — affiché

> Cas voisin, sans rapport avec votre fonction :
> 
> def prepare_controles():
>     plafond = 10
> 
>     def depasse(valeur):
>         return valeur > plafond
> 
>     plafond = 20
>     return depasse
> 
> verifie = prepare_controles()
> print(verifie(15))   # True
> 
> Le corps de `depasse` n'a pas mémorisé 10 au moment de sa définition : il lit `plafond` à l'appel, donc 20. C'est le même mécanisme que dans votre boucle. Comparez : combien de valeurs distinctes chaque lambda de votre liste peut-elle lire au final ?

### 11-py-liste-modifiee-en-parcours — Liste modifiée pendant qu'on la parcourt : des éléments sont sautés

`taches/nettoyage.py` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion **non émergée** · coût 0,0021 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0002 $ | 2,4 s |
| Question | affiché | — | 1/1 | non | non | 0,0004 $ | 7,4 s |
| Piste | affiché | — | 1/1 | non | non | 0,0006 $ | 5,5 s |
| Exemple | affiché | — | 1/1 | non | non | 0,0008 $ | 15,1 s |

**Indice** — affiché

Citation : `for tache in taches:
        if tache["terminee"]:
            taches.remove(tache)`

> Décrivez le déroulement de la boucle quand un retrait survient : combien d'éléments la boucle parcourt-elle encore, et lesquels ? Vous pouvez le vérifier seule avec une liste de trois tâches dont la première est terminée.

**Question** — affiché

> Après un retrait, la boucle reprend à l'indice suivant alors que les éléments ont reculé d'une place. Combien de tâches terminées consécutives faut-il pour qu'une seule subsiste ? Et si toutes les tâches sont terminées, que renvoie la fonction ?

**Piste** — affiché

> Notion en jeu : la mutation d'une liste pendant qu'on la parcourt. Deux points à documenter : l'entrée `list.remove` de la doc (ce qu'elle supprime exactement et ce qu'elle décale) et l'idiome de la bibliothèque standard qui construit une nouvelle liste au lieu de toucher à celle qu'on lit (compréhensions et `filter`, tutoriel Python section « Structures de données »). Ce que ce palier ajoute : distinguez la liste parcourue de la liste produite, et vérifiez si l'appelant attend que la liste reçue soit modifiée sur place.

**Exemple** — affiché

> Cas voisin : la boucle lit une liste et remplit une autre structure ; la liste lue ressort intacte.
> 
> ```python
> def doubler(nombres):
>     resultat = []
>     for n in nombres:
>         resultat.append(n * 2)
>     return resultat
> 
> base = [1, 2, 3]
> print(doubler(base))  # [2, 4, 6]
> print(base)           # [1, 2, 3] : la liste parcourue n'a pas bougé
> ```
> 
> Ce que ce palier ajoute aux précédents : la boucle peut se contenter de lire, et la liste rendue à l'appelant être une autre liste. Comparez avec votre fonction : chez vous, le retrait vise la liste même que la boucle est en train de parcourir, et l'appelant reçoit le même objet qu'il a fourni. Reste à décider laquelle des deux formes correspond à ce qu'il attend.

### 12-py-is-au-lieu-de-egal — is compare l'identité des objets, pas leur valeur

`comptes/verification.py` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0015 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0002 $ | 2,2 s |
| Question | affiché | — | 1/1 | non | non | 0,0002 $ | 2,2 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0005 $ | 3,2 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0007 $ | 5,1 s |

**Indice** — affiché

Citation : `return saisi.strip() is attendu`

> Portez votre attention sur l'opérateur de `saisi.strip() is attendu` : demandez-vous ce qu'il compare réellement entre deux chaînes construites séparément. Un test avec deux chaînes de même contenu, mais d'origines différentes, vous renseignera.

**Question** — affiché

> Quand votre fonction renvoie False pour une saisie correcte, quel test minimal, écrit avec deux chaînes égales mais obtenues par des chemins différents, vous montrerait le résultat attendu ? Que vous faudrait-il observer pour décider si c'est la comparaison ou la saisie qui est en cause ?

**Piste** — affiché

> La notion à creuser est la distinction entre identité d'objet et égalité de valeur : l'opérateur `is` interroge l'identité, pas le contenu. Deux documents utiles : la section « Opérateurs de comparaison » du tutoriel Python, et la fonction `id()`, qui montre que deux chaînes de même contenu peuvent avoir des identités distinctes. Si vous voulez voir quel opérateur est réellement exécuté, le module `dis` désassemble la fonction et affiche l'instruction correspondante.

**Exemple** — affiché

> Cas voisin, sur des listes :
> 
> x = [1, 2]
> y = [1, 2]
> print(x is y)   # False
> z = x
> print(z is x)   # True
> z.append(3)
> print(x)        # [1, 2, 3]
> print(id(x), id(y))   # deux identifiants distincts
> 
> `is` interroge l'identité de l'objet, pas son contenu. Transposez à votre cas : obtenez deux chaînes de même contenu par des chemins différents, affichez leur `id()`, puis cherchez l'opérateur qui compare le contenu et non l'objet.

### 13-rs-unwrap-saisie — unwrap sur une saisie utilisateur : panique au lieu d'une erreur

`src/commande.rs` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0007 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | non | non | 0,0003 $ | 4,4 s |
| Question | affiché | — | 1/1 | non | non | 0,0002 $ | 3,7 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0002 $ | 1,5 s |

**Indice** — affiché

Citation : `ligne.trim().parse().unwrap()`

> Que renvoie `parse` si la saisie n'est pas un nombre ? Regardez le type de retour avant le `unwrap` et ce que ce `unwrap` déclenche dans ce cas.

**Indice** — affiché

Citation : `io::stdin().read_line(&mut ligne).unwrap();`

> Sur une entrée fermée, `read_line` réussit mais ne lit rien. Que devient `ligne` alors, et le `parse` qui suit ?

**Question** — affiché

> Quand la saisie n'est pas un nombre, votre fonction renvoie-t-elle quelque chose à l'appelant, ou interrompt-elle tout le programme ? Regardez ce que l'appelant attend dans ce cas : une valeur, une erreur, ou les deux cas séparés ?

**Piste** — affiché

> Le point en jeu est la propagation d'erreur : convertir un `Result` en valeur de retour plutôt que de le dérouler par `unwrap`. Voyez la notion de `Result` dans std, la section « Recoverable Errors » du livre Rust, et la signature de `std::io::Stdin::read_line` pour le type d'erreur produit. Demandez-vous quel contrat vous voulez donner à l'appelant de `lire_quantite` : un `u32` garanti, ou un `Result` qu'il traitera lui-même. C'est cette décision qui fixe la suite, pas la syntaxe.

### 14-rs-fenetre-glissante — Borne de boucle : la dernière fenêtre manque, et soustraction qui déborde

`src/stats.rs` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0010 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | oui | non | 0,0002 $ | 8,6 s |
| Question | affiché | — | 1/1 | non | non | 0,0005 $ | 3,0 s |
| Piste | rien | — | 1/1 | non | non | 0,0003 $ | 4,5 s |
| Exemple | non atteint | — | — | non | non | 0,0000 $ | 0,0 s |

**Indice** — affiché

Citation : `0..valeurs.len() - fenetre`

> Comptez à la main, pour une tranche de 5 valeurs et une fenêtre de 2, combien de moyennes sont attendues. Comparez ce total au nombre de tours produits par la plage de la boucle.

**Indice** — affiché

Citation : `somme / fenetre as f64`

> Regardez ce qui se passe en amont de cette division quand l'appelant choisit une fenêtre nulle ou plus grande que la tranche : quel type a `fenetre`, et que produit une soustraction ou une division dans ces deux cas ?

**Question** — affiché

> Testez deux appels : une fenêtre de 0, puis une fenêtre plus grande que la tranche. Quelle expression de la première ligne est alors évaluée en premier, et que pouvez-vous observer sur la valeur obtenue avant même d'entrer dans la boucle ?

**Piste** — écarté : message vide ou trop long

> Deux notions se croisent ici. La première est le compte de fenêtres : une tranche de n valeurs découpée en fenêtres de taille f donne n - f + 1 fenêtres, ce qui se vérifie sur un petit exemple. La seconde est l'arithmétique de usize : la soustraction d'une grandeur non bornée est un underflow, et la division par une taille nulle est à traiter à part. Pour la première, la documentation de slice::windows dans la bibliothèque standard montre un découpage qui calcule les bornes à votre place ; comparez son compte de fenêtres au vôtre. Pour la seconde, relisez la section sur les types entiers (comportement en debug et en release) et décidez ce que votre fonction doit faire des entrées dégénérées avant d'écrire la boucle. Demandez-vous aussi ce que renvoie votre version actuelle quand la fenêtre vaut la taille de la tranche.

### 15-rs-clone-pour-l-emprunteur — clone partout pour contourner l'emprunteur, et liste vide qui panique

`src/inventaire.rs` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion **non émergée** · coût 0,0021 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0007 $ | 4,2 s |
| Question | affiché | — | 1/1 | non | non | 0,0003 $ | 2,4 s |
| Piste | affiché | — | 1/1 | non | non | 0,0008 $ | 2,5 s |
| Exemple | affiché | — | 1/1 | non | non | 0,0003 $ | 2,4 s |

**Indice** — affiché

Citation : `let mut meilleur = self.articles[0].clone();`

> Que renvoie cette fonction si `articles` est vide ? La première ligne s'appuie sur une hypothèse que le type `Vec<String>` ne garantit pas. Vérifiez quel comportement vous voulez dans ce cas avant de parcourir la liste.

**Question** — affiché

> Quand `articles` est vide, quelle valeur votre signature `-> String` vous force-t-elle à fabriquer ? Un appelant peut-il distinguer cette valeur d'un article réel ? Regardez ce que le type de retour autorise à exprimer, et ce qu'il vous oblige à inventer. 

**Piste** — affiché

> La notion en jeu est la correspondance entre l'absence de résultat et le type de retour : en Rust, cela se documente dans le module `std::option` (chapitre « Option » du livre et doc de `Option`). Voyez aussi le trait `std::iter::Iterator` et ses méthodes d'agrégation, qui traitent une séquence possiblement vide sans indice fixe. À lire en gardant votre question de départ : quel type rend l'absence explicite pour l'appelant ?

**Exemple** — affiché

> Un cas voisin, autre domaine :
> ```rust
> fn plus_petit_age(ages: &[u32]) -> Option<u32> {
>     ages.iter().copied().min()
> }
> 
> fn main() {
>     println!("{:?}", plus_petit_age(&[42, 17, 30])); // Some(17)
>     println!("{:?}", plus_petit_age(&[]));           // None
> }
> ```
> Le type de retour rend l'absence lisible par l'appelant ; l'agrégat de l'itérateur la produit sans accès indexé. Reste à transposer votre propre comparaison.

### 16-rs-iterateur-paresseux — map sans consommation : l'itérateur paresseux ne s'exécute jamais

`src/journal.rs` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0025 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0004 $ | 2,2 s |
| Question | affiché | — | 1/1 | non | non | 0,0009 $ | 2,5 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0005 $ | 2,3 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0007 $ | 5,2 s |

**Indice** — affiché

Citation : `lignes.iter().map(|ligne| archive.push(ligne.to_uppercase()));`

> Observez la valeur que produit cette expression et ce qui est fait de cette valeur. Question à vous poser : à quel moment le corps de la closure est-il réellement exécuté ?

**Question** — affiché

> Question à trancher par l'expérience : après un appel sur deux lignes, quelle longueur a `archive` ? Écrivez un test qui l'affiche. Ce résultat vous dira si le corps de la closure a été exécuté au moins une fois pour chaque ligne, et donc ce que l'expression attend encore pour être parcourue.

**Piste** — affiché

> La notion en jeu : les adaptateurs d'itérateur (`map`, `filter`…) sont paresseux, ils ne font que construire un nouvel itérateur ; rien ne s'exécute tant qu'un consommateur ne le parcourt pas (`for_each`, `collect`, `count`, une boucle `for`…). À lire : la page du module `std::iter`, en particulier la distinction entre adaptateurs et consommateurs, et le rôle de l'attribut `#[must_use]` sur `Map`. Second point à documenter : un adaptateur est destiné à transformer des valeurs ; produire un effet de bord depuis l'intérieur d'un `map` va à l'encontre de cet idiome, d'où le choix d'un consommateur adapté à l'effet visé.

**Exemple** — affiché

> Cas voisin, avec un autre adaptateur et un autre consommateur :
> 
> ```rust
> fn doubler(n: &i32) -> i32 {
>     println!("calcul pour {n}");
>     n * 2
> }
> 
> let xs = vec![1, 2];
> let en_attente = xs.iter().map(doubler); // aucun « calcul » affiché
> let total: i32 = en_attente.sum(); // les deux « calcul » sortent ici
> ```
> 
> L'adaptateur ne fait qu'empiler un travail ; c'est le consommateur (`sum`, `collect`, `count`, boucle `for`) qui déclenche l'exécution. Dans votre section, quel élément joue ce rôle ?

### 25-troc-annuler-sans-test — Revue : l'annulation, avec ses trois refus, n'a aucun test

`` · titulaire `testeur` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0023 $

- projet `troc`, première revue
- remarque `project` · cite docs/vision.md, src/emprunts/annuler.ts
- remarque `project` · cite src/notifications/interne.ts, src/emprunts/reserver.ts

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 4/4 | 2/2 | oui | non | 0,0023 $ | 11,4 s |

**Indice** — affiché

Citation : `docs/vision.md : Annuler ou prolonger une réservation. · src/emprunts/annuler.ts : export async function annuler(depot: Depot, id: string, par: string): Promise<boolean>`

> Votre vision annonce l'annulation d'une réservation ; aucun fichier de tests ne la couvre. Repérez, dans la liste des tests présents, ce qui manque et choisissez le comportement d'annulation à verrouiller en premier.

**Indice** — affiché

Citation : `src/notifications/interne.ts : export function notifier(voisin: string, message: string): void · src/emprunts/reserver.ts : export async function reserver(`

> La vision promet une notification à la réservation acceptée ; `notifier` et `relever` existent, aucun test ne les approche. Où voulez-vous que la notification parte, et qu'est-ce qui vous permettrait de le constater ?

### 26-troc-voisinage-ordre-des-arguments — Voisinage : arguments de estPeriodeValide inversés, visibles dans la signature du module importé

`src/emprunts/prolonger.ts` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0006 $

- projet `troc`, `src/emprunts/prolonger.ts`

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice (section) | panne : délai dépassé chez le fournisseur de modèles | — | — | non | non | 0,0000 $ | 30,0 s |
| Indice (voisinage) | affiché | 2/2 | 2/2 | oui | non | 0,0006 $ | 6,9 s |

**Indice (voisinage)** — affiché

Citation : `estPeriodeValide(emprunt.debut, nouvelleFin)`

> Comparez votre appel `estPeriodeValide(emprunt.debut, nouvelleFin)` à la signature déclarée dans src/emprunts/validation.ts. Le premier argument y porte un autre rôle que celui que vous lui donnez ici.

**Indice (voisinage)** — affiché

Citation : `const suite = { debut: emprunt.fin, fin: nouvelleFin };`

> Le document sur les dates indique que le jour de fin est inclus. Regardez ce que cela implique pour `debut` dans `suite = { debut: emprunt.fin, ... }` : quel jour est alors compté deux fois ?

### 30-budget-categories-sans-test — Classement par catégories : règles et priorités qu'aucun test ne vérifie

`` · titulaire `testeur` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion **non émergée** · coût 0,0019 $

- projet `budget`, première revue
- remarque `project` · cite docs/vision.md
- remarque `project` · cite src/import/mod.rs

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | non | non | 0,0019 $ | 7,1 s |

**Indice** — affiché

Citation : `docs/vision.md : Catégories attribuées par mots-clés du libellé`

> La carte ne montre des tests que dans le dossier tests/. Rien n'y vise src/categories/ ni src/config.rs, alors que le classement par mots-clés est au centre de la vision. Quel comportement de classement voudriez-vous verrouiller en premier ?

**Indice** — affiché

Citation : `src/import/mod.rs : pub fn est_complet(&self) -> bool`

> La vision demande qu'une ligne fautive n'empêche pas l'import des autres. Où est vérifié le cas d'une ligne mal formée au milieu d'un relevé par ailleurs valide ? Regardez du côté de l'état d'import partiel.

### 31-budget-voisinage-periode-inversee — Rapport du mois : arguments de Periode::new dans le mauvais ordre

`src/rapport/mensuel.rs` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0031 $

- projet `budget`, `src/rapport/mensuel.rs`

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice (section) | affiché | 1/1 | 1/1 | oui | non | 0,0025 $ | 13,1 s |
| Indice (voisinage) | affiché | 1/1 | 1/1 | oui | non | 0,0005 $ | 7,3 s |

**Indice (section)** — affiché

Citation : `Periode::new(mois, annee)?`

> Votre signature reçoit d'abord `annee`, puis `mois`. Vérifiez dans quel ordre `Periode::new` attend ses deux entiers : ce sont deux `u32`, donc le compilateur ne signalera rien si l'ordre diffère.

**Indice (voisinage)** — affiché

Citation : `Periode::new(mois, annee)?`

> À la ligne `let periode = Periode::new(mois, annee)?;`, comparez l'ordre de vos deux arguments avec celui attendu par la déclaration de `Periode::new` dans periode.rs. Que renvoie `contient` si les deux valeurs arrivent échangées ?

### 35-taches-test-sans-assertion — Des tests du planning qui ne vérifient rien

`` · titulaire `testeur` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0004 $

- projet `taches`, revue suivante (diff de 2)
- remarque `passage` · cite tests/test_planning.py

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | oui | non | 0,0004 $ | 3,6 s |

**Indice** — affiché

Citation : `tests/test_planning.py : assert isinstance(resultat, list)`

> Votre nouveau test dans tests/test_planning.py se termine par cette assertion. Regardez ce que cette ligne peut encore distinguer entre un planning qui respecte les jours libres et un planning qui les ignore : c'est là que se joue la valeur du test.

### 36-taches-voisinage-rappels-utc — Rappel du jour : date du jour locale comparée à une échéance que le dépôt renvoie en UTC

`taches/rappels.py` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0015 $

- projet `taches`, `taches/rappels.py`

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice (section) | affiché | 1/1 | 1/1 | non | non | 0,0002 $ | 6,2 s |
| Indice (voisinage) | affiché | 1/1 | 1/1 | oui | non | 0,0013 $ | 5,2 s |

**Indice (section)** — affiché

Citation : `tache.echeance.date()`

> Regardez ce que le modèle garantit sur le champ d'échéance : une tâche peut-elle exister sans échéance ? Si oui, que fait cette ligne dans ce cas de figure ?

**Indice (voisinage)** — affiché

Citation : `if tache.echeance.date() <= jour`

> Regardez d'où vient `jour` et d'où vient `tache.echeance` : deux horloges différentes se rencontrent dans cette ligne. Vérifiez le fuseau des dates renvoyées par le dépôt (ADR 0002) et l'endroit prévu pour la conversion locale.

