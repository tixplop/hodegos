#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Passages cités et réévaluation (phase 9d), avec `FakeLlmClient` et une horloge
//! simulée : remarque `passage` détachée, remarque `project` à réévaluer puis
//! maintenue ou levée au calme, réponse illisible, fichier supprimé ou déplacé depuis
//! Hodegos.

mod common;

use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use common::{Socket, data_dir, fake_llm, next_within, project_in};
use hodegos_llm::{FakeLlmClient, LlmError, LlmResponse};
use hodegos_server::{ManualClock, ReviewOptions, Server, ServerOptions};
use jiff::Timestamp;
use serde_json::{Value, json};

const ANNULER: &str = "export function annuler(id: string): void {\n  const brut = readFileSync(\"data/emprunts.json\", \"utf8\");\n  console.log(brut, id);\n}\n";
const EMAIL: &str =
    "export function prevenir(): void {\n  writeFileSync(\"data/emprunts.json\", \"[]\");\n}\n";
const DEPOT: &str = "export function lister(): string[] {\n  return [];\n}\n";

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::create_dir(root.join(".git")).unwrap();
    write(root, "src/emprunts/annuler.ts", ANNULER);
    write(root, "src/notifications/email.ts", EMAIL);
    write(root, "src/stockage/depot.ts", DEPOT);
    fs::create_dir_all(root.join("src/archives")).unwrap();
    write(
        root,
        ".hodegos/mentors/archi.md",
        "---\nid: archi\nname: Archi\ninitials: AR\nrole: Architecte de test\nmodel: strong\nscopes: [project]\nproject:\n  key_files: [\"src/**\"]\n---\nRelisez.\n",
    );
    dir
}

fn response(content: Value) -> Result<LlmResponse, LlmError> {
    Ok(LlmResponse {
        content: content.to_string(),
        cost_usd: 0.001,
        ..Default::default()
    })
}

fn review() -> Result<LlmResponse, LlmError> {
    response(json!({ "remarks": [
        { "about": "project", "quotes": [
            { "path": "src/emprunts/annuler.ts", "quote": "readFileSync(\"data/emprunts.json\"" },
            { "path": "src/notifications/email.ts", "quote": "writeFileSync(\"data/emprunts.json\"" }
          ], "level": "indice", "kind": "question",
          "message": "Comptez les fichiers qui touchent ce fichier de données.", "confidence": 0.9 },
        { "about": "passage", "quotes": [
            { "path": "src/stockage/depot.ts", "quote": "return [];" }
          ], "level": "indice", "kind": "observation",
          "message": "Regardez ce que renvoie cette fonction.", "confidence": 0.8 }
    ] }))
}

/// Revues automatiques coupées (aucun déclencheur) : seule la réévaluation tourne.
async fn open(dir: &Path, fake: Arc<FakeLlmClient>, clock: Arc<ManualClock>) -> Socket {
    let options = ServerOptions {
        review: ReviewOptions {
            triggers: vec![],
            tick: Duration::from_millis(20),
            ..ReviewOptions::default()
        },
        clock,
        ..common::options(dir, 0, fake_llm(fake))
    };
    let server = Server::bind(project_in(dir), options).await.unwrap();
    let port = server.port();
    let cookie = common::session_cookie(&server).await;
    tokio::spawn(server.serve(std::future::pending()));
    let origin = format!("http://127.0.0.1:{port}");
    let mut socket = common::connect(port, Some(&origin), Some(&cookie))
        .await
        .unwrap();
    common::welcome(&mut socket).await;
    socket
}

async fn request(socket: &mut Socket, id: &str, kind: &str, payload: Value) -> Value {
    common::send_raw(
        socket,
        &json!({ "v": 1, "type": kind, "id": id, "payload": payload }).to_string(),
    )
    .await;
    loop {
        let message = next_within(socket, Duration::from_secs(10))
            .await
            .expect("réponse");
        if message["re"] == id {
            return message;
        }
    }
}

/// Lance une revue et renvoie les identifiants de ses remarques (projet, passage).
async fn reviewed(socket: &mut Socket) -> (String, String) {
    request(
        socket,
        "r-1",
        "review_request",
        json!({ "mentor_ids": ["archi"] }),
    )
    .await;
    loop {
        let message = next_within(socket, Duration::from_secs(10))
            .await
            .expect("project_remarks");
        if message["type"] == "project_remarks" {
            let items = message["payload"]["items"].as_array().unwrap();
            assert_eq!(items.len(), 2, "{items:?}");
            return (
                items[0]["id"].as_str().unwrap().to_owned(),
                items[1]["id"].as_str().unwrap().to_owned(),
            );
        }
    }
}

/// Prochain `remark_update` de la remarque `id`, ou `None` pendant `wait`.
async fn update_of(socket: &mut Socket, id: &str, wait: Duration) -> Option<Value> {
    let deadline = tokio::time::Instant::now() + wait;
    loop {
        let left = deadline.checked_duration_since(tokio::time::Instant::now())?;
        let message = next_within(socket, left).await?;
        if message["type"] == "remark_update" && message["payload"]["id"] == id {
            return Some(message["payload"].clone());
        }
    }
}

fn start() -> Arc<ManualClock> {
    Arc::new(ManualClock::new(
        Timestamp::from_second(1_793_000_000).unwrap(),
    ))
}

const SOON: Duration = Duration::from_secs(5);
const QUIET: Duration = Duration::from_millis(500);

#[tokio::test]
async fn passage_detache_et_remarque_de_projet_maintenue_puis_levee() {
    let dir = project();
    let root = dir.path();
    let fake = Arc::new(FakeLlmClient::new([review()]));
    let clock = start();
    let mut socket = open(root, fake.clone(), clock.clone()).await;
    let (project_id, passage_id) = reviewed(&mut socket).await;

    // Enregistrement depuis Hodegos : le passage cité disparaît, la remarque aussi.
    let file = request(
        &mut socket,
        "c-1",
        "open_file",
        json!({ "path": "src/stockage/depot.ts" }),
    )
    .await;
    assert_eq!(
        file["payload"]["project_remarks"][0]["id"],
        passage_id.as_str()
    );
    request(
        &mut socket,
        "c-2",
        "save",
        json!({ "path": "src/stockage/depot.ts", "content": DEPOT.replace("return [];", "return lire();"),
            "base_version": file["payload"]["version"], "skip_pass": true }),
    )
    .await;
    let detached = update_of(&mut socket, &passage_id, SOON)
        .await
        .expect("détachée");
    assert_eq!(detached["status"], "detached");
    assert_eq!(detached["anchors"][0]["status"], "gone");
    let listed = request(&mut socket, "c-3", "project_remarks_get", Value::Null).await;
    let ids: Vec<&str> = listed["payload"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, [project_id.as_str()]);

    // Modification faite ailleurs : un passage de la remarque de projet disparaît.
    write(
        root,
        "src/notifications/email.ts",
        "export function prevenir(): void {}\n",
    );
    let waiting = update_of(&mut socket, &project_id, SOON)
        .await
        .expect("à réévaluer");
    assert_eq!(waiting["status"], "to_review");
    let statuses: Vec<(&str, &str)> = waiting["anchors"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| (a["path"].as_str().unwrap(), a["status"].as_str().unwrap()))
        .collect();
    assert_eq!(
        statuses,
        [
            ("src/emprunts/annuler.ts", "ok"),
            ("src/notifications/email.ts", "gone")
        ]
    );
    // Pas avant le calme.
    assert_eq!(update_of(&mut socket, &project_id, QUIET).await, None);
    assert_eq!(
        fake.requests().len(),
        1,
        "{:?}",
        fake.requests()
            .iter()
            .map(|r| r.schema_name.clone())
            .collect::<Vec<_>>()
    );

    // Au calme : maintenue, avec une nouvelle citation.
    fake.push(response(json!({ "verdict": "keep", "quotes": [
        { "path": "src/emprunts/annuler.ts", "quote": "console.log(brut, id)" }
    ], "reason": "" })));
    clock.advance(Duration::from_secs(181));
    let kept = update_of(&mut socket, &project_id, SOON)
        .await
        .expect("maintenue");
    assert_eq!(kept["status"], "open");
    assert_eq!(kept["anchors"].as_array().unwrap().len(), 1);
    assert_eq!(kept["anchors"][0]["quote"], "console.log(brut, id)");
    assert_eq!(kept["anchors"][0]["line_hint"], 3);
    assert_eq!(
        kept["thread"].as_array().unwrap().len(),
        1,
        "message inchangé"
    );
    let sent = fake.requests().last().unwrap().clone();
    assert!(
        sent.messages[0]
            .content
            .contains("Réévaluation d'une remarque sur le projet")
    );
    assert!(sent.messages[1].content.contains(
        "<passages_disparus>\n<citation chemin=\"src/notifications/email.ts\">writeFileSync(&quot;data/emprunts.json&quot;</citation>"
    ) || sent.messages[1].content.contains("<citation chemin=\"src/notifications/email.ts\">writeFileSync("),
        "{}", sent.messages[1].content);
    assert_eq!(
        sent.schema["required"],
        json!(["verdict", "quotes", "reason"])
    );

    // Puis levée : la raison s'ajoute au fil.
    write(
        root,
        "src/emprunts/annuler.ts",
        "export function annuler(id: string): void {}\n",
    );
    let waiting = update_of(&mut socket, &project_id, SOON)
        .await
        .expect("à réévaluer");
    assert_eq!(waiting["status"], "to_review");
    fake.push(response(json!({ "verdict": "withdraw", "quotes": [],
        "reason": "Annuler ne lit plus le fichier de données lui-même." })));
    clock.advance(Duration::from_secs(181));
    let withdrawn = update_of(&mut socket, &project_id, SOON)
        .await
        .expect("levée");
    assert_eq!(withdrawn["status"], "resolved");
    let thread = withdrawn["thread"].as_array().unwrap();
    assert_eq!(
        thread.last().unwrap()["text"],
        "Annuler ne lit plus le fichier de données lui-même."
    );
    assert_eq!(thread.last().unwrap()["author"], "mentor");
    let status = request(&mut socket, "c-4", "project_remarks_get", Value::Null).await;
    assert!(status["payload"]["items"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn reponse_illisible_puis_fichiers_deplaces_et_supprimes_depuis_hodegos() {
    let dir = project();
    let root = dir.path();
    let fake = Arc::new(FakeLlmClient::new([review()]));
    let clock = start();
    let mut socket = open(root, fake.clone(), clock.clone()).await;
    let (project_id, _) = reviewed(&mut socket).await;

    // Déplacé depuis Hodegos : la citation suit le fichier.
    let moved = request(
        &mut socket,
        "m-1",
        "move_entry",
        json!({ "from": "src/emprunts/annuler.ts", "to_dir": "src/archives" }),
    )
    .await;
    assert_eq!(moved["type"], "entry_moved", "{moved}");
    let listed = request(&mut socket, "m-2", "project_remarks_get", Value::Null).await;
    let remark = listed["payload"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == project_id.as_str())
        .unwrap()
        .clone();
    assert_eq!(remark["anchors"][0]["path"], "src/archives/annuler.ts");
    assert_eq!(remark["status"], "open");

    // Écriture impossible : le déplacement est annulé, la citation ne bouge pas.
    let id = project_in(root).id().as_str().to_owned();
    let stored = data_dir(root).join(&id).join("project_remarks.json");
    let saved = fs::read_to_string(&stored).unwrap();
    fs::remove_file(&stored).unwrap();
    fs::create_dir(&stored).unwrap();
    let refused = request(
        &mut socket,
        "m-3",
        "move_entry",
        json!({ "from": "src/archives/annuler.ts", "to_dir": "src/emprunts" }),
    )
    .await;
    assert_eq!(refused["type"], "error", "{refused}");
    assert!(root.join("src/archives/annuler.ts").exists());
    fs::remove_dir(&stored).unwrap();
    fs::write(&stored, saved).unwrap();

    // Supprimé depuis Hodegos : le passage disparaît, la remarque est à réévaluer.
    let deleted = request(
        &mut socket,
        "d-1",
        "delete_entry",
        json!({ "path": "src/notifications/email.ts" }),
    )
    .await;
    assert_eq!(deleted["type"], "entry_deleted", "{deleted}");
    let waiting = update_of(&mut socket, &project_id, SOON)
        .await
        .expect("à réévaluer");
    assert_eq!(waiting["status"], "to_review");
    assert_eq!(waiting["anchors"][1]["status"], "gone");

    // Réponse illisible, deux fois : la remarque reste à réévaluer, sans nouvel essai
    // avant la revue suivante du mentor.
    fake.push(response(json!("pas un verdict")));
    fake.push(response(json!("toujours pas")));
    // La suppression est aussi vue par la carte (regroupée sur 500 ms) : le calme compte
    // depuis ce recalcul.
    tokio::time::sleep(Duration::from_millis(900)).await;
    clock.advance(Duration::from_secs(181));
    tokio::time::sleep(Duration::from_millis(600)).await;
    assert_eq!(fake.requests().len(), 3, "une relance, puis rien");
    let listed = request(&mut socket, "d-2", "project_remarks_get", Value::Null).await;
    assert_eq!(listed["payload"]["items"][0]["status"], "to_review");
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(fake.requests().len(), 3);

    // Après une revue du mentor, la réévaluation est retentée.
    fake.push(response(json!({ "remarks": [] })));
    fake.push(response(
        json!({ "verdict": "withdraw", "quotes": [], "reason": "Le fichier a été supprimé." }),
    ));
    request(
        &mut socket,
        "d-3",
        "review_request",
        json!({ "mentor_ids": ["archi"] }),
    )
    .await;
    // Le calme compte depuis le dernier changement (la suppression, vue par la carte).
    clock.advance(Duration::from_secs(181));
    let withdrawn = update_of(&mut socket, &project_id, SOON).await;
    let names: Vec<String> = fake
        .requests()
        .iter()
        .map(|r| r.schema_name.clone())
        .collect();
    let withdrawn = withdrawn.unwrap_or_else(|| panic!("levée : {names:?}"));
    assert_eq!(withdrawn["status"], "resolved");
}
