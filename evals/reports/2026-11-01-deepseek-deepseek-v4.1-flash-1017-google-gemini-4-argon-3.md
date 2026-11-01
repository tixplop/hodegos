# Évaluation des mentors — 2026-11-01 13:37

1 cas, coût total 0,0017 $. Un modèle est retenu s'il ne propose aucune solution (motifs `must_not`, brouillons écartés compris) et fait émerger la notion (`must_mention`) dans 70 % des cas au moins, avec 80 % de citations valides au moins.

## Verdict par modèle

| Niveau · modèle | Cas | Notion émergée | Citations valides | Solution (brouillons) | Solution (affichée) | JSON invalides | Pannes | Casting différent | Coût | Jetons (entrée / sortie) | Durée médiane | Durée p90 | Verdict |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| fast · deepseek/deepseek-v4.1-flash-1017 | 1 | 1/1 (100 %) | 3/3 (100 %) | 0 | 0 | 0 | 0 | 0 | 0,0017 $ | 3238 / 3206 | 6,7 s | 11,0 s | **retenu** |

## Par sorte de cas

| Sorte | Cas | Réussis | Citations valides | Solution (brouillons) | Coût |
| --- | --- | --- | --- | --- | --- |
| Voisinage | 1 | 1 | 3/3 | 0 | 0,0017 $ |

Une revue réussit si une remarque affichée fait émerger la notion et cite un fichier attendu ; un cas de voisinage, si la passe avec le voisinage la fait émerger ; une réévaluation, si le verdict est celui attendu.

## Voisinage

| Cas | Notion (section seule) | Notion (voisinage) | Coût (section) | Coût (voisinage) |
| --- | --- | --- | --- | --- |
| 26-troc-voisinage-ordre-des-arguments | non | oui | 0,0012 $ | 0,0005 $ |

## Cas

### 26-troc-voisinage-ordre-des-arguments — Voisinage : arguments de estPeriodeValide inversés, visibles dans la signature du module importé

`src/emprunts/prolonger.ts` · titulaire `mentor-langage` · modèle `deepseek/deepseek-v4.1-flash-1017` · notion émergée · coût 0,0017 $

- projet `troc`, `src/emprunts/prolonger.ts`

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Indice (section) | affiché | 1/1 | 1/1 | non | non | 0,0012 $ | 11,0 s |
| Indice (voisinage) | affiché | 2/2 | 2/2 | oui | non | 0,0005 $ | 6,7 s |

**Indice (section)** — affiché

Citation : `const suite = { debut: emprunt.fin, fin: nouvelleFin };`

> Regardez la ligne `const suite = { debut: emprunt.fin, fin: nouvelleFin }` : posez à voix haute ce que devient la fin de l'emprunt si la date reçue est antérieure à `emprunt.fin`. Vérifiez aussi ce que contrôle exactement `estPeriodeValide`.

**Indice (voisinage)** — affiché

Citation : `estPeriodeValide(emprunt.debut, nouvelleFin)`

> Regardez dans quel ordre vous passez les deux dates à `estPeriodeValide`. Comparez-les aux noms des paramètres du fichier de validation : est-ce que l'ordre de l'appel correspond à celui de la signature ?

**Indice (voisinage)** — affiché

Citation : `const suite = { debut: emprunt.fin, fin: nouvelleFin };`

> `suite.debut` vaut `emprunt.fin`. La décision 0002 dit qu'une période va d'un jour de début à un jour de fin inclus. Demandez-vous ce que produit `seChevauchent` quand un voisin commence exactement ce jour-là : est-ce une gêne ou non ?

