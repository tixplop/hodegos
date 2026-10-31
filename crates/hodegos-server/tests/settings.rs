#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Réglages (phase 8d) : modèles, effort, clé, budget et réévaluation, contre un faux
//! OpenRouter local (catalogue, clé) et `FakeLlmClient` : aucun appel réseau.

mod common;

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::Json;
use axum::http::{HeaderMap, StatusCode};
use common::{Socket, data_dir, exchange, next, next_within, open_socket_with, send_raw};
use hodegos_llm::{ApiKey, Effort, FakeLlmClient, LlmClient, LlmConfig};
use hodegos_server::{Llm, SettingsSource};
use serde_json::{Value, json};

const KEY: &str = "sk-or-secrete-a3f9";
const SHIPPED_FAST: &str = "deepseek/deepseek-v4.1-flash-1017";
const SHIPPED_STRONG: &str = "google/gemini-4-argon";

const CONFIG: &str = "\
# Réglages personnels

[llm]
# Mentor du langage
fast = \"deepseek/deepseek-v4.1-flash-1017\" # ADR 0003
strong = \"google/gemini-4-argon\"
";

/// Faux OpenRouter : catalogue public et vérification de la clé.
async fn fake_openrouter() -> String {
    let models = json!({ "data": [
        { "id": SHIPPED_FAST, "supported_parameters": ["structured_outputs"],
          "pricing": { "prompt": "0.0000003", "completion": "0.0000012" } },
        { "id": SHIPPED_STRONG, "supported_parameters": ["structured_outputs", "reasoning"],
          "pricing": { "prompt": "0.000002", "completion": "0.000008" } },
        { "id": "autre/rapide", "supported_parameters": ["structured_outputs", "reasoning"],
          "pricing": { "prompt": "0.000001", "completion": "0.000004" } },
        { "id": "autre/sans-schema", "supported_parameters": ["response_format"],
          "pricing": { "prompt": "0.000001", "completion": "0.000001" } }
    ] });
    let app = axum::Router::new()
        .route(
            "/api/v1/models",
            axum::routing::get(move || {
                let models = models.clone();
                async move { Json(models) }
            }),
        )
        .route(
            "/api/v1/key",
            axum::routing::get(|headers: HeaderMap| async move {
                if headers["authorization"] == format!("Bearer {KEY}").as_str() {
                    (StatusCode::OK, Json(json!({ "data": {} })))
                } else {
                    (
                        StatusCode::UNAUTHORIZED,
                        Json(json!({ "error": { "message": "clé refusée" } })),
                    )
                }
            }),
        );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/api/v1", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    url
}

struct Setup {
    project: tempfile::TempDir,
    config: tempfile::TempDir,
    fake: Arc<FakeLlmClient>,
    base_url: String,
}

async fn setup() -> Setup {
    let config = tempfile::tempdir().unwrap();
    fs::write(config.path().join("config.toml"), CONFIG).unwrap();
    Setup {
        project: tempfile::tempdir().unwrap(),
        config,
        fake: Arc::new(FakeLlmClient::default()),
        base_url: fake_openrouter().await,
    }
}

fn llm(setup: &Setup, key: Option<&str>, from_env: bool, efforts: (Effort, Effort)) -> Llm {
    let mut config = LlmConfig::load(setup.config.path()).unwrap();
    config.base_url = setup.base_url.clone();
    config.fast_effort = efforts.0;
    config.strong_effort = efforts.1;
    let fake = setup.fake.clone();
    Llm {
        client: Ok(fake.clone()),
        config,
        settings: Some(SettingsSource {
            config_dir: setup.config.path().to_path_buf(),
            key: key.map(ApiKey::new),
            key_from_env: from_env,
            connect: Arc::new(move |_, _| Ok(fake.clone() as Arc<dyn LlmClient>)),
        }),
    }
}

async fn socket(setup: &Setup, key: Option<&str>, from_env: bool) -> Socket {
    open_socket_with(
        setup.project.path(),
        llm(setup, key, from_env, (Effort::Default, Effort::Default)),
    )
    .await
}

async fn request(socket: &mut Socket, kind: &str, payload: Value) -> Value {
    let mut reply = exchange(
        socket,
        json!({ "v": 1, "type": kind, "id": "r", "payload": payload }),
    )
    .await;
    while reply["re"] != "r" {
        reply = next(socket).await;
    }
    reply
}

fn set_payload(fast: &str, fast_effort: &str, strong: &str, strong_effort: &str) -> Value {
    json!({ "fast": fast, "strong": strong, "fast_effort": fast_effort,
        "strong_effort": strong_effort,
        "budget": { "monthly_cap_usd": 10.0, "warn_at": 0.8 } })
}

fn config_text(setup: &Setup) -> String {
    fs::read_to_string(setup.config.path().join("config.toml")).unwrap()
}

/// Messages spontanés jusqu'à `eval_done` compris.
async fn until_eval_done(socket: &mut Socket) -> Vec<Value> {
    let mut messages = Vec::new();
    loop {
        let message = next_within(socket, Duration::from_secs(20))
            .await
            .expect("évaluation sans fin");
        let done = message["type"] == "eval_done";
        messages.push(message);
        if done {
            return messages;
        }
    }
}

#[tokio::test]
async fn les_reglages_disent_l_origine_de_la_cle_sans_la_renvoyer() {
    let setup = setup().await;
    let mut socket = socket(&setup, Some(KEY), false).await;
    let settings = request(&mut socket, "settings_get", Value::Null).await;
    assert_eq!(settings["type"], "settings", "{settings}");
    let payload = &settings["payload"];
    assert_eq!(payload["fast"], SHIPPED_FAST);
    assert_eq!(payload["fast_effort"], "default");
    assert_eq!(payload["key_source"], "file");
    assert_eq!(payload["key_hint"], "a3f9");
    assert_eq!(payload["editable"], true);
    assert!(!settings.to_string().contains(KEY), "{settings}");
    // Les choix livrés sont connus : résultat de l'ADR 0003.
    assert_eq!(payload["last_eval"]["fast"]["origin"], "builtin");
    assert_eq!(payload["last_eval"]["fast"]["passed"], true);
    assert_eq!(payload["last_eval"]["strong"]["model"], SHIPPED_STRONG);

    let (mut env, _project) = socket_with_env(&setup).await;
    let settings = request(&mut env, "settings_get", Value::Null).await;
    assert_eq!(settings["payload"]["key_source"], "env");
}

/// Connexion avec une clé lue dans `OPENROUTER_API_KEY` ; le projet vit avec la connexion.
async fn socket_with_env(setup: &Setup) -> (Socket, tempfile::TempDir) {
    let project = tempfile::tempdir().unwrap();
    let socket = open_socket_with(
        project.path(),
        llm(setup, Some(KEY), true, (Effort::Default, Effort::Default)),
    )
    .await;
    (socket, project)
}

#[tokio::test]
async fn model_check_lit_le_catalogue() {
    let setup = setup().await;
    let mut socket = socket(&setup, Some(KEY), false).await;
    let info = request(&mut socket, "model_check", json!({ "id": "autre/rapide" })).await;
    assert_eq!(info["type"], "model_info", "{info}");
    assert_eq!(info["payload"]["exists"], true);
    assert_eq!(info["payload"]["structured_outputs"], true);
    assert_eq!(info["payload"]["reasoning"], true);
    assert_eq!(info["payload"]["prompt_price"], 0.000001);
    let missing = request(&mut socket, "model_check", json!({ "id": "inconnu/x" })).await;
    assert_eq!(missing["payload"]["exists"], false);
}

#[tokio::test]
async fn un_modele_sans_sorties_structurees_ou_un_effort_non_pris_en_charge_sont_refuses() {
    let setup = setup().await;
    let mut socket = socket(&setup, Some(KEY), false).await;
    for (payload, needle) in [
        (
            set_payload("autre/sans-schema", "default", SHIPPED_STRONG, "default"),
            "sorties structurées",
        ),
        (
            set_payload(SHIPPED_FAST, "high", SHIPPED_STRONG, "default"),
            "effort",
        ),
        (
            set_payload("inconnu/x", "default", SHIPPED_STRONG, "default"),
            "inconnu",
        ),
    ] {
        let refused = request(&mut socket, "settings_set", payload).await;
        assert_eq!(refused["type"], "error", "{refused}");
        assert_eq!(refused["payload"]["code"], "model_refused");
        assert!(
            refused["payload"]["message"]
                .as_str()
                .unwrap()
                .contains(needle),
            "{refused}"
        );
    }
    assert_eq!(config_text(&setup), CONFIG, "rien n'est écrit");
}

#[tokio::test]
async fn changer_un_niveau_annonce_l_estimation_puis_evalue_apres_confirmation() {
    let setup = setup().await;
    let mut socket = socket(&setup, Some(KEY), false).await;
    let payload = set_payload("autre/rapide", "medium", SHIPPED_STRONG, "default");

    let required = request(&mut socket, "settings_set", payload.clone()).await;
    assert_eq!(required["type"], "eval_required", "{required}");
    let p = &required["payload"];
    assert_eq!(p["level"], "fast");
    assert!(p["cases"].as_u64().unwrap() >= 15, "{p}");
    assert!(p["estimated_cost"].as_f64().unwrap() > 0.0, "{p}");
    assert!(p["estimated_duration_ms"].as_u64().unwrap() > 0, "{p}");
    assert_eq!(p["max_cost"], 3.0);
    assert_eq!(p["basis"], "prompts");
    assert_eq!(
        config_text(&setup),
        CONFIG,
        "rien n'est écrit avant confirmation"
    );
    assert!(setup.fake.requests().is_empty());

    let mut confirmed = payload.clone();
    confirmed["confirm_eval"] = json!(true);
    let settings = request(&mut socket, "settings_set", confirmed).await;
    assert_eq!(settings["type"], "settings", "{settings}");
    assert_eq!(settings["payload"]["fast"], "autre/rapide");
    assert_eq!(settings["payload"]["fast_effort"], "medium");
    assert_eq!(settings["payload"]["eval_running"], "fast");
    let text = config_text(&setup);
    assert!(
        text.contains("# Mentor du langage\nfast = \"autre/rapide\" # ADR 0003\n"),
        "{text}"
    );
    assert!(text.contains("fast_effort = \"medium\""), "{text}");

    let messages = until_eval_done(&mut socket).await;
    let progress: Vec<&Value> = messages
        .iter()
        .filter(|m| m["type"] == "eval_progress")
        .collect();
    assert!(!progress.is_empty());
    assert_eq!(progress.last().unwrap()["payload"]["done"], p["cases"]);
    let done = &messages.last().unwrap()["payload"];
    assert_eq!(done["model"], "autre/rapide");
    assert_eq!(done["effort"], "medium");
    // Le faux client n'a rien à répondre : notion jamais atteinte, réglage signalé.
    assert_eq!(done["passed"], false);
    assert_eq!(done["previous"]["model"], SHIPPED_FAST);
    let report = done["report"].as_str().unwrap();
    assert!(
        Path::new(report).starts_with(data_dir(setup.project.path()).join("evals")),
        "{report}"
    );
    assert!(report.ends_with("-autre-rapide-medium.md"), "{report}");
    assert!(
        fs::read_to_string(report)
            .unwrap()
            .contains("# Évaluation des mentors")
    );

    // Les appels de l'évaluation portent le nouveau modèle et son effort.
    let sent = setup.fake.requests();
    assert!(!sent.is_empty());
    assert!(
        sent.iter()
            .all(|r| r.model == "autre/rapide" && r.effort == Effort::Medium)
    );

    // Le résultat est connu : les réglages le rappellent, avec l'ancien réglage.
    let settings = request(&mut socket, "settings_get", Value::Null).await;
    let last = &settings["payload"]["last_eval"]["fast"];
    assert_eq!(last["origin"], "ide");
    assert_eq!(last["passed"], false);
    assert_eq!(settings["payload"]["eval_running"], Value::Null);

    // Revenir à l'ancien réglage, déjà évalué et retenu : pas de nouvelle évaluation.
    let before = setup.fake.requests().len();
    let back = request(
        &mut socket,
        "settings_set",
        set_payload(SHIPPED_FAST, "default", SHIPPED_STRONG, "default"),
    )
    .await;
    assert_eq!(back["type"], "settings", "{back}");
    assert_eq!(back["payload"]["eval_running"], Value::Null);
    assert_eq!(setup.fake.requests().len(), before);
    assert!(config_text(&setup).contains("fast = \"deepseek/deepseek-v4.1-flash-1017\""));
}

#[tokio::test]
async fn deux_niveaux_a_evaluer_sont_refuses() {
    let setup = setup().await;
    let mut socket = socket(&setup, Some(KEY), false).await;
    let refused = request(
        &mut socket,
        "settings_set",
        set_payload("autre/rapide", "default", "autre/rapide", "default"),
    )
    .await;
    assert_eq!(refused["payload"]["code"], "invalid_settings", "{refused}");
}

#[tokio::test]
async fn le_budget_s_applique_sans_evaluation() {
    let setup = setup().await;
    let mut socket = socket(&setup, Some(KEY), false).await;
    let mut payload = set_payload(SHIPPED_FAST, "default", SHIPPED_STRONG, "default");
    payload["budget"] = json!({ "monthly_cap_usd": 4.5, "warn_at": 0.5 });
    let settings = request(&mut socket, "settings_set", payload).await;
    assert_eq!(settings["payload"]["budget"]["monthly_cap_usd"], 4.5);
    assert!(config_text(&setup).contains("monthly_cap_usd = 4.5"));
    payload = set_payload(SHIPPED_FAST, "default", SHIPPED_STRONG, "default");
    payload["budget"] = json!({ "monthly_cap_usd": 4.5, "warn_at": 2.0 });
    let refused = request(&mut socket, "settings_set", payload).await;
    assert_eq!(refused["payload"]["code"], "invalid_settings");
}

/// Journal capturé, pour vérifier que la clé n'y apparaît jamais.
#[derive(Clone, Default)]
struct Captured(Arc<Mutex<Vec<u8>>>);

impl std::io::Write for Captured {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn la_cle_est_ecrite_en_600_jamais_renvoyee_ni_journalisee() {
    let logs = Captured::default();
    let writer = logs.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .with_writer(move || writer.clone())
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);

    let setup = setup().await;
    let mut socket = socket(&setup, None, false).await;
    let settings = request(&mut socket, "settings_get", Value::Null).await;
    assert_eq!(settings["payload"]["key_source"], "none");
    let test = request(&mut socket, "provider_test", Value::Null).await;
    assert_eq!(test["payload"]["ok"], false);

    let new_key = "sk-or-nouvelle-cle-b7c2";
    send_raw(
        &mut socket,
        &json!({ "v": 1, "type": "api_key_set", "id": "k", "payload": { "key": new_key } })
            .to_string(),
    )
    .await;
    let mut seen = Vec::new();
    let reply = loop {
        let message = next(&mut socket).await;
        seen.push(message.to_string());
        if message["re"] == "k" {
            break message;
        }
    };
    assert_eq!(reply["type"], "settings", "{reply}");
    assert_eq!(reply["payload"]["key_source"], "file");
    assert_eq!(reply["payload"]["key_hint"], "b7c2");
    let path = setup.config.path().join("credentials.toml");
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert!(fs::read_to_string(&path).unwrap().contains(new_key));

    // Mauvaise clé pour le faux OpenRouter : le test échoue sans la montrer.
    let test = request(&mut socket, "provider_test", Value::Null).await;
    assert_eq!(test["payload"]["ok"], false, "{test}");
    seen.push(test.to_string());
    assert!(seen.iter().all(|m| !m.contains(new_key)), "{seen:?}");
    let captured = String::from_utf8(logs.0.lock().unwrap().clone()).unwrap();
    assert!(
        captured.contains("clé OpenRouter enregistrée"),
        "{captured}"
    );
    assert!(!captured.contains(new_key), "{captured}");
}

#[tokio::test]
async fn la_variable_d_environnement_prime_et_le_dit() {
    let setup = setup().await;
    let (mut socket, _project) = socket_with_env(&setup).await;
    let refused = request(&mut socket, "api_key_set", json!({ "key": "sk-autre" })).await;
    assert_eq!(refused["payload"]["code"], "key_from_env", "{refused}");
    assert!(!setup.config.path().join("credentials.toml").exists());
    let test = request(&mut socket, "provider_test", Value::Null).await;
    assert_eq!(test["payload"]["ok"], true, "{test}");
}

/// Passe de l'Architecte (niveau fort) sur un ADR, et effort reçu par le modèle.
async fn strong_pass_effort(strong_effort: Effort, strong: &str) -> Effort {
    let setup = setup().await;
    let adr = "# ADR 0001 — Stockage\n\n## Décision\n\nOn garde tout en mémoire.\n";
    fs::create_dir_all(setup.project.path().join("docs/adr")).unwrap();
    fs::write(setup.project.path().join("docs/adr/0001-stockage.md"), adr).unwrap();
    setup.fake.push(Ok(hodegos_llm::LlmResponse {
        content: json!({ "remarks": [] }).to_string(),
        ..Default::default()
    }));
    let mut config = llm(&setup, Some(KEY), false, (Effort::Default, strong_effort));
    config.config.strong = Some(strong.to_owned());
    let mut socket = open_socket_with(setup.project.path(), config).await;
    // Le catalogue se lit au démarrage quand un effort est choisi.
    tokio::time::sleep(Duration::from_millis(300)).await;
    let file = request(
        &mut socket,
        "open_file",
        json!({ "path": "docs/adr/0001-stockage.md" }),
    )
    .await;
    assert_eq!(file["type"], "file", "{file}");
    request(
        &mut socket,
        "cast_override",
        json!({ "path": "docs/adr/0001-stockage.md", "section_id": "h:decision",
            "mentor_id": "architecte", "activate": true }),
    )
    .await;
    for _ in 0..20 {
        if !setup.fake.requests().is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let sent = setup.fake.requests();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].model, strong);
    sent[0].effort
}

#[tokio::test]
async fn l_effort_n_est_envoye_qu_aux_modeles_qui_l_acceptent() {
    assert_eq!(
        strong_pass_effort(Effort::Medium, SHIPPED_STRONG).await,
        Effort::Medium
    );
    // config.toml modifié à la main : le catalogue dit non, rien n'est envoyé.
    assert_eq!(
        strong_pass_effort(Effort::High, "autre/sans-schema").await,
        Effort::Default
    );
}
