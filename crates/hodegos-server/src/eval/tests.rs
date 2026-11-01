//! Évaluation avec `FakeLlmClient` : aucun appel réseau.

use std::path::Path;

use hodegos_llm::{FakeLlmClient, LlmError, LlmResponse, Tokens};
use hodegos_mentors::{Level, MentorSources, Registry};
use serde_json::{Value, json};

use super::case::{Case, builtin_cases, load_case, load_cases};
use super::run::{Models, Outcome, Spend, run_case};
use super::{Step, evaluate, models_report_path};

const RESERVER: &str = "\
import { Emprunt } from \"../modele\";

export function reserver(debut: Date, emprunts: Emprunt[]) {
  const dejaPris = emprunts.find((e) => e.debut === debut);
  return !dejaPris;
}
";

fn registry() -> Registry {
    Registry::load(&MentorSources {
        user_dir: None,
        project_dir: None,
    })
}

fn models() -> Models {
    Models {
        fast: Some("test/rapide".to_owned()),
        strong: Some("test/fort".to_owned()),
        ..Models::default()
    }
}

fn case(levels: Vec<Level>) -> Case {
    Case {
        name: "01-dates".to_owned(),
        title: "Dates".to_owned(),
        file: "src/emprunts/reserver.ts".to_owned(),
        language: hodegos_core::Language::TypeScript,
        content: RESERVER.to_owned(),
        section: "fn:reserver".to_owned(),
        expected_mentor: "mentor-langage".to_owned(),
        levels,
        must_mention: vec!["référence".to_owned()],
        must_not: vec!["getTime() ===".to_owned()],
        kind: super::case::Kind::Pass,
    }
}

fn spend(max_usd: f64) -> Spend {
    Spend {
        max_usd,
        used_usd: 0.0,
        on_cost: Box::new(|_| {}),
    }
}

fn ok(content: Value) -> Result<LlmResponse, LlmError> {
    Ok(LlmResponse {
        content: content.to_string(),
        cost_usd: 0.01,
        tokens: Tokens {
            prompt: 1000,
            completion: 100,
        },
    })
}

fn remark(message: &str) -> Result<LlmResponse, LlmError> {
    ok(
        json!({ "remarks": [{ "quote": "e.debut === debut", "level": "indice",
        "kind": "observation", "message": message, "confidence": 0.9 }] }),
    )
}

fn reply(level: &str, message: &str) -> Result<LlmResponse, LlmError> {
    ok(json!({ "replies": [{ "level": level, "message": message, "confidence": 0.8 }] }))
}

const ALL: [Level; 4] = [Level::Indice, Level::Question, Level::Piste, Level::Exemple];

#[tokio::test]
async fn joue_les_paliers_comme_palier_suivant_avec_le_fil_reel() {
    let fake = FakeLlmClient::new([
        remark("Regardez comment cette ligne compare deux dates."),
        reply("question", "Que renvoie new Date(1) === new Date(1) ?"),
        reply(
            "piste",
            "=== compare des références ; cherchez la valeur d'une date.",
        ),
        // Le modèle tente la solution : la garde l'écarte, mais le brouillon compte.
        reply(
            "exemple",
            "```ts\nreturn e.debut.getTime() === debut.getTime();\n```",
        ),
    ]);
    let result = run_case(
        &fake,
        &registry(),
        &models(),
        &case(ALL.to_vec()),
        &mut spend(3.0),
    )
    .await;

    assert_eq!(result.error, None);
    assert!(result.casting_ok());
    assert_eq!(result.model.as_deref(), Some("test/rapide"));
    let outcomes: Vec<&Outcome> = result.levels.iter().map(|l| &l.outcome).collect();
    assert_eq!(
        outcomes,
        [
            &Outcome::Shown,
            &Outcome::Shown,
            &Outcome::Shown,
            &Outcome::Nothing
        ]
    );
    assert_eq!(result.levels[0].valid_quotes, Some((1, 1)));
    assert!(result.levels[2].mentions);
    assert!(result.mentions());
    assert_eq!(result.levels[3].leaks_drafts, ["getTime() ==="]);
    assert!(result.levels[3].leaks_shown.is_empty());
    assert_eq!(result.leaks_drafts(), 1);
    assert_eq!(
        result.levels[3].drafts[0].rejected,
        Some("garde « pas de solution » : exemple qui recopie un fragment de la section")
    );
    assert!((result.cost_usd() - 0.04).abs() < 1e-9);

    // Le palier piste part avec le fil : remarque d'origine et message du palier question.
    let requests = fake.requests();
    assert_eq!(requests.len(), 4);
    assert_eq!(requests[0].schema_name, "remarques");
    let user = &requests[2].messages[1].content;
    assert!(
        user.contains("<citation>e.debut === debut</citation>"),
        "{user}"
    );
    assert!(
        user.contains("<paliers_donnes>indice, question</paliers_donnes>"),
        "{user}"
    );
    assert!(
        user.contains("Que renvoie new Date(1) === new Date(1) ?"),
        "{user}"
    );
    assert!(user.ends_with("Palier demandé : piste."));
    assert_eq!(requests[2].schema_name, "reponse");
    // Première passe : la section entière en ajout dans le diff.
    assert!(
        requests[0].messages[1]
            .content
            .contains("+  return !dejaPris;")
    );
}

#[tokio::test]
async fn un_brouillon_ecarte_par_la_garde_compte_quand_meme() {
    let fake = FakeLlmClient::new([ok(json!({ "remarks": [
        { "quote": "e.debut === debut", "level": "indice", "kind": "observation",
          "message": "Écrivez :\n```ts\ne.debut.getTime() === debut.getTime()\n```", "confidence": 0.9 },
        { "quote": "e.debut === debut", "level": "indice", "kind": "observation",
          "message": "Regardez comment cette ligne compare deux dates.", "confidence": 0.8 }
    ] }))]);
    let result = run_case(
        &fake,
        &registry(),
        &models(),
        &case(vec![Level::Indice]),
        &mut spend(3.0),
    )
    .await;
    let level = &result.levels[0];
    assert_eq!(
        level.shown,
        ["Regardez comment cette ligne compare deux dates."]
    );
    assert_eq!(level.leaks_drafts, ["getTime() ==="]);
    assert!(level.leaks_shown.is_empty());
    let rejected = level.drafts[0].rejected.unwrap();
    assert!(rejected.starts_with("garde"), "{rejected}");
    assert_eq!(level.drafts[1].rejected, None);
}

#[tokio::test]
async fn sans_remarque_au_premier_palier_les_suivants_ne_sont_pas_atteints() {
    let fake = FakeLlmClient::new([ok(json!({ "remarks": [] }))]);
    let result = run_case(
        &fake,
        &registry(),
        &models(),
        &case(ALL.to_vec()),
        &mut spend(3.0),
    )
    .await;
    assert_eq!(result.levels[0].outcome, Outcome::Nothing);
    assert!(
        result.levels[1..]
            .iter()
            .all(|l| l.outcome == Outcome::NotReached)
    );
    assert_eq!(fake.requests().len(), 1);
    assert!(!result.mentions());
    assert!(result.complete());
}

#[tokio::test]
async fn une_panne_arrete_le_cas_sans_nouvelle_tentative() {
    let fake = FakeLlmClient::new([remark("Regardez."), Err(LlmError::Timeout)]);
    let result = run_case(
        &fake,
        &registry(),
        &models(),
        &case(ALL.to_vec()),
        &mut spend(3.0),
    )
    .await;
    assert!(matches!(
        result.levels[1].outcome,
        Outcome::ProviderError(_)
    ));
    assert_eq!(result.levels[2].outcome, Outcome::NotReached);
    assert_eq!(fake.requests().len(), 2);
    assert!(!result.complete());
}

#[tokio::test]
async fn l_evaluation_annonce_chaque_cas_et_compte_les_jetons() {
    let fake = FakeLlmClient::new([remark("Regardez."), ok(json!({ "remarks": [] }))]);
    let cases = [case(vec![Level::Indice]), case(vec![Level::Indice])];
    let mut steps = Vec::new();
    let results = evaluate(
        &fake,
        &registry(),
        &models(),
        &cases,
        &mut spend(1.0),
        |step| {
            steps.push(match step {
                Step::Start { index, .. } => format!("début {index}"),
                Step::Done { results } => format!("fin {}", results.len()),
            });
        },
    )
    .await;
    assert_eq!(steps, ["début 1", "fin 1", "début 2", "fin 2"]);
    assert_eq!(
        results[0].tokens,
        Tokens {
            prompt: 1000,
            completion: 100
        }
    );
    assert_eq!(
        results[1].tokens.prompt, 1000,
        "jetons par cas, pas cumulés"
    );
}

#[tokio::test]
async fn max_cost_arrete_l_evaluation() {
    let fake = FakeLlmClient::new([
        remark("Regardez."),
        reply("question", "Que renvoie cette comparaison ?"),
    ]);
    // 0,01 $ par appel : le plafond est atteint après le premier.
    let mut spend = spend(0.01);
    let result = run_case(
        &fake,
        &registry(),
        &models(),
        &case(ALL.to_vec()),
        &mut spend,
    )
    .await;
    assert_eq!(fake.requests().len(), 1);
    assert!(
        result.levels[1..]
            .iter()
            .all(|l| l.outcome == Outcome::BudgetStopped)
    );
    assert!(!result.complete());
    let again = run_case(
        &fake,
        &registry(),
        &models(),
        &case(ALL.to_vec()),
        &mut spend,
    )
    .await;
    assert!(
        again
            .levels
            .iter()
            .all(|l| l.outcome == Outcome::BudgetStopped)
    );
}

#[tokio::test]
async fn section_introuvable_ou_modele_absent() {
    let fake = FakeLlmClient::new([]);
    let mut missing = case(ALL.to_vec());
    missing.section = "fn:annuler".to_owned();
    let result = run_case(&fake, &registry(), &models(), &missing, &mut spend(3.0)).await;
    assert!(result.error.unwrap().contains("fn:annuler introuvable"));

    let only_strong = Models {
        fast: None,
        strong: Some("test/fort".to_owned()),
        ..Models::default()
    };
    let result = run_case(
        &fake,
        &registry(),
        &only_strong,
        &case(ALL.to_vec()),
        &mut spend(3.0),
    )
    .await;
    assert!(result.error.unwrap().contains("« fast » non configuré"));
    assert!(fake.requests().is_empty());
}

#[tokio::test]
async fn un_casting_different_est_signale() {
    let fake = FakeLlmClient::new([ok(json!({ "remarks": [] }))]);
    let mut other = case(vec![Level::Indice]);
    other.expected_mentor = "architecte".to_owned();
    let result = run_case(&fake, &registry(), &models(), &other, &mut spend(3.0)).await;
    assert!(!result.casting_ok());
    assert_eq!(result.mentor.as_deref(), Some("mentor-langage"));
}

#[test]
fn rapport_ecrit_sans_ecraser_celui_du_jour() {
    let dir = tempfile::tempdir().unwrap();
    let models = Models {
        fast: Some("vendeur/Modèle-1".to_owned()),
        strong: Some("vendeur/Modèle-1".to_owned()),
        ..Models::default()
    };
    let first = models_report_path(dir.path(), &models).unwrap();
    std::fs::write(&first, "a").unwrap();
    let second = models_report_path(dir.path(), &models).unwrap();
    let name = first.file_name().unwrap().to_string_lossy().into_owned();
    assert!(name.ends_with("-vendeur-mod-le-1.md"), "{name}");
    assert_ne!(first, second);
    assert!(second.to_string_lossy().ends_with("-2.md"));
}

/// Mentors livrés, ceux de la phase 9 compris (installés après l'évaluation 9f).
fn phase9_registry() -> Registry {
    registry()
}

/// Aucun motif interdit n'est déjà écrit dans le projet : sinon le mentor ne pourrait
/// même pas le citer sans « divulguer ».
fn assert_not_written(case: &Case, files: &[&super::case::Files]) {
    for pattern in &case.must_not {
        let lower = pattern.to_lowercase();
        for (path, text) in files.iter().flat_map(|f| f.iter()) {
            assert!(
                !text.to_lowercase().contains(&lower),
                "{} : must_not « {pattern} » figure déjà dans {path}",
                case.name
            );
        }
    }
}

/// Les cas livrés se lisent, leur section existe et le casting donne le mentor
/// attendu ; les cas sur projet se préparent (citations, diff, voisinage) : vérifiable
/// sans modèle.
#[test]
fn les_cas_livres_sont_coherents() {
    use super::case::Kind;
    use super::project::{Workspace, neighborhood_request, prepare_reevaluation, prepare_review};

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../evals");
    let cases = load_cases(&root, None).unwrap();
    assert!(cases.len() >= 35, "{} cas seulement", cases.len());
    // Les cas intégrés au binaire sont ceux du dépôt, projets d'exemple compris.
    let builtin = builtin_cases(None).unwrap();
    let summary = |cases: &[Case]| -> Vec<String> {
        cases
            .iter()
            .map(|c| format!("{} {} {:?}", c.name, c.content, c.kind))
            .collect()
    };
    assert_eq!(summary(&builtin), summary(&cases));
    let registry = phase9_registry();
    let mut languages = std::collections::BTreeSet::new();
    let mut mentors = std::collections::BTreeSet::new();
    let mut kinds = std::collections::BTreeMap::new();
    for case in &cases {
        let reloaded = load_case(&root.join("cases").join(&case.name)).unwrap();
        assert_eq!(reloaded.name, case.name);
        *kinds.entry(case.kind_name()).or_insert(0) += 1;
        mentors.insert(case.expected_mentor.clone());
        match &case.kind {
            Kind::Pass | Kind::Neighborhood(_) => {
                let all = hodegos_core::sections(case.language, &case.content);
                let section = all
                    .iter()
                    .find(|s| s.id == case.section)
                    .unwrap_or_else(|| {
                        panic!(
                            "{} : section {} introuvable ({:?})",
                            case.name,
                            case.section,
                            all.iter().map(|s| &s.id).collect::<Vec<_>>()
                        )
                    });
                let text = section.text(&case.content);
                let casting = hodegos_mentors::cast(
                    &registry,
                    &hodegos_mentors::CastRequest {
                        path: &case.file,
                        language: case.language.as_str(),
                        headings: hodegos_mentors::section_headings(case.language, Some(section)),
                        text: &text,
                        forced: None,
                    },
                );
                assert_eq!(
                    casting.titular.as_deref(),
                    Some(case.expected_mentor.as_str()),
                    "{} : casting",
                    case.name
                );
                if let Kind::Neighborhood(sample) = &case.kind {
                    assert_not_written(case, &[&sample.files]);
                    let workspace = Workspace::new(sample).unwrap();
                    let mentor = registry.get(&case.expected_mentor).unwrap();
                    let size = |widened| {
                        let (request, _) =
                            neighborhood_request(&workspace, mentor, "m", &models(), case, widened)
                                .unwrap();
                        request.messages[1].content.clone()
                    };
                    let (alone, widened) = (size(false), size(true));
                    assert!(
                        widened.len() > alone.len() + 50,
                        "{} : le voisinage n'ajoute rien",
                        case.name
                    );
                } else {
                    for pattern in &case.must_not {
                        assert!(
                            !text.to_lowercase().contains(&pattern.to_lowercase()),
                            "{} : must_not « {pattern} » figure déjà dans la section",
                            case.name
                        );
                    }
                    languages.insert(case.language.as_str());
                }
            }
            Kind::Review(review) => {
                assert_not_written(case, &[&review.sample.files, &review.sample.after]);
                let mentor = registry.get(&case.expected_mentor).unwrap_or_else(|| {
                    panic!("{} : mentor {} inconnu", case.name, case.expected_mentor)
                });
                for path in &review.cited_files {
                    assert!(
                        review.sample.files.contains_key(path)
                            || review.sample.after.contains_key(path),
                        "{} : {path} absent du projet",
                        case.name
                    );
                }
                for path in review.must_not_by_file.keys() {
                    assert!(
                        review.sample.files.contains_key(path)
                            || review.sample.after.contains_key(path),
                        "{} : must_not pour {path}, absent du projet",
                        case.name
                    );
                }
                let prepared = prepare_review(mentor, "m", &models(), review)
                    .unwrap_or_else(|e| panic!("{} : {e}", case.name));
                assert_eq!(
                    prepared.request.schema_name, "revue",
                    "{} : schéma",
                    case.name
                );
                if !review.sample.after.is_empty() {
                    assert!(
                        prepared
                            .sent
                            .iter()
                            .any(|(_, text)| text.starts_with("---")),
                        "{} : after/ ne change rien",
                        case.name
                    );
                }
            }
            Kind::Reevaluation(reevaluation) => {
                assert_not_written(
                    case,
                    &[&reevaluation.sample.files, &reevaluation.sample.after],
                );
                let mentor = registry.get(&case.expected_mentor).unwrap();
                prepare_reevaluation(mentor, "m", &models(), case.levels[0], reevaluation)
                    .unwrap_or_else(|e| panic!("{} : {e}", case.name));
            }
        }
    }
    assert_eq!(
        languages.into_iter().collect::<Vec<_>>(),
        ["javascript", "markdown", "python", "rust", "typescript"]
    );
    assert_eq!(
        mentors.into_iter().collect::<Vec<_>>(),
        ["architecte", "mentor-langage", "mentor-produit", "testeur"]
    );
    assert!(kinds["review"] >= 9 && kinds["neighborhood"] >= 3 && kinds["reevaluation"] >= 3);
}

fn project_case(name: &str) -> Case {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../evals");
    load_cases(&root, Some(name)).unwrap().remove(0)
}

#[tokio::test]
async fn revue_jouee_sur_le_projet_avec_le_diff_et_la_garde() {
    let case = project_case("23-troc");
    let fake = FakeLlmClient::new([ok(json!({ "remarks": [
        { "about": "project", "level": "indice", "kind": "observation", "confidence": 0.9,
          "quotes": [{ "path": "src/emprunts/historique.ts",
                       "quote": "readFileSync(\"data/archives.json\", \"utf8\")" }],
          "message": "Relisez la décision 0001 : ce module lit lui-même un fichier de données." },
        // Citation jamais envoyée, peu sûre, avec un fragment de solution : écartée, mais
        // comptée.
        { "about": "project", "level": "indice", "kind": "observation", "confidence": 0.1,
          "quotes": [{ "path": "src/notifications/email.ts", "quote": "JSON.parse(readFileSync(FILE, \"utf8\"))" }],
          "message": "Il suffirait d'écrire depot.lire().archives." }
    ] }))]);
    let result = run_case(&fake, &phase9_registry(), &models(), &case, &mut spend(3.0)).await;

    assert_eq!(result.error, None, "{:?}", result.notes);
    assert_eq!(result.kind, "review");
    assert_eq!(result.model.as_deref(), Some("test/fort"));
    let level = &result.levels[0];
    assert_eq!(level.outcome, Outcome::Shown);
    assert!(result.mentions(), "la bonne remarque cite historique.ts");
    assert_eq!(level.valid_quotes, Some((1, 2)));
    assert_eq!(result.quotes(), (1, 2));
    assert_eq!(level.leaks_drafts, ["depot.lire().archives"]);
    assert!(level.leaks_shown.is_empty());
    let requests = fake.requests();
    assert_eq!(requests[0].schema_name, "revue");
    let user = &requests[0].messages[1].content;
    assert!(user.contains("archivesDe"), "le diff part");
    assert!(
        user.contains("# troc — carte du projet"),
        "{}",
        &user[..200]
    );
    assert!(result.notes.iter().any(|n| n.contains("revue suivante")));

    // Coût d'une journée type : les mentors de projet des brouillons, au coût mesuré.
    let day = super::day::typical_day(&phase9_registry(), std::slice::from_ref(&result)).unwrap();
    let ids: Vec<&str> = day.mentors.iter().map(|m| m.id.as_str()).collect();
    assert_eq!(ids, ["architecte", "mentor-produit", "testeur"]);
    assert!(day.mentors[0].measured && !day.mentors[2].measured);
    assert!((day.mentors[0].cost_per_review - 0.01).abs() < 1e-9);
    assert!(day.monthly() > 0.0);
    let summaries = super::report::summarize(std::slice::from_ref(&result));
    let text =
        super::report::markdown("2026-11-01 12:00", &[result], &summaries, false, Some(&day));
    assert!(text.contains("## Par sorte de cas"));
    assert!(text.contains("## Coût d'une revue"));
    assert!(text.contains("## Coût d'une journée type"));
    assert!(text.contains("| Revue | 1 | 1 | 1/2 | 1 |"), "{text}");
}

#[tokio::test]
async fn voisinage_compare_section_seule_et_voisinage() {
    let case = project_case("26-troc");
    let quoted = |message: &str| {
        ok(
            json!({ "remarks": [{ "quote": "estPeriodeValide(emprunt.debut, nouvelleFin)",
            "level": "indice", "kind": "observation", "message": message, "confidence": 0.9,
            "see_also": [] }] }),
        )
    };
    let fake = FakeLlmClient::new([
        quoted("Regardez cette condition."),
        quoted("Comparez l'ordre des arguments avec la signature du module importé."),
    ]);
    let result = run_case(&fake, &phase9_registry(), &models(), &case, &mut spend(3.0)).await;
    assert_eq!(result.error, None);
    assert_eq!(result.levels.len(), 2);
    assert_eq!(result.levels[0].context, Some("section"));
    assert!(!result.levels[0].mentions);
    assert!(result.levels[1].mentions);
    assert!(result.mentions());
    let requests = fake.requests();
    assert!(!requests[0].messages[1].content.contains("<voisinage>"));
    assert!(
        requests[1].messages[1]
            .content
            .contains("estPeriodeValide(fin: Date, debut: Date)")
    );
}

#[tokio::test]
async fn reevaluation_jugee_sur_le_verdict_attendu() {
    let case = project_case("27-troc");
    let fake = FakeLlmClient::new([ok(json!({ "verdict": "withdraw", "quotes": [],
        "reason": "La vision prévoit désormais ce rappel dans la première version." }))]);
    let result = run_case(&fake, &phase9_registry(), &models(), &case, &mut spend(3.0)).await;
    assert_eq!(result.error, None);
    assert!(result.mentions());
    assert!(
        result
            .notes
            .iter()
            .any(|n| n.contains("verdict obtenu : `withdraw`"))
    );
    let user = &fake.requests()[0].messages[1].content;
    assert!(
        user.contains("Rappels par e-mail avant la date de retour."),
        "passage disparu"
    );

    // Maintenue à tort : le cas échoue.
    let fake = FakeLlmClient::new([ok(json!({ "verdict": "keep", "quotes": [
        { "path": "src/notifications/email.ts",
          "quote": "export function envoyerRappelEmail(emprunt: Emprunt, adresse: string): void {" }],
        "reason": "" }))]);
    let result = run_case(&fake, &phase9_registry(), &models(), &case, &mut spend(3.0)).await;
    assert!(!result.mentions());
    assert!(
        result
            .notes
            .iter()
            .any(|n| n.contains("verdict obtenu : `keep`"))
    );
}

#[test]
fn sans_mentor_de_projet_le_cas_n_est_pas_joue() {
    // Le Testeur désactivé : la revue est signalée, pas jouée, et ne compte pas dans
    // l'estimation.
    let case = project_case("25-troc");
    assert!(super::run::first_prompt(&registry(), &case).is_some());
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("testeur.md"),
        "---\nid: testeur\nenabled: false\n---\n",
    )
    .unwrap();
    let without = Registry::load(&MentorSources {
        user_dir: Some(dir.path().to_path_buf()),
        project_dir: None,
    });
    assert!(super::run::first_prompt(&without, &case).is_none());
}
