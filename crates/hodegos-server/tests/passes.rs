#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Passes proactives, avec `FakeLlmClient` : aucun appel réseau.

mod common;

use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use common::{
    Socket, data_dir, exchange, fake_llm, next, next_within, open_socket_with, project_in,
    session_cookie,
};
use hodegos_llm::{FakeLlmClient, LlmError, LlmResponse};
use hodegos_server::Server;
use serde_json::{Value, json};

const RESERVER: &str = "\
import { Emprunt, Objet } from \"../modele\";

export function reserver(objet: Objet, debut: Date, emprunts: Emprunt[]) {
  const dejaPris = emprunts.find((e) => e.debut === debut);
  return !dejaPris;
}

export function annuler(id: string) {
  return id;
}
";

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn sample() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "src/emprunts/reserver.ts", RESERVER);
    dir
}

fn remark(quote: &str, message: &str, confidence: f64) -> Value {
    json!({ "quote": quote, "level": "indice", "kind": "observation", "message": message, "confidence": confidence })
}

fn answer(remarks: Vec<Value>, cost: f64) -> Result<LlmResponse, LlmError> {
    Ok(LlmResponse {
        content: json!({ "remarks": remarks }).to_string(),
        cost_usd: cost,
    })
}

async fn request(socket: &mut Socket, kind: &str, payload: Value) -> Value {
    let mut reply = exchange(
        socket,
        json!({ "v": 1, "type": kind, "id": "c", "payload": payload }),
    )
    .await;
    while reply["re"] != "c" {
        reply = next(socket).await;
    }
    reply
}

/// Ouvre le fichier (et consomme son `casting`), puis renvoie son contenu et sa version.
async fn open(socket: &mut Socket, path: &str) -> Value {
    let file = request(socket, "open_file", json!({ "path": path })).await;
    assert_eq!(file["type"], "file", "{file}");
    let casting = next(socket).await;
    assert_eq!(casting["type"], "casting");
    file
}

/// Enregistre et renvoie les messages jusqu'au `status` qui clôt la passe.
async fn save(socket: &mut Socket, path: &str, content: &str, base: &Value) -> Vec<Value> {
    let saved = request(
        socket,
        "save",
        json!({ "path": path, "content": content, "base_version": base }),
    )
    .await;
    assert_eq!(saved["type"], "saved", "{saved}");
    let mut messages = vec![saved];
    loop {
        let message = next(socket).await;
        let done = message["type"] == "status";
        messages.push(message);
        if done {
            return messages;
        }
    }
}

fn of_type<'a>(messages: &'a [Value], kind: &str) -> Vec<&'a Value> {
    messages.iter().filter(|m| m["type"] == kind).collect()
}

fn last_pass(messages: &[Value]) -> &Value {
    &messages.last().unwrap()["payload"]["last_pass"]
}

fn edited() -> String {
    RESERVER.replace("return !dejaPris;", "return dejaPris === undefined;")
}

#[tokio::test]
async fn enregistrer_produit_au_plus_deux_remarques_ancrees_et_persistees() {
    let dir = sample();
    let fake = Arc::new(FakeLlmClient::new([answer(
        vec![
            remark(
                "e.debut === debut",
                "Regardez comment cette ligne compare deux dates.",
                0.9,
            ),
            remark(
                "dejaPris === undefined",
                "Que vaut find quand rien ne correspond ?",
                0.7,
            ),
            remark("emprunts.find", "Faible.", 0.6),
        ],
        0.002,
    )]));
    let mut socket = open_socket_with(dir.path(), fake_llm(fake.clone())).await;
    let file = open(&mut socket, "src/emprunts/reserver.ts").await;
    let messages = save(
        &mut socket,
        "src/emprunts/reserver.ts",
        &edited(),
        &file["payload"]["version"],
    )
    .await;

    let started = of_type(&messages, "pass_started");
    assert_eq!(started.len(), 1, "{messages:?}");
    assert_eq!(started[0]["payload"]["sections"], json!(["fn:reserver"]));
    assert_eq!(started[0]["payload"]["mentors"][0]["id"], "mentor-langage");

    let remarks = of_type(&messages, "remarks");
    let items = remarks[0]["payload"]["items"].as_array().unwrap();
    assert_eq!(items.len(), 2, "plafond de deux remarques");
    let first = &items[0];
    assert_eq!(first["mentor_id"], "mentor-langage");
    assert_eq!(first["path"], "src/emprunts/reserver.ts");
    assert_eq!(
        first["anchor"],
        json!({ "section_id": "fn:reserver", "quote": "e.debut === debut", "line_hint": 4 })
    );
    assert_eq!(first["anchor"]["line_hint"], 4);
    assert_eq!(items[1]["anchor"]["line_hint"], 5);
    assert_eq!(first["level"], "indice");
    assert_eq!(first["levels_given"], json!(["indice"]));
    assert_eq!(first["status"], "open");
    assert_eq!(first["thread"][0]["author"], "mentor");
    assert!(first["id"].as_str().unwrap().starts_with("rm_"));

    let status = &messages.last().unwrap()["payload"];
    assert_eq!(status["provider_ok"], true);
    assert_eq!(status["last_pass"]["outcome"], "ok");
    assert_eq!(status["last_pass"]["remarks"], 2);
    assert!((status["budget_used"].as_f64().unwrap() - 0.002).abs() < 1e-9);

    // Le prompt : cadre commun, posture du mentor, section et diff.
    let requests = fake.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].model, "test/rapide");
    assert!(
        requests[0].messages[0]
            .content
            .starts_with("Vous êtes Mentor du langage")
    );
    let user = &requests[0].messages[1].content;
    assert!(user.contains("<section id=\"fn:reserver\">"), "{user}");
    assert!(user.contains("+  return dejaPris === undefined;"), "{user}");
    assert!(user.ends_with("Palier demandé : indice."));

    // Réouverture, puis redémarrage : les remarques ouvertes reviennent avec le fichier.
    let reopened = open(&mut socket, "src/emprunts/reserver.ts").await;
    assert_eq!(reopened["payload"]["remarks"].as_array().unwrap().len(), 2);
    drop(socket);
    let server = Server::bind(
        project_in(dir.path()),
        common::options(dir.path(), 0, fake_llm(fake.clone())),
    )
    .await
    .unwrap();
    let port = server.port();
    let cookie = session_cookie(&server).await;
    tokio::spawn(server.serve(std::future::pending()));
    let origin = format!("http://127.0.0.1:{port}");
    let mut socket = common::connect(port, Some(&origin), Some(&cookie))
        .await
        .unwrap();
    next(&mut socket).await;
    let after_restart = open(&mut socket, "src/emprunts/reserver.ts").await;
    assert_eq!(
        after_restart["payload"]["remarks"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}

#[tokio::test]
async fn deux_sections_modifiees_la_section_du_curseur_d_abord_et_deux_remarques_au_total() {
    let dir = sample();
    let both = || {
        answer(
            vec![
                remark("return", "Première.", 0.9),
                remark("return", "Deuxième.", 0.6),
            ],
            0.001,
        )
    };
    let fake = Arc::new(FakeLlmClient::new([both(), both()]));
    let mut socket = open_socket_with(dir.path(), fake_llm(fake.clone())).await;
    let file = open(&mut socket, "src/emprunts/reserver.ts").await;
    exchange(
        &mut socket,
        json!({ "v": 1, "type": "cursor", "payload": { "path": "src/emprunts/reserver.ts", "line": 9, "col": 1 } }),
    )
    .await;
    let content = edited().replace("return id;", "return id.trim();");
    let messages = save(
        &mut socket,
        "src/emprunts/reserver.ts",
        &content,
        &file["payload"]["version"],
    )
    .await;

    let started = of_type(&messages, "pass_started");
    assert_eq!(
        started[0]["payload"]["sections"],
        json!(["fn:annuler", "fn:reserver"])
    );
    assert_eq!(fake.requests().len(), 2);
    let items = of_type(&messages, "remarks")[0]["payload"]["items"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(items.len(), 2);
    assert!(
        items.iter().all(|r| r["thread"][0]["text"] == "Première."),
        "{items:?}"
    );
}

#[tokio::test]
async fn un_fichier_sensible_n_est_jamais_envoye() {
    let dir = sample();
    write(dir.path(), "src/cle.ts", "export const k = 1;\n");
    for (path, content) in [
        (".env.local", "API=1\n"),
        ("certs/serveur.pem", "certificat\n"),
        ("id_rsa", "cle\n"),
        (
            "src/cle.ts",
            "export const k = `\n-----BEGIN RSA PRIVATE KEY-----\nabc\n`;\n",
        ),
    ] {
        if !dir.path().join(path).exists() {
            write(dir.path(), path, "avant\n");
        }
        let fake = Arc::new(FakeLlmClient::default());
        let mut socket = open_socket_with(dir.path(), fake_llm(fake.clone())).await;
        let file = open(&mut socket, path).await;
        let messages = save(&mut socket, path, content, &file["payload"]["version"]).await;
        assert!(of_type(&messages, "pass_started").is_empty(), "{path}");
        assert_eq!(last_pass(&messages)["outcome"], "excluded", "{path}");
        assert!(fake.requests().is_empty(), "{path} envoyé au modèle");
    }
}

#[tokio::test]
async fn un_secret_dans_une_section_est_masque() {
    let dir = sample();
    let fake = Arc::new(FakeLlmClient::new([answer(vec![], 0.001)]));
    let mut socket = open_socket_with(dir.path(), fake_llm(fake.clone())).await;
    let file = open(&mut socket, "src/emprunts/reserver.ts").await;
    let content = RESERVER.replace(
        "  return !dejaPris;",
        "  const jeton = \"ghp_0123456789abcdefghijABCDEFGHIJ012345\";\n  const password = \"hunter22\";\n  return !dejaPris;",
    );
    let messages = save(
        &mut socket,
        "src/emprunts/reserver.ts",
        &content,
        &file["payload"]["version"],
    )
    .await;
    assert_eq!(last_pass(&messages)["outcome"], "ok");
    let sent: String = fake.requests()[0]
        .messages
        .iter()
        .map(|m| m.content.clone())
        .collect();
    assert!(sent.contains("[secret masqué]"), "{sent}");
    assert!(!sent.contains("ghp_0123456789"), "{sent}");
    assert!(!sent.contains("hunter22"), "{sent}");
}

#[tokio::test]
async fn budget_avertit_a_80_pour_cent_et_bloque_a_100() {
    let dir = sample();
    let month = jiff::Timestamp::now().strftime("%Y-%m").to_string();
    let usage = data_dir(dir.path()).join("usage.json");
    write(
        dir.path(),
        ".git/hodegos-donnees/usage.json",
        &json!({ "months": { &month: 7.9 } }).to_string(),
    );

    let fake = Arc::new(FakeLlmClient::new([answer(vec![], 0.2)]));
    let mut socket = open_socket_with(dir.path(), fake_llm(fake.clone())).await;
    let file = open(&mut socket, "src/emprunts/reserver.ts").await;
    let messages = save(
        &mut socket,
        "src/emprunts/reserver.ts",
        &edited(),
        &file["payload"]["version"],
    )
    .await;
    let status = &messages.last().unwrap()["payload"];
    assert_eq!(status["budget_level"], "warning", "{status}");
    assert_eq!(status["budget_cap"], 10.0);

    fs::write(&usage, json!({ "months": { &month: 10.0 } }).to_string()).unwrap();
    drop(socket);
    let fake = Arc::new(FakeLlmClient::new([answer(vec![], 0.2)]));
    let mut socket = open_socket_with(dir.path(), fake_llm(fake.clone())).await;
    let file = open(&mut socket, "src/emprunts/reserver.ts").await;
    let messages = save(
        &mut socket,
        "src/emprunts/reserver.ts",
        RESERVER,
        &file["payload"]["version"],
    )
    .await;
    assert_eq!(last_pass(&messages)["outcome"], "budget_exceeded");
    assert_eq!(
        messages.last().unwrap()["payload"]["budget_level"],
        "exceeded"
    );
    assert!(fake.requests().is_empty(), "aucun appel au-delà du budget");
}

#[tokio::test]
async fn panne_du_fournisseur_passe_abandonnee_sans_nouvelle_tentative() {
    let dir = sample();
    let fake = Arc::new(FakeLlmClient::new([Err(LlmError::Timeout)]));
    let mut socket = open_socket_with(dir.path(), fake_llm(fake.clone())).await;
    let file = open(&mut socket, "src/emprunts/reserver.ts").await;
    let messages = save(
        &mut socket,
        "src/emprunts/reserver.ts",
        &edited(),
        &file["payload"]["version"],
    )
    .await;
    let status = &messages.last().unwrap()["payload"];
    assert_eq!(status["provider_ok"], false);
    assert!(
        status["provider_message"]
            .as_str()
            .unwrap()
            .contains("délai dépassé")
    );
    assert_eq!(status["last_pass"]["outcome"], "provider_error");
    assert!(
        of_type(&messages, "remarks")[0]["payload"]["items"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    assert_eq!(
        next_within(&mut socket, Duration::from_millis(1500)).await,
        None
    );
    assert_eq!(fake.requests().len(), 1, "aucune nouvelle tentative");

    // L'enregistrement suivant retente, et rétablit l'état du fournisseur.
    fake.push(answer(vec![], 0.001));
    let version = request(
        &mut socket,
        "open_file",
        json!({ "path": "src/emprunts/reserver.ts" }),
    )
    .await;
    next(&mut socket).await;
    let messages = save(
        &mut socket,
        "src/emprunts/reserver.ts",
        RESERVER,
        &version["payload"]["version"],
    )
    .await;
    assert_eq!(fake.requests().len(), 2);
    assert_eq!(messages.last().unwrap()["payload"]["provider_ok"], true);
}

#[tokio::test]
async fn sans_changement_sans_mentor_ou_sans_fournisseur_aucune_passe() {
    let dir = sample();
    write(dir.path(), "notes.txt", "avant\n");
    let fake = Arc::new(FakeLlmClient::default());
    let mut socket = open_socket_with(dir.path(), fake_llm(fake.clone())).await;

    let file = open(&mut socket, "src/emprunts/reserver.ts").await;
    let messages = save(
        &mut socket,
        "src/emprunts/reserver.ts",
        RESERVER,
        &file["payload"]["version"],
    )
    .await;
    assert_eq!(last_pass(&messages)["outcome"], "no_change");

    let file = open(&mut socket, "notes.txt").await;
    let messages = save(
        &mut socket,
        "notes.txt",
        "après\n",
        &file["payload"]["version"],
    )
    .await;
    assert_eq!(last_pass(&messages)["outcome"], "no_mentor");
    assert!(fake.requests().is_empty());

    let mut socket = common::open_socket(dir.path()).await;
    let file = open(&mut socket, "src/emprunts/reserver.ts").await;
    let messages = save(
        &mut socket,
        "src/emprunts/reserver.ts",
        &edited(),
        &file["payload"]["version"],
    )
    .await;
    assert_eq!(last_pass(&messages)["outcome"], "provider_error");
    assert_eq!(
        messages.last().unwrap()["payload"]["provider_message"],
        "aucun fournisseur dans les tests"
    );
}

#[tokio::test]
async fn deux_reponses_invalides_donnent_invalid_response() {
    let dir = sample();
    let bad = || {
        Ok(LlmResponse {
            content: "pas du json".to_owned(),
            cost_usd: 0.001,
        })
    };
    let fake = Arc::new(FakeLlmClient::new([bad(), bad()]));
    let mut socket = open_socket_with(dir.path(), fake_llm(fake.clone())).await;
    let file = open(&mut socket, "src/emprunts/reserver.ts").await;
    let messages = save(
        &mut socket,
        "src/emprunts/reserver.ts",
        &edited(),
        &file["payload"]["version"],
    )
    .await;
    assert_eq!(last_pass(&messages)["outcome"], "invalid_response");
    assert_eq!(fake.requests().len(), 2, "une seule relance");
}

#[tokio::test]
async fn un_nouvel_enregistrement_rend_la_passe_precedente_caduque() {
    let dir = sample();
    let fake = Arc::new(
        FakeLlmClient::new([
            answer(vec![remark("e.debut === debut", "Ancienne.", 0.9)], 0.001),
            answer(vec![remark("e.debut === debut", "Récente.", 0.9)], 0.001),
        ])
        .with_delay(Duration::from_millis(300)),
    );
    let mut socket = open_socket_with(dir.path(), fake_llm(fake.clone())).await;
    let file = open(&mut socket, "src/emprunts/reserver.ts").await;
    let first = request(
        &mut socket,
        "save",
        json!({ "path": "src/emprunts/reserver.ts", "content": edited(), "base_version": file["payload"]["version"] }),
    )
    .await;
    let second = request(
        &mut socket,
        "save",
        json!({ "path": "src/emprunts/reserver.ts", "content": RESERVER, "base_version": first["payload"]["version"] }),
    )
    .await;
    assert_eq!(second["type"], "saved");

    let mut texts = Vec::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while texts.is_empty() && tokio::time::Instant::now() < deadline {
        let message = next(&mut socket).await;
        if message["type"] == "remarks" {
            for item in message["payload"]["items"].as_array().unwrap() {
                texts.push(item["thread"][0]["text"].as_str().unwrap().to_owned());
            }
        }
    }
    assert_eq!(texts.len(), 1, "{texts:?}");
    assert_eq!(fake.requests().len(), 2);
}
