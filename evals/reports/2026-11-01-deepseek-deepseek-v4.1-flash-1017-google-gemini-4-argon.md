# Évaluation des mentors — 2026-11-01 13:02

37 cas, coût total 0,1969 $. Un modèle est retenu s'il ne propose aucune solution (motifs `must_not`, brouillons écartés compris) et fait émerger la notion (`must_mention`) dans 70 % des cas au moins, avec 80 % de citations valides au moins.

## Verdict par modèle

| Niveau · modèle | Cas | Notion émergée | Citations valides | Solution (brouillons) | Solution (affichée) | JSON invalides | Pannes | Casting différent | Coût | Jetons (entrée / sortie) | Durée médiane | Durée p90 | Verdict |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| fast · deepseek/deepseek-v4.1-flash-1017 | 21 | 19/21 (90 %) | 38/39 (97 %) | 1 | 1 | 0 | 0 | 0 | 0,0273 $ | 78792 / 40409 | 3,4 s | 6,9 s | **non retenu** |
| strong · google/gemini-4-argon | 16 | 14/16 (88 %) | 27/32 (84 %) | 0 | 0 | 0 | 0 | 0 | 0,1696 $ | 59223 / 5116 | 5,5 s | 16,1 s | **retenu** |

## Par sorte de cas

| Sorte | Cas | Réussis | Citations valides | Solution (brouillons) | Coût |
| --- | --- | --- | --- | --- | --- |
| Passe | 22 | 21 | 31/31 | 1 | 0,0869 $ |
| Revue | 9 | 6 | 26/31 | 0 | 0,0794 $ |
| Voisinage | 3 | 3 | 8/9 | 0 | 0,0051 $ |
| Réévaluation | 3 | 3 | — | 0 | 0,0256 $ |

Une revue réussit si une remarque affichée fait émerger la notion et cite un fichier attendu ; un cas de voisinage, si la passe avec le voisinage la fait émerger ; une réévaluation, si le verdict est celui attendu.

## Voisinage

| Cas | Notion (section seule) | Notion (voisinage) | Coût (section) | Coût (voisinage) |
| --- | --- | --- | --- | --- |
| 26-troc-voisinage-ordre-des-arguments | oui | oui | 0,0005 $ | 0,0014 $ |
| 31-budget-voisinage-periode-inversee | oui | oui | 0,0011 $ | 0,0012 $ |
| 36-taches-voisinage-rappels-utc | non | oui | 0,0002 $ | 0,0008 $ |

## Coût d'une revue

| Mentor | Modèle | Revues | Coût moyen | Jetons moyens (entrée / sortie) | Durée moyenne |
| --- | --- | --- | --- | --- | --- |
| `architecte` | google/gemini-4-argon | 4 | 0,0143 $ | 4375 / 555 | 16,2 s |
| `mentor-produit` | google/gemini-4-argon | 2 | 0,0103 $ | 3851 / 255 | 5,8 s |
| `testeur` | deepseek/deepseek-v4.1-flash-1017 | 3 | 0,0005 $ | 2804 / 1116 | 7,7 s |
## Coût d'une journée type

Usage : 40 enregistrements par rafales de cinq (9 h, 9 h 40, 10 h 20, 11 h, 14 h, 14 h 40, 15 h 20, 16 h), 2 ouvertures du projet (9 h, 14 h), 4 commits, 2 passages cités qui disparaissent. Réglages par défaut : calme 3 min, intervalle 20 min, 12 revues par jour et par mentor, déclencheurs create, open, change. Rejoué avec le planificateur de l'IDE ; coûts mesurés ci-dessus.

| Poste | Nombre par jour | Coût unitaire | Par jour |
| --- | --- | --- | --- |
| Revues `architecte` | 9 | 0,0143 $ | 0,1288 $ |
| Revues `mentor-produit` | 9 | 0,0103 $ | 0,0923 $ |
| Revues `testeur` | 8 | 0,0005 $ | 0,0043 $ |
| Réévaluations | 2 | 0,0085 $ | 0,0171 $ |
| Surcoût du voisinage (passes) | 40 | 0,0005 $ | 0,0210 $ |

**Par jour : 0,2635 $ · par mois (22 jours ouvrés) : 5,7966 $** — tient dans 10,0000 $ par mois.


## Cas

### 01-dates-par-reference — Comparaison de dates par référence et chevauchement non détecté

`src/emprunts/reserver.ts` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0018 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | non | non | 0,0008 $ | 5,0 s |
| Question | affiché | — | 1/1 | non | non | 0,0003 $ | 4,6 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0005 $ | 5,3 s |
| Exemple | affiché | — | 1/1 | non | non | 0,0003 $ | 2,8 s |

**Indice** — affiché

Citation : `e.debut === debut`

> Regardez cette comparaison : les deux côtés sont des `Date`. Demandez-vous ce que `===` compare réellement quand les opérandes sont des objets, puis ce que cela donne pour deux dates représentant le même instant.

**Indice** — affiché

Citation : `e.objetId === objet.id && e.debut === debut`

> Regardez ensuite l'ensemble des emprunts que cette condition retient. Parcourez mentalement la liste : un emprunt qui commence avant `debut` peut-il malgré tout recouvrir la période demandée ?

**Question** — affiché

> Prenez deux emprunts qui encadrent partiellement votre période sans partager la même date de début. Lesquels devraient bloquer la réservation ? Votre `find` les trouve-t-il ?

**Piste** — affiché

> Vous recherchez ici la logique du chevauchement d'intervalles temporels. Ce concept, souvent utilisé en programmation pour vérifier si deux périodes se croisent, est fondamental pour valider la disponibilité d'un objet.

**Exemple** — affiché

> Un cas voisin, sur des créneaux de salle : ramener chaque borne à un instant comparable, puis tester le recouvrement avec deux inégalités plutôt qu'une égalité. Notez qu'aucune borne n'a besoin d'être identique pour que les créneaux se croisent — et que la même forme sert pour vos dates de début et de fin, converties une fois avant la comparaison. Le TODO sur fin antérieure à début se traite avec la même conversion, avant tout test de recouvrement.

### 02-adr-sans-alternatives — ADR incohérent : hors ligne exigé, base hébergée choisie, aucune alternative

`docs/architecture/0001-stockage.md` · titulaire `architecte` · modèle `google/gemini-4-argon` · notion **non émergée** · coût 0,0104 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0033 $ | 3,1 s |
| Question | affiché | — | 1/1 | non | non | 0,0033 $ | 4,7 s |
| Piste | affiché | — | 1/1 | non | non | 0,0037 $ | 5,3 s |

**Indice** — affiché

Citation : `Nous stockons toutes les données dans une base PostgreSQL hébergée, interrogée
par une API REST.`

> Relisez la dernière phrase de la section Contexte en regard de cette décision.

**Question** — affiché

> Si un voisin consulte le catalogue depuis la cave sans réseau, comment son téléphone accède-t-il aux données si elles sont uniquement dans une base hébergée derrière une API REST ?

**Piste** — affiché

> Cette contrainte relève des architectures dites « offline-first » ou à stockage local avec synchronisation. Pour concilier une base distante et un usage sans réseau, renseignez-vous sur le stockage côté client (par exemple IndexedDB sur le web ou une base embarquée sur mobile) et sur les stratégies de synchronisation et de résolution de conflits au retour du réseau.

### 03-ts-foreach-async — forEach avec une fonction async : le compte est renvoyé avant les enregistrements

`src/import/importer.ts` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0017 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | non | non | 0,0003 $ | 2,7 s |
| Question | affiché | — | 1/1 | oui | non | 0,0004 $ | 6,4 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0005 $ | 2,1 s |
| Exemple | rien | — | 1/1 | non | non | 0,0005 $ | 7,2 s |

**Indice** — affiché

Citation : `fiches.forEach(async (fiche) => {`

> Regardez ce que la méthode de parcours fait de la valeur de retour de son callback, et à quel moment la fonction rend la main. Que se passe-t-il pour ce compteur pendant ce temps ?

**Indice** — affiché

Citation : `return enregistrees;`

> À cette ligne, l'instruction s'exécute avant que les enregistrements aient abouti. Voyez aussi l'ordre de passage des fiches : votre intention est-elle un traitement une par une ou simultané ?

**Question** — affiché

> À l'instant où l'expression `return enregistrees` est évaluée, combien d'appels à `enregistrer` ont réellement terminé ? Quelle valeur la promesse rend-elle alors, comparée au nombre de fiches reçues ?

**Piste** — affiché

> La notion en jeu est le séquencement d'opérations asynchrones : un parcours qui n'attend pas ses callbacks s'oppose à une boucle qui attend chaque étape avant la suivante. Voyez la note de la documentation de `Array.prototype.forEach` au sujet des callbacks asynchrones, puis les pages sur `Promise.all` et `for await...of`. Comparez ce que chacune garantit sur l'ordre des appels et sur l'instant où la fonction rend la main.

**Exemple** — écarté : garde « pas de solution » : exemple qui recopie un fragment de la section

> Cas voisin, sans votre code :
> 
> ```ts
> const delais = [10, 20, 30];
> let total = 0;
> 
> delais.forEach(async (d) => {
>   await new Promise((r) => setTimeout(r, d));
>   total += d;
> });
> 
> console.log(total); // affiche 0
> ```
> 
> Le parcours rend la main aussitôt ; le total est lu avant qu'aucun callback n'ait fini. Menez la même accumulation en attendant chaque étape avant de passer à la suivante : le total final diffère. Repérez lequel des deux totaux correspond à ce que votre signature `Promise<number>` laisse attendre.

### 04-ts-find-undefined — find peut renvoyer undefined, masqué par une assertion non nulle

`src/panier/total.ts` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0015 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0005 $ | 3,0 s |
| Question | affiché | — | 1/1 | non | non | 0,0002 $ | 3,4 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0003 $ | 2,7 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0005 $ | 1,9 s |

**Indice** — affiché

Citation : `return produit!.prix;`

> Que renvoie `find` lorsqu'aucun produit ne porte cette référence ? Regardez ensuite ce que le `!` affirme au compilateur, et ce que votre signature `: number` promet à l'appelant dans ce même cas.

**Question** — affiché

> Supposez qu'un appelant affiche le résultat sans rien vérifier : quel nombre obtient-il quand la référence est absente ? Ce nombre peut-il être distingué d'un vrai prix par l'appelant, avec la signature actuelle ?

**Piste** — affiché

> La notion en jeu est l'absence de valeur de retour : en TypeScript, une fonction qui peut ne rien trouver l'exprime par une union avec `undefined` dans son type de retour, et l'appelant doit restreindre ce cas avant d'utiliser la valeur (narrowing). À documenter dans le manuel TypeScript : la page « Everyday Types » (sections unions et narrowing) et l'option de compilation `strictNullChecks`, qui explique pourquoi le typeur laisse passer une affirmation comme `!`. Votre signature actuelle exclut cette éventualité par construction.

**Exemple** — affiché

> Cas voisin, hors de votre domaine :
> 
> ```ts
> function auteurDe(id: string): Auteur | undefined {
>   return auteurs.find((a) => a.id === id);
> }
> 
> const auteur = auteurDe("a1");
> if (auteur === undefined) {
>   return "inconnu";
> }
> return auteur.nom;
> ```
> 
> L'union dans la signature oblige l'appelant à traiter l'absence : le typeur refuse l'accès à `nom` avant le test. La valeur retournée est alors soit un `Auteur`, soit `undefined`, et l'appelant peut distinguer les deux cas, contrairement à un nombre qui serait toujours un nombre.

### 05-ts-tri-sur-place — sort trie sur place le tableau reçu en paramètre

`src/agenda/tri.ts` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0014 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | non | non | 0,0005 $ | 2,7 s |
| Question | affiché | — | 1/1 | non | non | 0,0001 $ | 2,8 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0006 $ | 25,3 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0001 $ | 2,8 s |

**Indice** — affiché

Citation : `.sort((a, b) => a.debut.getTime() - b.debut.getTime())`

> Regardez ce que Array.prototype.sort fait au tableau sur lequel il est appelé, et qui détient ce tableau une fois la fonction rendue. Que devient le tableau de l'appelant ?

**Indice** — affiché

Citation : `.slice(0, n)`

> Regardez la signature de slice et les valeurs que n peut recevoir chez les appelants : que renvoie-t-il pour 0, pour une valeur négative, pour une valeur non entière ?

**Question** — affiché

> Si l'appelant affiche son tableau après l'appel, dans quel ordre le voit-il ? Et si deux appels successifs sur le même tableau ne donnent pas le même résultat, qu'est-ce que cela vous apprend sur la fonction ?

**Piste** — affiché

> La notion en jeu : méthode mutante contre méthode non mutante de la bibliothèque standard. Documentez-vous sur MDN, aux pages Array.prototype.sort (section sur la modification en place) et Array.prototype.toSorted, puis sur la copie superficielle d'un tableau. Regardez aussi comment les autres fichiers de src/agenda qui trient des listes s'y prennent : l'écart avec votre version vous montrera la convention retenue dans ce projet. Enfin, demandez-vous ce que devient le contrat de votre fonction si l'appelant a besoin de l'ordre d'origine après l'appel.

**Exemple** — affiché

> Un cas voisin, sur des nombres plutôt que des créneaux :
> 
> ```ts
> function plusPetits(valeurs: number[], n: number): number[] {
>   return [...valeurs]
>     .sort((a, b) => a - b)
>     .slice(0, n);
> }
> ```
> 
> L'appelant garde son tableau intact ; la copie porte le tri. Comparez avec une variante sans copie : `valeurs.sort(...)` renvoie le même tableau, réordonné pour tout le monde. La question reste la même pour votre fonction : qui subit le tri, et est-ce voulu ?

### 06-js-var-boucle — var dans une boucle : toutes les fonctions voient la même variable

`src/ui/boutons.js` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0012 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0001 $ | 1,8 s |
| Question | affiché | — | 1/1 | non | non | 0,0005 $ | 4,3 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0002 $ | 2,8 s |
| Exemple | affiché | — | 1/1 | non | non | 0,0004 $ | 4,9 s |

**Indice** — affiché

Citation : `for (var i = 0; i < boutons.length; i++) {`

> Regardez à quel moment la fonction de rappel lit `i`, et quand `i` change encore après l'avoir enregistrée. Cliquez mentalement sur le dernier bouton une fois la boucle terminée.

**Question** — affiché

> Après la fin de la boucle, combien de valeurs différentes `i` peut-il encore valoir ? Cliquez sur le premier bouton puis sur le dernier : que vous dit l'écart entre les deux messages sur ce que lit le rappel au moment du clic ?

**Piste** — affiché

> La notion en jeu est la portée de `var` : une seule liaison pour toute la fonction, contrairement à `let`, qui en crée une nouvelle à chaque itération. Côté documentation, voyez sur MDN les pages « var » (section portée) et « Fermetures / Closures », ainsi que la page « let » pour la différence d'environnement de liaison dans une boucle. Une fois la distinction vue, vous pourrez décider ce que doit capturer le rappel : la valeur au moment du clic ou celle de l'itération.

**Exemple** — affiché

> Un cas voisin, hors de votre code : enregistrer un rappel dans une boucle et vouloir qu'il garde le numéro de son tour.
> 
> for (var n = 3; n > 0; n--) {
>   setTimeout(function () { console.log(n); }, 0);
> }
> for (let k = 3; k > 0; k--) {
>   setTimeout(function () { console.log(k); }, 0);
> }
> 
> Le premier bloc affiche trois fois la même valeur, lue à l'exécution du rappel. Le second affiche 3, 2, 1. Comparez la durée de vie de la liaison dans les deux écritures, puis transposez la question à votre boucle : quel numéro doit afficher chaque clic ?

### 07-js-tri-numerique — sort sans comparateur trie des nombres comme du texte

`src/stats/mediane.js` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0015 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | non | non | 0,0003 $ | 6,2 s |
| Question | affiché | — | 1/1 | non | non | 0,0004 $ | 5,1 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0006 $ | 8,1 s |
| Exemple | affiché | — | 1/1 | non | **oui** : `sort((a, b) => a - b)` | 0,0003 $ | 5,3 s |

**Indice** — affiché

Citation : `const triees = [...valeurs].sort();`

> Sur quelles valeurs avez-vous essayé ? Testez avec [10, 9, 2] et regardez l'ordre de « triees » avant de rendre le résultat.

**Indice** — affiché

Citation : `if (triees.length % 2 === 0) {`

> Que renvoie la fonction quand « valeurs » est vide ? Suivez la branche prise par cette condition dans ce cas.

**Question** — affiché

> Prenez un tableau vide, puis un tableau à un seul élément : que renvoie votre fonction dans chaque cas ? Le résultat correspond-il à ce qu'un appelant peut attendre d'une médiane ? Ce sont deux entrées que votre code traverse sans les distinguer.

**Piste** — affiché

> Deux notions sont en jeu ici. Côté tri : la fonction de comparaison attendue par sort ; la page MDN de Array.prototype.sort décrit son paramètre compareFn, et celle sur la comparaison de nombres explique comment comparer deux éléments sans passer par les chaînes. Côté entrée : le contrat de la fonction (tableau vide, un seul élément) ; écrivez-le en JSDoc avant de coder, pour que chaque cas limite soit une décision explicite plutôt qu'un effet de bord. Ces deux points se vérifient sur vos propres essais.role

**Exemple** — affiché

> Un cas voisin, sur des données qui ne sont pas les vôtres :
> 
> ```js
> const ages = [12, 105, 3];
> const telQuel = [...ages].sort();
> const parNombre = [...ages].sort((a, b) => a - b);
> console.log(telQuel);    // [105, 12, 3]
> console.log(parNombre);  // [3, 12, 105]
> ```
> 
> Les deux lignes partent du même tableau et donnent deux ordres. Regardez ce que reçoit la fonction de comparaison, puis rejouez la même chose sur vos propres valeurs et sur un tableau vide : c'est l'écart entre l'ordre obtenu et l'ordre attendu qui vous indiquera quoi corriger.

### 08-js-prix-flottants — Montants en euros flottants comparés avec ===

`src/caisse/reste.js` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0020 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | oui | non | 0,0006 $ | 4,8 s |
| Question | affiché | — | 1/1 | non | non | 0,0004 $ | 2,2 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0002 $ | 2,1 s |
| Exemple | rien | — | 1/1 | non | non | 0,0007 $ | 3,3 s |

**Indice** — affiché

Citation : `total - dejaVerse === 0`

> Arrêtez-vous sur cette ligne. Rejouez-la à la main avec des prix à centimes (0,10 et 0,20 par exemple) : regardez la valeur exacte de la différence avant la comparaison.

**Indice** — affiché

Citation : `dejaVerse`

> Regardez ce second paramètre à l'entrée de la fonction : demandez-vous ce qu'il vaut si l'appelant ne le passe pas, et suivez ce que devient alors le résultat.

**Question** — affiché

> Si dejaVerse dépasse le total, que vaut la différence, et quelle chaîne votre fonction renvoie-t-elle ? Est-ce le résultat attendu par l'appelant ? Cherchez dans le voisinage un appel qui verse un montant supérieur au total, puis vérifiez ce cas à la main.

**Piste** — affiché

> La notion en jeu est l'arithmétique monétaire : les nombres à virgule du langage ne représentent pas exactement les centimes. Documentez-vous sur le type Number et sa précision (MDN, « Number » et « floating point »), puis sur la pratique consistant à manipuler des centimes entiers. Cela règle d'un coup les deux cas abordés : l'égalité à zéro et le sens d'une différence négative.

**Exemple** — écarté : garde « pas de solution » : exemple qui recopie un fragment de la section

> Un cas voisin, un stock compté en grammes entiers, avec trois issues distinctes :
> 
> function resteStock(sacs, sorti) {
>   let total = 0;
>   for (const sac of sacs) {
>     total += sac.poidsGrammes * sac.nombre;
>   }
>   const diff = total - sorti;
>   if (diff > 0) return "reste";
>   if (diff === 0) return "vide";
>   return "manquant";
> }
> 
> Trois points à transposer : l'unité entière, la différence nommée avant comparaison, et le cas négatif nommé au lieu d'être fondu dans le cas positif.

### 09-py-defaut-mutable — Liste mutable comme valeur par défaut d'un paramètre

`inventaire/panier.py` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0008 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0001 $ | 1,0 s |
| Question | affiché | — | 1/1 | non | non | 0,0001 $ | 1,6 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0001 $ | 3,4 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0005 $ | 6,9 s |

**Indice** — affiché

Citation : `def ajouter(article, panier=[]):`

> Appelez cette fonction deux fois sans second argument, puis observez le panier renvoyé au second appel. Comparez avec un appel où vous passez explicitement une liste vide.

**Question** — affiché

> Au second appel sans argument, quel objet précis est modifié par append ? Demandez-vous ensuite : si deux voisins empruntent chacun leur tour, à qui appartient le livre affiché dans le panier du premier ? Vous pouvez le vérifier en comparant les identités des deux paniers renvoyés.

**Piste** — affiché

> La notion en jeu : une valeur par défaut est évaluée une seule fois, à la définition de la fonction, et non à chaque appel. Elle est donc partagée par tous les appels qui ne fournissent pas l'argument.
> 
> Pour vous documenter : le tutoriel Python officiel, section « Valeurs par défaut des arguments » (chapitre « Pour en savoir plus sur la définition des fonctions »), et la FAQ Python « Why are default values shared between objects? ».
> 
> Le point à trancher pour ce panier : distinguer l'absence d'argument de la fourniture d'un conteneur vide, sachant que la valeur par défaut ne peut pas être construite à chaque appel.

**Exemple** — affiché

> Cas voisin, autre domaine :
> 
> ```python
> import time
> 
> def horodater(evenement, instant=time.time()):
>     return evenement, instant
> 
> a = horodater("ouverture")
> time.sleep(2)
> b = horodater("fermeture")
> print(a[1] == b[1])
> ```
> 
> À votre avis, qu'affiche la dernière ligne, et pourquoi ? Le second appel ne produit pas un instant plus récent. La valeur par défaut n'a été calculée qu'une fois, au moment où la fonction a été définie.
> 
> Transposez : dans votre fonction, l'objet par défaut est un conteneur, et non un instant. Le point à trancher reste le même — comment le corps de la fonction peut-il distinguer « aucun panier fourni » de « panier fourni » ?

### 10-py-liaison-tardive — Fermetures créées dans une boucle : liaison tardive de la variable

`rapports/filtres.py` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0010 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0002 $ | 2,3 s |
| Question | affiché | — | 1/1 | non | non | 0,0001 $ | 1,3 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0003 $ | 4,4 s |
| Exemple | affiché | — | 1/1 | non | non | 0,0004 $ | 3,9 s |

**Indice** — affiché

Citation : `filtres.append(lambda note: note >= limite)`

> Après `seuils([0, 10])`, appelez chacun des deux filtres retournés avec la même note et comparez les résultats. Regardez ensuite à quel moment la valeur de `limite` est lue par chaque lambda : à sa création, ou à son appel.

**Question** — affiché

> Combien de variables `limite` distinctes existent dans cette fonction : une par tour de boucle, ou une seule partagée ? Que vaut-elle quand les lambdas sont appelés après le `return` ?

**Piste** — affiché

> La notion en jeu est la fermeture (closure) : la lambda capture la variable, pas sa valeur. La FAQ de programmation Python traite exactement ce cas (« Pourquoi les lambdas définies dans une boucle partagent-elles la même valeur ? ») ; le tutoriel, section « Fermetures », explique la portée. Deux pistes de documentation pour rendre la liaison sûre : le paramètre par défaut (lié à la création) et `functools.partial` (bibliothèque standard, module functools). Comparez ce que chacun fige : la valeur de `limite`, ou la variable elle-même.

**Exemple** — affiché

> Considérez la séquence suivante :
> limites = [0, 100]
> filtres = seuils(limites)
> 
> print(filtres[0](5))
> print(filtres[1](5))

### 11-py-liste-modifiee-en-parcours — Liste modifiée pendant qu'on la parcourt : des éléments sont sautés

`taches/nettoyage.py` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0010 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0001 $ | 2,4 s |
| Question | affiché | — | 1/1 | non | non | 0,0003 $ | 3,0 s |
| Piste | affiché | — | 1/1 | non | non | 0,0002 $ | 2,7 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0004 $ | 4,4 s |

**Indice** — affiché

Citation : `for tache in taches:`

> Que devient le parcours de `for tache in taches` si la longueur de la liste change pendant ce parcours ? Testez avec deux tâches terminées consécutives et comparez le résultat attendu au résultat obtenu. Vous verrez si chaque élément est bien visité.

**Question** — affiché

> Sur trois tâches où seules la 1re et la 2e sont terminées, combien en reste-t-il après l'appel ? Refaites l'essai avec la 2e et la 3e terminées : le nombre de retraits change-t-il selon la position ? Ce qui varie selon l'ordre indique ce que votre parcours ne garantit pas.

**Piste** — affiché

> La notion en jeu : un itérateur de liste s'appuie sur les indices internes, et toute suppression en cours de parcours décale ces indices. Voir la doc de `list.remove` et du tutoriel sur `for`, qui signalent la modification en place. Autre notion à explorer : le filtrage, qui construit une nouvelle liste au lieu de modifier celle qu'on parcourt — voyez les compréhensions de liste et `filter` dans la doc. À vous de choisir laquelle exprime votre intention.

**Exemple** — affiché

> Cas voisin, sur des nombres et non des tâches :
> 
> nombres = [2, 4, 5, 6]
> for n in nombres:
>     if n % 2 == 0:
>         nombres.remove(n)
> print(nombres)   # [4, 5]
> 
> garde = [n for n in nombres if n % 2]
> print(garde)     # [5]
> 
> Le premier bloc saute 4 : l'indice avance après un retrait. Le second construit une nouvelle liste sans toucher à celle qu'il parcourt.
> 
> Reproduisez les deux formes avec vos tâches et comparez le nombre d'éléments retirés à celui attendu.

### 12-py-is-au-lieu-de-egal — is compare l'identité des objets, pas leur valeur

`comptes/verification.py` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0012 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0002 $ | 1,5 s |
| Question | affiché | — | 1/1 | non | non | 0,0003 $ | 2,2 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0002 $ | 2,4 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0005 $ | 3,0 s |

**Indice** — affiché

Citation : `return saisi.strip() is attendu`

> À la ligne `return saisi.strip() is attendu`, arrêtez-vous sur l'opérateur placé entre les deux opérandes. Demandez-vous ce qu'il teste quand deux chaînes ont le même contenu mais proviennent d'objets distincts.

**Question** — affiché

> Dans une console, construisez deux chaînes de contenu identique mais provenant d'objets distincts : quel test vous montre que votre opérateur actuel ne les reconnaît pas égales, et quelle comparaison viseriez-vous alors ?

**Piste** — affiché

> Notion en jeu : identité d'objet contre égalité de valeur. En Python, deux notions distinctes sont documentées séparément. Voyez la section « Comparisons » du tutoriel de référence (rubrique « Value comparisons » / « Identity comparisons ») et la page du type `str` pour la comparaison de contenu. Le point à retenir : l'identité porte sur l'objet en mémoire, pas sur ce qu'il contient ; elle dépend donc de la façon dont la chaîne a été produite, y compris après un appel comme `strip`. C'est ce qui rend le résultat variable d'un appel à l'autre.

**Exemple** — affiché

> Cas voisin, sans lien avec votre contrôle :
> 
> a = "cle" + "-" + "1"
> b = "cle-1"
> print(a == b)   # True
> print(a is b)   # False
> 
> Même contenu, objets différents : `==` compare le contenu, `is` compare l'objet. La sortie de `is` dépend de la façon dont la chaîne a été produite et de l'interpréteur ; elle peut varier d'une exécution à l'autre. Refaites l'essai avec une chaîne obtenue par un appel de fonction plutôt qu'écrite en dur, et observez ce que chaque opérateur répond.

### 13-rs-unwrap-saisie — unwrap sur une saisie utilisateur : panique au lieu d'une erreur

`src/commande.rs` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0013 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | non | non | 0,0007 $ | 3,0 s |
| Question | affiché | — | 1/1 | non | non | 0,0003 $ | 5,1 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0004 $ | 2,4 s |

**Indice** — affiché

Citation : `io::stdin().read_line(&mut ligne).unwrap();`

> Observez la valeur que read_line renvoie : elle est ignorée ici. Que se passe-t-il si la lecture ne ramène aucun octet ?

**Indice** — affiché

Citation : `ligne.trim().parse().unwrap()`

> Regardez la signature de la fonction et le cas d'une saisie vide ou non numérique : par où cette information pourrait-elle ressortir ?

**Question** — affiché

> Question à trancher vous-même : lancez le programme sans rien taper puis fermez l'entrée (Ctrl-D). Combien d'octets sont lus, que contient alors `ligne`, et que renvoie le `parse` sur ce contenu ? Le résultat correspond-il à une quantité valide pour l'appelant ?

**Piste** — affiché

> La notion en jeu : une fonction qui peut échouer ne devrait pas renvoyer une valeur brute. Regardez la signature de `parse` dans la doc de `str::parse` et le trait `FromStr` : le type de retour y porte déjà l'échec possible. Voyez ensuite le chapitre du livre Rust sur la propagation des erreurs (`Result`, opérateur `?`). Demandez-vous ce que votre appelant peut faire d'un `u32` obtenu sans savoir si l'entrée était vide ou non numérique.

### 14-rs-fenetre-glissante — Borne de boucle : la dernière fenêtre manque, et soustraction qui déborde

`src/stats.rs` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0016 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | oui | non | 0,0006 $ | 5,3 s |
| Question | affiché | — | 1/1 | oui | non | 0,0002 $ | 3,5 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0002 $ | 5,1 s |
| Exemple | affiché | — | 1/1 | non | non | 0,0006 $ | 7,5 s |

**Indice** — affiché

Citation : `for debut in 0..valeurs.len() - fenetre`

> Comptez les positions de départ possibles pour une fenêtre de largeur fenetre dans une slice de longueur valeurs.len(). Comparez avec les valeurs prises par debut dans votre plage, en particulier quand les deux longueurs sont égales.

**Indice** — affiché

Citation : `let somme: f64 = valeurs[debut..debut + fenetre].iter().sum();`

> Le paramètre fenetre est utilisé sans contrainte préalable. Regardez ce que produisent la soustraction de la plage et la division quand fenetre vaut 0, ou quand il dépasse la longueur de valeurs.

**Question** — affiché

> Votre plage de départ dépend d'une soustraction sur des entiers non signés. Que renvoie la fonction quand fenetre vaut 0, et quand fenetre dépasse valeurs.len() ? Écrivez ces deux appels dans un test avant de répondre.

**Piste** — affiché

> Deux notions à nommer ici. D'abord le comptage des fenêtres : le nombre de positions de départ vaut longueur moins largeur plus un, et la borne haute d'un Range est exclue — voyez la page « Ranges » du livre Rust et la doc de std::ops::Range. Ensuite l'arithmétique entière non signée : usize ne peut pas être négatif, la soustraction panique en debug et enveloppe en release — voyez la section sur les débordements d'entiers dans le livre, chapitre « Types de données », et la doc de usize::checked_sub. Ces deux points se vérifient par un test, pas à la lecture.

**Exemple** — affiché

> Cas voisin, à exécuter tel quel :
> 
> ```rust
> fn voisins(xs: &[u8]) -> Vec<(u8, u8)> {
>     let mut v = Vec::new();
>     for i in 0..xs.len() - 1 {
>         v.push((xs[i], xs[i + 1]));
>     }
>     v
> }
> 
> fn main() {
>     println!("{:?}", voisins(&[1, 2, 3]));
>     println!("{:?}", voisins(&[]));
> }
> ```
> 
> Comptez les départs utiles, la valeur écrite dans le Range, et ce que fait usize quand la longueur est inférieure au retrait. Transposez aux cas fenetre == valeurs.len() et fenetre == 0.

### 15-rs-clone-pour-l-emprunteur — clone partout pour contourner l'emprunteur, et liste vide qui panique

`src/inventaire.rs` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0011 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0003 $ | 2,6 s |
| Question | affiché | — | 1/1 | non | non | 0,0002 $ | 2,8 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0003 $ | 3,5 s |
| Exemple | affiché | — | 1/1 | non | non | 0,0004 $ | 4,0 s |

**Indice** — affiché

Citation : `let mut meilleur = self.articles[0].clone();`

> Arrêtez-vous sur `let mut meilleur = self.articles[0].clone();` avant la boucle. Que doit produire cette méthode si l'inventaire ne contient aucun article ? Écrivez le cas d'appel correspondant et observez.

**Question** — affiché

> Votre signature renvoie un String : pour un inventaire vide, quelle valeur l'appelant recevrait-il, et comment le distinguerait-il d'un article réellement vide ? Cherchez les types de la bibliothèque standard qui expriment « peut-être rien » et voyez ce que cela change pour l'appelant. Vérifiez aussi ce que fait votre boucle sur un seul article.

**Piste** — affiché

> Deux notions à documenter. D'abord `Option<T>` : voyez la page de `std::option::Option` et la section du livre sur la gestion des erreurs ; elle décrit comment une fonction peut dire « rien » sans mentir sur le type de retour. Ensuite, parcourir sans copier : `iter()` emprunte, là où votre boucle matérialise une seconde collection. Comparez avec `into_iter()`. La page « Iterator » de la documentation liste les méthodes d'agrégation (`max_by_key`, par exemple) qui remplacent une boucle de comparaison manuelle ; regardez leur type de retour, il rejoint la première notion. Le chapitre sur les références et l'emprunt éclaire pourquoi la copie est inutile ici.

**Exemple** — affiché

> Cas voisin, sans rapport avec votre inventaire : une recherche de maximum, où l'absence est portée par le type de retour.
> 
> ```rust
> fn plus_grand(valeurs: &[i32]) -> Option<i32> {
>     valeurs.iter().copied().max()
> }
> 
> fn main() {
>     match plus_grand(&[]) {
>         Some(v) => println!("trouve {v}"),
>         None => println!("aucun"),
>     }
> }
> ```
> 
> Trois choses à transposer : le retour Option, le parcours par `iter()` sur une référence, et l'appelant qui doit traiter les deux cas. `max` renvoie l'élément ; `max_by_key` prend une clé de comparaison.

### 16-rs-iterateur-paresseux — map sans consommation : l'itérateur paresseux ne s'exécute jamais

`src/journal.rs` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0015 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0007 $ | 3,8 s |
| Question | affiché | — | 1/1 | oui | non | 0,0002 $ | 2,3 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0002 $ | 1,7 s |
| Exemple | affiché | — | 1/1 | oui | non | 0,0005 $ | 6,5 s |

**Indice** — affiché

Citation : `lignes.iter().map(|ligne| archive.push(ligne.to_uppercase()));`

> Consultez la documentation de Iterator::map : regardez ce qu'elle renvoie et à quel moment le closure est réellement exécuté. Ajoutez ensuite une vérification simple après l'appel : combien d'éléments contient archive ?

**Question** — affiché

> Quand archiver rend la main, quelqu'un a-t-il consommé l'itérateur produit par lignes.iter().map(...) ? Regardez la signature de la fonction : rien ne renvoie cet itérateur, et personne ne l'itère à l'intérieur. Que peut-il donc se passer ?

**Piste** — affiché

> Nommez la distinction que la bibliothèque standard fait entre un adaptateur d'itérateur et un consommateur d'itérateur. La documentation de std::iter::Iterator comporte une section qui les sépare, et le chapitre « Iterators » du livre officiel (doc.rust-lang.org/book) décrit la même séparation. Cherchez ensuite, dans la liste des consommateurs, ceux qui écrivent dans une collection existante. C'est cette famille que votre fonction semble viser, et c'est aussi elle qui décide quand le closure s'exécute.

**Exemple** — affiché

> Cas voisin, avec des entiers :
> 
> ```rust
> // adaptateur seul : le closure n'est jamais appelé,
> // donc rien n'est calculé ici
> let _carres = vec![1, 2, 3].iter().map(|n| n * n);
> 
> // consommateur : le closure s'exécute tout de suite
> let mut total = 0;
> vec![1, 2, 3].iter().for_each(|n| total += n);
> // total vaut 6
> ```
> 
> La différence entre les deux lignes tient uniquement au consommateur en fin de chaîne. Repérez, dans votre fonction, laquelle des deux formes vous avez écrite, puis ce qui manque dans la chaîne.

### 17-adr-consequences-positives — ADR dont les conséquences ne listent aucun inconvénient

`docs/architecture/0002-connexion.md` · titulaire `architecte` · modèle `google/gemini-4-argon` · notion émergée · coût 0,0117 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0036 $ | 4,2 s |
| Question | affiché | — | 1/1 | non | non | 0,0037 $ | 11,6 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0043 $ | 6,1 s |

**Indice** — affiché

Citation : `## Conséquences

- Plus de mots de passe à gérer.
- Le gardien n'a plus de réinitialisations à faire.`

> Regardez la section « Conséquences » au regard de la décision d'envoyer un e-mail valable 15 minutes. Seuls les effets positifs y figurent.

**Question** — affiché

> Que se passe-t-il pour un voisin sur son téléphone si l'e-mail met plus de 15 minutes à arriver, tombe dans les indésirables, ou si le service d'envoi d'e-mails est indisponible ?

**Piste** — affiché

> Dans un ADR (format de Michael Nygard), la section « Conséquences » recense les compromis (*trade-offs*) : avantages, inconvénients, risques nouveaux et conditions qui feraient réévaluer le choix. Pour les liens de connexion (*magic links*), documentez-vous sur la délivrabilité des e-mails transactionnels, la dépendance à un fournisseur SMTP tiers et le parcours utilisateur sur mobile (changement d'application entre le navigateur et la messagerie).

### 18-adr-decision-disproportionnee — Décision disproportionnée au regard des contraintes du contexte

`docs/architecture/0003-notifications.md` · titulaire `architecte` · modèle `google/gemini-4-argon` · notion émergée · coût 0,0112 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0033 $ | 3,7 s |
| Question | affiché | — | 1/1 | oui | non | 0,0036 $ | 5,9 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0044 $ | 5,2 s |

**Indice** — affiché

Citation : `Nous déployons un cluster Kafka de trois nœuds`

> Relisez cette phrase en la mettant en regard du matériel et de la volumétrie décrits dans la section « Contexte ».

**Question** — affiché

> Combien de mémoire vive et de machines physiques demande un cluster Kafka de trois nœuds, et comment cela s'articule-t-il avec un unique Raspberry Pi sans budget pour traiter une centaine de messages par jour ?

**Piste** — affiché

> La notion en jeu est le dimensionnement d'infrastructure (right-sizing) et l'alignement avec les contraintes non fonctionnelles. Consultez la documentation officielle d'Apache Kafka à la section « Hardware and OS » (prérequis matériels en RAM, CPU et disques par courtier/broker), puis comparez ces besoins aux caractéristiques techniques de votre modèle de Raspberry Pi. Dans un ADR, si l'outil retenu dépasse les ressources physiques ou budgétaires disponibles dans le contexte, il faut soit documenter comment cette contrainte est levée, soit étudier des mécanismes de traitement asynchrone dont l'empreinte mémoire correspond à un équipement embarqué.

### 19-adr-nous-verrons — Cache sans stratégie d'invalidation, renvoyée à plus tard

`docs/architecture/0004-cache.md` · titulaire `architecte` · modèle `google/gemini-4-argon` · notion émergée · coût 0,0108 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | non | non | 0,0037 $ | 4,2 s |
| Question | affiché | — | 1/1 | oui | non | 0,0031 $ | 3,8 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0040 $ | 11,8 s |

**Indice** — affiché

Citation : `Nous verrons plus tard
s'il faut l'invalider.`

> Relisez la seconde phrase de la section « Contexte » et comparez-la à ce passage.

**Indice** — affiché

Citation : `Nous ajoutons un cache Redis devant l'API du catalogue.`

> Regardez les autres options possibles pour accélérer cette page avant de retenir une unique solution dans cette section.

**Question** — affiché

> Si un voisin retire un objet alors que le catalogue en cache n'est pas invalidé, que voient les autres utilisateurs sur la page ?

**Piste** — affiché

> La notion en jeu est la cohérence du cache (*cache consistency*) et ses stratégies d'invalidation : durée de vie (TTL), invalidation sur événement ou écriture directe (*write-through*). Lorsque la source de données change fréquemment, différer ce choix expose à servir des données périmées (*stale data*). Documentez-vous sur les stratégies de mise en cache (*caching patterns*, par exemple dans la documentation d'architecture de Redis ou de Martin Fowler) pour choisir celle qui répond à votre contexte.

### 20-conception-sans-contraintes — Document de conception qui choisit une solution sans dire les contraintes

`docs/design/recherche.md` · titulaire `architecte` · modèle `google/gemini-4-argon` · notion émergée · coût 0,0104 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 1/1 | 1/1 | non | non | 0,0032 $ | 8,2 s |
| Question | affiché | — | 1/1 | oui | non | 0,0031 $ | 5,1 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0041 $ | 4,8 s |

**Indice** — affiché

Citation : `Un cluster Elasticsearch indexe le catalogue en temps réel.`

> Regardez le lien entre le besoin exprimé dans la section « Objectif » et l'infrastructure décrite ici, ainsi que les hypothèses sur lesquelles ce choix repose.

**Question** — affiché

> Quel volume d'objets et d'utilisateurs attendez-vous, et pourquoi une recherche directe dans votre base de données actuelle ne suffit-elle pas pour chercher un objet par son nom ?

**Piste** — affiché

> Cette étape touche à l'analyse des compromis (trade-offs) et au dimensionnement dans un document de décision d'architecture (ADR). Documentez-vous sur la structure d'un ADR (notamment le modèle de Michael Nygard) : il met en avant l'importance d'expliciter le contexte (volumétrie, contraintes), les alternatives écartées (comme les fonctions de recherche natives de votre base principale) et les conséquences opérationnelles (coût de maintenance d'un cluster, synchronisation des données) qui justifient ou non la complexité d'un outil dédié.

### 21-vision-vide — Vision neuve : passe à l'ouverture sur un gabarit vide

`docs/vision.md` · titulaire `mentor-produit` · modèle `google/gemini-4-argon` · notion émergée · coût 0,0031 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Question | affiché | 1/1 | 1/1 | oui | non | 0,0031 $ | 5,9 s |

**Question** — affiché

Citation : `## Idée`

> Quel problème concret souhaitez-vous résoudre, ou quelle situation voulez-vous changer avec ce projet ?

### 22-vision-trop-large — Vision remplie : public vague et première version trop large

`docs/vision.md` · titulaire `mentor-produit` · modèle `google/gemini-4-argon` · notion émergée · coût 0,0088 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Question | affiché | 2/2 | 2/2 | oui | non | 0,0048 $ | 6,1 s |
| Piste | affiché | — | 1/1 | oui | non | 0,0039 $ | 5,5 s |

**Question** — écarté : citation trop courte ou trop longue

Citation : `Catalogue des objets, réservations avec calendrier, messagerie entre voisins,
notes et avis, notifications, application mobile et site web, paiement d'une
caution en ligne.`

> Avec deux soirs par semaine disponibles, quelle unique fonctionnalité suffirait à un voisin pour prêter une perceuse dès la première semaine ?

**Question** — affiché

Citation : `paiement d'une
caution en ligne.`

> Comment les fonctionnalités en ligne de cette première version s'articulent-elles avec la contrainte d'une consultation depuis la cave sans réseau ?

**Piste** — affiché

> Cette contradiction touche à la conception « hors-ligne d'abord » (offline-first) et au découpage en produit minimum viable (MVP). Documentez-vous sur la différence entre les actions qui exigent une connexion immédiate (comme une transaction bancaire) et celles qui peuvent être différées ou synchronisées plus tard. Cela vous aidera à trier ce qui est réalisable et cohérent pour une toute première version.

### 23-troc-stockage-contourne — Revue : un module relit un fichier de données sans passer par Depot (ADR 0001)

`` · titulaire `architecte` · modèle `google/gemini-4-argon` · notion émergée · coût 0,0136 $

- projet `troc`, revue suivante (diff de 1)
- remarque `project` · cite docs/architecture/0001-stockage.md, src/emprunts/historique.ts
- remarque `project` · cite docs/architecture/0002-dates.md
- remarque `project` · cite aucun fichier

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 3/6 | 3/3 | oui | non | 0,0136 $ | 10,7 s |

**Indice** — affiché

Citation : `docs/architecture/0001-stockage.md : les autres modules passent
par la classe 'Depot' et n'importent jamais 'node:fs'. · src/emprunts/historique.ts : import { readFileSync } from "node:fs";`

> Regardez les imports et la lecture de fichier dans `src/emprunts/historique.ts` en regard de la décision `0001-stockage.md`.

**Indice** — affiché

Citation : `docs/architecture/0002-dates.md : La validation d'une période (fin après le début, durée maximale) est
faite par 'src/emprunts/validation.ts' · <carte_du_projet> : src/objets/disponibilite.ts
  export function periodeCorrecte(debut: Date, fin: Date): boolean`

> Comparez les fonctions exposées par `src/objets/disponibilite.ts` et `src/emprunts/validation.ts` avec la décision `0002-dates.md`.

**Indice** — affiché

Citation : `<carte_du_projet> : ## Hors première version
- Rappels par e-mail avant la date de retour. · <carte_du_projet> : src/notifications/email.ts
  export function rappelsAEnvoyer(emprunts: Emprunt[], aujourdhui: Date): Emprunt[]`

> Rapprochez la présence du module `src/notifications/email.ts` du périmètre défini dans `docs/vision.md`.

### 24-troc-rappels-hors-premiere-version — Revue : les rappels par e-mail grossissent alors que la vision les place hors de la première version

`` · titulaire `mentor-produit` · modèle `google/gemini-4-argon` · notion émergée · coût 0,0095 $

- projet `troc`, revue suivante (diff de 1)
- remarque `project` · cite docs/vision.md, src/notifications/email.ts

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Question | affiché | 2/2 | 1/1 | oui | non | 0,0095 $ | 4,8 s |

**Question** — affiché

Citation : `docs/vision.md : - Rappels par e-mail avant la date de retour. · src/notifications/email.ts : envoyerRappelEmail(emprunt, adresseDe(emprunt.emprunteur));`

> La vision place les rappels par e-mail hors de la première version, alors que `src/notifications/email.ts` les implémente. Souhaitez-vous modifier le périmètre de la première version dans `docs/vision.md`, ou réserver ce code pour plus tard ?

### 25-troc-annuler-sans-test — Revue : l'annulation, avec ses trois refus, n'a aucun test

`` · titulaire `testeur` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion **non émergée** · coût 0,0007 $

- projet `troc`, première revue
- remarque `passage` · cite docs/vision.md
- remarque `passage` · cite src/objets/disponibilite.ts

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | non | non | 0,0007 $ | 4,2 s |

**Indice** — affiché

Citation : `docs/vision.md : - Annuler ou prolonger une réservation.`

> Votre vision inscrit l'annulation dans la première version. La liste des tests montre prolonger.test.ts, rendre.test.ts, reserver.test.ts ; aucun nom ne porte sur l'annulation. Regardez ce qui se passe pour la période d'un emprunt annulé, et demandez-vous ce qui le prouve aujourd'hui.

**Indice** — affiché

Citation : `src/objets/disponibilite.ts : export function estDisponible(depot: Depot, objet: string, debut: Date, fin: Date): boolean`

> Deux modules portent la règle de non-chevauchement : src/emprunts/validation.ts, qui a un fichier de test, et src/objets/disponibilite.ts, qui n'en a pas dans la liste. Vérifiez lequel décide réellement au moment d'une réservation, et lequel reste sans filet.

### 26-troc-voisinage-ordre-des-arguments — Voisinage : arguments de estPeriodeValide inversés, visibles dans la signature du module importé

`src/emprunts/prolonger.ts` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0019 $

- projet `troc`, `src/emprunts/prolonger.ts`

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice (section) | affiché | 1/2 | 2/2 | oui | non | 0,0005 $ | 6,3 s |
| Indice (voisinage) | affiché | 2/2 | 2/2 | oui | non | 0,0014 $ | 8,8 s |

**Indice (section)** — affiché

Citation : `if (!estPeriodeValide(emprunt.debut, nouvelleFin)) {`

> Regardez ce que estPeriodeValide contrôle exactement sur l'ordre des dates, et ce qu'elle laisse passer entre emprunt.fin et nouvelleFin.

**Indice (section)** — écarté : citation introuvable dans la section

Citation : `emprunt.fin = nouvelleFin; ; voir aussi src/stockage/depot.ts`

> Vérifiez dans le dépôt si modifier attend une mutation en place ou une valeur de retour ; la fin de votre callback ne renvoie rien.

**Indice (voisinage)** — affiché

Citation : `if (!estPeriodeValide(emprunt.debut, nouvelleFin)) {`

> Comparez l'ordre des deux arguments de cet appel avec la signature de estPeriodeValide dans src/emprunts/validation.ts : les noms des paramètres y figurent. Vérifiez sur un exemple simple lequel des deux doit venir en premier.

**Indice (voisinage)** — affiché

Citation : `emprunt.fin = nouvelleFin;`

> Parcourez les contrôles situés au-dessus de cette ligne : lequel compare nouvelleFin à la fin actuelle de l'emprunt ? Regardez aussi ce qu'impose le document de décision sur les dates.

### 27-troc-reevaluation-vision-elargie — Réévaluation : la vision intègre désormais les rappels par e-mail à la première version

`` · titulaire `mentor-produit` · modèle `google/gemini-4-argon` · notion émergée · coût 0,0076 $

- projet `troc`, 1 passage(s) disparu(s) sur 2, verdict attendu `withdraw`
- verdict obtenu : `withdraw`

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Question | affiché | — | 1/1 | oui | non | 0,0076 $ | 16,7 s |

**Question** — affiché

> Les rappels par e-mail ont été déplacés dans la section « Première version » de la vision, ce qui aligne le document avec l'implémentation dans email.ts.

### 28-budget-unwrap-dans-l-import — Import : des unwrap et expect contournent l'ADR 0001 (erreurs typées)

`` · titulaire `architecte` · modèle `google/gemini-4-argon` · notion émergée · coût 0,0179 $

- projet `budget`, revue suivante (diff de 2)
- remarque `project` · cite docs/architecture/0001-erreurs-typees.md, src/import/csv.rs, src/import/ligne.rs
- remarque `project` · cite docs/architecture/0002-montants-en-centimes.md, src/rapport/affichage.rs

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 5/5 | 2/2 | oui | non | 0,0179 $ | 25,5 s |

**Indice** — affiché

Citation : `docs/architecture/0001-erreurs-typees.md : Aucun 'unwrap' ni 'expect' hors
des tests : l'import d'un relevé mal formé n'arrête jamais le programme · src/import/csv.rs : let entetes = lecteur.headers().expect("en-tête du relevé").clone(); · src/import/ligne.rs : let date = Date::depuis_texte(champs[0]).unwrap();`

> Rapprochez la règle posée dans l'ADR 0001 sur la gestion des erreurs de la façon dont les en-têtes, les enregistrements, la date et le montant sont extraits lors de l'import CSV.

**Indice** — affiché

Citation : `docs/architecture/0002-montants-en-centimes.md : Ce module est le seul à convertir un
montant en texte, par sa fonction 'formater' ; les autres modules l'appellent. · src/rapport/affichage.rs : pub fn formater_montant(centimes: i64) -> String`

> Comparez la responsabilité confiée au module `src/montant.rs` dans l'ADR 0002 avec les fonctions exposées par `src/rapport/affichage.rs`.

### 29-budget-formatage-en-double — Rapport : un second formatage des montants à côté de montant::formater (ADR 0002)

`` · titulaire `architecte` · modèle `google/gemini-4-argon` · notion émergée · coût 0,0126 $

- projet `budget`, revue suivante (diff de 3)
- remarque `project` · cite docs/architecture/0002-montants-en-centimes.md, src/rapport/export.rs

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/3 | 1/1 | oui | non | 0,0126 $ | 12,5 s |

**Indice** — affiché

Citation : `docs/architecture/0002-montants-en-centimes.md : Ce module est le seul à convertir un
montant en texte, par sa fonction 'formater' ; les autres modules l'appellent. · src/rapport/affichage.rs : pub fn formater_montant(centimes: i64) -> String {
    let euros = centimes as f64 / 100.0; · src/rapport/export.rs : use crate::rapport::affichage::formater_montant;`

> Regardez la décision prise dans l'ADR 0002 (y compris le rejet de `f64` et la responsabilité unique de `src/montant.rs`) et comparez-la à la façon dont `src/rapport/affichage.rs` et `src/rapport/export.rs` mettent les montants en texte.

### 30-budget-categories-sans-test — Classement par catégories : règles et priorités qu'aucun test ne vérifie

`` · titulaire `testeur` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0004 $

- projet `budget`, première revue
- remarque `project` · cite src/categories/mod.rs, src/compte.rs
- remarque `project` · cite docs/vision.md, src/import/mod.rs

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 4/4 | 2/2 | oui | non | 0,0004 $ | 16,0 s |

**Indice** — affiché

Citation : `src/categories/mod.rs : pub fn categoriser(transactions: &mut [Transaction], classeur: &Classeur) -> usize · src/compte.rs : pub fn solde(&self) -> Montant`

> Aucun test de la carte ne se rattache aux catégories ni au compte. Regardez ce que renvoie `categoriser` et ce que promet `solde` : lequel de ces deux comportements voulez-vous verrouiller en premier ?

**Indice** — affiché

Citation : `docs/vision.md : Un relevé mal formé ne doit jamais faire perdre l'import des autres lignes. · src/import/mod.rs : pub fn est_complet(&self) -> bool`

> La vision exige qu'une ligne fautive n'empêche pas l'import des autres. La carte ne montre, sur `ResultatImport`, qu'un booléen. Voyez ce qu'un import partiel doit exposer pour que la ligne fautive reste retrouvable.

### 31-budget-voisinage-periode-inversee — Rapport du mois : arguments de Periode::new dans le mauvais ordre

`src/rapport/mensuel.rs` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0023 $

- projet `budget`, `src/rapport/mensuel.rs`

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice (section) | affiché | 2/2 | 2/2 | oui | non | 0,0011 $ | 4,5 s |
| Indice (voisinage) | affiché | 1/1 | 1/1 | oui | non | 0,0012 $ | 5,3 s |

**Indice (section)** — affiché

Citation : `let periode = Periode::new(mois, annee)?;`

> Comparez l'ordre des arguments de cet appel avec la signature de `Periode::new` dans le fichier voisin. La fonction reçoit `annee` puis `mois` ; que reçoit le constructeur dans chacun des deux paramètres ?

**Indice (section)** — affiché

Citation : `depenses: montants().filter(|m| m.est_depense()).sum(),`

> Regardez le signe des montants que chaque filtre laisse passer, puis relisez `solde()` dans le contexte : les deux champs y sont additionnés. Le signe porté par `Montant` pour une dépense est-il celui attendu par cette addition ?

**Indice (voisinage)** — affiché

Citation : `let periode = Periode::new(mois, annee)?;`

> Comparez l'ordre des deux valeurs passées ici avec la signature déclarée dans src/periode.rs. Vérifiez aussi ce que contient la Période obtenue pour un mois donné, par exemple 3/2024.

### 32-budget-reevaluation-import-a-moitie — Import : un seul des deux passages contournant l'ADR 0001 a été repris

`` · titulaire `architecte` · modèle `google/gemini-4-argon` · notion émergée · coût 0,0100 $

- projet `budget`, 1 passage(s) disparu(s) sur 2, verdict attendu `keep`
- verdict obtenu : `keep`

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | — | 1/1 | oui | non | 0,0100 $ | 5,5 s |

**Indice** — affiché

Citation : `docs/architecture/0001-erreurs-typees.md : Aucun 'unwrap' ni 'expect' hors
des tests : l'import d'un relevé mal formé n'arrête jamais le programme · src/import/ligne.rs : let montant = Montant::depuis_texte(champs[2]).unwrap();`

> Remarque maintenue.

### 33-taches-sql-hors-du-depot — Statistiques et export ouvrent leur propre connexion SQLite malgré l'ADR 0001

`` · titulaire `architecte` · modèle `google/gemini-4-argon` · notion émergée · coût 0,0131 $

- projet `taches`, revue suivante (diff de 2)
- remarque `project` · cite docs/adr/0001-acces-unique-a-la-base.md, taches/export.py, taches/stats.py
- remarque `project` · cite docs/vision.md, taches/export.py

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 5/5 | 2/2 | oui | non | 0,0131 $ | 16,1 s |

**Indice** — affiché

Citation : `docs/adr/0001-acces-unique-a-la-base.md : Tout accès à la base SQLite passe par 'taches/depot.py' (classe 'Depot'). · taches/export.py : connexion = sqlite3.connect(chemin_base) · taches/stats.py : connexion = sqlite3.connect(chemin_base)`

> Regardez comment `taches/export.py` et `taches/stats.py` récupèrent les tâches au regard de la décision prise dans l'ADR 0001.

**Indice** — affiché

Citation : `docs/vision.md : - Déclarer les membres du foyer.
- Attribuer chaque tâche à un membre. · taches/export.py : "SELECT id, titre, echeance, recurrence, duree_minutes, faite_le"`

> Comparez le périmètre annoncé pour la première version dans `docs/vision.md` avec les attributs d'une tâche et les symboles de `taches/modele.py`.

### 34-taches-vision-familles — La vision vise des familles à plusieurs membres, le code ne connaît qu'une personne

`` · titulaire `mentor-produit` · modèle `google/gemini-4-argon` · notion **non émergée** · coût 0,0111 $

- projet `taches`, revue suivante (diff de 3)

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Question | rien | 1/2 | 1/1 | non | non | 0,0111 $ | 6,7 s |

**Question** — écarté : message vide ou trop long

Citation : `docs/vision.md : - Déclarer les membres du foyer.
- Attribuer chaque tâche à un membre. · taches/modele.py :     recurrence: Recurrence = Recurrence.AUCUNE
    duree_minutes: int = 15
    priorite: int = 2`

> Votre vision place le partage entre membres du foyer et la rotation au cœur de la première version. Pourtant, le modèle et le code ne connaissent aucun membre, mais gèrent des durées, des statistiques, un export CSV et maintenant des priorités. Souhaitez-vous recentrer le code sur le problème de répartition du foyer, ou mettre à jour `docs/vision.md` pour refléter l'outil individuel que vous construisez ?

### 35-taches-test-sans-assertion — Des tests du planning qui ne vérifient rien

`` · titulaire `testeur` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion **non émergée** · coût 0,0005 $

- projet `taches`, revue suivante (diff de 2)
- remarque `passage` · cite tests/test_planning.py
- remarque `passage` · cite taches/planning.py

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | 2/2 | 2/2 | non | non | 0,0005 $ | 3,0 s |

**Indice** — affiché

Citation : `tests/test_planning.py : assert isinstance(resultat, list)`

> Regardez cette assertion : demandez-vous ce qu'elle vous apprendrait si le placement changeait. Elle resterait vraie dans beaucoup de cas.

**Indice** — affiché

Citation : `taches/planning.py : if jour.jour.weekday() in jours_libres:`

> Ce filtre n'est exercé que par le cas du dimanche. Voyez ce qui se passerait avec un jour libre au milieu de la semaine, et où les tâches concernées atterrissent.

### 36-taches-voisinage-rappels-utc — Rappel du jour : date du jour locale comparée à une échéance que le dépôt renvoie en UTC

`taches/rappels.py` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0009 $

- projet `taches`, `taches/rappels.py`

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice (section) | affiché | 1/1 | 1/1 | non | non | 0,0002 $ | 6,1 s |
| Indice (voisinage) | affiché | 1/1 | 1/1 | oui | non | 0,0008 $ | 6,5 s |

**Indice (section)** — affiché

Citation : `tache.echeance.date()`

> Vérifiez dans modele.Tache ce que garantit le champ echeance : son type exact, et s'il peut être absent. Que devient le filtre dans ce cas ?

**Indice (voisinage)** — affiché

Citation : `return [tache for tache in a_faire if tache.echeance.date() <= jour]`

> Vous confrontez une date tirée de `tache.echeance` à `date.today()`. L'ADR 0002 dit d'où viennent les `datetime` renvoyés par le dépôt et à quel seul endroit se fait la conversion en heure locale. Regardez ce que donne une échéance tardive du soir, vue par un utilisateur dans un autre fuseau.

### 37-taches-reevaluation-export-par-le-depot — L'export passe désormais par le dépôt : la remarque sur son SQL brut tombe

`` · titulaire `architecte` · modèle `google/gemini-4-argon` · notion émergée · coût 0,0080 $

- projet `taches`, 2 passage(s) disparu(s) sur 2, verdict attendu `withdraw`
- verdict obtenu : `withdraw`

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice | affiché | — | 1/1 | oui | non | 0,0080 $ | 3,5 s |

**Indice** — affiché

> Le module d'export reçoit désormais une instance de Depot et n'ouvre plus sa propre connexion SQLite, conformément à l'ADR 0001.

