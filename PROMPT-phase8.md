# Prompt de la phase 8 pour Claude Code

Installation, à la racine du dépôt (n'écrase rien d'existant) :

```sh
unzip hodegos-phase8.zip -d /tmp/hodegos-phase8
cp -rn /tmp/hodegos-phase8/docs/. docs/
git add docs/phase8.md docs/adr/0005-* docs/adr/0006-* docs/adr/0007-* docs/maquettes/phase8
git commit -m "docs(plan): ajouter la spécification et les maquettes de la phase 8"
```

Puis coller le texte ci-dessous dans une nouvelle session.

---

On démarre la phase 8 : le confort de l'IDE. La spécification unique est
docs/phase8.md, découpée en six sous-phases (8a à 8f). Les décisions sont dans
les ADR 0005 (terminal intégré et confiné), 0006 (remarques supprimées avec le
fichier) et 0007 (évaluation lancée depuis l'IDE) ; les écrans et cas d'usage
dans docs/maquettes/phase8/.

1. Vérifie dans docs/plan.md que les phases 1 à 7 sont cochées. S'il en manque,
   arrête-toi et liste-les.
2. Intègre la phase 8 aux documents, sans coder : section « Phase 8 » dans
   docs/plan.md avec les critères de fin de docs/phase8.md, une section courte
   dans docs/cadrage.md qui renvoie à docs/phase8.md, la règle de suppression
   de l'ADR 0006 dans le format d'une remarque, la mention de l'ADR 0007 dans
   l'ADR 0003, et dans CLAUDE.md « huit phases » et le lien vers docs/phase8.md.
   Signale toute contradiction entre la spécification et le code ou les
   documents actuels ; si une question reste ouverte, pose-la et consigne la
   réponse dans docs/phase8.md. Un commit `docs(plan): …`.
3. Propose le plan de la sous-phase 8a seulement : fichiers par crate et dans
   ui/, messages ajoutés au protocole, tests pour chaque critère. Attends ma
   validation.

Ensuite, une sous-phase à la fois, dans l'ordre 8a → 8f : plan court, validation,
implémentation, cargo test, clippy et fmt, docs/protocole.md à jour dans le même
commit, critères cochés, puis compte rendu avec la vérification à la main. On ne
passe pas à la sous-phase suivante sans mon accord.

Points d'attention :
- La garde « pas de solution », le cadre commun des prompts et ce qui part vers
  le modèle ne changent pas. Le terminal n'envoie rien aux mentors.
- Aucun nouveau port ni service : tout passe par la WebSocket authentifiée.
- Aucun appel réseau dans les tests (faux serveur local, FakeLlmClient).
- Nouvelles dépendances : signale-les dans le plan de la sous-phase (toml_edit
  en 8d, un crate de pseudo-terminal et xterm.js empaqueté par
  scripts/vendor-codemirror.sh en 8f), versions épinglées.
- 8f commence par le prototype décrit dans docs/phase8.md. Consigne son résultat
  dans l'ADR 0005 et arrête-toi pour me le présenter avant toute intégration ;
  si bubblewrap ne fonctionne pas sous WSL2, on rediscute la décision.
