# Évaluation des mentors — 2026-10-30 08:41

22 cas, coût total 0,0851 $. Un modèle est retenu s'il ne propose aucune solution (motifs `must_not`, brouillons écartés compris) et fait émerger la notion (`must_mention`) dans 70 % des cas au moins.

## Verdict par modèle

| Niveau · modèle | Cas | Notion émergée | Solution (brouillons) | Solution (affichée) | JSON invalides | Pannes | Casting différent | Coût | Durée médiane | Durée p90 | Verdict |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| fast · deepseek/deepseek-v4.1-flash-1017 | 15 | 14/15 (93 %) | 1 | 0 | 0 | 0 | 0 | 0,0232 $ | 3,0 s | 5,8 s | **non retenu** |
| strong · google/gemini-4-argon | 7 | 5/7 (71 %) | 0 | 0 | 0 | 0 | 0 | 0,0619 $ | 6,4 s | 16,9 s | **retenu** |

## Cas

### 01-dates-par-reference — Comparaison de dates par référence et chevauchement non détecté

`src/emprunts/reserver.ts` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0022 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | oui | non | 0,0007 $ | 3,7 s |
| Question | affiché | — | 1/1 | non | non | 0,0002 $ | 3,5 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0006 $ | 2,6 s |
| Exemple | rien | — | 1/1 | non | non | 0,0008 $ | 4,0 s |

**Indice** — affiché

Citation : `e.debut === debut`

> Arrêtez-vous sur `e.debut === debut` : quel type ont ces deux valeurs, et que compare exactement `===` pour ce type ? Testez avec deux dates représentant le même instant mais construites séparément.

**Indice** — affiché

Citation : `const dejaPris = emprunts.find(`

> La recherche ne retient que les emprunts dont le début est identique. Regardez ce que cela suppose sur la position des périodes : deux réservations peuvent-elles se recouvrir sans commencer au même instant ?

**Question** — affiché

> Une fois que deux dates du même instant seront reconnues égales, ce test suffira-t-il ? Prenez un emprunt existant qui commence avant `debut` et se termine après : votre `find` le détecte-t-il ? Et un emprunt qui commence pendant votre période ?

**Piste** — affiché

> Les deux points précédents relèvent d'une même notion : le chevauchement d'intervalles. Deux périodes se recouvrent dès que l'une commence avant que l'autre ne finisse, dans les deux sens. Voyez la section « intersection » d'un cours d'algorithmique, ou la page MDN sur la comparaison de dates. Pour le TODO, la notion est la validation d'entrée : à quel moment contrôler l'invariant fin ≥ début, avant ou après la recherche de conflit ?

**Exemple** — écarté : garde « pas de solution » : exemple qui recopie un fragment de la section

> Cas voisin, sur des bornes entières :
> 
> ```ts
> type Plage = { debut: number; fin: number };
> 
> function seChevauchent(a: Plage, b: Plage): boolean {
>   return a.debut < b.fin && b.debut < a.fin;
> }
> 
> const a = { debut: 0, fin: 10 };
> console.log(seChevauchent(a, { debut: 5, fin: 15 }));  // true
> console.log(seChevauchent(a, { debut: 10, fin: 20 })); // false
> ```
> 
> Le contact en 10 ne compte pas comme conflit. L'invariant se vérifie sur les bornes, avant la recherche. Reste à transposer aux instants de vos dates.

### 02-adr-sans-alternatives — ADR incohérent : hors ligne exigé, base hébergée choisie, aucune alternative

`docs/architecture/0001-stockage.md` · titulaire `architecte` · modèle `google/gemini-4-argon` · notion émergée · coût 0,0099 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0030 $ | 14,3 s |
| Question | affiché | — | 1/1 | non | non | 0,0031 $ | 15,4 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0037 $ | 7,0 s |

**Indice** — affiché

Citation : `Nous stockons toutes les données dans une base PostgreSQL hébergée, interrogée
par une API REST.`

> Relisez la dernière phrase de la section « Contexte » en regardant ce passage.

**Question** — affiché

> Si toutes les données sont uniquement sur un serveur distant interrogé par une API REST, que se passe-t-il quand un voisin ouvre l'application à la cave sans réseau ?

**Piste** — affiché

> La contrainte du contexte relève d'une architecture dite « hors ligne d'abord » (*offline-first*) ou du stockage local sur le terminal client avec synchronisation. Renseignez-vous sur les stratégies de cache et de persistance côté client (par exemple via la documentation MDN sur le stockage côté client et les applications web hors ligne) pour préciser comment la décision répond à ce besoin ou pourquoi elle y renonce.

### 03-ts-foreach-async — forEach avec une fonction async : le compte est renvoyé avant les enregistrements

`src/import/importer.ts` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0010 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | non | non | 0,0004 $ | 4,4 s |
| Question | affiché | — | 1/1 | non | non | 0,0002 $ | 1,1 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0001 $ | 1,6 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0004 $ | 5,5 s |

**Indice** — affiché

Citation : `fiches.forEach(async (fiche) => {`

> Regardez la valeur que renvoie le callback de `forEach` et ce que `forEach` en fait : rien ne l'attend.

**Indice** — affiché

Citation : `return enregistrees;`

> Comparez l'instant où cette ligne s'exécute avec celui où les `await enregistrer(fiche)` se terminent réellement. Regardez aussi ce qui se passe si `enregistrer` rejette.

**Question** — affiché

> À l'instant où `return enregistrees` s'exécute, combien d'appels à `enregistrer` ont réellement terminé ? Vérifiez avec un `console.log` après l'appel de `importerTout` sur deux fiches : le nombre affiché correspond-il à ce que vous attendez ?

**Piste** — affiché

> La notion en jeu : attendre un ensemble de promesses. `forEach` n'en collecte aucune ; cherchez du côté de `Promise.all` et de la boucle `for...of` avec `await`. Documentation : MDN, pages « Promise.all » et « for await...of ». À lire avant de choisir comment accumuler le compteur. Bonne continuation.

**Exemple** — affiché

> Cas voisin, hors de votre code :
> 
> ```ts
> async function verifierToutes(urls: string[]): Promise<string[]> {
>   const etats: string[] = [];
>   for (const url of urls) {
>     etats.push(await verifier(url));
>   }
>   return etats;
> }
> 
> async function verifierEnParallele(urls: string[]): Promise<string[]> {
>   return Promise.all(urls.map((u) => verifier(u)));
> }
> ```
> 
> La première attend chaque appel l'un après l'autre ; la seconde laisse `Promise.all` attendre l'ensemble et rendre un tableau de résultats, dans l'ordre des entrées. Dans les deux cas, rien n'est lu avant la fin réelle des appels.

### 04-ts-find-undefined — find peut renvoyer undefined, masqué par une assertion non nulle

`src/panier/total.ts` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0014 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | non | non | 0,0008 $ | 4,6 s |
| Question | affiché | — | 1/1 | non | non | 0,0001 $ | 1,6 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0002 $ | 1,6 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0003 $ | 2,7 s |

**Indice** — affiché

Citation : `return produit!.prix;`

> Regardez ce que renvoie `find` lorsqu'aucun élément ne correspond, puis suivez ce que l'opérateur `!` fait de cette valeur avant l'accès à `.prix`.

**Indice** — affiché

Citation : `export function prixDe(produits: Produit[], reference: string): number {`

> La signature annonce `number`. Repérez dans votre code l'endroit où est décidé ce que reçoit l'appelant quand la référence demandée n'est pas dans la liste.

**Question** — affiché

> Si la référence demandée n'existe dans aucun produit, que devrait renvoyer `prixDe` pour rester utilisable par ses appelants ? Que renvoie-t-elle en réalité à l'exécution dans ce cas ?

**Piste** — affiché

> Le palier piste ajoute la notion : ici c'est la façon dont TypeScript modélise l'absence. La signature de `find` inclut explicitement l'absence de résultat, ce que l'assertion non-nulle efface du type sans rien vérifier au moment de l'exécution. Cette même question se pose au niveau de la signature de `prixDe` : son type de retour devra décider comment l'absence est représentée (union avec `undefined`, ou valeur de repli) et donc comment vos appelants en tiennent compte. À documenter dans le manuel TypeScript, aux sections sur `strictNullChecks`, le rétrécissement de type (narrowing) et la gestion des `undefined`.

**Exemple** — affiché

> Cas voisin, un annuaire :
> 
> ```ts
> function nomDe(id: number): string | undefined {
>   return annuaire.get(id)?.nom;
> }
> 
> const nom = nomDe(3);
> if (nom === undefined) {
>   console.log("absent");
> } else {
>   console.log(nom.toUpperCase());
> }
> ```
> 
> Deux choses à observer : le type de retour annonce l'absence, donc l'appelant est obligé de la traiter ; et `?.` propage l'absence au lieu de la masquer. La variante consiste à échouer volontairement côté producteur (`throw`) : c'est un choix de contrat, à trancher avant d'écrire le corps.

### 05-ts-tri-sur-place — sort trie sur place le tableau reçu en paramètre

`src/agenda/tri.ts` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0025 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | non | non | 0,0004 $ | 2,7 s |
| Question | affiché | — | 1/1 | non | non | 0,0005 $ | 2,4 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0008 $ | 1,7 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0008 $ | 3,0 s |

**Indice** — affiché

Citation : `.sort((a, b) => a.debut.getTime() - b.debut.getTime())`

> Regardez le contrat de Array.prototype.sort : que devient le tableau `creneaux` lui-même après l'appel ? Vérifiez l'état de l'argument reçu par l'appelant, avant et après.

**Indice** — affiché

Citation : `.slice(0, n)`

> Testez votre fonction avec n valant 0, puis un nombre négatif. Comparez le résultat attendu au résultat obtenu pour ces deux entrées.

**Question** — affiché

> Sur les bornes de `n` : que renvoie `slice(0, n)` pour `n = 0`, pour `n` supérieur au nombre de créneaux, et pour `n` négatif ? Ces trois réponses correspondent-elles à ce que votre contrat annonce à l'appelant ?

**Piste** — affiché

> Notion en jeu : la pureté d'une fonction de tri. Trier sur place a un effet de bord visible de l'appelant ; la bibliothèque standard expose une variante non mutante à consulter sur MDN (Array.prototype.toSorted), ainsi que la notion de copie superficielle de tableau. Point à vérifier ensuite : une copie du tableau ne duplique pas les objets Creneau, qui restent partagés avec l'appelant. Demandez-vous si votre contrat doit annoncer ce partage.

**Exemple** — affiché

> Exemple voisin, sur ce que protège une copie de tableau :
> 
> ```ts
> type Point = { x: number };
> const p: Point = { x: 1 };
> const liste = [p, { x: 2 }];
> const copie = [...liste];   // nouveau tableau
> copie[0].x = 99;            // liste[0].x vaut 99 aussi
> copie[1] = { x: 7 };        // liste[1] reste inchangé
> ```
> 
> La copie protège la structure du tableau, pas les objets qu'il contient. Reste à trancher dans votre contrat : un Creneau renvoyé peut-il être modifié par l'appelant sans effet de bord chez vous ?

### 06-js-var-boucle — var dans une boucle : toutes les fonctions voient la même variable

`src/ui/boutons.js` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0009 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | oui | non | 0,0002 $ | 1,5 s |
| Question | affiché | — | 1/1 | oui | non | 0,0001 $ | 2,7 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0002 $ | 2,6 s |
| Exemple | affiché | — | 1/1 | non | non | 0,0005 $ | 4,5 s |

**Indice** — affiché

Citation : `for (var i = 0; i < boutons.length; i++)`

> Regardez la variable utilisée dans la boucle et demandez-vous ce qu'elle vaut à l'instant où le clic se produit, comparé à l'instant où la fonction est créée. Combien de fonctions sont créées, et partagent-elles la même variable ? Testez avec plusieurs boutons.

**Question** — affiché

> À la fin de la boucle, quelle est la dernière valeur prise par la variable de comptage ? Combien de gestionnaires de clic ont capturé cette même variable ? Si vous cliquez sur le deuxième bouton, quel texte s'affiche-t-il, et est-ce le comportement attendu ?

**Piste** — affiché

> La notion en jeu est la portée des variables combinée aux fermetures : chaque gestionnaire capture une variable, pas une valeur. Documentez-vous sur MDN aux pages « var », « let » et « Fermetures » (section sur la capture dans les boucles). Comparez la portée d'une déclaration de fonction avec celle d'une déclaration de bloc, puis relisez votre boucle en vous demandant dans quelle portée vit votre compteur. Vérifiez ensuite avec deux boutons et un clic sur chacun.

**Exemple** — affiché

> Un cas voisin, avec des minuteurs au lieu de boutons :
> 
> for (var n = 0; n < 3; n++) {
>   setTimeout(function () {
>     console.log(n);
>   }, 1000);
> }
> 
> Exécutez-le dans la console. Notez ce qui s'affiche et à quel moment. Comptez combien de fonctions sont créées et combien de variables elles partagent. Recommencez avec un tableau de trois éléments au lieu du compteur : la forme du résultat reste la même que dans votre code.

### 07-js-tri-numerique — sort sans comparateur trie des nombres comme du texte

`src/stats/mediane.js` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0011 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | non | non | 0,0003 $ | 4,5 s |
| Question | affiché | — | 1/1 | oui | non | 0,0001 $ | 3,8 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0003 $ | 3,9 s |
| Exemple | affiché | — | 1/1 | non | non | 0,0004 $ | 3,8 s |

**Indice** — affiché

Citation : `const triees = [...valeurs].sort();`

> Testez votre fonction avec [10, 9, 2] et affichez triees avant de lire la suite : l'ordre obtenu ne correspond pas à l'ordre numérique attendu.

**Indice** — affiché

Citation : `return triees[milieu];`

> Déroulez à la main le cas où valeurs est vide : regardez ce que vaut milieu, puis ce que vaut triees[milieu] à cette position.

**Question** — affiché

> Après avoir vu l'ordre de `triees` sur [10, 9, 2], demandez-vous : sur quoi `sort()` compare-t-il deux éléments quand on ne lui passe rien ? Vérifiez avec [100, 20, 3] : l'ordre est-il celui des nombres ou celui des chaînes ?

**Piste** — affiché

> La notion en jeu est le comparateur : `sort` accepte une fonction de comparaison, et c'est le signe de sa valeur de retour (négatif, nul, positif) qui décide de la position relative de deux éléments. À documenter sur MDN, page `Array.prototype.sort`, section sur le paramètre `compareFn`. Lisez aussi le contrat exact de ce retour (nombre négatif, zéro, positif) : c'est lui qui fixe l'ordre, pas la valeur comparée elle-même.

**Exemple** — affiché

> Un cas voisin, sur d'autres données que les vôtres :
> 
> const mots = ['poire', 'abricot', 'kiwi'];
> mots.sort((a, b) => a.length - b.length);
> // longueurs croissantes
> 
> const fiches = [{ n: 3 }, { n: 1 }];
> fiches.sort((a, b) => b.n - a.n);
> // décroissant : c'est le signe du retour qui inverse l'ordre
> 
> fiches.sort(() => 0);
> // zéro : aucune paire n'est départagée, l'ordre d'origine est conservé
> 
> Observez que la fonction ne reçoit que deux éléments à la fois et ne renvoie jamais une position. Comparez ensuite avec le tri sans argument sur ces mêmes données : quel type de valeur est alors comparé ?

### 08-js-prix-flottants — Montants en euros flottants comparés avec ===

`src/caisse/reste.js` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0017 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | oui | non | 0,0008 $ | 3,2 s |
| Question | affiché | — | 1/1 | non | non | 0,0002 $ | 3,5 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0001 $ | 1,6 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0005 $ | 2,5 s |

**Indice** — affiché

Citation : `total - dejaVerse === 0`

> Arrêtez-vous sur `=== 0` : quelle valeur exacte cette différence peut-elle prendre quand les montants comportent des centimes ? Vérifiez vous-même le résultat de quelques additions décimales en javascript.

**Indice** — affiché

Citation : `? "payé" : "à payer"`

> Ce ternaire ne sépare que deux issues. Regardez ce que renvoie la fonction dans le cas où `dejaVerse` dépasse `total`, puis ce qu'attend l'appelant.

**Question** — affiché

> Votre fonction ne rend que deux textes. Que renvoie-t-elle si `dejaVerse` dépasse le total, ou s'il manque un centime ? Ces deux situations produisent-elles le même affichage, et est-ce ce que la caisse doit montrer ?

**Piste** — affiché

> Notion en jeu : l'arithmétique en virgule flottante et la comparaison de flottants. Deux pistes de documentation : la page MDN sur Number.EPSILON (comparaison à tolérance) et celle sur l'arrondi (Math.round, Intl.NumberFormat) pour raisonner en centimes entiers. Second point : votre fonction rend deux valeurs possibles. Repérez la structure qui permet plusieurs issues distinctes selon le signe de la différence, plutôt qu'un seul test d'égalité.

**Exemple** — affiché

> Cas voisin, hors de votre fichier : un compteur en unités entières et une comparaison à trois issues.
> 
> ```js
> function etatStock(stock, seuil) {
>   const ecart = stock - seuil;
>   if (ecart < 0) return "manquant";
>   if (ecart > 0) return "excedent";
>   return "pile";
> }
> ```
> 
> Et pour comparer deux flottants :
> 
> ```js
> const proche = (a, b) => Math.abs(a - b) < 1e-9;
> ```
> 
> Repérez dans ces deux extraits ce qui remplace votre test d'égalité, puis transcrivez l'idée à vos montants.

### 09-py-defaut-mutable — Liste mutable comme valeur par défaut d'un paramètre

`inventaire/panier.py` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0011 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | oui | non | 0,0002 $ | 1,5 s |
| Question | affiché | — | 1/1 | non | non | 0,0002 $ | 1,7 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0002 $ | 3,8 s |
| Exemple | affiché | — | 1/1 | non | non | 0,0005 $ | 3,7 s |

**Indice** — affiché

Citation : `panier=[]`

> Portez votre attention sur la portée de cette valeur par défaut : elle est créée une seule fois, au moment de la définition de la fonction. Que contient le panier renvoyé au deuxième appel effectué sans argument ?

**Question** — affiché

> Si deux voisins distincts appellent ajouter sans fournir de panier, leurs emprunts doivent-ils se retrouver dans la même liste ? Quelle expérience, lancez-la deux fois sans argument, vous le montrerait ? Et que faudrait-il pour qu'un appelant reparte d'une liste vide ?

**Piste** — affiché

> La notion en jeu s'appelle « argument par défaut mutable ». Elle est documentée dans la FAQ officielle de Python, section « Pourquoi les arguments par défaut mutables posent-ils problème ? », et dans le tutoriel au paragraphe sur les valeurs par défaut. Vous y trouverez le motif courant : une valeur par défaut non mutable qui sert de repère, puis la construction de la liste à l'intérieur de la fonction. Lisez aussi la notion de valeur sentinelle (None) et demandez-vous ce qui distingue « l'appelant n'a rien fourni » de « l'appelant a fourni une liste vide ».

**Exemple** — affiché

> Cas voisin, sur un autre type de conteneur (dict) :
> 
> ```python
> def noter(note, carnet={}):
>     carnet[len(carnet)] = note
>     return carnet
> 
> print(noter("a"))  # {0: 'a'}
> print(noter("b"))  # {0: 'a', 1: 'b'} : carnet partagé
> 
> def noter2(note, carnet=None):
>     if carnet is None:
>         carnet = {}
>     carnet[len(carnet)] = note
>     return carnet
> ```
> 
> Comparez la durée de vie des deux valeurs par défaut. Repérez que noter2 distingue encore « rien fourni » de « conteneur vide fourni ».

### 10-py-liaison-tardive — Fermetures créées dans une boucle : liaison tardive de la variable

`rapports/filtres.py` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0015 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0002 $ | 2,1 s |
| Question | affiché | — | 1/1 | non | non | 0,0004 $ | 4,1 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0003 $ | 5,8 s |
| Exemple | affiché | — | 1/1 | non | non | 0,0007 $ | 2,6 s |

**Indice** — affiché

Citation : `filtres.append(lambda note: note >= limite)`

> Portez votre attention sur `limite` dans la lambda : à quel moment cette valeur est-elle lue, par rapport à la fin de la boucle ? Appelez les trois filtres de `seuils([1, 2, 3])` et comparez leurs résultats.

**Question** — affiché

> Parmi les noms que la lambda lit, lesquels sont résolus à l'appel et non à sa création ? Vérifiez en affichant le résultat des trois filtres après la boucle, puis en comparant avec un filtre construit hors boucle pour une seule limite.

**Piste** — affiché

> Pour nommer ce que le fil précédent faisait remarquer : c'est la « fermeture tardive » (late binding) des fermetures Python. La notion à réviser est la portée lexicale, les règles LEGB et la capture des variables libres. Côté documentation : la FAQ Python pose exactement ce cas (questions sur les lambdas définies dans une boucle), le tutoriel sur les portées et les mots-clés global/nonlocal, et la page functools.partial pour lier une valeur à la construction. Un test utile pour avancer : à quel moment chaque valeur de limite devient-elle fixe, à la création ou à l'appel ?

**Exemple** — affiché

> Cas voisin, hors de votre code :
> 
> ```python
> compteur = 0
> 
> def lire():
>     return compteur
> 
> compteur = 10
> print(lire())
> compteur = 20
> print(lire())
> ```
> 
> Les deux appels portent sur la même fonction, pourtant la valeur rendue change. Le nom n'est pas figé à la définition : il est relu à chaque appel, dans l'état où se trouve la variable à cet instant. Votre lambda se comporte de la même façon avec `limite`, sauf qu'elle est créée plusieurs fois pendant que la boucle fait varier ce nom.

### 11-py-liste-modifiee-en-parcours — Liste modifiée pendant qu'on la parcourt : des éléments sont sautés

`taches/nettoyage.py` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0017 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | oui | non | 0,0001 $ | 1,0 s |
| Question | affiché | — | 1/1 | non | non | 0,0004 $ | 2,5 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0004 $ | 5,1 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0008 $ | 9,8 s |

**Indice** — affiché

Citation : `for tache in taches:
        if tache["terminee"]:
            taches.remove(tache)`

> Regardez ce qui se passe quand l'itération et la modification de `taches` ont lieu dans la même boucle. Testez avec deux tâches terminées consécutives et comparez la longueur obtenue à celle attendue.

**Question** — affiché

> Après `taches.remove(tache)`, à quelle position se retrouve la tâche qui suivait dans la liste, et quelle tâche la boucle examine-t-elle au tour suivant ?

**Piste** — affiché

> La notion en jeu est la mutation d'une séquence pendant son itération : la documentation Python la signale dans le tutoriel (section sur les listes, types de séquences mutables) et dans la FAQ. Deux approches s'opposent ici, et le choix conditionne tout le reste : filtrer pour produire une nouvelle liste (compréhension avec condition) ou modifier la liste reçue en place. Documentez-vous sur la compréhension de liste et sur `filter`, puis décidez ce que le nom `retirer_terminees` promet à l'appelant : renvoyer une liste filtrée ou changer celle qu'on lui passe. Tant que ce contrat n'est pas tranché, les tests de longueur resteront ambigus.

**Exemple** — affiché

> Un cas voisin, sur une autre liste et un autre critère :
> 
> ```python
> nombres = [1, 3, 4, 5]
> for n in nombres:
>     if n % 2 == 1:
>         nombres.remove(n)
> print(nombres)   # [3, 4] : le 3 n'a jamais ete examine
> ```
> 
> La trace affiche les elements visités : après un retrait, la position courante pointe déjà sur l'élément suivant, que la boucle ne revoit pas. Notez aussi que rien ne s'affiche comme une erreur : un conteneur comme un dict, lui, lève RuntimeError quand sa taille change pendant le parcours. Le silence n'est donc pas une preuve de correction ; pour vérifier, comparez la sortie à la liste attendue écrite à la main.

### 12-py-is-au-lieu-de-egal — is compare l'identité des objets, pas leur valeur

`comptes/verification.py` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0010 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0004 $ | 0,8 s |
| Question | affiché | — | 1/1 | non | non | 0,0002 $ | 1,3 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0002 $ | 2,5 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0002 $ | 5,9 s |

**Indice** — affiché

Citation : `return saisi.strip() is attendu`

> Regardez l'opérateur employé dans `saisi.strip() is attendu`. Il ne compare pas le contenu des deux chaînes. Demandez-vous ce que `is` teste réellement sur deux chaînes construites séparément, puis ce que vous voulez vraiment comparer ici.

**Question** — affiché

> Quel test en une ligne, dans l'interpréteur, vous montrerait si deux chaînes de même contenu mais construites séparément sont vues comme identiques ? Et ce test, que vous dirait-il sur la façon dont votre fonction doit décider ?

**Piste** — affiché

> La notion en jeu : identité d'objet contre égalité de valeur. En Python, deux noms peuvent désigner le même objet ou deux objets distincts de même contenu ; l'identité est stable pour un objet donné, l'égalité dépend du type et peut être redéfinie par l'objet lui-même. Où se documenter : la page « Comparisons » du tutoriel Python et l'entrée des opérateurs de comparaison dans la référence du langage ; le modèle de données indique aussi comment un type décide de son égalité. Ce que ce palier ajoute : cherchez quel opérateur correspond à l'intention de comparer un contenu saisi par un humain à une valeur de référence.

**Exemple** — affiché

> Cas voisin, avec des chaînes construites de deux façons :
> 
> a = "".join(["ab", "c"])
> b = "abc"
> print(a is b)   # False : deux objets distincts
> print(a == b)   # True  : même contenu
> print(a is a)   # True  : même objet
> 
> Le résultat de `is` dépend de la façon dont chaque objet a été produit, pas de son contenu. Avec `==`, le type de l'objet décide de ce que « même contenu » signifie. Transposez ce test au couple formé par votre saisie normalisée et la valeur de référence, puis observez ce que renvoie chaque opérateur avant d'écrire la ligne finale.

### 13-rs-unwrap-saisie — unwrap sur une saisie utilisateur : panique au lieu d'une erreur

`src/commande.rs` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0015 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | non | non | 0,0005 $ | 2,8 s |
| Question | affiché | — | 1/1 | non | non | 0,0002 $ | 5,1 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0003 $ | 6,9 s |
| Exemple | rien | — | 1/1 | non | **oui** : `-> Result<u32` | 0,0004 $ | 5,8 s |

**Indice** — affiché

Citation : `io::stdin().read_line(&mut ligne).unwrap();`

> Regardez ce que renvoie `read_line` quand le flux se termine sans aucun caractère (entrée fermée, Ctrl-D), et l'état de `ligne` à cet instant. Le `unwrap()` ici ne dit rien de ce cas.

**Indice** — affiché

Citation : `ligne.trim().parse().unwrap()`

> Regardez la valeur de retour de `parse` pour un texte qui n'est pas un entier, ou un entier négatif ou trop grand pour `u32`. Que devient le programme dans ces cas ?

**Question** — affiché

> Si l'utilisateur tape autre chose qu'un nombre, quelle valeur votre fonction renvoie-t-elle aujourd'hui, et l'appelant peut-il distinguer ce cas d'une saisie valide ? Quel contrat voulez-vous donner à lire_quantite ?

**Piste** — affiché

> Deux notions se croisent ici. D'abord la lecture : `read_line` renvoie un `Result<usize, io::Error>` ; la variante `Ok(0)` y signale une fin de flux, et `ErrorKind::UnexpectedEof` en est le pendant documenté dans `std::io::ErrorKind`. Ensuite la conversion : `parse` renvoie un `Result`, dont le type d'erreur dépend de la cible choisie. Le point de décision se situe dans la signature de votre fonction : un `u32` seul ne peut pas transporter ces deux cas, alors qu'un `Result` le peut. Voyez le chapitre « Recoverable Errors with Result » du Rust Book, puis la doc de `std::io::BufRead::read_line` et de `std::str::FromStr`. Pensez aussi aux appelants : changer le type de retour les oblige à traiter les cas.

**Exemple** — écarté : garde « pas de solution » : exemple qui reprend le code de la section

> Un voisin, hors de votre domaine (lecture générique sur un flux, taille d'enregistrement) :
> 
> enum Erreur { FluxVide, Format }
> 
> fn lire_taille(flux: &mut impl BufRead) -> Result<u32, Erreur> {
>     let mut tampon = String::new();
>     let lus = flux.read_line(&mut tampon).map_err(|_| Erreur::FluxVide)?;
>     if lus == 0 {
>         return Err(Erreur::FluxVide);
>     }
>     tampon.trim().parse().map_err(|_| Erreur::Format)
> }
> 
> Deux points à comparer avec votre section : le nombre d'octets lus est testé avant toute conversion, et le type de retour porte les deux cas jusqu'à l'appelant. Placez ces deux questions dans votre propre signature.

### 14-rs-fenetre-glissante — Borne de boucle : la dernière fenêtre manque, et soustraction qui déborde

`src/stats.rs` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0021 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | oui | non | 0,0007 $ | 7,8 s |
| Question | affiché | — | 1/1 | oui | non | 0,0001 $ | 1,3 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0009 $ | 3,3 s |
| Exemple | rien | — | 1/1 | non | non | 0,0005 $ | 6,4 s |

**Indice** — affiché

Citation : `for debut in 0..valeurs.len() - fenetre`

> Comptez les fenêtres qu'une série de 5 valeurs avec fenêtre 3 devrait produire, puis celles que cette borne produit réellement. Vérifiez aussi ce que devient la soustraction quand fenetre atteint ou dépasse valeurs.len().

**Indice** — affiché

Citation : `somme / fenetre as f64`

> Que vaut ce quotient si fenetre vaut 0 ? Et la somme d'une tranche vide ? Écrivez deux petits cas de test pour le voir.

**Question** — affiché

> Vous avez vérifié la borne haute. Maintenant : pour une série de longueur égale à la fenêtre, combien de fenêtres complètes existe-t-il, et que renvoie la fonction dans ce cas ? Et si la fenêtre vaut 0, que valent la borne et la division ?

**Piste** — affiché

> Piste : deux notions se croisent ici. D'abord le dépassement d'entier : sur `usize`, une soustraction qui passe sous zéro panique en debug et boucle en release — voyez la section du livre Rust sur les opérateurs arithmétiques et le dépassement. Ensuite la notion de fenêtre glissante, que la bibliothèque standard fournit déjà : voyez `slice::windows` dans la documentation de `slice`, avec le nombre d'itérations qu'elle annonce. Comparer ce nombre au vôtre éclaire la borne.

**Exemple** — écarté : garde « pas de solution » : exemple qui reprend le code de la section

> Cas voisin, autre fonction (maximum sur fenêtres), pour observer la forme :
> 
> fn maxi_glissant(valeurs: &[i32], n: usize) -> Vec<i32> {
>     if n == 0 || n > valeurs.len() {
>         return Vec::new();
>     }
>     valeurs
>         .windows(n)
>         .map(|f| *f.iter().max().unwrap())
>         .collect()
> }
> 
> Deux points à transposer : le garde est posé avant tout calcul, et `windows` annonce lui-même combien d'éléments il produit. Comparez ce nombre à celui de votre boucle.

### 15-rs-clone-pour-l-emprunteur — clone partout pour contourner l'emprunteur, et liste vide qui panique

`src/inventaire.rs` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion **non émergée** · coût 0,0016 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | non | non | 0,0004 $ | 3,9 s |
| Question | affiché | — | 1/1 | non | non | 0,0001 $ | 2,3 s |
| Piste | affiché | — | 1/1 | non | non | 0,0004 $ | 1,9 s |
| Exemple | affiché | — | 1/1 | non | non | 0,0007 $ | 3,9 s |

**Indice** — affiché

Citation : `let mut meilleur = self.articles[0].clone();`

> Regardez ce que produit cette ligne quand l'inventaire ne contient aucun article : quel chemin d'exécution est possible, et est-ce cohérent avec un retour en String ?

**Indice** — affiché

Citation : `if article.len() > meilleur.len()`

> Vérifiez la définition de String::len dans la documentation : que mesure-t-elle exactement, et est-ce la grandeur que vous souhaitez comparer ici ?

**Question** — affiché

> Sur un inventaire vide, que devrait renvoyer `plus_long` pour rester utile à l'appelant ? En regardant la signature, quel choix vous semble cohérent avec ce que le type de retour autorise ?

**Piste** — affiché

> Notion en jeu : l'accès par indice sur un Vec comparé à un accès qui signale l'absence. Documentez-vous sur `std::vec::Vec`, section Index et méthode `get`, puis sur `Option` dans `std::option` : les deux premières rubriques de chaque page suffisent. Vous y verrez que l'indice suppose une longueur non nulle, alors que l'autre forme rend l'absence explicite. Une fois ce vocabulaire en main, confrontez-le à votre signature : elle décide à votre place de ce qu'un inventaire vide peut produire.

**Exemple** — affiché

> Un cas voisin, sur des entiers et un slice, où la fonction ne suppose jamais une longueur non nulle :
> 
> fn plus_cher(prix: &[u32]) -> Option<u32> {
>     let mut top = *prix.first()?;
>     for p in prix {
>         if *p > top {
>             top = *p;
>         }
>     }
>     Some(top)
> }
> 
> `first` rend un `Option` : le `?` transforme l'absence en sortie `None`, sans passer par un indice. L'appelant lit ensuite `None` comme « rien à comparer », au lieu de subir un arrêt. Regardez ce que cela change pour votre signature.

### 16-rs-iterateur-paresseux — map sans consommation : l'itérateur paresseux ne s'exécute jamais

`src/journal.rs` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0017 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | oui | non | 0,0011 $ | 8,8 s |
| Question | affiché | — | 1/1 | non | non | 0,0002 $ | 5,8 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0002 $ | 1,5 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0002 $ | 2,9 s |

**Indice** — affiché

Citation : `lignes.iter().map(|ligne| archive.push(ligne.to_uppercase()))`

> Les adaptateurs d'itérateur de ce genre sont paresseux : à quel moment le corps de la closure est-il réellement exécuté, et qu'est-ce qui déclenche ce parcours ?

**Indice** — affiché

Citation : `map(|ligne| archive.push(ligne.to_uppercase()))`

> Regardez aussi les avertissements du compilateur sur cette ligne : ils indiquent quelque chose sur la valeur produite par l'expression.

**Question** — affiché

> Écrivez un appelant qui passe deux lignes puis affiche la longueur du Vec : combien d'éléments compte-t-il après l'appel, et ce résultat correspond-il à ce que le nom de la fonction annonce ?

**Piste** — affiché

> Deux notions à documenter : la distinction entre adaptateurs et consommateurs d'itérateur (page « Iterator » de la documentation standard, section sur les méthodes qui consomment l'itérateur), et l'attribut `must_use` qui accompagne ces adaptateurs. Vérifiez aussi ce que le compilateur signale pour cette expression : ce message pointe vers la même notion.

**Exemple** — affiché

> Cas voisin, sans votre code :
> 
> ```rust
> let mut vus: Vec<i32> = Vec::new();
> let n = (1..4).map(|x| x * 2).count(); // n == 3, vus est vide
> let doubles: Vec<i32> = (1..4).map(|x| x * 2).collect();
> // doubles == [2, 4, 6]
> ```
> 
> `map` ne calcule rien ; ici `count` et `collect` sont les consommateurs qui déclenchent le parcours, chacun avec un effet différent. Repérez, dans votre fonction, quel élément joue le rôle de consommateur, et lequel manque pour que les valeurs produites aient une destination.

### 17-adr-consequences-positives — ADR dont les conséquences ne listent aucun inconvénient

`docs/architecture/0002-connexion.md` · titulaire `architecte` · modèle `google/gemini-4-argon` · notion émergée · coût 0,0106 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0031 $ | 5,0 s |
| Question | affiché | — | 1/1 | non | non | 0,0032 $ | 3,9 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0044 $ | 16,2 s |

**Indice** — affiché

Citation : `## Conséquences`

> Regardez la nature des points listés dans cette section par rapport à la dépendance introduite dans la section Décision (« envoyé par e-mail »).

**Question** — affiché

> Vos conséquences listent uniquement les avantages. Que se passe-t-il pour un voisin si le courriel met plus de 15 minutes à arriver ou tombe dans les indésirables, et dans quel cas devriez-vous revenir sur cette décision ?

**Piste** — affiché

> Dans une décision d'architecture (format de Michael Nygard), la section « Conséquences » recense l'ensemble des effets : positifs, négatifs et neutres. Ici, la notion clé est la « délivrabilité des courriels » (filtres anti-spam, délais de remise, dépendance à un fournisseur SMTP tiers) ainsi que le parcours sur mobile (changer d'application entre le navigateur et la boîte de réception). Vous pouvez vous documenter sur le modèle d'ADR de Nygard et sur les compromis d'ergonomie et de sécurité des liens magiques (*magic links*). Précisez aussi le seuil (par exemple un taux d'échec ou de nouvelles sollicitations du gardien) qui invaliderait ce choix.

### 18-adr-decision-disproportionnee — Décision disproportionnée au regard des contraintes du contexte

`docs/architecture/0003-notifications.md` · titulaire `architecte` · modèle `google/gemini-4-argon` · notion émergée · coût 0,0095 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0030 $ | 4,7 s |
| Question | affiché | — | 1/1 | oui | non | 0,0030 $ | 7,1 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0035 $ | 16,2 s |

**Indice** — affiché

Citation : `un cluster Kafka de trois nœuds`

> Relisez la première phrase de la section Contexte en regardant les ressources matérielles décrites ici.

**Question** — affiché

> De combien de mémoire vive et de machines disposez-vous sur ce Raspberry Pi pour héberger ces trois nœuds Kafka, et que nécessite réellement le volume de cent notifications par jour ?

**Piste** — affiché

> Cette situation relève du dimensionnement (sizing) et de l'adéquation aux contraintes matérielles (les « Architecture Drivers » d'un ADR). Consultez la documentation officielle d'Apache Kafka, section « Hardware and OS » (notamment la mémoire RAM recommandée par nœud JVM), et comparez ces prérequis aux capacités d'un unique Raspberry Pi.

### 19-adr-nous-verrons — Cache sans stratégie d'invalidation, renvoyée à plus tard

`docs/architecture/0004-cache.md` · titulaire `architecte` · modèle `google/gemini-4-argon` · notion **non émergée** · coût 0,0100 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/2 | 2/2 | non | non | 0,0036 $ | 5,8 s |
| Question | affiché | — | 1/1 | non | non | 0,0030 $ | 6,4 s |
| Piste | affiché | — | 1/1 | non | non | 0,0035 $ | 4,4 s |

**Indice** — écarté : citation introuvable dans la section

Citation : `Nous verrons plus tard s'il faut l'invalider.`

> Relisez la seconde phrase de la section Contexte en regard de ce passage.

**Indice** — affiché

Citation : `Nous ajoutons un cache Redis devant l'API du catalogue.`

> Regardez si d'autres pistes que ce choix précis apparaissent dans votre document.

**Question** — affiché

> Quelles autres façons de réduire ces deux secondes d'affichage avez-vous écartées avant de retenir Redis, et pour quelles raisons ?

**Piste** — affiché

> Dans une décision d'architecture (ADR), on attend une trace des « options envisagées » (ou alternatives écartées). Renseignez-vous sur le format MADR (Markdown Architectural Decision Records) : il montre comment lister les autres pistes (optimiser la requête, paginer, mettre en cache ailleurs) et expliquer pourquoi elles ont été rejetées au profit de votre choix.

### 20-conception-sans-contraintes — Document de conception qui choisit une solution sans dire les contraintes

`docs/design/recherche.md` · titulaire `architecte` · modèle `google/gemini-4-argon` · notion émergée · coût 0,0108 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | oui | non | 0,0037 $ | 8,4 s |
| Question | affiché | — | 1/1 | oui | non | 0,0033 $ | 20,3 s |
| Piste | affiché | — | 1/1 | non | non | 0,0038 $ | 5,5 s |

**Indice** — affiché

Citation : `Un cluster Elasticsearch indexe le catalogue en temps réel.`

> Mettez en regard ce choix technique avec la section « Objectif » et les hypothèses de volume (nombre d'objets, de voisins).

**Indice** — affiché

Citation : `## Solution`

> Dans un document de conception, regardez la place réservée aux alternatives envisagées et aux conséquences (coût opérationnel, synchronisation) du choix retenu.

**Question** — affiché

> Combien d'objets et d'utilisateurs prévoyez-vous ? Quelles options plus simples avez-vous écartées pour chercher un objet par son nom, et quels coûts d'exploitation ce cluster ajoute-t-il ?

**Piste** — affiché

> Cette étape relève du dimensionnement (sizing) et de l'analyse des compromis dans un ADR (Architecture Decision Record). Documentez-vous sur le format ADR de Michael Nygard (sections Contexte, Décision, Conséquences) et sur le principe YAGNI. Pour justifier un moteur d'indexation dédié, le document doit chiffrer la volumétrie et la latence attendues, lister les alternatives écartées (comme une requête sur la base existante) et exposer les coûts de synchronisation et d'exploitation.

### 21-vision-vide — Vision neuve : passe à l'ouverture sur un gabarit vide

`docs/vision.md` · titulaire `mentor-produit` · modèle `google/gemini-4-argon` · notion émergée · coût 0,0028 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Question | affiché | 1/1 | 1/1 | oui | non | 0,0028 $ | 4,1 s |

**Question** — affiché

Citation : `## Idée`

> Quel problème précis souhaitez-vous résoudre avec ce projet, et que permettra-t-il de faire concrètement ?

### 22-vision-trop-large — Vision remplie : public vague et première version trop large

`docs/vision.md` · titulaire `mentor-produit` · modèle `google/gemini-4-argon` · notion **non émergée** · coût 0,0082 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Question | affiché | 2/2 | 2/2 | non | non | 0,0042 $ | 5,5 s |
| Piste | affiché | — | 1/1 | non | non | 0,0040 $ | 16,9 s |

**Question** — écarté : citation trop courte ou trop longue

Citation : `Catalogue des objets, réservations avec calendrier, messagerie entre voisins,
notes et avis, notifications, application mobile et site web, paiement d'une
caution en ligne.`

> Avec deux soirs par semaine, quelle est l'unique fonctionnalité de cette liste qui permettrait à deux voisins de réaliser un premier prêt complet dès la semaine prochaine ?

**Question** — affiché

Citation : `application mobile et site web, paiement d'une
caution en ligne.`

> Comment le paiement en ligne et ces deux supports s'articulent-ils avec votre contrainte d'une consultation depuis la cave, sans réseau ?

**Piste** — affiché

> Deux notions sont en jeu ici : la conception « hors-ligne d'abord » (offline-first) et le « produit minimum viable » (MVP). Renseignez-vous sur les contraintes de synchronisation qu'impose un usage sans réseau, notamment face à une action qui exige une connexion immédiate comme un paiement en ligne. Documentez-vous aussi sur le découpage d'un MVP : comment isoler une seule fonctionnalité essentielle sur un seul support pour tenir dans un budget de deux soirs par semaine.

