#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Mentors de projet (phase 9c), avec `FakeLlmClient` et un faux catalogue local :
//! un mentor `[section, project]` dans ses deux portées, estimation, revue demandée,
//! remarques sur plusieurs fichiers, reprises dans la vue Projet, budget, panne et
//! revue caduque. Aucun appel réseau.

mod common;

use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use axum::Json;
use common::{Socket, fake_llm, next, next_within};
use hodegos_llm::{BudgetConfig, FakeLlmClient, LlmError, LlmResponse};
use hodegos_server::Llm;
use serde_json::{Value, json};

const ANNULER: &str = "\
import { lister } from \"../stockage/depot\";
import { readFileSync } from \"fs\";

export function annuler(id: string): void {
  const brut = readFileSync(\"data/emprunts.json\", \"utf8\");
  console.log(brut, id);
}
";

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(root, ".gitignore", "cache/\n");
    write(root, "cache/genere.ts", "export function genere() {}\n");
    write(root, "src/.env", "API_KEY=abcdef1234567\n");
    write(
        root,
        "docs/architecture/0001-stockage.md",
        "# Stockage\n\nStatut : accepté\n\n## Décision\n\nUn seul module, src/stockage/depot.ts, lit et écrit data/emprunts.json.\n",
    );
    write(root, "src/emprunts/annuler.ts", ANNULER);
    write(
        root,
        "src/notifications/email.ts",
        "import { writeFileSync } from \"fs\";\n\nexport function prevenir(): void {\n  writeFileSync(\"data/emprunts.json\", \"[]\");\n}\n",
    );
    write(
        root,
        "src/stockage/depot.ts",
        "export function lister(): string[] {\n  return [];\n}\n",
    );
    write(
        root,
        ".hodegos/mentors/archi.md",
        "---\nid: archi\nname: Archi\ninitials: AR\nrole: Architecte de test\nmode: proactive\nmodel: strong\npriority: 100\nscopes: [section, project]\nproject:\n  key_files: [\"docs/architecture/**\", \"src/**\"]\n  max_remarks: 2\ntriggers:\n  languages: [typescript]\n---\nRelisez à l'échelle {scope}.\n",
    );
    dir
}

/// Faux catalogue public : seul `GET /models` est servi.
async fn catalog() -> String {
    let models = json!({ "data": [
        { "id": "test/fort", "supported_parameters": ["structured_outputs"],
          "pricing": { "prompt": "0.000002", "completion": "0.000008" } }
    ] });
    let app = axum::Router::new().route(
        "/api/v1/models",
        axum::routing::get(move || {
            let models = models.clone();
            async move { Json(models) }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/api/v1", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    url
}

/// Serveur sans revues automatiques (phase 9d) : ces tests portent sur la revue demandée.
async fn open_socket_with(dir: &Path, llm: Llm) -> Socket {
    let options = hodegos_server::ServerOptions {
        review: hodegos_server::ReviewOptions {
            enabled: false,
            ..Default::default()
        },
        ..common::options(dir, 0, llm)
    };
    let server = hodegos_server::Server::bind(common::project_in(dir), options)
        .await
        .unwrap();
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

async fn llm(fake: Arc<FakeLlmClient>) -> Llm {
    let mut llm = fake_llm(fake);
    llm.config.base_url = catalog().await;
    llm
}

fn response(content: Value) -> Result<LlmResponse, LlmError> {
    Ok(LlmResponse {
        content: content.to_string(),
        cost_usd: 0.002,
        ..Default::default()
    })
}

fn review_answer() -> Result<LlmResponse, LlmError> {
    response(json!({ "remarks": [
        { "about": "project", "quotes": [
            { "path": "src/emprunts/annuler.ts", "quote": "readFileSync(\"data/emprunts.json\"" },
            { "path": "src/notifications/email.ts", "quote": "writeFileSync(\"data/emprunts.json\"" },
            { "path": "src/emprunts/annuler.ts", "quote": "console.log" },
            { "path": "cache/genere.ts", "quote": "export function genere" }
          ],
          "level": "indice", "kind": "question",
          "message": "Comptez les fichiers qui écrivent dans ce fichier de données. Combien en prévoit la décision ?",
          "confidence": 0.8 },
        { "about": "passage", "quotes": [
            { "path": "src/stockage/depot.ts", "quote": "introuvable ici" } ],
          "level": "indice", "kind": "observation", "message": "Regardez.", "confidence": 0.9 },
        { "about": "project", "quotes": [], "level": "indice", "kind": "next_step",
          "message": "Relisez la décision sur le stockage avant d'ajouter un module.", "confidence": 0.6 }
    ] }))
}

fn reply(level: &str, message: &str) -> Result<LlmResponse, LlmError> {
    response(json!({ "replies": [{ "level": level, "message": message, "confidence": 0.8 }] }))
}

/// Réponse à la demande `id`, et les messages spontanés reçus avant elle.
async fn request(socket: &mut Socket, id: &str, kind: &str, payload: Value) -> (Value, Vec<Value>) {
    common::send_raw(
        socket,
        &json!({ "v": 1, "type": kind, "id": id, "payload": payload }).to_string(),
    )
    .await;
    let mut seen = Vec::new();
    loop {
        let message = next_within(socket, Duration::from_secs(10))
            .await
            .expect("aucune réponse");
        if message["re"] == id {
            return (message, seen);
        }
        seen.push(message);
    }
}

/// Messages poussés jusqu'à `review_done` compris (ceux déjà reçus d'abord).
async fn until_done(socket: &mut Socket, mut seen: Vec<Value>) -> Vec<Value> {
    while !seen.iter().any(|m| m["type"] == "review_done") {
        seen.push(
            next_within(socket, Duration::from_secs(10))
                .await
                .expect("review_done"),
        );
    }
    seen
}

fn of<'a>(messages: &'a [Value], kind: &str) -> &'a Value {
    messages
        .iter()
        .find(|m| m["type"] == kind)
        .unwrap_or_else(|| panic!("{kind} absent : {messages:?}"))
}

#[tokio::test]
async fn un_mentor_des_deux_portees_relit_sa_section_puis_le_projet() {
    let dir = project();
    let root = dir.path();
    let fake = Arc::new(FakeLlmClient::new([response(json!({ "remarks": [
        { "quote": "readFileSync(", "level": "indice", "kind": "question",
          "message": "D'où vient ce fichier ?", "confidence": 0.9 }
    ] }))]));
    let mut socket = open_socket_with(root, llm(fake.clone()).await).await;

    // Portée section : casting et passe, comme avant.
    let (file, _) = request(
        &mut socket,
        "c-1",
        "open_file",
        json!({ "path": "src/emprunts/annuler.ts" }),
    )
    .await;
    let casting = next(&mut socket).await;
    assert_eq!(casting["payload"]["titular"]["id"], "archi");
    let (saved, mut seen) = request(
        &mut socket,
        "c-2",
        "save",
        json!({ "path": "src/emprunts/annuler.ts", "content": ANNULER.replace("brut, id", "id, brut"),
            "base_version": file["payload"]["version"] }),
    )
    .await;
    assert_eq!(saved["type"], "saved");
    while !seen.iter().any(|m| m["type"] == "remarks") {
        seen.push(next(&mut socket).await);
    }
    assert_eq!(
        of(&seen, "remarks")["payload"]["items"][0]["mentor_id"],
        "archi"
    );
    assert!(
        fake.requests()[0].messages[0]
            .content
            .ends_with("à l'échelle section.")
    );

    // Estimation avant toute revue : prix du catalogue.
    let (estimate, _) = request(
        &mut socket,
        "c-3",
        "review_estimate",
        json!({ "mentor_ids": ["archi"] }),
    )
    .await;
    assert_eq!(estimate["type"], "review_estimate", "{estimate}");
    let first = &estimate["payload"]["mentors"][0];
    assert_eq!(first["basis"], "price");
    let tokens = first["tokens"].as_f64().unwrap();
    let cost = first["cost"].as_f64().unwrap();
    assert!((cost - (tokens * 0.000002 + 800.0 * 0.000008)).abs() < 1e-9);
    assert!(first["detail"].as_str().unwrap().contains("première revue"));
    assert_eq!(estimate["payload"]["total_cost"], first["cost"]);
    assert!(estimate["payload"]["budget_left"].as_f64().unwrap() > 0.0);

    // Revue demandée : carte, fichiers clés, aucun diff, remarques sur plusieurs fichiers.
    fake.push(review_answer());
    let (started, seen) = request(
        &mut socket,
        "c-4",
        "review_request",
        json!({ "mentor_ids": ["archi"] }),
    )
    .await;
    assert_eq!(started["type"], "review_started", "{started}");
    assert_eq!(started["payload"]["trigger"], "request");
    let review_id = started["payload"]["review_id"].as_str().unwrap().to_owned();
    assert!(review_id.starts_with("rv_"));
    let pushed = until_done(&mut socket, seen).await;
    let done = of(&pushed, "review_done");
    assert_eq!(done["payload"]["outcomes"], json!({ "archi": "ok" }));

    let sent = fake.requests().last().unwrap().clone();
    let (system, user) = (&sent.messages[0].content, &sent.messages[1].content);
    assert!(system.contains("Revue du projet"));
    assert!(system.contains("Au plus 2 remarques."));
    assert!(system.ends_with("à l'échelle projet."));
    assert!(user.starts_with("<carte_du_projet>\n# "), "{user}");
    assert!(user.contains("<fichier chemin=\"src/emprunts/annuler.ts\" forme=\"entier\">"));
    assert!(user.contains("première revue : aucun diff"));
    for absent in ["cache/", "genere", "abcdef1234567", ".env"] {
        assert!(!user.contains(absent), "{absent} envoyé :\n{user}");
    }
    assert!(sent.schema["properties"]["remarks"]["items"]["properties"]["about"].is_object());

    let items = of(&pushed, "project_remarks")["payload"]["items"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(
        items.len(),
        2,
        "passage sans sa citation écarté : {items:?}"
    );
    let remark = &items[0];
    assert!(remark["id"].as_str().unwrap().starts_with("rp_"));
    assert_eq!(remark["scope"], "project");
    assert_eq!(remark["about"], "project");
    assert_eq!(remark["review_id"], review_id.as_str());
    let anchors: Vec<(&str, &str, u64)> = remark["anchors"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| {
            (
                a["path"].as_str().unwrap(),
                a["section_id"].as_str().unwrap(),
                a["line_hint"].as_u64().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        anchors,
        vec![
            ("src/emprunts/annuler.ts", "fn:annuler", 5),
            ("src/notifications/email.ts", "fn:prevenir", 4),
        ]
    );
    assert_eq!(items[1]["kind"], "next_step");
    assert!(items[1]["anchors"].as_array().unwrap().is_empty());
    let blocks: Vec<&str> = remark["context"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["block"].as_str().unwrap())
        .collect();
    assert_eq!(blocks[..2], ["map", "key_files"]);
    assert!(blocks.ends_with(&["diff", "open_remarks"]));

    // Deuxième revue : diff depuis la première, remarques ouvertes rappelées ;
    // l'estimation part de la revue précédente.
    write(
        root,
        "src/notifications/email.ts",
        "import { writeFileSync } from \"fs\";\n\nexport function prevenir(): void {\n  writeFileSync(\"data/emprunts.json\", \"[1]\");\n}\n",
    );
    let (estimate, _) = request(
        &mut socket,
        "c-5",
        "review_estimate",
        json!({ "mentor_ids": ["archi"] }),
    )
    .await;
    assert_eq!(estimate["payload"]["mentors"][0]["basis"], "last");
    fake.push(response(json!({ "remarks": [] })));
    let (_, seen) = request(
        &mut socket,
        "c-6",
        "review_request",
        json!({ "mentor_ids": ["archi"] }),
    )
    .await;
    until_done(&mut socket, seen).await;
    let user = fake.requests().last().unwrap().messages[1].content.clone();
    assert!(
        user.contains(
            "<diff_depuis_derniere_revue>\n<fichier chemin=\"src/notifications/email.ts\">"
        ),
        "{user}"
    );
    assert!(user.contains("+  writeFileSync(\"data/emprunts.json\", \"[1]\");"));
    assert!(user.contains("<message>Comptez les fichiers qui écrivent"));

    // Historique et mentors de projet.
    let (reviews, _) = request(&mut socket, "c-7", "reviews_get", Value::Null).await;
    assert_eq!(reviews["payload"]["items"].as_array().unwrap().len(), 2);
    assert_eq!(reviews["payload"]["items"][1]["mentors"][0]["remarks"], 2);
    let mentor = &reviews["payload"]["mentors"][0];
    assert_eq!(mentor["id"], "archi");
    assert!(mentor["last_review_at"].is_string());
    assert_eq!(mentor["reviewing"], false);
}

#[tokio::test]
async fn paliers_reponse_et_fermeture_dans_la_vue_projet() {
    let dir = project();
    let fake = Arc::new(FakeLlmClient::new([review_answer()]));
    let mut socket = open_socket_with(dir.path(), llm(fake.clone()).await).await;
    let (_, seen) = request(
        &mut socket,
        "c-1",
        "review_request",
        json!({ "mentor_ids": ["archi"] }),
    )
    .await;
    let pushed = until_done(&mut socket, seen).await;
    let items = of(&pushed, "project_remarks")["payload"]["items"].clone();
    let id = items[0]["id"].as_str().unwrap().to_owned();
    let next_step = items[1]["id"].as_str().unwrap().to_owned();

    // Palier suivant : carte, fichiers cités, fil.
    fake.push(reply(
        "question",
        "Combien de fichiers ouvrent data/emprunts.json ?",
    ));
    let (update, _) = request(&mut socket, "c-2", "hint_next", json!({ "remark_id": id })).await;
    assert_eq!(update["type"], "remark_update", "{update}");
    assert_eq!(update["payload"]["scope"], "project");
    assert_eq!(update["payload"]["level"], "question");
    assert_eq!(update["payload"]["thread"].as_array().unwrap().len(), 2);
    let sent = fake.requests().last().unwrap().clone();
    assert!(sent.messages[0].content.contains("Remarque sur le projet"));
    assert!(sent.messages[1].content.contains("<fichiers_cites>\n<fichier chemin=\"src/emprunts/annuler.ts\" forme=\"section fn:annuler\">"));
    assert!(sent.messages[1].content.contains("<fil>"));

    // Réponse : notée tout de suite, puis la relance du mentor arrive.
    fake.push(reply("question", "Et la décision, qu'en dit-elle ?"));
    let (answered, _) = request(
        &mut socket,
        "c-3",
        "remark_reply",
        json!({ "remark_id": id, "text": "Deux fichiers." }),
    )
    .await;
    assert_eq!(answered["payload"]["status"], "answered");
    assert_eq!(answered["payload"]["thread"][2]["author"], "user");
    let relaunched = loop {
        let message = next_within(&mut socket, Duration::from_secs(5))
            .await
            .expect("relance");
        if message["type"] == "remark_update" {
            break message;
        }
    };
    assert_eq!(relaunched["payload"]["thread"].as_array().unwrap().len(), 4);

    // Sans citation, pas de palier exemple : la piste est le dernier.
    for (index, level) in ["question", "piste"].iter().enumerate() {
        fake.push(reply(level, "Regardez la décision."));
        let (update, _) = request(
            &mut socket,
            &format!("p-{index}"),
            "hint_next",
            json!({ "remark_id": next_step }),
        )
        .await;
        assert_eq!(update["payload"]["level"], *level, "{update}");
    }
    let (refused, _) = request(
        &mut socket,
        "c-4",
        "hint_next",
        json!({ "remark_id": next_step }),
    )
    .await;
    assert_eq!(refused["type"], "error");
    assert!(
        refused["payload"]["message"]
            .as_str()
            .unwrap()
            .contains("piste")
    );

    // Fermeture : la remarque quitte la vue Projet, et tout survit au redémarrage.
    let (closed, _) = request(
        &mut socket,
        "c-5",
        "remark_dismiss",
        json!({ "remark_id": next_step, "reason": "resolved" }),
    )
    .await;
    assert_eq!(closed["payload"]["status"], "resolved");
    let mut again =
        open_socket_with(dir.path(), llm(Arc::new(FakeLlmClient::default())).await).await;
    let (remarks, _) = request(&mut again, "c-6", "project_remarks_get", Value::Null).await;
    let items = remarks["payload"]["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["id"], id.as_str());
    assert_eq!(items[0]["thread"].as_array().unwrap().len(), 4);
    let (reviews, _) = request(&mut again, "c-7", "reviews_get", Value::Null).await;
    assert_eq!(reviews["payload"]["items"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn budget_panne_et_revue_caduque() {
    let dir = project();
    let fake = Arc::new(FakeLlmClient::new([Err(LlmError::Timeout)]));
    let mut budgeted = llm(fake.clone()).await;
    budgeted.config.budget = BudgetConfig {
        monthly_cap_usd: 0.0,
        warn_at: 0.8,
    };
    let mut socket = open_socket_with(dir.path(), budgeted).await;

    // Budget atteint : refus sans confirmation, revue avec.
    let (refused, _) = request(
        &mut socket,
        "c-1",
        "review_request",
        json!({ "mentor_ids": ["archi"] }),
    )
    .await;
    assert_eq!(refused["payload"]["code"], "budget_exceeded");
    let (unknown, _) = request(
        &mut socket,
        "c-2",
        "review_request",
        json!({ "mentor_ids": ["inconnu"] }),
    )
    .await;
    assert_eq!(unknown["payload"]["code"], "not_found");

    // Panne : issue signalée, l'instantané ne bouge pas (toujours la première revue).
    let (_, seen) = request(
        &mut socket,
        "c-3",
        "review_request",
        json!({ "mentor_ids": ["archi"], "confirm_over_budget": true }),
    )
    .await;
    let pushed = until_done(&mut socket, seen).await;
    assert_eq!(
        of(&pushed, "review_done")["payload"]["outcomes"],
        json!({ "archi": "provider_error" })
    );
    let (reviews, _) = request(&mut socket, "c-4", "reviews_get", Value::Null).await;
    assert!(reviews["payload"]["mentors"][0]["last_review_at"].is_null());

    // Revue caduque : la seconde demande, partie pendant la première, l'emporte.
    let slow = Arc::new(
        FakeLlmClient::new([review_answer(), response(json!({ "remarks": [] }))])
            .with_delay(Duration::from_millis(400)),
    );
    let other = project();
    let mut socket = open_socket_with(other.path(), llm(slow.clone()).await).await;
    let (first, _) = request(
        &mut socket,
        "c-5",
        "review_request",
        json!({ "mentor_ids": ["archi"] }),
    )
    .await;
    let (second, _) = request(
        &mut socket,
        "c-6",
        "review_request",
        json!({ "mentor_ids": ["archi"] }),
    )
    .await;
    let mut done = Vec::new();
    while done.len() < 2 {
        let message = next_within(&mut socket, Duration::from_secs(5))
            .await
            .expect("review_done");
        if message["type"] == "review_done" {
            done.push(message);
        }
    }
    let outcome = |started: &Value| {
        done.iter()
            .find(|d| d["payload"]["review_id"] == started["payload"]["review_id"])
            .unwrap()["payload"]["outcomes"]["archi"]
            .clone()
    };
    assert_eq!(outcome(&first), "stale");
    assert_eq!(outcome(&second), "ok");
    let (remarks, _) = request(&mut socket, "c-7", "project_remarks_get", Value::Null).await;
    assert!(
        remarks["payload"]["items"].as_array().unwrap().is_empty(),
        "résultat caduc ignoré"
    );
}
