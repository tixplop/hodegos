#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Phase 8a : `git_status` à la connexion, après un enregistrement, et après un commit
//! ou un checkout faits hors de Hodegos.

mod common;

use std::fs;
use std::path::Path;
use std::time::Duration;

use common::{Socket, exchange, git, git_init, next, next_within, start_in};
use serde_json::{Value, json};

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

/// Connexion ; renvoie la charge du premier `git_status`.
async fn connect(dir: &Path) -> (Socket, Value) {
    let (port, cookie) = start_in(dir).await;
    let origin = format!("http://127.0.0.1:{port}");
    let mut socket = common::connect(port, Some(&origin), Some(&cookie))
        .await
        .unwrap();
    assert_eq!(next(&mut socket).await["type"], "status");
    let git = next(&mut socket).await;
    assert_eq!(git["type"], "git_status", "{git}");
    assert_eq!(next(&mut socket).await["type"], "project_status");
    (socket, git["payload"].clone())
}

/// Attend un `git_status` qui vérifie `wanted`.
async fn wait_status(socket: &mut Socket, wanted: impl Fn(&Value) -> bool) -> Value {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let mut seen = Vec::new();
    while let Some(left) = deadline.checked_duration_since(tokio::time::Instant::now()) {
        let Some(message) = next_within(socket, left).await else {
            break;
        };
        if message["type"] == "git_status" {
            if wanted(&message["payload"]) {
                return message["payload"].clone();
            }
            seen.push(message["payload"].clone());
        }
    }
    panic!("git_status attendu non reçu ; reçus : {seen:?}");
}

fn entries(payload: &Value) -> Vec<(String, String)> {
    payload["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| {
            (
                e["path"].as_str().unwrap().to_owned(),
                e["status"].as_str().unwrap().to_owned(),
            )
        })
        .collect()
}

fn committed_repo() -> Option<tempfile::TempDir> {
    let dir = tempfile::tempdir().unwrap();
    if !git_init(dir.path()) {
        return None;
    }
    write(dir.path(), "src/a.rs", "fn a() {}\n");
    write(dir.path(), "README.md", "# Troc\n");
    assert!(git(dir.path(), &["add", "."]));
    assert!(git(dir.path(), &["commit", "-q", "-m", "init"]));
    Some(dir)
}

#[tokio::test]
async fn sans_depot_git_status_est_nul() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "a.rs", "fn a() {}\n");
    let (_socket, payload) = connect(dir.path()).await;
    assert_eq!(payload, Value::Null);
}

#[tokio::test]
async fn statut_a_jour_apres_un_enregistrement_un_commit_et_un_checkout() {
    let Some(dir) = committed_repo() else {
        return;
    };
    write(dir.path(), "notes.md", "x\n");
    let (mut socket, first) = connect(dir.path()).await;
    assert_eq!(first["branch"], "main");
    assert_eq!(first["detached"], false);
    assert_eq!(entries(&first), [("notes.md".to_owned(), "U".to_owned())]);
    assert_eq!(first["changes"], 1);

    let opened = exchange(
        &mut socket,
        json!({ "v": 1, "type": "open_file", "id": "c-1", "payload": { "path": "src/a.rs" } }),
    )
    .await;
    assert_eq!(opened["type"], "file", "{opened}");
    let version = opened["payload"]["version"].clone();
    exchange(
        &mut socket,
        json!({ "v": 1, "type": "save", "id": "c-2",
            "payload": { "path": "src/a.rs", "content": "fn a() { }\n", "base_version": version } }),
    )
    .await;
    let saved = wait_status(&mut socket, |p| {
        entries(p).contains(&("src/a.rs".to_owned(), "M".to_owned()))
    })
    .await;
    assert_eq!(saved["changes"], 2);

    // Commit fait hors de Hodegos : seul le dossier .git change.
    assert!(git(dir.path(), &["commit", "-q", "-am", "deux"]));
    let committed = wait_status(&mut socket, |p| p["changes"] == 1).await;
    assert_eq!(
        entries(&committed),
        [("notes.md".to_owned(), "U".to_owned())]
    );

    assert!(git(dir.path(), &["checkout", "-q", "-b", "essai"]));
    wait_status(&mut socket, |p| p["branch"] == "essai").await;

    assert!(git(dir.path(), &["checkout", "-q", "--detach"]));
    let detached = wait_status(&mut socket, |p| p["detached"] == true).await;
    assert_eq!(detached["branch"], Value::Null);
}

#[tokio::test]
async fn creer_et_supprimer_rafraichissent_le_statut() {
    let Some(dir) = committed_repo() else {
        return;
    };
    let (mut socket, first) = connect(dir.path()).await;
    assert_eq!(first["changes"], 0);
    exchange(
        &mut socket,
        json!({ "v": 1, "type": "create_entry", "id": "c-1",
            "payload": { "path": "src/b.rs", "kind": "file" } }),
    )
    .await;
    wait_status(&mut socket, |p| {
        entries(p) == [("src/b.rs".to_owned(), "U".to_owned())]
    })
    .await;
    exchange(
        &mut socket,
        json!({ "v": 1, "type": "delete_entry", "id": "c-2", "payload": { "path": "README.md" } }),
    )
    .await;
    let deleted = wait_status(&mut socket, |p| p["changes"] == 2).await;
    assert_eq!(
        entries(&deleted),
        [("src/b.rs".to_owned(), "U".to_owned())],
        "un fichier suivi supprimé compte sans être dans l'arbre"
    );
}

#[tokio::test]
async fn projet_dans_un_sous_dossier_du_depot() {
    let Some(dir) = committed_repo() else {
        return;
    };
    write(dir.path(), "src/nouveau.rs", "fn n() {}\n");
    write(dir.path(), "hors.md", "x\n");
    let (_socket, payload) = connect(&dir.path().join("src")).await;
    assert_eq!(
        entries(&payload),
        [("nouveau.rs".to_owned(), "U".to_owned())]
    );
    assert_eq!(payload["changes"], 1);
}
