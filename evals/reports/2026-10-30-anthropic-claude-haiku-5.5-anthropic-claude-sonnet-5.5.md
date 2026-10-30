# Évaluation des mentors — 2026-10-30 06:05

20 cas, coût total 0,1406 $. Un modèle est retenu s'il ne propose aucune solution (motifs `must_not`, brouillons écartés compris) et fait émerger la notion (`must_mention`) dans 70 % des cas au moins.

## Verdict par modèle

| Niveau · modèle | Cas | Notion émergée | Solution (brouillons) | Solution (affichée) | JSON invalides | Pannes | Casting différent | Coût | Durée médiane | Durée p90 | Verdict |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| fast · anthropic/claude-haiku-5.5 | 15 | 14/15 (93 %) | 0 | 0 | 0 | 0 | 0 | 0,0274 $ | 4,0 s | 6,3 s | **retenu** |
| strong · anthropic/claude-sonnet-5.5 | 5 | 4/5 (80 %) | 0 | 0 | 0 | 0 | 0 | 0,1132 $ | 4,6 s | 6,8 s | **retenu** |

## Cas

### 01-dates-par-reference — Comparaison de dates par référence et chevauchement non détecté

`src/emprunts/reserver.ts` · titulaire `mentor-langage` · modèle `anthropic/claude-haiku-5.5` · notion émergée · coût 0,0018 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | oui | non | 0,0005 $ | 5,3 s |
| Question | affiché | — | 1/1 | oui | non | 0,0004 $ | 4,5 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0005 $ | 3,6 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0005 $ | 4,3 s |

**Indice** — affiché

Citation : `e.debut === debut`

> Regardez comment l'opérateur === se comporte lorsqu'il compare deux valeurs de type Date. Demandez-vous si deux dates égales en valeur sont forcément la même référence.

**Indice** — affiché

Citation : `const dejaPris = emprunts.find(`

> Relisez la condition du find. Que vérifie-t-elle exactement sur chaque emprunt existant, par rapport aux dates debut et fin demandées ?

**Question** — affiché

> Imaginez deux objets Date créés séparément mais avec la même valeur temporelle. Si vous comparez ces deux objets avec ===, que compare réellement l'opérateur ? Qu'est-ce qui permettrait de comparer les instants eux-mêmes, et que vaut ce résultat si le début est nul ou invalide ?

**Piste** — affiché

> La notion en jeu est l'égalité entre objets. Un objet Date est une référence : === teste si les deux variables pointent vers le même objet, pas si elles représentent le même instant. Pour comparer des instants, cherchez dans la documentation de l'objet Date, sur MDN, comment obtenir la valeur numérique d'une date (nombre de millisecondes depuis l'époque). Regardez aussi ce que vaut cette valeur pour une date invalide : c'est un point à vérifier avant de conclure sur la comparaison.

**Exemple** — affiché

> Voici un cas voisin, distinct de votre code : const a = new Date(0); const b = new Date(0); console.log(a === b); // false, car ce sont deux références différentes. console.log(a.getTime() === b.getTime()); // true, car on compare les nombres de millisecondes. Notez aussi qu'une date invalide donne NaN avec getTime(), et que NaN ne vaut jamais NaN avec ===. Transposez ce principe à vos variables debut et e.debut, en vérifiant d'abord ce qui se passe si l'une des deux dates est invalide.

### 02-adr-sans-alternatives — ADR incohérent : hors ligne exigé, base hébergée choisie, aucune alternative

`docs/architecture/0001-stockage.md` · titulaire `architecte` · modèle `anthropic/claude-sonnet-5.5` · notion émergée · coût 0,0245 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | non | non | 0,0062 $ | 3,6 s |
| Question | affiché | — | 1/1 | oui | non | 0,0078 $ | 4,8 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0105 $ | 6,8 s |

**Indice** — affiché

Citation : `Nous stockons toutes les données dans une base PostgreSQL hébergée, interrogée
par une API REST.`

> Relisez le paragraphe Contexte, en particulier les phrases sur les lieux de consultation (cage d'escalier, cave) et sur la disponibilité des données. Comparez-les ensuite mot à mot avec cette phrase de décision. Que remarquez-vous ?

**Indice** — affiché

Citation : `Nous stockons toutes les données dans une base PostgreSQL hébergée`

> Regardez la section Conséquences : elle mentionne des profils avec nom et numéro d'appartement. Cherchez où le document traite de cette catégorie de données, et de ce que la décision change pour elle.

**Question** — affiché

> Imaginez un voisin à la cave, sans réseau, qui ouvre le catalogue. Suivez sa demande étape par étape, du téléphone jusqu'à la base, selon votre décision. À quel endroit de ce trajet les données doivent-elles se trouver pour que la contrainte « disponibles hors ligne » soit respectée ? Que dit votre texte à ce sujet ?

**Piste** — affiché

> La notion en jeu est l'architecture « hors ligne d'abord » (offline-first). Elle oblige à dire où vit une copie des données côté téléphone, comment elle se met à jour, et ce qui se passe si deux personnes modifient la même chose sans réseau. Pour vous documenter : le guide MDN sur les Progressive Web Apps (volet hors ligne) et l'article « Local-first software » d'Ink & Switch. Ensuite, relisez votre décision : que précise-t-elle sur ces points ?

### 03-ts-foreach-async — forEach avec une fonction async : le compte est renvoyé avant les enregistrements

`src/import/importer.ts` · titulaire `mentor-langage` · modèle `anthropic/claude-haiku-5.5` · notion émergée · coût 0,0020 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0003 $ | 3,0 s |
| Question | affiché | — | 1/1 | oui | non | 0,0004 $ | 3,3 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0006 $ | 4,9 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0007 $ | 6,6 s |

**Indice** — affiché

Citation : `fiches.forEach(async (fiche) => {`

> Regardez ce que forEach fait des fonctions asynchrones qu'on lui passe. Demandez-vous aussi à quel moment l'instruction return s'exécute par rapport aux enregistrements.

**Question** — affiché

> Si forEach ne fait pas attendre sur les promesses renvoyées par son callback, que vaut enregistrees au moment précis où importerTout exécute return ? Vérifiez ensuite ce qui se passe si l'une des appels à enregistrer rejette : qui reçoit cette erreur, et qui la gère ?

**Piste** — affiché

> Le point à creuser est la façon dont forEach traite son callback : il n'attend pas les promesses que le callback renvoie. Consultez la documentation MDN de Array.prototype.forEach pour voir ce qui est dit du retour du callback. Pour attendre une série d'opérations asynchrones, regardez du côté de Promise.all et comparez-le à une boucle for...of avec await : l'un lance les opérations en parallèle, l'autre les enchaîne. Demandez-vous laquelle garantit que le compteur est complet avant le return, et comment une erreur de enregistrer remonte jusqu'à l'appelant de importerTout.

**Exemple** — affiché

> Voici un cas voisin, sans rapport avec votre fichier : l'envoi de plusieurs messages. async function envoyerTous(messages: string[]): Promise<number> { const resultats = await Promise.all(messages.map((m) => envoyer(m))); return resultats.length; } Ici, map renvoie un tableau de promesses, et await attend qu'elles soient toutes réglées avant le return. Si envoyer rejette, Promise.all rejette et l'erreur remonte jusqu'à l'appelant, qui peut la traiter avec try/catch. Comparez ce schéma avec votre fonction : à quel endroit la valeur retournée est-elle calculée par rapport à la fin des opérations ?

### 04-ts-find-undefined — find peut renvoyer undefined, masqué par une assertion non nulle

`src/panier/total.ts` · titulaire `mentor-langage` · modèle `anthropic/claude-haiku-5.5` · notion émergée · coût 0,0018 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0002 $ | 1,6 s |
| Question | affiché | — | 1/1 | oui | non | 0,0004 $ | 3,7 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0004 $ | 4,0 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0007 $ | 6,9 s |

**Indice** — affiché

Citation : `return produit!.prix;`

> Regardez l'opérateur ! après produits.find : que se passe-t-il à l'exécution si aucun produit ne correspond à la référence demandée ?

**Question** — affiché

> Si find ne trouve rien, il renvoie undefined. Que promet la signature de prixDe à celui qui l'appelle : un number dans tous les cas ? Et que devrait-il se passer, selon vous, quand la référence est absente du catalogue ?

**Piste** — affiché

> La notion en jeu est le type de retour de find : il est désigné par T | undefined, ce que TypeScript appelle un type union. L'opérateur ! indique au compilateur de considérer qu'une valeur n'est jamais absente, sans rien vérifier à l'exécution. Pour en savoir plus, consultez la documentation MDN sur Array.prototype.find et le chapitre « Narrowing » du manuel TypeScript (Handbook), ainsi que la section sur l'opérateur non-null assertion. Interrogez-vous ensuite sur ce que la signature de prixDe doit exprimer lorsque la référence n'existe pas : un nombre, ou la possibilité d'une absence.

**Exemple** — affiché

> Voici un cas voisin, sur d'autres données : export function nomDe(utilisateurs: Utilisateur[], email: string): string | undefined { const u = utilisateurs.find((x) => x.email === email); return u?.nom; } Ici, la signature annonce qu'un résultat absent est possible, et le type de retour le rend visible à l'appelant. Comparez avec votre fonction : que doit annoncer sa signature, et que doit faire le code quand find ne trouve rien ?

### 05-ts-tri-sur-place — sort trie sur place le tableau reçu en paramètre

`src/agenda/tri.ts` · titulaire `mentor-langage` · modèle `anthropic/claude-haiku-5.5` · notion émergée · coût 0,0020 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | oui | non | 0,0004 $ | 3,5 s |
| Question | affiché | — | 1/1 | non | non | 0,0005 $ | 4,8 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0004 $ | 4,2 s |
| Exemple | affiché | — | 1/1 | non | non | 0,0007 $ | 6,3 s |

**Indice** — affiché

Citation : `.sort((a, b) => a.debut.getTime() - b.debut.getTime())`

> Regardez où s'applique ce tri : sur le tableau reçu en paramètre, ou sur une copie. Demandez-vous si l'appelant retrouve son tableau dans le même ordre après l'appel à cette fonction.

**Question** — affiché

> Si l'appelant conserve sa référence au tableau d'origine, dans quel ordre le retrouve-t-il après l'appel ? Et que renvoie `slice` sur un tableau déjà trié : une nouvelle collection ou une partie de l'ancienne ?

**Piste** — affiché

> La notion en jeu est la mutation : certaines méthodes de tableau modifient le tableau sur lequel elles sont appelées, d'autres en renvoient un nouveau. Consultez la page MDN de `Array.prototype.sort` et repérez ce qu'elle indique sur le tableau d'origine. Cherchez ensuite les méthodes non mutantes de la même famille, et comparez ce que chacune renvoie avec ce que `slice` renvoie lui-même.

**Exemple** — affiché

> Voici un cas voisin, avec d'autres données : const notes = [3, 1, 2]; function trier(liste: number[]): number[] { return liste.sort((a, b) => a - b); } const triees = trier(notes); Avant de l'exécuter, prévoyez ce que console.log(notes) affiche, et si triees === notes vaut true ou false. Vérifiez ensuite dans un environnement d'exécution, puis comparez le résultat à votre raisonnement sur votre propre fonction.

### 06-js-var-boucle — var dans une boucle : toutes les fonctions voient la même variable

`src/ui/boutons.js` · titulaire `mentor-langage` · modèle `anthropic/claude-haiku-5.5` · notion émergée · coût 0,0019 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0003 $ | 2,9 s |
| Question | affiché | — | 1/1 | oui | non | 0,0004 $ | 4,3 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0004 $ | 3,2 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0008 $ | 7,1 s |

**Indice** — affiché

Citation : `for (var i = 0; i < boutons.length; i++)`

> Regardez la déclaration de la variable i dans la boucle, puis le moment où la fonction passée à addEventListener lit sa valeur. Demandez-vous à quel moment ce code s'exécute, et quelle valeur de i il voit alors.

**Question** — affiché

> Une fois la boucle terminée, quelle valeur porte i ? Au moment du clic, la fonction lit cette même variable : quel nombre s'affichera-t-il pour chaque bouton ? Demandez-vous ensuite si chaque itération possède sa propre copie de i, et ce qui change selon la portée où i est déclarée.

**Piste** — affiché

> La notion en jeu est la portée des variables : var est limitée à la fonction, alors que let et const sont limitées au bloc. Regardez comment les closures capturent une variable plutôt que sa valeur au moment de la création de la fonction. Pour vous documenter, cherchez « portée des variables » et « closures » dans la documentation MDN, ainsi que la différence entre var et let dans la section sur les déclarations. Comparez ce que voit la fonction du clic avec ce que la boucle a fait de i à chaque tour.

**Exemple** — affiché

> Voici un cas voisin, sans rapport avec vos boutons : let total = 1; const lire = () => total; total = 5; console.log(lire()); La fonction lire ne garde pas la valeur 1 au moment de sa création. Elle retrouve la variable total elle-même au moment de l'appel, et cette variable a été modifiée entre-temps, donc elle affiche 5. Une closure lit sa variable, pas la valeur qu'elle avait à sa création. Reprenez ce mécanisme dans votre boucle : au moment du clic, quelle variable la fonction retrouve-t-elle, et qui a changé sa valeur depuis la création du gestionnaire ?

### 07-js-tri-numerique — sort sans comparateur trie des nombres comme du texte

`src/stats/mediane.js` · titulaire `mentor-langage` · modèle `anthropic/claude-haiku-5.5` · notion émergée · coût 0,0018 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0004 $ | 3,3 s |
| Question | affiché | — | 1/1 | oui | non | 0,0004 $ | 3,1 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0004 $ | 3,6 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0007 $ | 6,0 s |

**Indice** — affiché

Citation : `[...valeurs].sort()`

> Regardez la ligne du tri. Sans fonction de comparaison, sort() ne compare pas les valeurs comme des nombres. Vérifiez sur un petit tableau de nombres à deux chiffres ce que produit ce tri, et comparez avec ce que vous attendez.

**Question** — affiché

> Quelle comparaison sort() utilise-t-il par défaut sur les éléments d'un tableau ? Si vous triez 10, 9 et 2, dans quel ordre obtenez-vous ces valeurs si elles sont comparées comme du texte ? Cet ordre correspond-il à celui dont la médiane a besoin ?

**Piste** — affiché

> La notion en jeu est la comparaison par défaut de Array.prototype.sort() : sans argument, les éléments sont convertis en chaînes de caractères et comparés selon l'ordre lexicographique des codes UTF-16. Pour trier des nombres, il faut fournir à sort() une fonction de comparaison qui reçoit deux éléments et renvoie un nombre négatif, nul ou positif. Documentez-vous sur la page MDN « Array.prototype.sort() », section sur la fonction de comparaison et sur l'ordre par défaut. Ensuite, reprenez votre calcul de la médiane : il repose sur des valeurs triées numériquement.

**Exemple** — affiché

> Voici un cas voisin, différent du vôtre. const mots = ['banane', 'Pomme', 'abricot']; mots.sort() renvoie ['Pomme', 'abricot', 'banane'] : la majuscule « P » (code 80) passe avant la minuscule « a » (code 97), car sort() compare des chaînes caractère par caractère, sans supposer de sens à leur contenu. Appliquez ce même raisonnement à vos nombres : comment sort() les transforme-t-il avant de les comparer, et que cela change-t-il pour 10 et 9 ? Vérifiez en console avec un petit tableau, puis comparez le résultat à l'ordre numérique attendu par la médiane.

### 08-js-prix-flottants — Montants en euros flottants comparés avec ===

`src/caisse/reste.js` · titulaire `mentor-langage` · modèle `anthropic/claude-haiku-5.5` · notion émergée · coût 0,0018 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0004 $ | 3,3 s |
| Question | affiché | — | 1/1 | non | non | 0,0004 $ | 3,4 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0005 $ | 4,7 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0006 $ | 4,9 s |

**Indice** — affiché

Citation : `return total - dejaVerse === 0`

> Regardez la comparaison finale à zéro et les montants en euros additionnés dans la boucle. Demandez-vous comment JavaScript représente les nombres décimaux et si le résultat d'une soustraction peut être exactement zéro.

**Question** — affiché

> Si vous additionnez plusieurs prix décimaux dans la boucle, le total obtenu est-il toujours exactement celui que vous attendez ? Pouvez-vous le vérifier vous-même dans la console avec deux ou trois valeurs simples, puis comparer le résultat à celui de votre calcul à la main ?

**Piste** — affiché

> La notion en jeu est l'arithmétique à virgule flottante. JavaScript stocke les nombres au format binaire IEEE 754, qui ne peut pas représenter exactement certaines fractions décimales comme 0,1. Les erreurs d'arrondi s'accumulent donc quand on additionne plusieurs prix, et une soustraction peut donner un résidu très petit au lieu de zéro. Pour vous documenter, cherchez « floating point arithmetic » dans la documentation MDN, à la rubrique sur les nombres. Vous pouvez aussi étudier comment les montants d'argent sont généralement manipulés, par exemple en travaillant sur des valeurs entières (des centimes) plutôt qu'en euros décimaux. Interrogez-vous ensuite sur ce que signifie « égal à zéro » pour un montant calculé de cette façon.

**Exemple** — affiché

> Voici un cas voisin, distinct de votre code :
> const a = 0.1 + 0.2;
> console.log(a === 0.3); // false
> const b = 10 + 20;
> console.log(b === 30); // true
> const c = 0.1 * 3;
> console.log(c - 0.3); // 5.551115123125783e-17, et non 0
> Observez comment un résultat proche de zéro peut apparaître alors que le calcul semble juste. Reprenez ensuite votre propre comparaison à zéro et demandez-vous quelles valeurs pourraient produire ce type de résidu.

### 09-py-defaut-mutable — Liste mutable comme valeur par défaut d'un paramètre

`inventaire/panier.py` · titulaire `mentor-langage` · modèle `anthropic/claude-haiku-5.5` · notion émergée · coût 0,0015 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | oui | non | 0,0002 $ | 1,8 s |
| Question | affiché | — | 1/1 | oui | non | 0,0003 $ | 3,2 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0003 $ | 2,2 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0007 $ | 6,0 s |

**Indice** — affiché

Citation : `panier=[]`

> Regardez la valeur par défaut du paramètre panier. Demandez-vous quand elle est évaluée, et ce qui se passe si plusieurs appels reçoivent la même liste.

**Question** — affiché

> Si vous appelez ajouter('pain') puis ajouter('lait') sans fournir de panier, que contient la liste renvoyée au second appel ? Demandez-vous aussi si la liste par défaut est créée à chaque appel, ou une seule fois au moment où la fonction est définie, et ce que cela change pour la personne qui reçoit le panier.

**Piste** — affiché

> La notion en jeu est l'évaluation des valeurs par défaut des paramètres en Python : elle a lieu une seule fois, à la définition de la fonction, et non à chaque appel. Une liste étant mutable, toutes les insertions partagent le même objet. Documentez-vous sur « mutable default arguments » dans la documentation officielle du tutoriel Python, section sur la définition de fonctions, et sur l'usage de None comme sentinelle.

**Exemple** — affiché

> Voici un cas voisin, distinct de votre fonction :
> def compter(mot, compteur={}):
>     compteur[mot] = compteur.get(mot, 0) + 1
>     return compteur
> 
> print(compter('a'))
> print(compter('b'))
> 
> Exécutez-le et observez la seconde sortie : qu'y trouvez-vous de « a » ? Comparez-la avec ce que vous attendiez d'un compteur qui ne reçoit que « b », puis relisez ce que devient la valeur par défaut d'un appel à l'autre.

### 10-py-liaison-tardive — Fermetures créées dans une boucle : liaison tardive de la variable

`rapports/filtres.py` · titulaire `mentor-langage` · modèle `anthropic/claude-haiku-5.5` · notion émergée · coût 0,0019 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0003 $ | 2,8 s |
| Question | affiché | — | 1/1 | non | non | 0,0004 $ | 3,5 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0006 $ | 4,9 s |
| Exemple | affiché | — | 1/1 | non | non | 0,0007 $ | 5,8 s |

**Indice** — affiché

Citation : `filtres.append(lambda note: note >= limite)`

> Regardez où la valeur de limite est lue dans la lambda : à quel moment Python la cherche-t-il, et quelle valeur contient alors cette variable une fois la boucle terminée ?

**Question** — affiché

> Imaginez que vous appelez le premier filtre renvoyé, après la fin de la boucle. Quelle valeur de limite utilise-t-il ? Et si chaque filtre lit la même variable, que faudrait-il qu'il se passe pour que chacun garde la limite qui lui correspond ?

**Piste** — affiché

> Cherchez du côté de la liaison tardive des variables libres dans les closures Python. La lambda ne mémorise pas la valeur de limite au moment de sa création : elle retient la variable elle-même, qui appartient à la fonction englobante. Pour voir ce qui se passe à la fin de la boucle, consultez la FAQ Python, qui contient une entrée sur les lambdas définies dans une boucle, ainsi que la section du tutoriel sur les fonctions imbriquées et les closures. Demandez-vous ensuite à quel moment la valeur est figée, et si ce moment pourrait être avancé à la création de chaque filtre.

**Exemple** — affiché

> Voici un cas voisin, sans rapport avec votre code : def multiplicateurs(): fonctions = [] ; for i in range(3): fonctions.append(lambda x: x * i) ; return fonctions. Exécutez ensuite print([f(10) for f in multiplicateurs()]) et comparez le résultat à ce que la boucle semblait vouloir produire. Demandez-vous quelle valeur de i chaque lambda utilise au moment de son appel, et à quel moment cette valeur est fixée.

### 11-py-liste-modifiee-en-parcours — Liste modifiée pendant qu'on la parcourt : des éléments sont sautés

`taches/nettoyage.py` · titulaire `mentor-langage` · modèle `anthropic/claude-haiku-5.5` · notion émergée · coût 0,0018 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0002 $ | 1,9 s |
| Question | affiché | — | 1/1 | oui | non | 0,0003 $ | 3,1 s |
| Piste | affiché | — | 1/1 | non | non | 0,0005 $ | 4,3 s |
| Exemple | affiché | — | 1/1 | non | non | 0,0008 $ | 7,1 s |

**Indice** — affiché

Citation : `taches.remove(tache)`

> Regardez ce qui se passe avec la liste pendant qu'elle est parcourue par la boucle for. Observez les indices des éléments avant et après un appel à remove.

**Question** — affiché

> Imaginez une liste de trois tâches, la deuxième terminée. Quand la boucle arrive sur cette deuxième tâche et la retire, que devient la tâche qui était en troisième position ? La boucle la verra-t-elle à l'itération suivante ?

**Piste** — affiché

> La notion en jeu est la modification d'une liste pendant son parcours. Un for lit la liste par position, et la retirer décale les éléments suivants vers des positions déjà passées. Pour vous documenter, lisez la section sur l'instruction for du tutoriel Python, puis la description de list.remove dans la référence des types de séquences. Cherchez aussi comment une fonction peut produire une nouvelle liste sans toucher à l'originale.

**Exemple** — affiché

> Voici un cas voisin, distinct de votre code. Suivez son exécution pas à pas, puis comparez avec le résultat affiché :
> lettres = ['x', 'x', 'y']
> for l in lettres:
>     if l == 'x':
>         lettres.remove(l)
> print(lettres)
> Le résultat contient encore un 'x'. Quelle position la boucle occupe-t-elle après le premier remove, et quel élément s'y trouve ?

### 12-py-is-au-lieu-de-egal — is compare l'identité des objets, pas leur valeur

`comptes/verification.py` · titulaire `mentor-langage` · modèle `anthropic/claude-haiku-5.5` · notion émergée · coût 0,0016 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | oui | non | 0,0002 $ | 1,7 s |
| Question | affiché | — | 1/1 | oui | non | 0,0004 $ | 4,1 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0005 $ | 4,3 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0005 $ | 4,7 s |

**Indice** — affiché

Citation : `return saisi.strip() is attendu`

> Regardez l'opérateur utilisé pour comparer saisi.strip() et attendu. Demandez-vous ce qu'il compare réellement : l'identité des objets ou leur contenu.

**Question** — affiché

> Imaginez que saisi.strip() et attendu contiennent exactement les mêmes caractères, mais qu'ils ont été créés séparément, l'un par une saisie, l'autre lu depuis un fichier. Ces deux objets sont-ils forcément le même objet en mémoire ? Et que vaut alors la question « sont-ils identiques ? » selon ce que vous voulez vérifier ici ?

**Piste** — affiché

> La notion en jeu est la distinction entre l'identité d'un objet (être le même objet en mémoire) et l'égalité de leurs valeurs (contenir les mêmes données). Ces deux idées ne se testent pas avec le même opérateur. Pour y voir clair, lisez la documentation officielle de Python sur les opérateurs de comparaison, dans le langage de référence, ainsi que la partie sur les objets, valeurs et types. Vérifiez aussi ce que disent ces pages sur la comparaison de chaînes de caractères. Ensuite, demandez-vous quelle question la fonction doit réellement poser.

**Exemple** — affiché

> Voici un cas voisin, distinct de votre fonction : a = [1, 2] puis b = [1, 2]. Avec a == b, Python répond True car les contenus sont égaux. Avec a is b, il répond False car ce sont deux objets différents en mémoire. Testez ces lignes dans un interpréteur, puis essayez b = a et observez à nouveau a is b. Comparez ensuite ce résultat à la question que pose votre fonction : deux saisies doivent-elles être le même objet, ou contenir le même texte ?

### 13-rs-unwrap-saisie — unwrap sur une saisie utilisateur : panique au lieu d'une erreur

`src/commande.rs` · titulaire `mentor-langage` · modèle `anthropic/claude-haiku-5.5` · notion émergée · coût 0,0017 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0002 $ | 1,9 s |
| Question | affiché | — | 1/1 | non | non | 0,0004 $ | 3,3 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0004 $ | 3,3 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0007 $ | 6,3 s |

**Indice** — affiché

Citation : `ligne.trim().parse().unwrap()`

> Que se passe-t-il, dans cette fonction, si l'utilisateur saisit une ligne vide ou un texte qui n'est pas un nombre ? Regardez ce que retournent les appels qui se terminent par unwrap().

**Question** — affiché

> Si la saisie est invalide, la fonction doit-elle arrêter le programme, ou laisser l'appelant décider quoi faire ? Dans le second cas, que pourrait exprimer la signature de la fonction pour rendre cette possibilité visible ?

**Piste** — affiché

> La notion en jeu est le type Result, qui représente une opération pouvant échouer, et l'opérateur ? qui transmet cette erreur à l'appelant. La méthode parse renvoie un Result dont l'erreur est de type ParseIntError, et read_line en renvoie un aussi. Vous pouvez lire le chapitre « Gestion des erreurs » du livre The Rust Programming Language, ainsi que la documentation de std::str::FromStr et de std::num::ParseIntError. Regardez ensuite comment la signature de lire_quantite pourrait refléter le fait qu'elle peut échouer.

**Exemple** — affiché

> Voici un cas voisin, différent de votre fonction : une fonction qui renvoie un Option et utilise ? pour sortir tôt quand la valeur manque.
> 
> fn premier_mot(s: &str) -> Option<&str> {
>     let mot = s.split_whitespace().next()?;
>     Some(mot)
> }
> 
> Vérifiez ce que fait ? quand next() ne trouve rien. Puis demandez-vous quel type de retour lire_quantite devrait avoir pour transmettre une erreur de read_line ou de parse à l'appelant, au lieu de paniquer.

### 14-rs-fenetre-glissante — Borne de boucle : la dernière fenêtre manque, et soustraction qui déborde

`src/stats.rs` · titulaire `mentor-langage` · modèle `anthropic/claude-haiku-5.5` · notion émergée · coût 0,0021 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | oui | non | 0,0004 $ | 3,2 s |
| Question | affiché | — | 1/1 | non | non | 0,0004 $ | 4,0 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0005 $ | 4,1 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0008 $ | 6,7 s |

**Indice** — affiché

Citation : `for debut in 0..valeurs.len() - fenetre {`

> Regardez la borne haute de cette boucle. Que vaut cette expression quand la fenêtre est plus grande que la série ? Vérifiez le type de `valeurs.len()` et de `fenetre`, et ce que la soustraction produit dans ce cas.

**Question** — affiché

> Imaginez une série de 3 valeurs et une fenêtre de 5. Quelle est la valeur de `valeurs.len() - fenetre` avec des `usize` ? Et que devrait renvoyer votre fonction dans ce cas, selon ce que vous voulez calculer ?

**Piste** — affiché

> La notion en jeu est l'arithmétique des entiers non signés en Rust. Une soustraction sur `usize` qui passe sous zéro ne produit pas de nombre négatif : en mode debug le programme panique, et en mode release le résultat revient à une valeur très grande. Le chapitre « Integer Overflow » du Rust Book et la documentation de `usize::checked_sub` expliquent ces comportements. Pour la signature de la fonction, la documentation de `slice::windows` présente une façon de parcourir toutes les fenêtres d'une tranche sans calculer de bornes à la main.

**Exemple** — affiché

> Voici un cas voisin, hors de votre fonction. Un stock doit couvrir une demande, et la soustraction peut passer sous zéro :
> 
> fn restant(stock: u32, demande: u32) -> Option<u32> {
>     stock.checked_sub(demande)
> }
> 
> fn main() {
>     println!("{:?}", restant(5, 3)); // Some(2)
>     println!("{:?}", restant(3, 5)); // None
> }
> 
> Le retour est None quand la soustraction passerait sous zéro. Transposez ce mécanisme à votre borne : que doit renvoyer votre fonction quand la fenêtre dépasse la série ?

### 15-rs-clone-pour-l-emprunteur — clone partout pour contourner l'emprunteur, et liste vide qui panique

`src/inventaire.rs` · titulaire `mentor-langage` · modèle `anthropic/claude-haiku-5.5` · notion **non émergée** · coût 0,0019 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0003 $ | 2,8 s |
| Question | affiché | — | 1/1 | non | non | 0,0003 $ | 2,9 s |
| Piste | affiché | — | 1/1 | non | non | 0,0006 $ | 5,6 s |
| Exemple | affiché | — | 1/1 | non | non | 0,0006 $ | 6,0 s |

**Indice** — affiché

Citation : `let mut meilleur = self.articles[0].clone();`

> Regardez la valeur de départ de `meilleur`. Que se passe-t-il si `self.articles` ne contient aucun élément ? Vérifiez quelle opération d'indexation est utilisée à cet endroit.

**Question** — affiché

> Si l'inventaire est vide, que devrait renvoyer `plus_long` ? Le type de retour `String` permet-il d'exprimer cette absence de résultat, ou faudrait-il changer la signature ?

**Piste** — affiché

> La notion en jeu est le panic : avec `[]`, Rust arrête le programme si la position demandée n'existe pas dans le `Vec`. Pour une absence de valeur prévue, la bibliothèque standard utilise plutôt `Option`. Documentez-vous sur `Option` dans le chapitre des types énumérés du Rust Book, puis sur la documentation de `Vec` dans std, section des méthodes d'accès aux éléments qui renvoient une `Option`. Le chapitre sur la gestion des erreurs compare aussi `Option`, `Result` et `panic!`.

**Exemple** — affiché

> Voici un cas voisin, sans rapport direct avec votre code : un Vec vide et la méthode get, qui renvoie une Option au lieu de paniquer. let notes: Vec<u32> = vec![]; let premiere = notes.get(0); println!("{:?}", premiere); Ce programme affiche None et ne s'arrête pas. Pour utiliser la valeur, il faut ensuite traiter les deux cas, Some et None, par exemple avec un match ou un if let. Demandez-vous ce que votre fonction doit renvoyer dans le cas None, puis observez comment la signature de retour doit changer pour le permettre.

### 16-rs-iterateur-paresseux — map sans consommation : l'itérateur paresseux ne s'exécute jamais

`src/journal.rs` · titulaire `mentor-langage` · modèle `anthropic/claude-haiku-5.5` · notion émergée · coût 0,0018 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0003 $ | 3,2 s |
| Question | affiché | — | 1/1 | oui | non | 0,0005 $ | 3,8 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0005 $ | 4,1 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0005 $ | 4,7 s |

**Indice** — affiché

Citation : `lignes.iter().map(|ligne| archive.push(ligne.to_uppercase()));`

> Regardez de près ce que renvoie `map` quand on l'appelle, puis ce qui se passe avec la valeur produite par cette expression. Vérifiez si la closure s'exécute à cet endroit.

**Question** — affiché

> Si cette ligne est exécutée, qu'est-ce qui consomme réellement l'itérateur produit par `map` ? Un adaptateur comme `map` fait-il du travail tant que personne ne le parcourt ? Et si ce n'est pas le cas ici, que devient `archive` après l'appel de `archiver` ?

**Piste** — affiché

> La notion en jeu est la paresse des itérateurs en Rust : les adaptateurs comme `map` ne font aucun travail tant qu'on ne les consomme pas. Pour vérifier, lisez la documentation du trait `Iterator` dans la bibliothèque standard, notamment la partie sur les adaptateurs et leur caractère paresseux, ainsi que les méthodes qui consomment un itérateur. Regardez aussi l'avertissement que le compilateur émet lorsqu'un adaptateur est utilisé sans être consommé, et ce que cet avertissement signifie. Ensuite, demandez-vous quelle méthode consomme réellement l'itérateur dans votre fonction.

**Exemple** — affiché

> Voici un cas voisin, sans rapport direct avec votre fonction : `let nombres = vec![1, 2, 3]; let _ = nombres.iter().map(|n| println!("{n}")); println!("fin");` Affiche seulement « fin ». La closure n'est jamais appelée, car `map` n'a rien produit à consommer. Comparez avec `nombres.iter().for_each(|n| println!("{n}"));`, où la méthode consomme l'itérateur et exécute la closure pour chaque élément. Observez ce qui est affiché dans chaque cas et ce que cela implique pour `archive`.

### 17-adr-consequences-positives — ADR dont les conséquences ne listent aucun inconvénient

`docs/architecture/0002-connexion.md` · titulaire `architecte` · modèle `anthropic/claude-sonnet-5.5` · notion **non émergée** · coût 0,0211 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | non | non | 0,0059 $ | 3,1 s |
| Question | affiché | — | 1/1 | non | non | 0,0068 $ | 4,0 s |
| Piste | affiché | — | 1/1 | non | non | 0,0085 $ | 5,1 s |

**Indice** — affiché

Citation : `- Plus de mots de passe à gérer.`

> Cette section ne liste que des effets favorables. Relisez la décision (lien par e-mail, valable 15 minutes) en vous demandant de quoi la connexion dépend désormais, et ce qui se passe quand cette dépendance fait défaut.

**Indice** — affiché

Citation : `- Le gardien n'a plus de réinitialisations à faire.`

> Le gardien ne gère plus de réinitialisations. Regardez si un autre type de demande d'aide pourrait apparaître à sa place, et si vous voulez noter ce qui vous ferait revenir sur cette décision.

**Question** — affiché

> Imaginez un voisin qui demande un lien, mais dont l'e-mail arrive après 20 minutes, ou qui n'a plus accès à sa messagerie. Que peut-il faire pour se connecter ? Qui l'aide alors ? Cette situation change-t-elle la charge du gardien, et comment le sauriez-vous une fois la décision appliquée ?

**Piste** — affiché

> La notion en jeu est celle des modes de défaillance d'une dépendance, à relier au critère de réexamen d'une décision. Pour la connexion sans mot de passe par lien, regardez la délivrabilité des e-mails et le parcours de secours quand la messagerie est inaccessible. Pour la forme, consultez le modèle d'ADR de Michael Nygard (section « Consequences »), puis la fiche « Authentication » de l'OWASP.

### 18-adr-decision-disproportionnee — Décision disproportionnée au regard des contraintes du contexte

`docs/architecture/0003-notifications.md` · titulaire `architecte` · modèle `anthropic/claude-sonnet-5.5` · notion émergée · coût 0,0230 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | oui | non | 0,0059 $ | 3,6 s |
| Question | affiché | — | 1/1 | oui | non | 0,0082 $ | 5,2 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0089 $ | 5,4 s |

**Indice** — affiché

Citation : `Nous déployons un cluster Kafka de trois nœuds pour acheminer les
notifications`

> Relisez la section Contexte : matériel, budget et volume y sont décrits. Comparez ces éléments, un par un, avec ce que la décision demande de déployer. Que constatez-vous pour chacun ?

**Indice** — affiché

Citation : `avec un service consommateur par type de message.`

> Regardez la section Conséquences : elle ne mentionne qu'un seul effet. Relisez la décision en vous demandant ce qu'elle impose à l'exploitation, et ce qui devrait vous faire revenir dessus.

**Question** — affiché

> Trois nœuds apportent une tolérance aux pannes si ils tournent sur des machines distinctes. Où seraient-ils placés ici, et que se passe-t-il pour eux si le Raspberry Pi s'arrête ? Quel besoin, absent à cent notifications par jour, ce cluster viendrait-il couvrir ? Quelle phrase de la décision le dirait ?

**Piste** — affiché

> Deux notions peuvent vous aider. D'abord le point unique de défaillance : cherchez comment la documentation de Kafka décrit la réplication et le quorum, et sur quelles machines les nœuds doivent tourner pour que cela serve. Ensuite les critères de décision d'un ADR : comparez débit, mémoire, coût d'exploitation et maintenance aux contraintes de votre Contexte. Ajoutez ensuite une section « Alternatives envisagées » qui dise pourquoi chacune est retenue ou écartée.ZZ

### 19-adr-nous-verrons — Cache sans stratégie d'invalidation, renvoyée à plus tard

`docs/architecture/0004-cache.md` · titulaire `architecte` · modèle `anthropic/claude-sonnet-5.5` · notion émergée · coût 0,0203 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | oui | non | 0,0061 $ | 3,4 s |
| Question | affiché | — | 1/1 | non | non | 0,0073 $ | 4,2 s |
| Piste | rien | — | 1/1 | non | non | 0,0070 $ | 4,6 s |

**Indice** — affiché

Citation : `Nous verrons plus tard
s'il faut l'invalider.`

> Relisez la phrase du contexte sur les voisins qui ajoutent et retirent des objets toute la journée, puis comparez-la à cette phrase de la décision. Qu'est-ce que le contexte laisse entendre sur la fraîcheur des données affichées ?

**Indice** — affiché

Citation : `Nous ajoutons un cache Redis devant l'API du catalogue.`

> Regardez la section Conséquences : elle ne mentionne qu'un seul effet. Relisez-la en pensant à ce que la décision ajoute à l'ensemble du système, et à ce qui vous ferait remettre cette décision en cause.

**Question** — affiché

> Imaginez qu'un voisin retire un objet à 10 h, alors que le cache a été rempli à 9 h 59. Que voit un autre voisin sur la page, et pendant combien de temps ? Quel décalage votre document juge-t-il acceptable, et à quel endroit cette limite est-elle écrite ?

**Piste** — écarté : confiance insuffisante

> placeholder

### 20-conception-sans-contraintes — Document de conception qui choisit une solution sans dire les contraintes

`docs/design/recherche.md` · titulaire `architecte` · modèle `anthropic/claude-sonnet-5.5` · notion émergée · coût 0,0242 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | oui | non | 0,0056 $ | 3,3 s |
| Question | affiché | — | 1/1 | non | non | 0,0081 $ | 4,8 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0105 $ | 7,2 s |

**Indice** — affiché

Citation : `Un cluster Elasticsearch indexe le catalogue en temps réel.`

> L'objectif parle de voisins qui cherchent un objet par son nom. Relisez-le en regard de ce que la solution met en place, puis demandez-vous quels volumes et quel nombre d'utilisateurs vous supposez, et où ce document les indique.

**Indice** — affiché

Citation : `la recherche passe par une API
dédiée.`

> Cette section décrit une seule voie. Cherchez dans le document où figurent les autres options envisagées, et ce qui ferait revoir ce choix plus tard.

**Question** — affiché

> Quel nombre d'objets et de recherches par jour imaginez-vous pour ce catalogue ? À partir de quel seuil une recherche par nom, sans cluster dédié, ne suffirait plus ? Et pourriez-vous vérifier ce seuil par un calcul ou un essai, puis l'écrire dans le document pour justifier ce choix ?

**Piste** — affiché

> La notion en jeu est l'estimation d'ordre de grandeur (dimensionnement) : chiffrer objets, recherches par jour et rythme d'ajouts, puis les comparer à ce qu'un cluster apporte. Cherchez « capacity planning » et la documentation officielle d'Elasticsearch sur le dimensionnement. Consultez aussi la forme d'un ADR : une section « Alternatives » explique pourquoi une option plus simple est écartée, chiffres à l'appui. Le « temps réel » mérite lui aussi un besoin exprimé, par exemple un délai acceptable avant qu'un objet soit trouvable.}]}entity wait, schema requires replies items with level, message, confidence — included. Done.}]}`}]}  Not valid—see correction below.{

