# Évaluation des mentors — 2026-10-30 10:34

22 cas, coût total 0,0837 $. Un modèle est retenu s'il ne propose aucune solution (motifs `must_not`, brouillons écartés compris) et fait émerger la notion (`must_mention`) dans 70 % des cas au moins.

## Verdict par modèle

| Niveau · modèle | Cas | Notion émergée | Solution (brouillons) | Solution (affichée) | JSON invalides | Pannes | Casting différent | Coût | Durée médiane | Durée p90 | Verdict |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| fast · deepseek/deepseek-v4.1-flash-1017 | 15 | 14/15 (93 %) | 0 | 0 | 0 | 0 | 0 | 0,0219 $ | 3,3 s | 6,5 s | **retenu** |
| strong · google/gemini-4-argon | 7 | 6/7 (86 %) | 0 | 0 | 0 | 0 | 0 | 0,0618 $ | 5,2 s | 9,3 s | **retenu** |

## Cas

### 01-dates-par-reference — Comparaison de dates par référence et chevauchement non détecté

`src/emprunts/reserver.ts` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0018 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | non | non | 0,0007 $ | 3,2 s |
| Question | affiché | — | 1/1 | oui | non | 0,0003 $ | 3,1 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0003 $ | 3,9 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0005 $ | 8,2 s |

**Indice** — affiché

Citation : `e.objetId === objet.id && e.debut === debut`

> Arrêtez-vous sur `e.debut === debut` : que compare exactement `===` lorsque les deux côtés sont des `Date` ? Deux dates au même instant peuvent-elles être considérées comme différentes par cette expression ? Vérifiez-le vous-même avec deux objets `Date` construits séparément.

**Indice** — affiché

Citation : `(e) => e.objetId === objet.id && e.debut === debut`

> Cette condition ne regarde qu'un seul instant. Que se passe-t-il si un emprunt existant commence avant `debut` et se termine après ? Et si `debut` tombe à l'intérieur d'un emprunt déjà enregistré ? Regardez les quatre champs `debut`/`fin` des deux périodes.

**Question** — affiché

> Votre recherche ne regarde que des débuts identiques. Que se passe-t-il si un emprunt existant commence avant `debut` et se termine après lui ? Posez la question autrement : à quelle condition deux intervalles [debut, fin] ne se chevauchent-ils pas ? Écrivez cette condition sur papier, puis vérifiez-la sur trois cas concrets dont un sans chevauchement.

**Piste** — affiché

> Les notions en jeu sont l'égalité de valeur sur les objets `Date` et le calcul de chevauchement entre deux intervalles fermés. Sur `Date`, la documentation à consulter est la page MDN de `Date.prototype.getTime` (et ses variantes de conversion en nombre) : elle explique comment ramener un instant à une valeur comparable. Pour le chevauchement, cherchez du côté de l'algèbre d'intervalles : elle décrit la condition de disjonction de deux segments avant de la nier pour obtenir l'intersection. Le TODO du contexte relève de la même famille de raisonnement : un invariant de validité à vérifier avant la recherche de conflit. Quel ordre logique entre ces vérifications garantit qu'aucune donnée incohérente n'est enregistrée ?

**Exemple** — affiché

> Exemple voisin — créneaux de salle, instants ramenés à un nombre de minutes :
> 
> ```ts
> function chevauche(a1: number, a2: number, b1: number, b2: number) {
>   return a1 <= b2 && b1 <= a2;
> }
> ```
> Trois choses à observer. [10,12] et [12,14] se touchent et sont déclarés en conflit : les bornes sont inclusives. Un créneau inversé [12,10] renvoie vrai contre presque tout : la validité de l'intervalle se contrôle avant, pas dans la formule. Deux créneaux au même instant mais rangés dans des objets distincts ne sont égaux que si vous comparez leur valeur numérique, jamais leur identité.

### 02-adr-sans-alternatives — ADR incohérent : hors ligne exigé, base hébergée choisie, aucune alternative

`docs/architecture/0001-stockage.md` · titulaire `architecte` · modèle `google/gemini-4-argon` · notion émergée · coût 0,0099 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0032 $ | 9,3 s |
| Question | affiché | — | 1/1 | non | non | 0,0031 $ | 8,9 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0036 $ | 4,3 s |

**Indice** — affiché

Citation : `Nous stockons toutes les données dans une base PostgreSQL hébergée, interrogée
par une API REST.`

> Relisez la dernière phrase de la section « Contexte » en regardant votre choix d'architecture.

**Question** — affiché

> Que se passe-t-il lorsqu'un voisin consulte le catalogue depuis son téléphone dans la cave, sans réseau, si toutes les données ne se trouvent que sur un serveur distant interrogé par une API REST ?

**Piste** — affiché

> Une API REST hébergée nécessite une connexion réseau active : seule, elle contredit la contrainte du mode hors ligne. Renseignez-vous sur les architectures « offline-first » ou « local-first », ainsi que sur le stockage côté client (cache local, base embarquée sur le téléphone) et les stratégies de synchronisation avec le serveur lorsque le réseau revient.

### 03-ts-foreach-async — forEach avec une fonction async : le compte est renvoyé avant les enregistrements

`src/import/importer.ts` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion **non émergée** · coût 0,0006 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0004 $ | 4,0 s |
| Question | rien | — | 1/1 | non | non | 0,0003 $ | 3,4 s |
| Piste | non atteint | — | — | non | non | 0,0000 $ | 0,0 s |
| Exemple | non atteint | — | — | non | non | 0,0000 $ | 0,0 s |

**Indice** — affiché

Citation : `  fiches.forEach(async (fiche) => {`

> Regardez la valeur de retour de `forEach` et l'instant où le rappel async se termine : comparez ce moment avec celui où `return enregistrees` s'exécute. Les deux ne sont pas ordonnés.

**Question** — écarté : confiance insuffisante

> Si `enregistrer` met du temps, quelle valeur `return` renvoie vraiment ? Et si l'une des promesses est rejetée, qui reçoit l'erreur, sachant que `forEach` ne l'attend pas ?

### 04-ts-find-undefined — find peut renvoyer undefined, masqué par une assertion non nulle

`src/panier/total.ts` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0010 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0001 $ | 3,5 s |
| Question | affiché | — | 1/1 | non | non | 0,0002 $ | 2,2 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0003 $ | 5,3 s |
| Exemple | rien | — | 1/1 | non | non | 0,0004 $ | 3,7 s |

**Indice** — affiché

Citation : `return produit!.prix;`

> Regardez le contrat de la ligne 2 et le sort réservé au résultat quand référence ne correspond à aucun produit. Demandez-vous ce que prixDe devrait renvoyer dans ce cas.

**Question** — affiché

> Que pouvez-vous affirmer du résultat de `find` avant de l'utiliser : est-il certain qu'un produit correspond toujours à `reference` ? Si vous appelez `prixDe` avec une référence absente du tableau, quelle valeur obtient `produit` et que devient `produit!.prix` à l'exécution ?

**Piste** — affiché

> Nommez la notion : votre signature annonce `number`, alors que la recherche produit une valeur possiblement absente. Le `!` efface cette information du type sans rien traiter à l'exécution. Deux voies à trancher dans le contrat : élargir le retour pour rendre l'absence visible à l'appelant, ou lever une erreur explicite quand la référence est inconnue. Pour la première, documentez-vous sur `strictNullChecks` et la façon dont TypeScript modélise une valeur absente (`undefined` dans une union) ; le manuel TypeScript, section sur l'opérateur d'assertion non-null, explique aussi pourquoi ce dernier ne vérifie rien. Choisissez la voie avant d'écrire la ligne suivante.

**Exemple** — écarté : confiance insuffisante

> Cas voisin, domaine différent :
> 
> ```ts
> type Reglage = { cle: string; valeur: number };
> 
> function lireReglage(reglages: Reglage[], cle: string) {
>   return reglages.find((r) => r.cle === cle)?.valeur; // number | undefined
> }
> 
> function lireReglageOuErreur(reglages: Reglage[], cle: string): number {
>   const r = reglages.find((x) => x.cle === cle);
>   if (!r) throw new Error(`réglage inconnu : ${cle}`);
>   return r.valeur;
> }
> ```
> 
> Les deux signatures annoncent deux contrats : la première force l'appelant à traiter l'absence, la seconde la convertit en erreur au point d'appel. Aucune ne laisse `undefined` filer dans un `number`.

### 05-ts-tri-sur-place — sort trie sur place le tableau reçu en paramètre

`src/agenda/tri.ts` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0016 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | non | non | 0,0004 $ | 2,5 s |
| Question | affiché | — | 1/1 | non | non | 0,0002 $ | 3,4 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0006 $ | 3,0 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0003 $ | 3,1 s |

**Indice** — affiché

Citation : `.sort((a, b) => a.debut.getTime() - b.debut.getTime())`

> Regardez l'effet de `sort` sur le tableau reçu en argument. Que devient le tableau de l'appelant après cet appel ?

**Indice** — affiché

Citation : `.slice(0, n)`

> Vérifiez le comportement de `slice` lorsque `n` vaut 0, une valeur négative, ou n'est pas un entier. Testez ces trois cas.

**Question** — affiché

> Si un appelant trie son agenda puis affiche la liste complète, obtient-il l'ordre qu'il avait avant l'appel ? Quelle garantie voulez-vous offrir dans le contrat de `prochainsCreneaux` : le tableau reçu reste-t-il intact, ou est-il modifiable ?

**Piste** — affiché

> La notion en jeu : mutabilité en place des méthodes de tableau. `sort` réordonne le tableau qu'on lui passe ; d'autres méthodes renvoient une copie. Pour choisir en connaissance de cause, comparez les signatures de `Array.prototype.sort` et `Array.prototype.toSorted` (MDN, rubrique Array) : voyez laquelle modifie le tableau d'origine, laquelle en crée un nouveau. Cela détermine ce que vous devez écrire pour que le tableau de l'appelant reste intact, et si `slice` suffit ou arrive trop tard. Question à vous poser : à quel moment de l'enchaînement la copie doit-elle avoir lieu ?

**Exemple** — affiché

> Cas voisin, autre méthode en place :
> 
> ```ts
> function troisPremiersRecus(notes: number[]): number[] {
>   return notes.reverse().slice(0, 3);
> }
> 
> const notes = [12, 5, 8, 1];
> const r = troisPremiersRecus(notes);
> // r  : [1, 8, 5]
> // notes : [1, 8, 5, 12]  <- l'appelant est modifié
> ```
> 
> Comparez avec une variante où la copie précède l'appel en place (`[...notes].reverse()`), puis une variante où la copie suit (`notes.reverse()` puis copie) : seule la position relative de la copie et de la méthode en place change le résultat visible chez l'appelant. Cherchez la même position relative dans votre enchaînement.

### 06-js-var-boucle — var dans une boucle : toutes les fonctions voient la même variable

`src/ui/boutons.js` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0010 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | oui | non | 0,0002 $ | 2,0 s |
| Question | affiché | — | 1/1 | non | non | 0,0003 $ | 3,3 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0002 $ | 1,4 s |
| Exemple | affiché | — | 1/1 | non | non | 0,0003 $ | 3,9 s |

**Indice** — affiché

Citation : `afficher("Bouton " + (i + 1));`

> Observez la variable `i` : dans quelle portée est-elle déclarée, et à quel instant la fonction de clic lit-elle sa valeur ? Comparez cet instant à celui où la boucle se termine.

**Question** — affiché

> Cliquez sur le premier bouton une fois la page chargée : quel numéro s'affiche, et quel numéro attendiez-vous ? Refaites l'essai sur le dernier bouton pour voir si le résultat change.

**Piste** — affiché

> La notion en jeu est la fermeture (closure) combinée à la portée de `var` : une fonction capture la variable elle-même, pas la valeur qu'elle contenait à cet instant. Deux pages à lire sur MDN : « Fermetures » et « Portée des variables et var/let/const ». La section sur `let` dans les boucles y décrit le comportement que vous attendez probablement, et la page « setTimeout dans une boucle » expose le même piège sous une autre forme.

**Exemple** — affiché

> Cas voisin, avec des fonctions rangées dans un tableau :
> 
> const t = [];
> for (var n = 0; n < 3; n++) t.push(() => console.log("var", n));
> for (let m = 0; m < 3; m++) t.push(() => console.log("let", m));
> t.forEach(f => f());
> 
> Les trois premières lignes affichées donnent la même valeur (la valeur finale), les trois suivantes donnent 0, 1, 2. La différence vient de l'endroit où la variable est créée, pas de la fonction.

### 07-js-tri-numerique — sort sans comparateur trie des nombres comme du texte

`src/stats/mediane.js` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0015 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | non | non | 0,0004 $ | 4,5 s |
| Question | affiché | — | 1/1 | non | non | 0,0005 $ | 3,0 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0004 $ | 3,6 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0002 $ | 2,6 s |

**Indice** — affiché

Citation : `const triees = [...valeurs].sort();`

> Regardez le comportement de `sort()` sans argument lorsque les éléments sont numériques : exécutez-la sur quelques valeurs de test dans la console et comparez l'ordre obtenu à l'ordre attendu.

**Indice** — affiché

Citation : `const milieu = Math.floor(triees.length / 2);`

> Passez à la fonction un tableau vide, puis un tableau d'un seul élément : vérifiez le résultat avant de considérer la fonction terminée.

**Question** — affiché

> Quels appels de test écririez-vous pour comparer `mediane()` à un résultat calculé à la main ? Choisissez quelques tableaux où l'ordre d'arrivée et l'ordre attendu diffèrent, puis notez ce que la fonction renvoie pour chacun.

**Piste** — affiché

> Notion en jeu : la coercition en chaîne. Le tri par défaut ne compare pas des nombres mais leurs représentations textuelles ; l'ordre obtenu dépend donc du nombre de chiffres, pas des valeurs. Pour vous documenter : MDN, Array.prototype.sort, section sur le paramètre de comparaison, et la page sur la conversion de type (ToString). Lisez-y ce qui se passe quand ce paramètre est omis, puis reliez cela à vos résultats de test.

**Exemple** — affiché

> Cas voisin, un tri par âge sur des objets :
> 
> const gens = [{age: 9}, {age: 100}, {age: 25}];
> gens.sort((a, b) => a.age - b.age);
> // [9, 25, 100]
> 
> const sans = [{age: 9}, {age: 100}, {age: 25}];
> sans.sort();
> // ordre inchangé ou dépendant du texte
> 
> Ce que ce palier ajoute : la forme générale d'un comparateur — deux paramètres, une valeur négative, nulle ou positive en retour. Repérez ce que chaque signe signifie pour la position relative des deux éléments, puis voyez si votre cas demande autre chose qu'une soustraction, par exemple une conversion préalable des éléments.

### 08-js-prix-flottants — Montants en euros flottants comparés avec ===

`src/caisse/reste.js` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0018 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | non | non | 0,0005 $ | 3,1 s |
| Question | affiché | — | 1/1 | oui | non | 0,0002 $ | 1,6 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0005 $ | 5,7 s |
| Exemple | affiché | — | 1/1 | non | non | 0,0006 $ | 6,5 s |

**Indice** — affiché

Citation : `return total - dejaVerse === 0 ? "payé" : "à payer";`

> À l'endroit de « total - dejaVerse === 0 », regardez de quel type sont les deux opérandes et comment les montants en euros sont représentés dans `lignes`.

**Indice** — affiché

Citation : `export function resteAPayer(lignes, dejaVerse)`

> Le nom annonce un reste à payer, la valeur de retour ne donne que deux libellés. Regardez ce qu'un appelant peut distinguer entre un total inférieur, égal ou supérieur à « dejaVerse ».

**Question** — affiché

> Dans la console, que renvoie 0.1 + 0.2 === 0.3 ? En quoi ce résultat change-t-il ce que peut valoir « total - dejaVerse » quand les prix ont des centimes ?

**Piste** — affiché

> Piste : la notion en jeu est le contrat de la fonction. Son nom annonce un reste, la valeur retournée est une étiquette, et seuls deux cas sont séparés alors qu'un reste peut être négatif, nul par arrondi, ou non nul. À documenter : la distinction entre une valeur calculée et l'état qu'on en dérive (MDN, gestion des erreurs et « Opérateurs de comparaison », section égalité stricte et valeurs numériques). Décidez ce que la fonction doit rendre, puis vérifiez chaque cas dans la console.

**Exemple** — affiché

> Exemple voisin, sans lien avec votre fonction :
> 
> function soldeCafe(cafes, offerts) {
>   let n = 0;
>   for (const c of cafes) n += c.quantite;
>   return n - offerts;
> }
> function etatCafe(cafes, offerts) {
>   const n = soldeCafe(cafes, offerts);
>   if (n > 0) return "à préparer";
>   if (n === 0) return "servi";
>   return "trop de cafés";
> }
> 
> Deux fonctions : l'une calcule, l'autre nomme l'état. Les trois cas de signe sont séparés, et le nom de la seconde diffère de ce qu'elle renvoie.
> 
> Une variante pour votre question, en 3 lignes : cherchez dans la console ce que vaut la comparaison d'un montant calculé avec 0 quand une remise décimale est appliquée, puis demandez-vous ce que la première fonction pourrait renvoyer à la place d'une étiquette.

### 09-py-defaut-mutable — Liste mutable comme valeur par défaut d'un paramètre

`inventaire/panier.py` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0012 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | oui | non | 0,0003 $ | 4,8 s |
| Question | affiché | — | 1/1 | non | non | 0,0004 $ | 2,3 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0002 $ | 1,7 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0002 $ | 2,5 s |

**Indice** — affiché

Citation : `def ajouter(article, panier=[]):`

> Regardez la valeur par défaut de `panier`. Appelez `ajouter` deux fois de suite sans second argument, puis comparez ce que renvoie chaque appel : demandez-vous d'où vient la liste du second retour.

**Question** — affiché

> Qu'attendez-vous d'un appel sans second argument quand aucun article n'a encore été ajouté ? Écrivez ce cas, exécutez-le deux fois, et regardez si les deux retours se comportent comme deux paniers indépendants ou comme un seul.

**Piste** — affiché

> La notion en jeu : les valeurs par défaut en Python sont évaluées une seule fois, au moment de la définition de la fonction, pas à chaque appel. L'objet `[]` écrit dans la signature est donc partagé entre tous les appels qui ne fournissent pas de second argument. À lire dans la FAQ Python (section « Programming FAQ ») : la question sur les arguments par défaut partagés entre appels, ainsi que la section du tutoriel sur les valeurs par défaut des arguments. Regardez aussi la notion de sentinelle pour représenter « aucun panier fourni ».

**Exemple** — affiché

> Cas voisin, autre valeur par défaut, même mécanisme :
> 
> import time
> 
> def journaliser(message, quand=time.time()):
>     print(quand, message)
> 
> journaliser("a")
> time.sleep(1)
> journaliser("b")
> 
> Les deux appels affichent le même horodatage. La valeur de `quand` est calculée une fois, à la définition de la fonction, pas à chaque appel. Comparez avec un troisième appel où vous fournissez `quand` explicitement : il change, lui. Transposez la même observation à votre paramètre : deux appels qui ne fournissent rien, puis un appel qui fournit un objet neuf.

### 10-py-liaison-tardive — Fermetures créées dans une boucle : liaison tardive de la variable

`rapports/filtres.py` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0018 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | oui | non | 0,0002 $ | 3,0 s |
| Question | affiché | — | 1/1 | non | non | 0,0002 $ | 2,9 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0007 $ | 2,9 s |
| Exemple | affiché | — | 1/1 | non | non | 0,0007 $ | 10,9 s |

**Indice** — affiché

Citation : `filtres.append(lambda note: note >= limite)`

> Observez quand `limite` est lue par le corps de chaque fonction rangée dans `filtres` : à la construction, ou au moment de l'appel ? Appelez `seuils([1, 2])`, puis passez la même note aux deux fonctions obtenues.

**Question** — affiché

> Si chaque fonction doit garder sa propre limite, quelle valeur faut-il lier au moment où la fonction est créée ? Écrivez un essai où la limite devient un paramètre de la fonction elle-même, puis appelez les deux fonctions : voient-elles encore la même valeur ?

**Piste** — affiché

> La notion en jeu porte un nom : la liaison tardive des variables libres dans une fermeture. Une variable lue depuis l'extérieur de la fonction n'est pas capturée à la création, elle est résolue à l'appel. Pour creuser, la FAQ Python traite ce point sous « Why do lambdas defined in a loop with different values all return the same result? », et la documentation sur les expressions lambda et les arguments par défaut complète la piste.

**Exemple** — affiché

> Cas voisin, autre contexte (des motifs, pas des notes) :
> 
> ```python
> def testeurs(motifs):
>     out = []
>     for motif in motifs:
>         out.append(lambda s: motif in s)
>         out.append(lambda s, m=motif: m in s)
>     return out
> 
> [g("abc") for g in testeurs(["ab", "cd"])]
> ```
> 
> Les quatre fonctions alternent deux façons de lier la variable. Appelez-les sur la même chaîne, puis comptez : un seul des deux groupes réagit encore à son motif d'origine.

### 11-py-liste-modifiee-en-parcours — Liste modifiée pendant qu'on la parcourt : des éléments sont sautés

`taches/nettoyage.py` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0022 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | oui | non | 0,0002 $ | 5,1 s |
| Question | affiché | — | 1/1 | non | non | 0,0005 $ | 2,8 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0004 $ | 5,3 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0011 $ | 3,5 s |

**Indice** — affiché

Citation : `for tache in taches:`

> Regardez la même liste `taches` : elle sert à la fois de source à `for tache in taches:` et de cible à `taches.remove(tache)`. Demandez-vous ce que l'itération survole quand la liste change de longueur en cours de route.

**Question** — affiché

> Prenez trois tâches terminées qui se suivent. Combien en reste-t-il après l'appel, et pourquoi ce nombre ? Vérifiez-le, puis demandez-vous quel parcours vous donnerait, à coup sûr, le résultat que vous attendez.

**Piste** — affiché

> La notion en jeu : mutation d'une séquence pendant son parcours. La documentation Python la traite explicitement, au tutoriel (section sur les listes, « More on Lists ») et dans la FAQ (« Programming FAQ », entrée sur la modification d'une liste pendant qu'on l'itère). Vous y trouverez les deux familles d'approches : produire une nouvelle liste à partir d'un critère, ou parcourir une copie. Lisez aussi la fiche de `list.remove` : elle précise ce qu'elle retire et ce qu'elle renvoie, ce qui compte pour l'appelant.

**Exemple** — affiché

> Cas voisin, autre domaine :
> 
> nombres = [4, -1, -2, 7, -3]
> for n in nombres:
>     if n < 0:
>         nombres.remove(n)
> print(nombres)   # [4, -2, 7]
> 
> Le -2 survit : après le retrait du -1, les éléments suivants se décalent d'un rang vers la gauche, et l'itération a déjà dépassé cette position. Un élément voisin d'un élément retiré n'est donc jamais examiné.
> 
> Comptez de la même façon, sur vos trois tâches terminées consécutives, combien d'éléments sont réellement visités par la boucle.

### 12-py-is-au-lieu-de-egal — is compare l'identité des objets, pas leur valeur

`comptes/verification.py` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0015 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0004 $ | 1,3 s |
| Question | affiché | — | 1/1 | non | non | 0,0004 $ | 2,1 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0002 $ | 5,1 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0005 $ | 4,1 s |

**Indice** — affiché

Citation : `return saisi.strip() is attendu`

> Regardez l'opérateur de la comparaison `saisi.strip() is attendu`. Testez avec deux chaînes distinctes mais de même contenu, par exemple une construite par concaténation. Que renvoie la fonction ?

**Question** — affiché

> Prenez la même valeur obtenue de deux façons : l'une écrite dans le code, l'autre lue depuis un fichier ou un `input()`. Le résultat de votre comparaison reste-t-il identique dans les deux cas ? Si non, qu'est-ce que la comparaison teste réellement : le contenu ou autre chose ?

**Piste** — affiché

> La notion en jeu : en Python, l'identité d'un objet et son égalité de valeur sont deux choses distinctes. Votre expression utilise l'opérateur qui interroge l'une ; pour une chaîne saisie par un utilisateur, c'est l'autre que vous voulez.
> 
> Pour vous documenter : le chapitre « Comparisons » du tutoriel officiel Python, et la fonction intégrée `id()` — comparez ce qu'elle renvoie pour deux chaînes de même contenu obtenues différemment. Voyez aussi ce que dit la doc sur les petites chaînes conservées en mémoire (

**Exemple** — affiché

> Un cas voisin, avec un autre type :
> 
> ```python
> a = [1, 2]
> b = list(a)
> 
> c = a
> 
> a == b   # True : même contenu
> a is b   # False : deux objets distincts
> c is a   # True : c'est le même objet
> 
> d = [1, 2]
> d is a   # False, alors que d == a est True
> ```
> 
> Ici `==` interroge le contenu, `is` l'objet lui-même. Un contenu qui vient de l'extérieur du code (lecture, saisie, concaténation) n'est pas l'objet écrit littéralement. Le résultat de `is` dépend de la provenance de la valeur, pas de ce qu'elle affiche.

### 13-rs-unwrap-saisie — unwrap sur une saisie utilisateur : panique au lieu d'une erreur

`src/commande.rs` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0011 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | non | non | 0,0005 $ | 2,8 s |
| Question | affiché | — | 1/1 | non | non | 0,0001 $ | 1,4 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0005 $ | 2,4 s |

**Indice** — affiché

Citation : `ligne.trim().parse().unwrap()`

> Arrêtez-vous sur `ligne.trim().parse().unwrap()`. Demandez-vous ce que cette expression produit quand la saisie contient autre chose qu'un nombre, et ce que devient le programme à ce moment-là.

**Indice** — affiché

Citation : `io::stdin().read_line(&mut ligne).unwrap();`

> Regardez la valeur renvoyée par `io::stdin().read_line(&mut ligne).unwrap();` : elle n'est pas lue. Que vaut-elle quand l'entrée standard est fermée, et que contient alors `ligne` ?

**Question** — affiché

> Si la saisie vaut « deux » ou « -3 », que devrait renvoyer votre fonction à l'appelant ? Votre signature actuelle, `-> u32`, permet-elle d'exprimer ce cas ?

**Piste** — affiché

> La notion en jeu est la propagation d'erreurs : `read_line` et `parse` renvoient tous deux un `Result`. Voyez le chapitre « Gestion des erreurs » du Rust Book (section `Result` et opérateur `?`), puis la doc de `str::parse` pour son type d'erreur. Cela vous aidera à décider ce que la fonction renvoie à l'appelant. Le parcours de l'entrée standard peut aussi se lire avec `io::Result`. 

### 14-rs-fenetre-glissante — Borne de boucle : la dernière fenêtre manque, et soustraction qui déborde

`src/stats.rs` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0015 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | oui | non | 0,0007 $ | 6,9 s |
| Question | affiché | — | 1/1 | non | non | 0,0001 $ | 1,3 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0002 $ | 4,9 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0005 $ | 2,9 s |

**Indice** — affiché

Citation : `for debut in 0..valeurs.len() - fenetre`

> Arrêtez-vous sur cette borne de boucle : son calcul et le type des opérandes. Faites le compte à la main avec 5 valeurs et une fenêtre de 2, puis avec une fenêtre plus grande que la tranche.

**Question** — affiché

> Avec 5 valeurs et une fenêtre de 2, comptez combien de moyennes complètes existent. Ce nombre est-il égal à valeurs.len() - fenetre ? Quelle expression donne le nombre de fenêtres réellement calculables, et que se passe-t-il si fenetre dépasse valeurs.len() ?

**Piste** — affiché

> Deux notions se croisent ici. D'abord les plages d'indices : `0..n` exclut `n`, et la documentation de `std::ops::Range` précise cette borne, ce qui décale le compte d'une fenêtre. Ensuite l'arithmétique sur `usize` : la soustraction est non signée et panique en cas de dépassement en mode debug, ce qui explique le cas d'une fenêtre plus grande que la tranche. Voyez `checked_sub` et `saturating_sub` sur les entiers. Côté std, regardez aussi les méthodes de tranche `windows` et `chunks` : elles évitent d'écrire soi-même ce découpage.

**Exemple** — affiché

> Cas voisin, des paires adjacentes plutôt que des fenêtres :
> 
> fn paires(v: &[i32]) -> Vec<i32> {
>     (0..v.len().saturating_sub(1))
>         .map(|i| v[i] + v[i + 1])
>         .collect()
> }
> 
> Sur [10,20,30] (len 3) : la plage vaut 0..2, donc deux indices 0 et 1, deux résultats. Le nombre de groupes de k éléments vaut len - k + 1 ; ici k = 2. La borne haute exclue de `..` décale ce compte d'une unité par rapport au calcul naïf. Avec une entrée de un seul élément, saturating_sub ramène la plage à 0..0, vide, sans dépassement. Transposez ce raisonnement à votre fenêtre.

### 15-rs-clone-pour-l-emprunteur — clone partout pour contourner l'emprunteur, et liste vide qui panique

`src/inventaire.rs` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0022 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | non | non | 0,0006 $ | 5,8 s |
| Question | affiché | — | 1/1 | oui | non | 0,0008 $ | 14,7 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0003 $ | 4,7 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0005 $ | 4,2 s |

**Indice** — affiché

Citation : `let mut meilleur = self.articles[0].clone();`

> Regardez l'accès indexé dans « let mut meilleur = self.articles[0].clone(); ». Demandez-vous ce que reçoit votre méthode quand l'inventaire est vide, et quel comportement vous voulez dans ce cas. Un test avec un inventaire vide tranche.

**Indice** — affiché

Citation : `if article.len() > meilleur.len() {`

> Regardez la comparaison « article.len() > meilleur.len() ». Que mesure exactement len() sur une String ? Comparez ce résultat avec ce qu'un utilisateur appellerait « le plus long » sur un texte contenant des caractères accentués.

**Question** — affiché

> Votre fonction prend `&self` et rend un `String` neuf. Que fait l'appelant de cette valeur : la garde-t-il longtemps, ou la lit-il seulement ? Si un emprunt suffisait, quel clone de votre boucle resterait nécessaire ? Testez avec un appel qui se contente d'afficher le résultat.

**Piste** — affiché

> Au palier piste, la notion en jeu est l'itération par emprunt : « for article in self.articles.clone() » duplique tout le vecteur avant de boucler, alors qu'un itérateur obtenu par emprunt parcourt les éléments sans copie. Voyez la rubrique « Iterators » du livre Rust et la page de std::iter pour la différence entre posséder et emprunter dans une boucle. Second point à documenter : la notion de maximum se calcule avec Iterator::max_by_key, qui rend une Option — ce qui relie votre boucle au cas vide déjà soulevé.

**Exemple** — affiché

> Cas voisin, autre domaine : chercher le plus ancien dans une liste empruntée.
> 
> ```rust
> fn aine<'a>(gens: &'a [(&str, u32)]) -> Option<&'a str> {
>     let mut best: Option<&'a (&str, u32)> = None;
>     for g in gens {
>         if best.map_or(true, |b| g.1 < b.1) {
>             best = Some(g);
>         }
>     }
>     best.map(|g| g.0)
> }
> ```
> 
> Remarquez l'accumulateur qui emprunte, et le type de retour qui porte l'absence de résultat. Écrivez un appel avec une liste vide, puis observez ce que reçoit votre fonction plus_long dans ce cas.

### 16-rs-iterateur-paresseux — map sans consommation : l'itérateur paresseux ne s'exécute jamais

`src/journal.rs` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0012 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0004 $ | 2,8 s |
| Question | affiché | — | 1/1 | non | non | 0,0004 $ | 1,8 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0003 $ | 4,3 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0002 $ | 6,5 s |

**Indice** — affiché

Citation : `lignes.iter().map(|ligne| archive.push(ligne.to_uppercase()));`

> Portez votre attention sur `lignes.iter().map(...)` : suivez ce que devient l'itérateur renvoyé par `map`. À quel moment le corps de la fermeture est-il exécuté, et ce moment arrive-t-il ici ?

**Question** — affiché

> Appelez `archiver` avec deux lignes dans un test, puis affichez `archive.len()`. Que lisez-vous ? Ce résultat vous dit à quel moment le contenu de la fermeture est réellement produit.

**Piste** — affiché

> La notion en jeu : la distinction entre un adaptateur d'itérateur et un consommateur d'itérateur, et la paresse (laziness) qui va avec. Documentez-vous sur la page de `std::iter::Iterator` : la section sur la paresse et la liste des méthodes terminales (`for_each`, `collect`, `count`, `sum`, `fold`). Repérez ensuite dans cette liste celle qui consomme l'itérateur et se nomme d'après la méthode `for` de la boucle. Regardez aussi les types attendus : un adaptateur renvoie un nouvel itérateur, un consommateur peut renvoyer `()` ou une valeur ; l'avertissement du compilateur sur un résultat non utilisé est un indice de ce qui vous échappe peut-être.

**Exemple** — affiché

> Cas voisin, hors de votre code :
> 
> ```rust
> let carres = (1..=4).map(|n| n * n); // rien ne s'exécute encore
> let somme: i32 = carres.sum();        // consommation ici
> println!("{somme}"); // 30
> 
> let mut total = 0;
> (1..=4).for_each(|n| total += n); // consommation, sans valeur de retour
> ```
> 
> Même structure : une chaîne d'adaptateurs inertes, puis une méthode terminale qui déclenche le travail. Repérez la méthode terminale dans votre propre chaîne.

### 17-adr-consequences-positives — ADR dont les conséquences ne listent aucun inconvénient

`docs/architecture/0002-connexion.md` · titulaire `architecte` · modèle `google/gemini-4-argon` · notion émergée · coût 0,0105 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0033 $ | 5,8 s |
| Question | affiché | — | 1/1 | non | non | 0,0033 $ | 3,1 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0039 $ | 5,4 s |

**Indice** — affiché

Citation : `- Plus de mots de passe à gérer.
- Le gardien n'a plus de réinitialisations à faire.`

> Regardez cette liste au regard de la décision prise (envoi d'un e-mail valable 15 minutes sur téléphone) : seuls les effets positifs y figurent pour l'instant.

**Question** — affiché

> Que se passe-t-il pour le voisin et pour le système si l'e-mail met plus de 15 minutes à arriver, tombe dans les indésirables, ou si le voisin n'a pas configuré sa boîte mail sur son téléphone ?

**Piste** — affiché

> Dans une décision d'architecture (ADR, format de Michael Nygard), la section « Conséquences » recense aussi les contreparties acceptées et les nouvelles dépendances introduites. Renseignez-vous sur les limites de l'authentification par « lien magique » (magic link) : délivrabilité des e-mails (SPF, DKIM, filtres anti-spam), dépendance à un fournisseur d'envoi, et parcours sur mobile lorsque le lien s'ouvre dans un autre navigateur que celui de départ.

### 18-adr-decision-disproportionnee — Décision disproportionnée au regard des contraintes du contexte

`docs/architecture/0003-notifications.md` · titulaire `architecte` · modèle `google/gemini-4-argon` · notion émergée · coût 0,0102 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0031 $ | 3,9 s |
| Question | affiché | — | 1/1 | oui | non | 0,0033 $ | 3,9 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0038 $ | 4,9 s |

**Indice** — affiché

Citation : `un cluster Kafka de trois nœuds`

> Relisez la première phrase de votre section « Contexte » concernant l'infrastructure disponible, puis comparez-la à ce déploiement.

**Question** — affiché

> Quelle quantité de mémoire vive et combien de machines physiques un cluster Kafka de trois nœuds exige-t-il au minimum, comparé aux ressources de l'unique Raspberry Pi mentionné dans le contexte ?

**Piste** — affiché

> La notion en jeu est le dimensionnement d'infrastructure (right-sizing) et l'alignement avec les contraintes matérielles. Consultez la documentation officielle d'Apache Kafka à la rubrique « Hardware and OS » (section « Memory ») pour vérifier l'empreinte mémoire minimale d'un seul nœud. Comparez ensuite ces prérequis ainsi que le besoin de trois machines distinctes avec la fiche technique d'un Raspberry Pi unique et un débit de cent messages par jour.

### 19-adr-nous-verrons — Cache sans stratégie d'invalidation, renvoyée à plus tard

`docs/architecture/0004-cache.md` · titulaire `architecte` · modèle `google/gemini-4-argon` · notion émergée · coût 0,0103 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | non | non | 0,0037 $ | 4,0 s |
| Question | affiché | — | 1/1 | oui | non | 0,0029 $ | 13,6 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0036 $ | 4,7 s |

**Indice** — affiché

Citation : `Nous verrons plus tard
s'il faut l'invalider.`

> Relisez cette phrase à la lumière de la deuxième phrase de votre section « Contexte ».

**Indice** — affiché

Citation : `Nous ajoutons un cache Redis devant l'API du catalogue.`

> Regardez comment ce choix spécifique se relie au problème énoncé dans le contexte, par rapport à d'autres pistes possibles.

**Question** — affiché

> Si un voisin retire un objet alors que le cache n'est pas invalidé, que verront les autres utilisateurs sur la page du catalogue ?

**Piste** — affiché

> Cette situation relève des stratégies d'invalidation de cache et du compromis entre fraîcheur des données (cohérence) et performance. Documentez-vous sur les approches courantes : expiration par durée de vie (TTL), invalidation sur événement (purge lors d'une écriture) ou écriture simultanée (write-through). Choisir l'une de ces approches fait partie intégrante de la décision d'architecture.

### 20-conception-sans-contraintes — Document de conception qui choisit une solution sans dire les contraintes

`docs/design/recherche.md` · titulaire `architecte` · modèle `google/gemini-4-argon` · notion émergée · coût 0,0099 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | oui | non | 0,0033 $ | 5,6 s |
| Question | affiché | — | 1/1 | oui | non | 0,0029 $ | 4,3 s |
| Piste | affiché | — | 1/1 | non | non | 0,0038 $ | 6,3 s |

**Indice** — affiché

Citation : `Un cluster Elasticsearch indexe le catalogue en temps réel.`

> Regardez la section « Objectif » et les hypothèses de volume implicites de ce choix.

**Indice** — affiché

Citation : `Chaque ajout
d'objet déclenche une réindexation`

> Observez les implications opérationnelles et les alternatives ou conséquences absentes du document.

**Question** — affiché

> Combien d'objets et d'utilisateurs simultanés attendez-vous, et pourquoi votre base de données actuelle ne suffit-elle pas pour chercher un objet par son nom ?

**Piste** — affiché

> C'est un enjeu de dimensionnement et de coût opérationnel (principe YAGNI). Dans un document de conception ou un ADR, on documente les ordres de grandeur attendus (nombre d'entrées, requêtes par seconde), les alternatives écartées (comme la recherche native de votre base de données principale) et les conséquences négatives du choix (synchronisation des données, maintenance du cluster). Renseignez-vous sur le format ADR de Michael Nygard (sections Context, Decision, Consequences) pour structurer cette justification.

### 21-vision-vide — Vision neuve : passe à l'ouverture sur un gabarit vide

`docs/vision.md` · titulaire `mentor-produit` · modèle `google/gemini-4-argon` · notion émergée · coût 0,0029 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Question | affiché | 1/1 | 1/1 | oui | non | 0,0029 $ | 9,3 s |

**Question** — affiché

Citation : `## Idée`

> Quel problème concret souhaitez-vous résoudre, et comment une personne utiliserait votre projet au quotidien ?

### 22-vision-trop-large — Vision remplie : public vague et première version trop large

`docs/vision.md` · titulaire `mentor-produit` · modèle `google/gemini-4-argon` · notion **non émergée** · coût 0,0081 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Question | affiché | 2/2 | 2/2 | non | non | 0,0044 $ | 5,3 s |
| Piste | affiché | — | 1/1 | non | non | 0,0037 $ | 5,2 s |

**Question** — écarté : citation trop courte ou trop longue

Citation : `Catalogue des objets, réservations avec calendrier, messagerie entre voisins,
notes et avis, notifications, application mobile et site web, paiement d'une
caution en ligne.`

> Avec deux soirs par semaine, combien de temps estimez-vous nécessaire pour construire l'ensemble de ces fonctionnalités ? Que faut-il garder au minimum pour qu'un premier prêt de perceuse ait lieu entre deux voisins ?

**Question** — affiché

Citation : `application mobile et site web, paiement d'une
caution en ligne.`

> Vos contraintes précisent que les voisins consultent souvent depuis la cave, sans réseau. Comment le paiement en ligne ou la messagerie se comportent-ils dans cette situation ?

**Piste** — affiché

> Cette situation touche à deux notions de gestion de produit : la conception « hors-ligne d'abord » (offline-first) et le découpage en produit minimum viable (MVP). Renseignez-vous sur le MVP et la méthode de priorisation MoSCoW pour distinguer ce qui est réalisable sans connexion (et en deux soirs par semaine) de ce qui nécessite un réseau actif ou un délai plus long.

