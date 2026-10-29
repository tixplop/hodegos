#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::fs;
use std::path::Path;
use std::time::Duration;

use common::{Socket, exchange, next, next_within, open_socket};
use serde_json::{Value, json};

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

/// Projet d'exemple : un fichier par langage, plus ce qui doit rester caché.
fn sample() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(root, ".git/config", "[core]");
    write(root, ".gitignore", "target/\n.env\n");
    write(root, ".env", "OPENROUTER_API_KEY=secret");
    write(root, "target/debug/app", "bin");
    write(root, "src/main.rs", "fn main() {}\n");
    write(root, "src/emprunts/reserver.ts", "export const a = 1;\n");
    write(root, "ui/app.js", "console.log(1);\n");
    write(root, "outils/script.py", "print(1)\n");
    write(root, "docs/adr/0001.md", "# Stockage\n");
    write(root, "Cargo.toml", "[package]\n");
    fs::write(root.join("logo.png"), [0x89, b'P', b'N', b'G', 0xff]).unwrap();
    dir
}

/// Envoie une demande et renvoie sa réponse ; les `casting` spontanés sont ignorés.
async fn request(socket: &mut Socket, id: &str, kind: &str, payload: Value) -> Value {
    let mut reply = exchange(
        socket,
        json!({ "v": 1, "type": kind, "id": id, "payload": payload }),
    )
    .await;
    while BACKGROUND.contains(&reply["type"].as_str().unwrap_or_default()) && reply["re"] != id {
        reply = next(socket).await;
    }
    assert_eq!(reply["re"], id, "{reply}");
    reply
}

/// Messages spontanés étrangers aux fichiers : interlocuteurs, état, passes.
const BACKGROUND: [&str; 4] = ["casting", "status", "pass_started", "remarks"];

fn background(message: &Value) -> bool {
    BACKGROUND.contains(&message["type"].as_str().unwrap_or_default())
}

/// Prochain message autre qu'un message d'arrière-plan.
async fn next_event(socket: &mut Socket) -> Value {
    loop {
        let message = next(socket).await;
        if !background(&message) {
            return message;
        }
    }
}

/// Messages autres que d'arrière-plan reçus pendant `wait`.
async fn events_within(socket: &mut Socket, wait: Duration) -> Vec<Value> {
    let deadline = tokio::time::Instant::now() + wait;
    let mut events = Vec::new();
    while let Some(left) = deadline.checked_duration_since(tokio::time::Instant::now()) {
        match next_within(socket, left).await {
            Some(message) if !background(&message) => events.push(message),
            Some(_) => {}
            None => break,
        }
    }
    events
}

/// `open_file` ; un fichier ouvert est suivi de son `casting`, consommé ici.
async fn open(socket: &mut Socket, path: &str) -> Value {
    let reply = request(socket, "open", "open_file", json!({ "path": path })).await;
    if reply["type"] == "file" {
        let casting = next(socket).await;
        assert_eq!(casting["type"], "casting", "{casting}");
    }
    reply
}

fn names(reply: &Value) -> Vec<String> {
    reply["payload"]["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| {
            format!(
                "{}:{}",
                e["kind"].as_str().unwrap(),
                e["name"].as_str().unwrap()
            )
        })
        .collect()
}

#[tokio::test]
async fn l_arborescence_n_affiche_ni_git_ni_les_fichiers_ignores() {
    let dir = sample();
    let mut socket = open_socket(dir.path()).await;

    let root = request(&mut socket, "c-1", "list_dir", json!({ "path": "" })).await;
    assert_eq!(root["type"], "dir");
    assert_eq!(root["payload"]["path"], "");
    assert_eq!(
        names(&root),
        [
            "dir:docs",
            "dir:outils",
            "dir:src",
            "dir:ui",
            "file:.gitignore",
            "file:Cargo.toml",
            "file:logo.png"
        ]
    );
    assert_eq!(root["payload"]["entries"][0]["mentors"], json!([]));

    let src = request(&mut socket, "c-2", "list_dir", json!({ "path": "src" })).await;
    assert_eq!(names(&src), ["dir:emprunts", "file:main.rs"]);
}

#[tokio::test]
async fn ouvre_un_fichier_de_chaque_langage() {
    let dir = sample();
    let mut socket = open_socket(dir.path()).await;
    for (path, language, content) in [
        ("src/main.rs", "rust", "fn main() {}\n"),
        (
            "src/emprunts/reserver.ts",
            "typescript",
            "export const a = 1;\n",
        ),
        ("ui/app.js", "javascript", "console.log(1);\n"),
        ("outils/script.py", "python", "print(1)\n"),
        ("docs/adr/0001.md", "markdown", "# Stockage\n"),
        ("Cargo.toml", "text", "[package]\n"),
    ] {
        let reply = open(&mut socket, path).await;
        assert_eq!(reply["type"], "file", "{reply}");
        let payload = &reply["payload"];
        assert_eq!(payload["path"], path);
        assert_eq!(payload["language"], language, "{path}");
        assert_eq!(payload["content"], content);
        assert_eq!(payload["version"].as_str().unwrap().len(), 16);
        assert!(
            !payload["sections"].as_array().unwrap().is_empty(),
            "{path}"
        );
        assert_eq!(payload["remarks"], json!([]));
    }
}

#[tokio::test]
async fn enregistre_et_renvoie_la_nouvelle_version() {
    let dir = sample();
    let mut socket = open_socket(dir.path()).await;
    let file = open(&mut socket, "src/main.rs").await;
    let base = file["payload"]["version"].clone();

    let saved = request(
        &mut socket,
        "c-3",
        "save",
        json!({ "path": "src/main.rs", "content": "fn main() { }\n", "base_version": base }),
    )
    .await;
    assert_eq!(saved["type"], "saved", "{saved}");
    assert_eq!(saved["payload"]["path"], "src/main.rs");
    assert_ne!(saved["payload"]["version"], base);
    assert_eq!(
        fs::read_to_string(dir.path().join("src/main.rs")).unwrap(),
        "fn main() { }\n"
    );

    let reopened = open(&mut socket, "src/main.rs").await;
    assert_eq!(reopened["payload"]["version"], saved["payload"]["version"]);
}

#[tokio::test]
async fn un_enregistrement_par_hodegos_ne_produit_pas_file_changed() {
    let dir = sample();
    let mut socket = open_socket(dir.path()).await;
    let file = open(&mut socket, "src/main.rs").await;
    request(
        &mut socket,
        "c-4",
        "save",
        json!({ "path": "src/main.rs", "content": "fn a() {}\n",
                "base_version": file["payload"]["version"] }),
    )
    .await;
    let events = events_within(&mut socket, Duration::from_millis(1500)).await;
    assert!(
        events.is_empty(),
        "message inattendu après l'enregistrement : {events:?}"
    );
}

#[tokio::test]
async fn une_modification_externe_produit_file_changed() {
    let dir = sample();
    let mut socket = open_socket(dir.path()).await;
    open(&mut socket, "src/main.rs").await;

    write(dir.path(), "src/main.rs", "fn b() {}\n");
    let changed = next_event(&mut socket).await;
    assert_eq!(changed["type"], "file_changed", "{changed}");
    assert!(changed.get("re").is_none());
    assert_eq!(changed["payload"]["path"], "src/main.rs");

    let reopened = open(&mut socket, "src/main.rs").await;
    assert_eq!(
        reopened["payload"]["version"],
        changed["payload"]["version"]
    );
    assert_eq!(reopened["payload"]["content"], "fn b() {}\n");
}

#[tokio::test]
async fn une_suppression_externe_produit_file_changed_sans_version() {
    let dir = sample();
    let mut socket = open_socket(dir.path()).await;
    open(&mut socket, "src/main.rs").await;

    fs::remove_file(dir.path().join("src/main.rs")).unwrap();
    let changed = next_event(&mut socket).await;
    assert_eq!(changed["type"], "file_changed", "{changed}");
    assert_eq!(changed["payload"]["version"], Value::Null);
}

#[tokio::test]
async fn un_fichier_non_ouvert_ne_produit_rien() {
    let dir = sample();
    let mut socket = open_socket(dir.path()).await;
    open(&mut socket, "src/main.rs").await;
    write(dir.path(), "src/autre.rs", "fn c() {}\n");
    let events = events_within(&mut socket, Duration::from_millis(1500)).await;
    assert!(events.is_empty(), "{events:?}");
}

#[tokio::test]
async fn conflit_de_version_puis_ecrasement() {
    let dir = sample();
    let mut socket = open_socket(dir.path()).await;
    let file = open(&mut socket, "src/main.rs").await;
    let base = file["payload"]["version"].clone();

    write(dir.path(), "src/main.rs", "fn ailleurs() {}\n");
    let changed = next_event(&mut socket).await;
    assert_eq!(changed["type"], "file_changed");

    let conflict = request(
        &mut socket,
        "c-5",
        "save",
        json!({ "path": "src/main.rs", "content": "fn moi() {}\n", "base_version": base }),
    )
    .await;
    assert_eq!(conflict["type"], "error", "{conflict}");
    assert_eq!(conflict["payload"]["code"], "version_conflict");
    assert_eq!(
        conflict["payload"]["version"],
        changed["payload"]["version"]
    );
    assert_eq!(
        fs::read_to_string(dir.path().join("src/main.rs")).unwrap(),
        "fn ailleurs() {}\n"
    );

    let saved = request(
        &mut socket,
        "c-6",
        "save",
        json!({ "path": "src/main.rs", "content": "fn moi() {}\n",
                "base_version": conflict["payload"]["version"] }),
    )
    .await;
    assert_eq!(saved["type"], "saved", "{saved}");
    assert_eq!(
        fs::read_to_string(dir.path().join("src/main.rs")).unwrap(),
        "fn moi() {}\n"
    );
}

#[tokio::test]
async fn les_chemins_hors_projet_git_ou_ignores_sont_refuses() {
    let dir = sample();
    let outside = tempfile::tempdir().unwrap();
    write(outside.path(), "secret.txt", "x");
    std::os::unix::fs::symlink(
        outside.path().join("secret.txt"),
        dir.path().join("lien.txt"),
    )
    .unwrap();
    let mut socket = open_socket(dir.path()).await;

    for path in [
        "../voisin.txt",
        "/etc/passwd",
        "lien.txt",
        ".git/config",
        ".env",
        "target/debug/app",
    ] {
        let reply = open(&mut socket, path).await;
        assert_eq!(
            reply["payload"]["code"], "path_forbidden",
            "{path} : {reply}"
        );
        let reply = request(
            &mut socket,
            "c-7",
            "save",
            json!({ "path": path, "content": "x", "base_version": "0" }),
        )
        .await;
        assert_eq!(
            reply["payload"]["code"], "path_forbidden",
            "{path} : {reply}"
        );
    }
    let reply = request(&mut socket, "c-8", "list_dir", json!({ "path": "target" })).await;
    assert_eq!(reply["payload"]["code"], "path_forbidden");
}

#[tokio::test]
async fn binaire_absent_ou_dossier() {
    let dir = sample();
    let mut socket = open_socket(dir.path()).await;
    let reply = open(&mut socket, "logo.png").await;
    assert_eq!(reply["payload"]["code"], "not_text", "{reply}");
    let reply = open(&mut socket, "absent.rs").await;
    assert_eq!(reply["payload"]["code"], "not_found", "{reply}");
    let reply = open(&mut socket, "src").await;
    assert_eq!(reply["payload"]["code"], "not_found", "{reply}");
}
