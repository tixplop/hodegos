//! Évaluation avec `FakeLlmClient` : aucun appel réseau.

use std::path::Path;

use hodegos_llm::{FakeLlmClient, LlmError, LlmResponse};
use hodegos_mentors::{Level, MentorSources, Registry};
use serde_json::{Value, json};

use super::case::{Case, load_case, load_cases};
use super::run::{Models, Outcome, Spend, run_case};
use super::write_report;

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
        // La solution en une ligne ne reprend qu'un identifiant de la section (debut) :
        // la garde la laisse passer, l'évaluation la relève.
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
    assert!(result.levels.iter().all(|l| l.outcome == Outcome::Shown));
    assert_eq!(result.levels[0].valid_quotes, Some((1, 1)));
    assert!(result.levels[2].mentions);
    assert!(result.mentions());
    assert_eq!(result.levels[3].leaks_drafts, ["getTime() ==="]);
    assert_eq!(result.levels[3].leaks_shown, ["getTime() ==="]);
    assert_eq!(result.leaks_drafts(), 1);
    assert_eq!(result.levels[3].drafts[0].rejected, None);
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
    };
    let first = write_report(dir.path(), &models, "a").unwrap();
    let second = write_report(dir.path(), &models, "b").unwrap();
    let name = first.file_name().unwrap().to_string_lossy().into_owned();
    assert!(name.ends_with("-vendeur-mod-le-1.md"), "{name}");
    assert_ne!(first, second);
    assert!(second.to_string_lossy().ends_with("-2.md"));
}

/// Les cas livrés se lisent, leur section existe et le casting donne le mentor
/// attendu : vérifiable sans modèle.
#[test]
fn les_cas_livres_sont_coherents() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../evals");
    let cases = load_cases(&root, None).unwrap();
    assert!(cases.len() >= 20, "{} cas seulement", cases.len());
    let registry = registry();
    let mut languages = std::collections::BTreeSet::new();
    let mut mentors = std::collections::BTreeSet::new();
    for case in &cases {
        let reloaded = load_case(&root.join("cases").join(&case.name)).unwrap();
        assert_eq!(reloaded.name, case.name);
        let all = hodegos_core::sections(case.language, &case.content);
        let section = all
            .iter()
            .find(|s| s.id == case.section)
            .unwrap_or_else(|| panic!("{} : section {} introuvable", case.name, case.section));
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
        // Aucun motif interdit n'est déjà écrit dans la section : sinon le mentor ne
        // pourrait même pas la citer sans « divulguer ».
        for pattern in &case.must_not {
            assert!(
                !text.to_lowercase().contains(&pattern.to_lowercase()),
                "{} : must_not « {pattern} » figure déjà dans la section",
                case.name
            );
        }
        languages.insert(case.language.as_str());
        mentors.insert(case.expected_mentor.clone());
    }
    assert_eq!(
        languages.into_iter().collect::<Vec<_>>(),
        ["javascript", "markdown", "python", "rust", "typescript"]
    );
    assert_eq!(
        mentors.into_iter().collect::<Vec<_>>(),
        ["architecte", "mentor-langage"]
    );
}
