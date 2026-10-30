# Évaluation des mentors — 2026-10-30 08:31

2 cas, coût total 0,0110 $. Un modèle est retenu s'il ne propose aucune solution (motifs `must_not`, brouillons écartés compris) et fait émerger la notion (`must_mention`) dans 70 % des cas au moins.

## Verdict par modèle

| Niveau · modèle | Cas | Notion émergée | Solution (brouillons) | Solution (affichée) | JSON invalides | Pannes | Casting différent | Coût | Durée médiane | Durée p90 | Verdict |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| strong · google/gemini-4-argon | 2 | 1/2 (50 %) | 0 | 0 | 0 | 0 | 0 | 0,0110 $ | 5,1 s | 6,1 s | **non retenu** |

## Cas

### 21-vision-vide — Vision neuve : passe à l'ouverture sur un gabarit vide

`docs/vision.md` · titulaire `mentor-produit` · modèle `google/gemini-4-argon` · notion émergée · coût 0,0028 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Question | affiché | 1/1 | 1/1 | oui | non | 0,0028 $ | 3,2 s |

**Question** — affiché

Citation : `## Idée`

> Quel problème concret souhaitez-vous résoudre, et comment votre projet y répond-il ?

### 22-vision-trop-large — Vision remplie : public vague et première version trop large

`docs/vision.md` · titulaire `mentor-produit` · modèle `google/gemini-4-argon` · notion **non émergée** · coût 0,0082 $

| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Question | affiché | 2/2 | 2/2 | non | non | 0,0044 $ | 6,1 s |
| Piste | affiché | — | 1/1 | non | non | 0,0038 $ | 5,1 s |

**Question** — écarté : citation trop courte ou trop longue

Citation : `Catalogue des objets, réservations avec calendrier, messagerie entre voisins,
notes et avis, notifications, application mobile et site web, paiement d'une
caution en ligne.`

> Avec deux soirs par semaine disponibles, quel est le strict minimum dans cette liste qui permettrait à deux voisins de se prêter une perceuse une première fois ?

**Question** — affiché

Citation : `application mobile et site web, paiement d'une
caution en ligne.`

> Comment le paiement en ligne et la messagerie se comportent-ils face à votre contrainte d'une consultation depuis la cave, sans réseau ?

**Piste** — affiché

> Cette contradiction touche à la conception « hors-ligne d'abord » (offline-first) et à la définition d'un Produit Minimum Viable (MVP). Renseignez-vous sur ces deux notions : comment distinguer les fonctionnalités qui exigent une connexion immédiate (comme une transaction bancaire en direct) de celles qui peuvent être consultées localement, et comment réduire le périmètre initial pour respecter votre temps disponible (deux soirs par semaine).

