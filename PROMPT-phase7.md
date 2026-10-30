# Prompt de la phase 7 pour Claude Code

À coller dans une nouvelle session, à la racine du dépôt, après avoir déposé les
fichiers du complément.

---

On démarre la phase 7 : le kick-off d'un projet. La spécification est dans
docs/phase7-kickoff.md, la décision dans docs/adr/0004-kickoff-par-document.md ;
le mentor, le gabarit et deux cas d'évaluation sont déjà en place
(mentors/mentor-produit.md, templates/vision.md, evals/cases/03-*, 04-*).

1. Vérifie dans docs/plan.md que les critères des phases 1 à 6 sont cochés. S'il
   en manque, arrête-toi et liste-les : on ne commence pas la phase 7 sans eux.
2. Lis docs/phase7-kickoff.md et l'ADR 0004, puis relis les sections concernées de
   docs/cadrage.md (format d'un mentor, format d'une remarque, casting) et
   docs/protocole.md. Signale toute contradiction entre la spécification et le
   code ou les documents actuels. Ne code rien à cette étape.
3. Propose le plan de la phase 7 : modifications par crate, ajouts au protocole,
   tests prévus pour chaque critère de fin, et les mises à jour de docs/cadrage.md,
   docs/plan.md, docs/protocole.md et CLAUDE.md. Attends ma validation.

Une fois le plan validé, implémente-le. La garde « pas de solution » et le cadre
commun des prompts ne changent pas. Fais passer cargo test, clippy et fmt, coche
les critères, puis arrête-toi et fais-moi un compte rendu : ce qui est fait,
comment le vérifier à la main avec `hodegos new ~/projets/essai`, et ce qui reste
ouvert pour les trois points de départ.
