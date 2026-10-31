#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Activer un mentor depuis le panneau (phase 8b) : choix forcé et passe demandée,
//! avec `FakeLlmClient` : aucun appel réseau.

mod common;

use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use common::{Socket, exchange, fake_llm, next, next_within, open_socket_with, send_raw};
use hodegos_llm::{BudgetConfig, FakeLlmClient, LlmError, LlmResponse};
use serde_json::{Value, json};

const PATH: &str = "src/emprunts/reserver.ts";

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
    write(dir.path(), PATH, RESERVER);
    dir
}

fn answer(quote: &str, message: &str) -> Result<LlmResponse, LlmError> {
    Ok(LlmResponse {
        content: json!({ "remarks": [{ "quote": quote, "level": "indice", "kind": "observation",
            "message": message, "confidence": 0.9 }] })
        .to_string(),
        cost_usd: 0.002,
        ..Default::default()
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

/// Ouvre le fichier (et consomme son `casting`), puis renvoie sa version.
async fn open(socket: &mut Socket, path: &str) -> Value {
    let file = request(socket, "open_file", json!({ "path": path })).await;
    assert_eq!(file["type"], "file", "{file}");
    assert_eq!(next(socket).await["type"], "casting");
    file["payload"]["version"].clone()
}

async fn cursor(socket: &mut Socket, line: usize) {
    send_raw(
        socket,
        &json!({ "v": 1, "type": "cursor", "payload": { "path": PATH, "line": line, "col": 1 } })
            .to_string(),
    )
    .await;
}

/// Messages spontanés jusqu'au `status` qui clôt la passe.
async fn until_status(socket: &mut Socket) -> Vec<Value> {
    let mut messages = Vec::new();
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

fn activate(section_id: Option<&str>, mentor_id: Option<&str>, confirm: bool) -> Value {
    json!({ "path": PATH, "section_id": section_id, "mentor_id": mentor_id,
        "activate": true, "confirm_over_budget": confirm })
}

#[tokio::test]
async fn activer_relit_la_section_du_curseur_sans_changement() {
    let dir = sample();
    let fake = Arc::new(FakeLlmClient::new([answer(
        "e.debut === debut",
        "Comment ces deux dates sont-elles comparées ?",
    )]));
    let mut socket = open_socket_with(dir.path(), fake_llm(fake.clone())).await;
    open(&mut socket, PATH).await;
    cursor(&mut socket, 8).await;
    assert_eq!(
        next(&mut socket).await["payload"]["section_id"],
        "fn:annuler"
    );

    let casting = request(
        &mut socket,
        "cast_override",
        activate(None, Some("architecte"), false),
    )
    .await;
    assert_eq!(casting["type"], "casting", "{casting}");
    assert_eq!(casting["payload"]["titular"]["id"], "architecte");
    let messages = until_status(&mut socket).await;
    let started = of_type(&messages, "pass_started");
    assert_eq!(started.len(), 1, "{messages:?}");
    assert_eq!(started[0]["payload"]["sections"], json!(["fn:annuler"]));
    assert_eq!(started[0]["payload"]["mentors"][0]["id"], "architecte");
    let status = &messages.last().unwrap()["payload"];
    assert_eq!(status["last_pass"]["outcome"], "ok", "{status}");

    let sent = fake.requests();
    assert_eq!(sent.len(), 1);
    assert_eq!(
        sent[0].model, "test/fort",
        "l'Architecte est sur le modèle fort"
    );
    let user = &sent[0].messages[1].content;
    assert!(user.contains("<section id=\"fn:annuler\">"), "{user}");
    assert!(user.contains("<diff>\n\n</diff>"), "diff vide : {user}");
}

#[tokio::test]
async fn la_section_du_message_l_emporte_sur_le_curseur() {
    let dir = sample();
    let fake = Arc::new(FakeLlmClient::new([answer(
        "e.debut === debut",
        "Comment ces deux dates sont-elles comparées ?",
    )]));
    let mut socket = open_socket_with(dir.path(), fake_llm(fake.clone())).await;
    open(&mut socket, PATH).await;

    request(
        &mut socket,
        "cast_override",
        activate(Some("fn:reserver"), Some("architecte"), false),
    )
    .await;
    let messages = until_status(&mut socket).await;
    let started = of_type(&messages, "pass_started");
    assert_eq!(started[0]["payload"]["sections"], json!(["fn:reserver"]));
    let remarks = of_type(&messages, "remarks");
    assert_eq!(remarks[0]["payload"]["items"][0]["mentor_id"], "architecte");
    assert_eq!(remarks[0]["payload"]["items"][0]["anchor"]["line_hint"], 4);
}

#[tokio::test]
async fn revenir_au_choix_automatique_ne_lance_rien() {
    let dir = sample();
    let fake = Arc::new(FakeLlmClient::default());
    let mut socket = open_socket_with(dir.path(), fake_llm(fake.clone())).await;
    open(&mut socket, PATH).await;
    cursor(&mut socket, 4).await;
    next(&mut socket).await;

    let casting = request(
        &mut socket,
        "cast_override",
        activate(Some("fn:reserver"), None, false),
    )
    .await;
    assert_eq!(casting["type"], "casting");
    assert_eq!(casting["payload"]["forced"], Value::Null);
    assert_eq!(
        next_within(&mut socket, Duration::from_millis(500)).await,
        None
    );

    // Sans `activate` : choix forcé seul, comme avant la phase 8b.
    let casting = request(
        &mut socket,
        "cast_override",
        json!({ "path": PATH, "section_id": "fn:reserver", "mentor_id": "architecte" }),
    )
    .await;
    assert_eq!(casting["payload"]["titular"]["id"], "architecte");
    assert_eq!(
        next_within(&mut socket, Duration::from_millis(500)).await,
        None
    );
    assert!(fake.requests().is_empty());
}

#[tokio::test]
async fn au_dela_du_budget_l_activation_attend_confirmation() {
    let dir = sample();
    let fake = Arc::new(FakeLlmClient::new([answer(
        "e.debut === debut",
        "Comment ces deux dates sont-elles comparées ?",
    )]));
    let mut llm = fake_llm(fake.clone());
    llm.config.budget = BudgetConfig {
        monthly_cap_usd: 0.0,
        warn_at: 0.8,
    };
    let mut socket = open_socket_with(dir.path(), llm).await;
    open(&mut socket, PATH).await;

    let refused = request(
        &mut socket,
        "cast_override",
        activate(Some("fn:reserver"), Some("architecte"), false),
    )
    .await;
    assert_eq!(refused["type"], "error");
    assert_eq!(refused["payload"]["code"], "budget_exceeded");
    assert!(fake.requests().is_empty());
    // Le choix forcé n'est pas appliqué : la nouvelle tentative repart d'un état propre.
    cursor(&mut socket, 9).await;
    next(&mut socket).await;
    cursor(&mut socket, 4).await;
    let casting = next(&mut socket).await;
    assert_eq!(
        casting["payload"]["titular"]["id"], "mentor-langage",
        "{casting}"
    );

    let confirmed = request(
        &mut socket,
        "cast_override",
        activate(Some("fn:reserver"), Some("architecte"), true),
    )
    .await;
    assert_eq!(confirmed["payload"]["titular"]["id"], "architecte");
    let messages = until_status(&mut socket).await;
    assert_eq!(of_type(&messages, "pass_started").len(), 1, "{messages:?}");
    assert_eq!(fake.requests().len(), 1);
}

#[tokio::test]
async fn enregistrer_puis_activer_ne_fait_qu_une_passe() {
    let dir = sample();
    let fake = Arc::new(FakeLlmClient::new([answer(
        "dejaPris === undefined",
        "Que renvoie find quand rien ne correspond ?",
    )]));
    let mut socket = open_socket_with(dir.path(), fake_llm(fake.clone())).await;
    let version = open(&mut socket, PATH).await;
    let edited = RESERVER.replace("return !dejaPris;", "return dejaPris === undefined;");

    let saved = request(
        &mut socket,
        "save",
        json!({ "path": PATH, "content": edited, "base_version": version, "skip_pass": true }),
    )
    .await;
    assert_eq!(saved["type"], "saved", "{saved}");
    assert_eq!(
        next_within(&mut socket, Duration::from_millis(500)).await,
        None,
        "aucune passe après un enregistrement avec skip_pass"
    );

    request(
        &mut socket,
        "cast_override",
        activate(Some("fn:reserver"), Some("architecte"), false),
    )
    .await;
    let messages = until_status(&mut socket).await;
    assert_eq!(of_type(&messages, "pass_started").len(), 1);
    let sent = fake.requests();
    assert_eq!(sent.len(), 1, "une seule passe");
    assert!(
        sent[0].messages[1]
            .content
            .contains("return dejaPris === undefined;"),
        "la version enregistrée"
    );
}

#[tokio::test]
async fn un_fichier_sensible_active_n_est_pas_envoye() {
    let dir = sample();
    write(
        dir.path(),
        "src/cle.ts",
        "export const k = `\n-----BEGIN RSA PRIVATE KEY-----\nabc\n`;\n",
    );
    let fake = Arc::new(FakeLlmClient::default());
    let mut socket = open_socket_with(dir.path(), fake_llm(fake.clone())).await;
    open(&mut socket, "src/cle.ts").await;

    request(
        &mut socket,
        "cast_override",
        json!({ "path": "src/cle.ts", "mentor_id": "architecte", "activate": true }),
    )
    .await;
    let messages = until_status(&mut socket).await;
    assert!(of_type(&messages, "pass_started").is_empty());
    assert_eq!(
        messages.last().unwrap()["payload"]["last_pass"]["outcome"],
        "excluded"
    );
    assert!(fake.requests().is_empty());
}
