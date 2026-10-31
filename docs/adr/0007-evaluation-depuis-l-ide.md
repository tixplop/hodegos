# ADR 0007 — Évaluation lancée depuis l'IDE à chaque changement de modèle ou d'effort

Statut : accepté · 31 octobre 2026

## Contexte

L'ADR 0003 impose une nouvelle évaluation à tout changement de modèle. La
phase 8 permet de changer, depuis les réglages, le modèle et l'effort de
raisonnement de chaque niveau. L'effort change la qualité et le coût des
réponses autant qu'un changement de modèle. Une évaluation coûte de l'argent et
du temps ; elle ne doit pas partir sans que la personne le sache.

Aujourd'hui, `hodegos eval evals/` lit ses cas dans un dossier du dépôt : un
binaire installé n'en a pas.

## Décision

- Tout changement d'identifiant **ou d'effort** d'un niveau déclenche
  l'évaluation de ce niveau, sauf vers un réglage qui a déjà un résultat retenu
  (évaluation de l'IDE, ou choix livrés de l'ADR 0003) : revenir à l'ancien
  réglage ne coûte rien (décidé le 31 octobre 2026). Un niveau à la fois.
- La personne est prévenue **avant** : dialogue avec le niveau, le nombre de
  cas, le coût et la durée estimés (d'après le dernier passage, sinon le prix du
  modèle), et le plafond de l'évaluation. Annuler n'écrit rien.
- L'évaluation tourne dans le serveur sur les cas **intégrés au binaire**. Le
  rapport va dans `~/.local/share/hodegos/evals/`, jamais dans le projet ; la
  dépense compte dans le budget.
- Un résultat sous le seuil de l'ADR 0003 n'annule pas le réglage : il est
  signalé, avec « Revenir à l'ancien réglage ».
- L'effort est un réglage de niveau, pas de mentor.
- Sans passage mesuré, l'estimation multiplie le prix du modèle par les jetons
  estimés d'après les prompts des cas intégrés : les rapports de l'ADR 0003
  n'avaient pas noté les jetons. Ils le sont désormais dans chaque rapport.

## Alternatives écartées

- Avertir seulement et laisser lancer `hodegos eval` à la main : la règle de
  l'ADR 0003 ne serait pas tenue.
- Annuler automatiquement un réglage sous le seuil : la personne peut vouloir
  essayer ; elle garde la main.
- Effort par mentor : multiplie les combinaisons à évaluer.

## Conséquences

- `rust-embed` intègre `evals/cases/` ; `hodegos eval` utilise ces cas par
  défaut.
- Les rapports de l'IDE sont personnels ; ceux de `evals/reports/` restent ceux
  des choix livrés (ADR 0003).
