# Prompt de démarrage pour Claude Code

À coller tel quel dans la première session, lancée à la racine du dépôt.

---

Tu démarres l'implémentation de Hodegos, un IDE personnel où des mentors IA guident
l'utilisateur sans écrire à sa place.

1. Lis CLAUDE.md, puis docs/cadrage.md, docs/plan.md et docs/protocole.md en entier.
   Parcours docs/maquettes/ et ui/styles/README.md pour la référence visuelle.
2. Résume en dix lignes au plus ce que tu as compris du MVP et des règles non
   négociables, et liste les points du cadrage qui te semblent ambigus ou
   contradictoires. Ne code rien à cette étape.
3. Propose le plan détaillé de la phase 1 (Squelette) uniquement : fichiers créés,
   crates et dépendances avec leur version, découpage en étapes, tests prévus pour
   chaque critère de fin. Attends ma validation avant d'écrire du code.

Une fois le plan validé, implémente la phase 1, fais passer cargo test, clippy et
fmt, coche les critères dans docs/plan.md, puis arrête-toi et fais-moi un compte
rendu : ce qui est fait, comment le vérifier à la main, ce qui reste ouvert.
Ne commence pas la phase 2 sans mon accord.
