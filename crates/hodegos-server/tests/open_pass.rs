#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Passe à l'ouverture (`on_open`) et fichier ouvert à la connexion, avec
//! `FakeLlmClient` : aucun appel réseau.

mod common;

use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use common::{Socket, data_dir, exchange, fake_llm, next, next_within, project_in, session_cookie};
use hodegos_llm::{FakeLlmClient, LlmError, LlmResponse};
use hodegos_server::{Llm, Server};
use serde_json::{Value, json};

const VISION: &str = "docs/vision.md";
const SECTIONS: [&str; 6] = [
    "h:essai-vision",
    "h:idee",
    "h:pour-qui",
    "h:ce-que-je-veux-apprendre",
    "h:contraintes",
    "h:premiere-version",
];

fn template() -> String {
    include_str!("../../../templates/vision.md")
        .replace("{project_name}", "essai")
        .replace("{date}", "2026-10-30")
}

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), VISION, &template());
    write(dir.path(), "README.md", "# essai\n\nUn projet.\n");
    dir
}

fn next_step(message: &str, confidence: f64) -> Value {
    json!({ "quote": "", "level": "question", "kind": "next_step", "message": message, "confidence": confidence })
}

fn answer(remarks: Vec<Value>) -> Result<LlmResponse, LlmError> {
    Ok(LlmResponse {
        content: json!({ "remarks": remarks }).to_string(),
        cost_usd: 0.001,
    })
}

/// Une réponse par section de la vision.
fn six(make: impl Fn(usize) -> Result<LlmResponse, LlmError>) -> Arc<FakeLlmClient> {
    Arc::new(FakeLlmClient::new((0..SECTIONS.len()).map(make)))
}

async fn start(dir: &Path, llm: Llm, initial_file: Option<&str>) -> (Socket, Value) {
    let mut options = common::options(dir, 0, llm);
    options.initial_file = initial_file.map(str::to_owned);
    let server = Server::bind(project_in(dir), options).await.unwrap();
    let port = server.port();
    let cookie = session_cookie(&server).await;
    tokio::spawn(server.serve(std::future::pending()));
    let origin = format!("http://127.0.0.1:{port}");
    let mut socket = common::connect(port, Some(&origin), Some(&cookie))
        .await
        .unwrap();
    let status = next(&mut socket).await;
    assert_eq!(status["type"], "status", "{status}");
    (socket, status)
}

/// Ouvre un fichier ; renvoie `file` puis tous les messages qui suivent le `casting`,
/// jusqu'au `status` qui clôt une passe, ou rien si aucune passe ne part.
async fn open(socket: &mut Socket, path: &str) -> (Value, Vec<Value>) {
    let mut file = exchange(
        socket,
        json!({ "v": 1, "type": "open_file", "id": "o", "payload": { "path": path } }),
    )
    .await;
    while file["re"] != "o" {
        file = next(socket).await;
    }
    assert_eq!(file["type"], "file", "{file}");
    let casting = next(socket).await;
    assert_eq!(casting["type"], "casting", "{casting}");
    let mut after = Vec::new();
    while let Some(message) = next_within(socket, Duration::from_millis(800)).await {
        let done = message["type"] == "status";
        after.push(message);
        if done {
            break;
        }
    }
    (file, after)
}

fn of_type<'a>(messages: &'a [Value], kind: &str) -> Vec<&'a Value> {
    messages.iter().filter(|m| m["type"] == kind).collect()
}

#[tokio::test]
async fn premiere_ouverture_passe_sur_toutes_les_sections_et_deux_remarques_au_plus() {
    let dir = project();
    let fake = six(|n| {
        answer(vec![next_step(
            "Quel problème voulez-vous résoudre, et pour qui ?",
            0.6 + 0.05 * n as f64,
        )])
    });
    let (mut socket, _) = start(dir.path(), fake_llm(fake.clone()), None).await;
    let (file, after) = open(&mut socket, VISION).await;

    let started = of_type(&after, "pass_started");
    assert_eq!(started.len(), 1, "{after:?}");
    assert_eq!(started[0]["payload"]["sections"], json!(SECTIONS));
    assert_eq!(started[0]["payload"]["mentors"][0]["id"], "mentor-produit");

    let items = of_type(&after, "remarks")[0]["payload"]["items"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(items.len(), 2, "plafond de deux remarques par passe");
    let sections = file["payload"]["sections"].as_array().unwrap();
    for item in &items {
        assert_eq!(item["mentor_id"], "mentor-produit");
        assert_eq!(item["kind"], "next_step");
        assert_eq!(item["anchor"]["quote"], "");
        let section = sections
            .iter()
            .find(|s| s["id"] == item["anchor"]["section_id"])
            .unwrap();
        assert_eq!(
            item["anchor"]["line_hint"], section["start_line"],
            "ancrée sur le titre de la section"
        );
    }
    assert_eq!(
        after.last().unwrap()["payload"]["last_pass"]["outcome"],
        "ok"
    );

    // Six appels au modèle fort, sans les consignes en commentaires HTML.
    let requests = fake.requests();
    assert_eq!(requests.len(), 6);
    for request in &requests {
        assert_eq!(request.model, "test/fort");
        let user = &request.messages[1].content;
        assert!(!user.contains("<!--"), "{user}");
        assert!(!user.contains("La plus petite version utile"), "{user}");
    }

    // Réouverture : des remarques existent, aucune nouvelle passe.
    let (reopened, after) = open(&mut socket, VISION).await;
    assert!(after.is_empty(), "{after:?}");
    assert_eq!(reopened["payload"]["remarks"].as_array().unwrap().len(), 2);
    assert_eq!(fake.requests().len(), 6);

    // Une citation vide reste ancrée après un enregistrement : pas de détachement.
    fake.push(answer(vec![]));
    fake.push(answer(vec![]));
    let content = template().replace(
        "<!-- En deux ou trois phrases : quel problème, pour qui, et ce que le projet change. -->",
        "Prêter des objets entre voisins.",
    );
    let saved = exchange(
        &mut socket,
        json!({ "v": 1, "type": "save", "id": "s", "payload": {
            "path": VISION, "content": content, "base_version": reopened["payload"]["version"] } }),
    )
    .await;
    assert_eq!(saved["type"], "saved", "{saved}");
    let mut messages = Vec::new();
    loop {
        let message = next(&mut socket).await;
        let done = message["type"] == "status";
        messages.push(message);
        if done {
            break;
        }
    }
    assert!(
        of_type(&messages, "remark_update").is_empty(),
        "{messages:?}"
    );
}

#[tokio::test]
async fn une_passe_aboutie_sans_remarque_ne_se_refait_pas_meme_apres_redemarrage() {
    let dir = project();
    let fake = six(|_| answer(vec![]));
    let (mut socket, _) = start(dir.path(), fake_llm(fake.clone()), None).await;
    let (_, after) = open(&mut socket, VISION).await;
    assert_eq!(of_type(&after, "pass_started").len(), 1);
    assert_eq!(after.last().unwrap()["payload"]["last_pass"]["remarks"], 0);
    assert!(
        data_dir(dir.path())
            .join(project_in(dir.path()).id().as_str())
            .join("opened.json")
            .exists()
    );

    let (_, after) = open(&mut socket, VISION).await;
    assert!(after.is_empty(), "{after:?}");
    drop(socket);

    let (mut socket, _) = start(dir.path(), fake_llm(fake.clone()), None).await;
    let (_, after) = open(&mut socket, VISION).await;
    assert!(after.is_empty(), "{after:?}");
    assert_eq!(fake.requests().len(), 6);
}

#[tokio::test]
async fn apres_une_panne_la_passe_est_retentee_a_l_ouverture_suivante() {
    let dir = project();
    let fake = six(|_| Err(LlmError::Timeout));
    let (mut socket, _) = start(dir.path(), fake_llm(fake.clone()), None).await;
    let (_, after) = open(&mut socket, VISION).await;
    assert_eq!(
        after.last().unwrap()["payload"]["last_pass"]["outcome"],
        "provider_error"
    );
    assert_eq!(fake.requests().len(), 6);

    for _ in 0..6 {
        fake.push(answer(vec![]));
    }
    let (_, after) = open(&mut socket, VISION).await;
    assert_eq!(of_type(&after, "pass_started").len(), 1, "{after:?}");
    assert_eq!(
        after.last().unwrap()["payload"]["last_pass"]["outcome"],
        "ok"
    );
    assert_eq!(fake.requests().len(), 12);
}

#[tokio::test]
async fn budget_atteint_aucun_appel_a_l_ouverture() {
    let dir = project();
    let month = jiff::Timestamp::now().strftime("%Y-%m").to_string();
    write(
        dir.path(),
        ".git/hodegos-donnees/usage.json",
        &json!({ "months": { &month: 10.0 } }).to_string(),
    );
    let fake = six(|_| answer(vec![]));
    let (mut socket, _) = start(dir.path(), fake_llm(fake.clone()), None).await;
    let (_, after) = open(&mut socket, VISION).await;
    assert_eq!(after.len(), 1, "{after:?}");
    assert_eq!(
        after[0]["payload"]["last_pass"]["outcome"],
        "budget_exceeded"
    );
    assert!(fake.requests().is_empty());
}

#[tokio::test]
async fn aucune_passe_a_l_ouverture_sans_titulaire_on_open() {
    let dir = project();
    write(
        dir.path(),
        "docs/architecture/0001-stockage.md",
        "# Stockage\n\n## Contexte\n\nTexte.\n",
    );
    write(
        dir.path(),
        "src/a.ts",
        "export function a() {\n  return 1;\n}\n",
    );
    let fake = six(|_| answer(vec![]));
    let (mut socket, _) = start(dir.path(), fake_llm(fake.clone()), None).await;
    for path in [
        "README.md",
        "docs/architecture/0001-stockage.md",
        "src/a.ts",
    ] {
        let (_, after) = open(&mut socket, path).await;
        assert!(after.is_empty(), "{path} : {after:?}");
    }
    drop(socket);

    // Le même Mentor produit, surchargé dans le projet avec on_open: false.
    let builtin = include_str!("../../../mentors/mentor-produit.md");
    write(
        dir.path(),
        ".hodegos/mentors/mentor-produit.md",
        &builtin.replace("on_open: true", "on_open: false"),
    );
    let (mut socket, _) = start(dir.path(), fake_llm(fake.clone()), None).await;
    let (_, after) = open(&mut socket, VISION).await;
    assert!(after.is_empty(), "{after:?}");
    assert!(fake.requests().is_empty());
}

#[tokio::test]
async fn initial_file_dans_le_premier_status_seulement() {
    let dir = project();
    let (mut socket, status) = start(dir.path(), common::no_llm(), Some(VISION)).await;
    assert_eq!(status["payload"]["initial_file"], VISION);

    // Sans fournisseur, la passe d'ouverture s'arrête tout de suite : son `status`
    // ne porte plus le fichier initial.
    let (_, after) = open(&mut socket, VISION).await;
    assert_eq!(after.len(), 1, "{after:?}");
    assert_eq!(after[0]["payload"]["initial_file"], Value::Null);
    assert_eq!(
        after[0]["payload"]["last_pass"]["outcome"],
        "provider_error"
    );

    let (_, status) = start(dir.path(), common::no_llm(), None).await;
    assert_eq!(status["payload"]["initial_file"], Value::Null);
}
