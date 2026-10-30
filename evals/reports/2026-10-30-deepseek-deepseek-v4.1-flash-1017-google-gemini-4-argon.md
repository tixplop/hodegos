# Évaluation des mentors — 2026-10-30 05:59

20 cas, coût total 0,0745 $. Un modèle est retenu s'il ne propose aucune solution (motifs `must_not`, brouillons écartés compris) et fait émerger la notion (`must_mention`) dans 70 % des cas au moins.

## Verdict par modèle

| Niveau · modèle | Cas | Notion émergée | Solution (brouillons) | Solution (affichée) | JSON invalides | Pannes | Casting différent | Coût | Durée médiane | Durée p90 | Verdict |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| fast · deepseek/deepseek-v4.1-flash-1017 | 15 | 15/15 (100 %) | 0 | 0 | 0 | 0 | 0 | 0,0213 $ | 2,9 s | 5,7 s | **retenu** |
| strong · google/gemini-4-argon | 5 | 5/5 (100 %) | 0 | 0 | 0 | 0 | 0 | 0,0531 $ | 4,7 s | 6,4 s | **retenu** |

## Cas

### 01-dates-par-reference — Comparaison de dates par référence et chevauchement non détecté

`src/emprunts/reserver.ts` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0031 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | oui | non | 0,0006 $ | 9,4 s |
| Question | affiché | — | 1/1 | oui | non | 0,0006 $ | 4,2 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0004 $ | 4,9 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0015 $ | 4,3 s |

**Indice** — affiché

Citation : `e.debut === debut`

> Arrêtez-vous sur la comparaison `e.debut === debut`. Que compare cet opérateur lorsqu'il porte sur deux objets `Date` ? Vérifiez avec deux dates distinctes représentant le même instant.

**Indice** — affiché

Citation : `(e) => e.objetId === objet.id && e.debut === debut`

> La variable `fin` n'intervient pas dans la recherche de conflit. Demandez-vous quels couples de dates se chevauchent réellement, puis vérifiez votre règle sur un emprunt qui encadre la période demandée.

**Question** — affiché

> Pour un même objet, votre prédicat ne regarde que la borne de début. Si un emprunt existant commence avant `debut` et se termine après, votre test le détecte-t-il ? Vérifiez avec un intervalle qui chevauche sans partager la même date de départ.

**Piste** — affiché

> Deux notions sont en jeu. La première est la comparaison d'instants : la documentation MDN de `Date`, section `getTime()` et `valueOf()`, explique pourquoi deux `Date` distincts ne s'égalisent pas par `===` et quelle valeur numérique sert de référence commune. La seconde est le recouvrement d'intervalles : cherchez « interval overlap » ou « algèbre des intervalles » pour la condition qui décide si deux plages [début, fin] se chevauchent, y compris quand elles ne partagent aucune borne. Documentez ces deux points avant de retoucher votre prédicat.

**Exemple** — affiché

> Un cas voisin, sur deux plages numériques et deux instants :
> 
> const a = new Date("2024-01-01T00:00:00Z");
> const b = new Date("2024-01-01T00:00:00Z");
> a === b                        // false
> a.getTime() === b.getTime()    // true
> 
> // [a1,a2] et [b1,b2] se chevauchent ?
> const chevauche = (a1, a2, b1, b2) => a1 < b2 && b1 < a2;
> chevauche(1, 10, 5, 15)  // true
> chevauche(1, 5, 10, 15)  // false
> 
> Les comparaisons portent sur des valeurs comparables, pas sur des objets.

### 02-adr-sans-alternatives — ADR incohérent : hors ligne exigé, base hébergée choisie, aucune alternative

`docs/architecture/0001-stockage.md` · titulaire `architecte` · modèle `google/gemini-4-argon` · notion émergée · coût 0,0099 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0031 $ | 5,2 s |
| Question | affiché | — | 1/1 | non | non | 0,0031 $ | 3,8 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0037 $ | 4,4 s |

**Indice** — affiché

Citation : `Nous stockons toutes les données dans une base PostgreSQL hébergée, interrogée
par une API REST.`

> Relisez la dernière phrase de la section « Contexte » en regard de cette décision.

**Question** — affiché

> Que se passe-t-il lorsqu'un voisin consulte le catalogue depuis son téléphone à la cave, sans réseau, si toutes les données ne se trouvent que dans la base hébergée ?

**Piste** — affiché

> Le contexte impose un fonctionnement hors ligne, mais la décision décrit un stockage uniquement distant. Renseignez-vous sur les architectures « offline-first » (ou « local-first ») et le stockage local sur l'appareil (par exemple IndexedDB ou le cache applicatif sur navigateur, SQLite sur mobile) combiné à un mécanisme de synchronisation avec la base centrale.

### 03-ts-foreach-async — forEach avec une fonction async : le compte est renvoyé avant les enregistrements

`src/import/importer.ts` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0013 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0003 $ | 3,9 s |
| Question | affiché | — | 1/1 | non | non | 0,0002 $ | 2,2 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0002 $ | 3,1 s |
| Exemple | rien | — | 1/1 | non | non | 0,0006 $ | 3,6 s |

**Indice** — affiché

Citation : `fiches.forEach(async (fiche) => {`

> Regardez « fiches.forEach(async (fiche) => { » et « return enregistrees; » : à quel moment précis le return s'exécute-t-il par rapport au corps du callback ? Ajoutez un affichage avant le return pour observer la valeur de enregistrees.

**Question** — affiché

> Au moment où le return lit enregistrees, combien de callbacks ont déjà franchi leur await ? Séparez ce qui s'exécute pendant l'appel de forEach et ce qui s'exécute après : la réponse vous dit pourquoi le compteur rendu est toujours le même.

**Piste** — affiché

> La notion en jeu : une fonction de rappel asynchrone renvoie une promesse, et certaines méthodes de tableau l'ignorent purement et simplement. Cherchez dans la documentation MDN, section Array.prototype.forEach, ce qu'elle dit du résultat renvoyé par le callback : vous y verrez que forEach n'attend rien et ne collecte rien. Comparez avec map, qui renvoie un tableau de ce que chaque callback produit, et lisez la page Promise.all, qui attend un ensemble de promesses et rend leurs résultats dans l'ordre. Le point d'articulation à trancher vous-même : quelle méthode transforme vos fiches en promesses, et qui attend ces promesses avant de lire le compteur ?

**Exemple** — écarté : garde « pas de solution » : exemple qui recopie un fragment de la section

> Cas voisin, autre domaine :
> 
> async function compterRecus(liens: string[]): Promise<number> {
>   let recus = 0;
>   for (const lien of liens) {
>     await recevoir(lien);
>     recus += 1;
>   }
>   return recus;
> }
> 
> Ici l'attente est dans la boucle elle-même : le return ne peut pas passer avant la dernière incrémentation. Un enchaînement séquentiel se lit ainsi.
> 
> La variante parallèle existe aussi : elle ne compte pas dans la boucle, elle rassemble d'abord les promesses dans un tableau, les attend ensemble, puis lit le compteur. Comparez les deux ordres d'exécution sur vos fiches.

### 04-ts-find-undefined — find peut renvoyer undefined, masqué par une assertion non nulle

`src/panier/total.ts` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0012 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0004 $ | 2,7 s |
| Question | affiché | — | 1/1 | non | non | 0,0001 $ | 1,0 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0003 $ | 4,0 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0004 $ | 3,8 s |

**Indice** — affiché

Citation : `return produit!.prix;`

> Le `!` de cette ligne s'adresse au compilateur. Regardez ce que `find` renvoie quand aucune référence ne correspond, puis ce que devient ce retour à l'exécution. Vous pourrez ensuite décider quel contrat `prixDe` expose à ses appelants.

**Question** — affiché

> Si aucun produit ne porte cette référence, quel résultat voulez-vous que `prixDe` rende à l'appelant : un nombre, un signal d'absence, une erreur ? Une fois ce choix fixé, une autre question : quel type de retour l'exprime le plus fidèlement ?

**Piste** — affiché

> La notion en jeu est le type de retour : `find` renvoie un type qui inclut `undefined`, et c'est ce type que votre signature peut refléter ou non. Documentez-vous sur le « narrowing » et sur `strictNullChecks` dans le manuel TypeScript, section « Everyday Types » et « Narrowing » ; côté bibliothèque, la page MDN de `Array.prototype.find` précise la valeur de retour en cas d'absence. Vous y verrez comment le compilateur suit ce cas et pourquoi une assertion le contourne au lieu de le traiter. Cela vous aidera à choisir entre un type de retour qui admet l'absence et un contrat qui l'interdit.

**Exemple** — affiché

> Un cas voisin, volontairement hors de votre fichier :
> 
> ```ts
> function trouver(id: string): Client | undefined {
>   return clients.find((c) => c.id === id);
> }
> 
> function nomDe(id: string): string {
>   const c = trouver(id);
>   if (c === undefined) throw new Error(`inconnu: ${id}`);
>   return c.nom;
> }
> ```
> 
> Ce qui change par rapport à un retour typé `number` : l'absence figure dans la signature de `trouver`. L'appelant ne peut pas lire `.nom` sans avoir traité le cas ; aucune assertion n'est nécessaire. Un second appelant peut choisir un autre comportement (valeur par défaut, `undefined` propagé) sans que `trouver` impose le sien.

### 05-ts-tri-sur-place — sort trie sur place le tableau reçu en paramètre

`src/agenda/tri.ts` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0010 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0002 $ | 2,4 s |
| Question | affiché | — | 1/1 | non | non | 0,0002 $ | 1,7 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0002 $ | 1,8 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0004 $ | 3,2 s |

**Indice** — affiché

Citation : `creneaux
    .sort((a, b) => a.debut.getTime() - b.debut.getTime())`

> Regardez ce qu'il advient du tableau reçu par la fonction, du point de vue de l'appelant, quand cette ligne s'exécute. Demandez-vous si le contrat que vous voulez offrir autorise ce changement.

**Question** — affiché

> Si vous étiez l'appelant et conserviez une référence sur votre tableau, quelle observation, faite juste après l'appel, vous convaincrait que le contrat est respecté ? Écrivez-la comme une vérification que vous pouvez exécuter vous-même.

**Piste** — affiché

> La notion en jeu est la distinction entre opération en place et opération qui produit une copie : Array.prototype.sort modifie le tableau et renvoie ce même tableau. D'où le comportement observé chez l'appelant.
> 
> Pour vous documenter : la page MDN d'Array.prototype.sort, rubrique sur le tri sur place et la valeur de retour, puis celle d'Array.prototype.toSorted, qui renvoie un nouveau tableau. Voyez aussi ce que dit TypeScript pour ReadonlyArray : il signale l'appel des méthodes mutantes, ce qui rend le contrat lisible dans la signature.

**Exemple** — affiché

> Cas voisin, sur une autre méthode qui travaille elle aussi en place :
> 
> ```ts
> export function troisDernieres(etapes: string[]): string[] {
>   return etapes.slice(-3);        // l'entrée reste intacte
> }
> 
> export function inversees(etapes: string[]): string[] {
>   const copie = etapes.slice();
>   return copie.reverse();         // sans la copie, etapes changerait chez l'appelant
> }
> ```
> 
> Le second exemple rend le choix visible : accepter la modification, ou copier d'abord. Votre signature doit dire lequel des deux contrats vous tenez.

### 06-js-var-boucle — var dans une boucle : toutes les fonctions voient la même variable

`src/ui/boutons.js` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0012 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0002 $ | 1,8 s |
| Question | affiché | — | 1/1 | non | non | 0,0001 $ | 5,6 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0002 $ | 2,9 s |
| Exemple | affiché | — | 1/1 | non | non | 0,0006 $ | 4,8 s |

**Indice** — affiché

Citation : `var i = 0`

> Piste de lecture : comparez le moment où `i` change et le moment où le corps du clic s'exécute. Que vaut `i` quand l'utilisateur clique, longtemps après la fin de la boucle ? Testez avec trois boutons pour observer le libellé affiché.

**Question** — affiché

> Comment chaque fonction de clic pourrait-elle conserver la valeur de l'indice au moment où elle est créée, plutôt que de la relire plus tard ? Indice de vérification : ajoutez un console.log dans la boucle et dans le clic, comparez les ordres d'exécution.

**Piste** — affiché

> Ce que ce palier ajoute : nommer la notion et le lieu pour l'étudier.
> 
> Deux notions se combinent ici. D'abord la portée des variables en JavaScript : une déclaration faite avec `var` dans un bloc n'y est pas confinée. Ensuite la fermeture (closure) : la fonction de clic capture la variable elle-même, pas la valeur qu'elle avait à cet instant. C'est pourquoi l'unique variable partagée suffit à toutes les fonctions.
> 
> Pour vous documenter : sur MDN, les pages « var », « let » et « Fermetures » (section sur les closures dans une boucle). Lisez en particulier ce qui distingue une variable de portée de bloc d'une variable de portée de fonction, et la différence entre capturer une variable et capturer une valeur.
> 
> À vous de choisir laquelle des deux notions vous voulez faire bouger.

**Exemple** — affiché

> Cas voisin : une file de tâches planifiées.
> 
> ```
> function planifier(file) {
>   for (var k = 0; k < file.length; k++) {
>     setTimeout(function () { lancer(file[k]); }, 0);
>   }
> }
> ```
> 
> Ici chaque rappel relit `k` et `file[k]` au moment de son exécution, quand la boucle est terminée depuis longtemps.
> 
> Variante qui fige la valeur au moment de l'enregistrement :
> 
> ```
> function planifier(file) {
>   for (const tache of file) {
>     setTimeout(function () { lancer(tache); }, 0);
>   }
> }
> ```
> 
> Comparez ce que lit chaque rappel : une liaison unique partagée dans le premier cas, une liaison propre à chaque tour dans le second. Reste à transposer cela au lien entre l'indice et le libellé affiché.

### 07-js-tri-numerique — sort sans comparateur trie des nombres comme du texte

`src/stats/mediane.js` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0011 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | non | non | 0,0003 $ | 3,2 s |
| Question | affiché | — | 1/1 | non | non | 0,0005 $ | 7,8 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0002 $ | 2,2 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0002 $ | 11,0 s |

**Indice** — affiché

Citation : `const triees = [...valeurs].sort();`

> Regardez la ligne « const triees = [...valeurs].sort(); ». Essayez de tête avec [10, 2, 33] : quel ordre obtenez-vous, et est-ce celui qu'attend le calcul du milieu ?

**Indice** — affiché

Citation : `const milieu = Math.floor(triees.length / 2);`

> Que renvoie l'appel avec un tableau vide ? Déroulez les indices utilisés dans chaque branche pour ce cas.

**Question** — affiché

> Question de contrôle : une médiane ne doit pas dépendre de l'ordre d'entrée. Appelez la fonction sur [10, 2, 33] puis sur [33, 2, 10] : les deux résultats sont-ils égaux ? Que vous apprend l'écart ?

**Piste** — affiché

> La notion en jeu : le tri par défaut compare des chaînes, pas des nombres. Documentez-vous sur MDN, page « Array.prototype.sort », section du paramètre de comparaison : elle décrit la fonction que sort attend pour imposer un ordre numérique et le signe que doit renvoyer son résultat. Vérifiez ensuite, sur vos deux appels, si l'écart observé disparaît une fois cet ordre imposé.

**Exemple** — affiché

> Cas voisin, sur des chaînes, pour observer le contrat du comparateur :
> 
> const mots = ['poire', 'kiwi', 'abricot'];
> const a = [...mots].sort();
> const b = [...mots].sort((x, y) => x.length - y.length);
> // a : ordre par défaut
> // b : ordre par longueur
> 
> Ce qui change n'est pas la valeur renvoyée mais son signe : négatif place x avant y, positif après, zéro laisse l'ordre indifférent. Le tri par défaut applique ce même mécanisme à une représentation textuelle.
> 
> Transposez : quel signe devez-vous obtenir pour que 10 se place après 2 ?

### 08-js-prix-flottants — Montants en euros flottants comparés avec ===

`src/caisse/reste.js` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0014 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | non | non | 0,0007 $ | 5,2 s |
| Question | affiché | — | 1/1 | oui | non | 0,0003 $ | 3,9 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0002 $ | 1,3 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0002 $ | 1,8 s |

**Indice** — affiché

Citation : `return total - dejaVerse === 0 ? "payé" : "à payer";`

> Regardez la valeur que vous comparez à 0 : elle vient d'une somme de nombres à virgule. Interrogez-vous sur ce que vaut exactement cette différence lorsque les prix comportent des décimales.

**Indice** — affiché

Citation : `for (const ligne of lignes) {`

> Observez ce que devient `total` si une ligne n'a pas de quantité, et ce que vaut `dejaVerse` s'il n'est pas fourni. Repérez où ces deux valeurs entrent dans le calcul.

**Question** — affiché

> En console, que vaut 0.1 + 0.2 - 0.3 ? Transposez : un total de 0,30 € et 0,30 € déjà versés. Que teste alors exactement votre === 0 ?

**Piste** — affiché

> La notion en jeu : les nombres JavaScript sont des flottants binaires (IEEE 754), donc la plupart des décimaux ne sont pas représentables exactement et une somme de prix peut ne pas retomber sur la valeur attendue. Documentez-vous sur MDN, pages « Number » et « Number.EPSILON », où la comparaison de flottants est traitée. Deux pistes de conception s'ouvrent ensuite, à trancher vous-même : soit comparer avec une tolérance, soit calculer en unités entières (centimes) pour que l'égalité redevienne fiable.

**Exemple** — affiché

> Deux cas voisins, jamais votre code :
> 
> const centimes = (e) => Math.round(e * 100);
> const a = centimes(0.10) + centimes(0.20); // 30
> const b = centimes(0.30);                  // 30
> a === b;                                  // true
> 
> const proche = (x, y, eps = 1e-9) => Math.abs(x - y) < eps;
> proche(0.1 + 0.2, 0.3); // true
> 
> À vous de choisir lequel sert votre intention, puis d'observer ce que chacun implique pour des valeurs proches mais distinctes.

### 09-py-defaut-mutable — Liste mutable comme valeur par défaut d'un paramètre

`inventaire/panier.py` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0019 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | oui | non | 0,0002 $ | 1,3 s |
| Question | affiché | — | 1/1 | non | non | 0,0003 $ | 1,5 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0002 $ | 1,2 s |
| Exemple | affiché | — | 1/1 | non | non | 0,0012 $ | 3,8 s |

**Indice** — affiché

Citation : `def ajouter(article, panier=[]):`

> Portez votre attention sur la valeur par défaut de `panier` : demandez-vous à quel moment précis cet objet est créé, et combien de fois cela se produit si la fonction est appelée plusieurs fois.

**Question** — affiché

> Appelez `ajouter` deux fois de suite sans fournir `panier`, puis comparez les deux valeurs de retour. Que constatez-vous sur le contenu du second ?

**Piste** — affiché

> La notion en jeu est l'évaluation des valeurs par défaut à la définition de la fonction, et non à chaque appel. Deux entrées de la documentation officielle la traitent : le tutoriel Python, section sur la définition des fonctions (paragraphe des valeurs par défaut), et la FAQ Python, entrée sur le partage des arguments par défaut entre les appels. Vous y trouverez aussi l'idiome de la valeur sentinelle, qui rend l'intention explicite.

**Exemple** — affiché

> Cas voisin, avec un dictionnaire au lieu d'une liste :
> 
> def ranger(cle, boite={}):
>     boite[cle] = True
>     return boite
> 
> x = ranger("vis")
> y = ranger("ecrou")
> print(x is y)
> print(y)
> 
> Exécutez ces lignes et lisez les deux sorties. Le second appel ne repart pas d'une boîte vide. Refaites l'essai en passant la boîte explicitement à chaque appel, puis comparez les deux résultats.

### 10-py-liaison-tardive — Fermetures créées dans une boucle : liaison tardive de la variable

`rapports/filtres.py` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0009 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0002 $ | 3,6 s |
| Question | affiché | — | 1/1 | non | non | 0,0002 $ | 1,8 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0002 $ | 2,9 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0003 $ | 3,2 s |

**Indice** — affiché

Citation : `filtres.append(lambda note: note >= limite)`

> Observez le lien entre la variable de boucle `limite` et le corps du lambda : à quel moment la valeur est-elle lue ? Testez avec deux limites distinctes et comparez ce que renvoient les fonctions obtenues.

**Question** — affiché

> Prenez deux limites, 5 et 10. Appelez les deux fonctions obtenues avec la même note, par exemple 7. Quels résultats obtenez-vous, et qu'attendiez-vous ? Quelle valeur de limite chaque fonction utilise-t-elle au moment où vous l'appelez ?

**Piste** — affiché

> La notion en jeu est la fermeture (closure) et sa liaison tardive : une fonction définie dans une boucle capture la variable, pas sa valeur du tour courant. La documentation à consulter est la FAQ Python, entrée sur les variables capturées dans les boucles, et la section sur les expressions lambda du tutoriel. Vous y verrez aussi les deux mécanismes idiomatiques qui forcent la capture d'une valeur fixe dès la création de la fonction ; comparez-les à votre boucle pour choisir celui qui exprime le mieux votre intention.

**Exemple** — affiché

> Cas voisin, autre domaine :
> 
> ```python
> regles = []
> for facteur in (2, 3):
>     regles.append(lambda x: x * facteur)
> print([r(10) for r in regles])   # ?
> 
> def regle(facteur):
>     return lambda x: x * facteur
> 
> regles = [regle(f) for f in (2, 3)]
> print([r(10) for r in regles])   # ?
> ```
> 
> Exécutez les deux blocs et comparez les listes affichées. Le premier montre l'effet de la liaison tardive ; le second fixe la valeur au moment de la création. Vous pourrez comparer au mécanisme de votre boucle.

### 11-py-liste-modifiee-en-parcours — Liste modifiée pendant qu'on la parcourt : des éléments sont sautés

`taches/nettoyage.py` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0015 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | oui | non | 0,0001 $ | 1,0 s |
| Question | affiché | — | 1/1 | non | non | 0,0002 $ | 1,3 s |
| Piste | affiché | — | 1/1 | non | non | 0,0004 $ | 4,8 s |
| Exemple | affiché | — | 1/1 | non | non | 0,0009 $ | 10,8 s |

**Indice** — affiché

Citation : `for tache in taches:`

> Regardez ce qui se passe sur la liste pendant que la boucle la parcourt. Que devient le curseur d'itération quand un élément est retiré par `taches.remove(tache)` ? Testez avec deux tâches terminées consécutives pour observer le résultat.

**Question** — affiché

> Prenez une liste de trois tâches toutes terminées. Combien de tâches voulez-vous voir rester en sortie ? Notez ce nombre, puis appelez votre fonction : le résultat correspond-il ? Si non, quel mécanisme de la boucle explique l'écart ?

**Piste** — affiché

> La notion en jeu : la mutation d'une séquence pendant qu'un itérateur la parcourt. En Python, la boucle `for` s'appuie sur un itérateur qui garde une position, et la suppression décale les éléments restants sous cette position.
> 
> À lire dans la documentation : le tutoriel Python, section sur les listes, où ce cas est décrit explicitement, ainsi que la page sur les types séquentiels et la fonction `filter`, qui illustre l'approche « construire un nouvel objet » plutôt que « modifier en place ».
> 
> Question à trancher avant d'écrire : votre fonction doit-elle renvoyer la même liste que celle reçue, ou peut-elle en renvoyer une autre ?

**Exemple** — affiché

> Un cas voisin, pour rendre visible la position de l'itérateur et l'état de la liste à chaque tour :
> 
> ```python
> nombres = [10, 20, 20, 30]
> for i, n in enumerate(nombres):
>     print(i, n, nombres)
> ```
> 
> Sortie :
> ```
> 0 10 [10, 20, 20, 30]
> 1 20 [10, 20, 20, 30]
> 2 20 [10, 20, 20, 30]
> 3 30 [10, 20, 20, 30]
> ```
> 
> Refaites la même trace en retirant l'élément quand `n == 20`, puis relevez quelle position n'est jamais visitée.

### 12-py-is-au-lieu-de-egal — is compare l'identité des objets, pas leur valeur

`comptes/verification.py` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0012 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0005 $ | 2,4 s |
| Question | affiché | — | 1/1 | oui | non | 0,0001 $ | 1,3 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0002 $ | 2,6 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0003 $ | 3,5 s |

**Indice** — affiché

Citation : `return saisi.strip() is attendu`

> Arrêtez-vous sur l'opérateur de comparaison dans `return saisi.strip() is attendu`. Testez à part, avec deux chaînes de même contenu construites séparément, et observez le résultat.

**Question** — affiché

> Deux questions pour trancher seul : sur quoi porte la comparaison que vous utilisez, la valeur portée par les deux chaînes ou l'identité des objets auxquels les noms se réfèrent ? Et dans votre fonction, les deux paramètres peuvent-ils désigner le même objet ?

**Piste** — affiché

> La notion en jeu est la distinction identité / égalité en Python : `is` interroge l'identité des objets, l'égalité de contenu se compare autrement. Documentation à relire : la section « Comparisons » du tutoriel Python (opérateurs `is`, `is not` et `==`) et la page `id()` de la bibliothèque standard. Un point à vérifier en passant : CPython peut réutiliser un même objet chaîne pour des littéraux identiques, ce qui rend un test avec deux littéraux peu concluant ; construisez plutôt vos chaînes par concaténation ou par `input`, pour que l'observation porte sur deux objets distincts.

**Exemple** — affiché

> Cas voisin, avec des listes plutôt que des chaînes :
> 
> ```python
> a = [1, 2]
> b = [1, 2]
> c = a
> print(a is b)   # False : deux objets distincts
> print(a == b)   # True  : même contenu
> print(a is c)   # True  : même objet
> ```
> 
> Et un piège de mesure : `x = 256; y = 256; print(x is y)` affiche True, mais `x = 257; y = 257` peut afficher False. L'observation dépend donc de la façon dont les objets sont créés, pas du contenu. Transposez : laquelle des deux colonnes de résultats correspond à la question que pose votre fonction ?

### 13-rs-unwrap-saisie — unwrap sur une saisie utilisateur : panique au lieu d'une erreur

`src/commande.rs` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0013 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | non | non | 0,0002 $ | 10,0 s |
| Question | affiché | — | 1/1 | oui | non | 0,0002 $ | 2,8 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0003 $ | 2,3 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0006 $ | 7,2 s |

**Indice** — affiché

Citation : `ligne.trim().parse().unwrap()`

> Que renvoie `parse` si la saisie contient autre chose que des chiffres, ou rien du tout ? Regardez le type attendu par `ligne.trim()` dans ce cas, et ce que fait `unwrap` face à un échec.

**Indice** — affiché

Citation : `io::stdin().read_line(&mut ligne).unwrap();`

> `read_line` renvoie une valeur qui compte les octets lus. Intéressez-vous à ce que signifie cette valeur quand l'entrée se termine sans rien contenir.

**Question** — affiché

> Dans votre fil, vous avez identifié qu'`unwrap` panique sur une entrée non numérique. Question : que devrait faire votre programme dans ce cas — s'arrêter, redemander la saisie, ou renvoyer une erreur à l'appelant ? La réponse change ce que la signature de `lire_quantite` doit exprimer.

**Piste** — affiché

> Piste : la notion en jeu est la propagation d'erreurs. `std::str::FromStr` définit le type d'erreur que produit `parse`, et `std::io::Error` celui de `read_line` — deux erreurs distinctes à faire cohabiter. Le chapitre « Gestion des erreurs » du Rust Book, ainsi que la documentation de `Result` (méthodes `map_err`, opérateur `?`) et de `FromStr`, montrent comment les faire remonter jusqu'à l'appelant. À vous de choisir ce que la signature de `lire_quantite` doit alors exprimer.

**Exemple** — affiché

> Exemple voisin (lecture d'un booléen depuis un fichier, pas votre stdin) :
> 
> ```rust
> use std::fs;
> use std::io;
> 
> fn lire_drapeau(chemin: &str) -> Result<bool, Box<dyn std::error::Error>> {
>     let contenu = fs::read_to_string(chemin)?;
>     let valeur = contenu.trim().parse::<bool>().map_err(|e| {
>         io::Error::new(io::ErrorKind::InvalidData, e)
>     })?;
>     Ok(valeur)
> }
> ```
> 
> Le `?` fait remonter l'erreur d'entrée/sortie ; `map_err` traduit l'erreur de `parse` en une autre erreur. La signature expose le cas d'échec plutôt que de faire tomber le programme.

### 14-rs-fenetre-glissante — Borne de boucle : la dernière fenêtre manque, et soustraction qui déborde

`src/stats.rs` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0014 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | non | non | 0,0005 $ | 5,7 s |
| Question | affiché | — | 1/1 | non | non | 0,0002 $ | 2,8 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0003 $ | 2,5 s |
| Exemple | rien | — | 1/1 | non | non | 0,0005 $ | 4,7 s |

**Indice** — affiché

Citation : `for debut in 0..valeurs.len() - fenetre`

> Prenez 5 valeurs et fenetre = 3, puis comptez sur papier le nombre de moyennes attendu et le nombre de tours de cette boucle. Regardez aussi ce que devient valeurs.len() - fenetre quand fenetre est plus grand que la longueur de la tranche.

**Indice** — affiché

Citation : `moyennes.push(somme / fenetre as f64);`

> Quelle valeur obtient-on ici si l'appelant passe fenetre = 0 ? Décidez si c'est un cas que la fonction accepte ou non, et où cela doit être vérifié.

**Question** — affiché

> Que doit renvoyer votre fonction si fenetre vaut 0 ? Combien de fenêtres une tranche contient-elle alors, et que produit votre division par fenetre as f64 ? Vérifiez ce cas avant de conclure.

**Piste** — affiché

> Deux notions à documenter. D'abord, le type des bornes : `usize` n'est pas signé, donc une soustraction qui « passe sous zéro » n'a pas de valeur représentable ; voyez la section sur les débordements d'entiers (panic en debug, enroulement en release) et les méthodes `checked_sub` / `saturating_sub` de `usize`. Ensuite, la découpe elle-même : `slice::windows`, dans la documentation de `[T]`, parcourt exactement les fenêtres d'une longueur donnée. Comparez le nombre de fenêtres qu'elle produit avec celui de votre boucle pour une tranche de longueur n.

**Exemple** — écarté : garde « pas de solution » : exemple qui reprend le code de la section

> Cas voisin, autre but, même forme de parcours :
> 
> fn ecarts(valeurs: &[f64]) -> Vec<f64> {
>     valeurs.windows(2)
>         .map(|paire| paire[1] - paire[0])
>         .collect()
> }
> 
> Avec 5 valeurs ce voisin produit 4 écarts ; windows(2) sur une tranche de longueur n en produit n-1. Le nombre de résultats dépend donc de la longueur de la tranche et de la taille de fenêtre, comme chez vous.
> 
> Point de comparaison sur les bornes : ce voisin ne soustrait jamais depuis un usize. Si la tranche fait moins de 2 éléments, il ne produit rien au lieu de paniquer. Regardez ce que donne votre borne `0..valeurs.len() - fenetre` quand la tranche est plus courte que fenetre.

### 15-rs-clone-pour-l-emprunteur — clone partout pour contourner l'emprunteur, et liste vide qui panique

`src/inventaire.rs` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0014 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | oui | non | 0,0003 $ | 2,0 s |
| Question | affiché | — | 1/1 | non | non | 0,0001 $ | 2,6 s |
| Piste | affiché | — | 1/1 | non | non | 0,0002 $ | 2,5 s |
| Exemple | affiché | — | 1/1 | non | non | 0,0008 $ | 5,5 s |

**Indice** — affiché

Citation : `let mut meilleur = self.articles[0].clone();`

> Que renvoie l'accès à un index sur un vecteur vide ? Regardez comment un inventaire sans articles traverserait cette fonction.

**Indice** — affiché

Citation : `for article in self.articles.clone()`

> Observez ce que produit ce clone sur la collection, puis ce que devient `article` dans la boucle. Comptez les copies intermédiaires que vous créez.

**Question** — affiché

> Si `self.articles` est vide, quelle ligne s'exécute en premier, et que se passe-t-il à cet instant ? Puis : un inventaire vide a-t-il un plus long article, et que devrait alors renvoyer l'appelant ?

**Piste** — affiché

> Votre boucle reproduit avec des index ce que l'itérateur sait déjà faire, et l'accès `[0]` introduit un cas d'échec que le type ne signale pas. Les méthodes d'agrégation de `Iterator` (`max`, `max_by_key`) rendent ce cas visible dans la signature, car elles renvoient une valeur facultative. Documentez-vous sur `std::iter::Iterator`, section « methods », et sur le type `Option` : cherchez laquelle de ces méthodes exprime « le plus long » sans index.

**Exemple** — affiché

> Cas voisin, sur des prix et non des chaînes :
> 
> fn plus_cher(prix: &[u32]) -> Option<u32> {
>     prix.iter().copied().max()
> }
> 
> match plus_cher(&[]) {
>     Some(p) => println!("{p}"),
>     None => println!("panier vide"),
> }
> 
> Le type de retour rend l'absence visible, et c'est l'appelant qui tranche. Transposez : chez vous, qui doit décider du cas vide, et que déclare votre signature actuelle ?

### 16-rs-iterateur-paresseux — map sans consommation : l'itérateur paresseux ne s'exécute jamais

`src/journal.rs` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0016 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | oui | non | 0,0002 $ | 1,2 s |
| Question | affiché | — | 1/1 | non | non | 0,0003 $ | 2,8 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0003 $ | 4,7 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0008 $ | 5,2 s |

**Indice** — affiché

Citation : `lignes.iter().map(|ligne| archive.push(ligne.to_uppercase()));`

> Regardez ce que renvoie `.map(|ligne| archive.push(...))` et cherchez qui consomme la valeur produite dans cette fonction. Le compilateur signale aussi un avertissement sur cette expression : lisez-le.

**Question** — affiché

> Écrivez un test : un vecteur de deux lignes, une archive vide, puis appelez la fonction et affichez la longueur de l'archive. Combien de lignes obtenez-vous, et qu'est-ce que cela vous dit sur le moment où le corps de la closure est exécuté ?

**Piste** — affiché

> La notion en jeu : itérateurs paresseux et distinction adaptateurs / consommateurs. `map` est un adaptateur : il ne fait rien tant qu'aucun consommateur ne parcourt l'itérateur. Où se documenter : la page de `std::iter::Iterator` (sections sur l'évaluation paresseuse et la liste des adaptateurs vs consommateurs), le chapitre « Iterators » du Rust Book, et la doc du lint `unused_must_use` pour comprendre l'avertissement déjà vu. Question pour vous situer : dans votre fonction, quel élément du code joue le rôle de consommateur ?

**Exemple** — affiché

> Un cas voisin : un adaptateur enchaîné, puis un consommateur qui déclenche le travail.
> 
> ```rust
> let valeurs = [1, 2, 3];
> 
> // adaptateur seul : aucun corps exécuté
> let carres = valeurs.iter().map(|v| v * v);
> 
> // consommateur : le calcul a lieu
> let total: i32 = valeurs.iter().map(|v| v * v).sum();
> println!("{total}");
> ```
> 
> Remarquez aussi la forme en commentaire du haut : le résultat de `carres` est gardé, rien n'est « perdu ». Quand une chaîne d'adaptateurs ne sert qu'à provoquer un effet de bord, le type produit est souvent `()` — c'est exactement ce que compte l'avertissement déjà vu. Comparez donc la valeur de retour de vos adaptateurs avec ce que la fonction est censée rendre.

### 17-adr-consequences-positives — ADR dont les conséquences ne listent aucun inconvénient

`docs/architecture/0002-connexion.md` · titulaire `architecte` · modèle `google/gemini-4-argon` · notion émergée · coût 0,0109 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0031 $ | 5,4 s |
| Question | affiché | — | 1/1 | oui | non | 0,0036 $ | 4,5 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0042 $ | 5,3 s |

**Indice** — affiché

Citation : `## Conséquences`

> Regardez la nature des points listés dans cette section. Pensez aussi à ce que la décision implique désormais pour l'envoi des messages et pour le parcours des voisins sur leur téléphone.

**Question** — affiché

> Vos deux points sont des avantages. Quels sont les inconvénients ou les nouvelles dépendances de ce choix ? Que se passe-t-il si un e-mail arrive après 15 minutes ou dans les indésirables ? Quel parcours un voisin doit-il suivre sur son téléphone pour passer de l'application à sa boîte mail, et quel critère vous ferait revenir sur cette décision ?

**Piste** — affiché

> Dans une décision d'architecture (modèle de Michael Nygard), la section « Conséquences » documente les compromis (trade-offs) : avantages, mais aussi coûts, risques et conditions de réévaluation. Renseignez-vous sur l'authentification par « lien magique » (magic link) et ses limites : la dépendance à la délivrabilité des e-mails (délais, filtres anti-spam), le risque de prélecture des liens par les serveurs de messagerie, et la friction sur mobile (changement d'application ou d'onglet). Documentez ces contraintes et le seuil qui invaliderait ce choix.

### 18-adr-decision-disproportionnee — Décision disproportionnée au regard des contraintes du contexte

`docs/architecture/0003-notifications.md` · titulaire `architecte` · modèle `google/gemini-4-argon` · notion émergée · coût 0,0103 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0031 $ | 4,8 s |
| Question | affiché | — | 1/1 | oui | non | 0,0031 $ | 3,5 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0042 $ | 6,9 s |

**Indice** — affiché

Citation : `un cluster Kafka de trois nœuds`

> Relisez la première phrase de votre section « Contexte » concernant l'infrastructure disponible, puis comparez-la à cette architecture.

**Question** — affiché

> Combien de mémoire vive et de machines physiques demande un cluster Kafka de trois nœuds, et de quelles ressources disposez-vous sur ce Raspberry Pi sans budget pour traiter cent messages par jour ?

**Piste** — affiché

> La notion en jeu est l'adéquation aux contraintes matérielles (dimensionnement) et l'évitement de la sur-ingénierie (principe YAGNI). Consultez la documentation officielle d'Apache Kafka à la rubrique « Hardware and OS » (section « Memory ») pour vérifier les prérequis d'un seul nœud en mémoire vive. Comparez ensuite ces exigences avec la fiche technique de votre modèle de Raspberry Pi. Dans un ADR, la décision doit rester compatible avec les contraintes physiques et budgétaires posées dans le contexte.

### 19-adr-nous-verrons — Cache sans stratégie d'invalidation, renvoyée à plus tard

`docs/architecture/0004-cache.md` · titulaire `architecte` · modèle `google/gemini-4-argon` · notion émergée · coût 0,0106 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | non | non | 0,0041 $ | 4,8 s |
| Question | affiché | — | 1/1 | oui | non | 0,0029 $ | 4,2 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0036 $ | 6,4 s |

**Indice** — affiché

Citation : `Nous verrons plus tard
s'il faut l'invalider.`

> Relisez la seconde phrase de la section « Contexte » et comparez-la à ce choix.

**Indice** — affiché

Citation : `Nous ajoutons un cache Redis devant l'API du catalogue.`

> Regardez si d'autres pistes (avant d'ajouter un composant dédié) ont été mentionnées.

**Question** — affiché

> Que verra une personne qui consulte le catalogue juste après qu'un voisin a retiré un objet, si le cache n'est pas invalidé ?

**Piste** — affiché

> Cette situation relève de la cohérence du cache (cache consistency) et des stratégies d'invalidation ou d'expiration (TTL, invalidation à l'écriture). Documentez-vous sur les patrons « Cache-Aside » et « Write-Through » dans la documentation officielle de Redis (section « Caching patterns ») pour choisir et décrire comment le catalogue reste à jour lorsque des objets sont ajoutés ou retirés.

### 20-conception-sans-contraintes — Document de conception qui choisit une solution sans dire les contraintes

`docs/design/recherche.md` · titulaire `architecte` · modèle `google/gemini-4-argon` · notion émergée · coût 0,0114 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | oui | non | 0,0041 $ | 4,4 s |
| Question | affiché | — | 1/1 | oui | non | 0,0033 $ | 3,7 s |
| Piste | affiché | — | 1/1 | non | non | 0,0039 $ | 4,7 s |

**Indice** — affiché

Citation : `Un cluster Elasticsearch indexe le catalogue en temps réel.`

> Regardez du côté de l'objectif énoncé dans le contexte et des hypothèses de volume (nombre de voisins et d'objets) qui sous-tendent ce choix d'infrastructure.

**Indice** — affiché

Citation : `## Solution`

> Observez la structure globale du document : voyez les pistes étudiées avant d'arriver à cette solution ainsi que les impacts opérationnels de ce choix.

**Question** — affiché

> Combien d'objets et de requêtes simultanées attendez-vous à l'échelle d'un voisinage, et quelles solutions plus simples (comme la base de données principale) avez-vous écartées avant de choisir un cluster dédié ?

**Piste** — affiché

> En architecture logicielle, ce point touche au dimensionnement (sizing), au principe YAGNI (You Aren't Gonna Need It) et au coût opérationnel (synchronisation entre deux sources de données, maintenance d'un cluster). Dans un ADR (Architecture Decision Record, format de Michael Nygard), il est attendu de documenter les ordres de grandeur dans le contexte, de lister les alternatives étudiées (comme la recherche plein texte native de votre base de données principale) avec la raison de leur rejet, et d'expliciter les conséquences négatives de la solution retenue.

