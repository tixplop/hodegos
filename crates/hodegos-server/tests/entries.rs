#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Phase 8a : créer et supprimer depuis l'arbre ; remarques, choix forcés et passe
//! d'ouverture oubliés avec un fichier supprimé depuis Hodegos (ADR 0006).

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use common::{
    Socket, data_dir, exchange, git, git_init, next, next_within, open_socket, project_in,
};
use serde_json::{Value, json};

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(root, ".gitignore", "target/\n*.log\n");
    write(root, "src/a.rs", "fn a() {}\n");
    write(root, "src/garde.rs", "fn garde() {}\n");
    write(root, "docs/x.md", "# X\n");
    dir
}

/// Messages spontanés étrangers à l'arbre.
const BACKGROUND: [&str; 6] = [
    "casting",
    "status",
    "pass_started",
    "remarks",
    "git_status",
    "map_updated",
];

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

async fn next_event(socket: &mut Socket) -> Value {
    loop {
        let message = next(socket).await;
        if !BACKGROUND.contains(&message["type"].as_str().unwrap_or_default()) {
            return message;
        }
    }
}

async fn events_within(socket: &mut Socket, wait: Duration) -> Vec<Value> {
    let deadline = tokio::time::Instant::now() + wait;
    let mut events = Vec::new();
    while let Some(left) = deadline.checked_duration_since(tokio::time::Instant::now()) {
        match next_within(socket, left).await {
            Some(m) if !BACKGROUND.contains(&m["type"].as_str().unwrap_or_default()) => {
                events.push(m)
            }
            Some(_) => {}
            None => break,
        }
    }
    events
}

fn names(dir: &Value) -> Vec<String> {
    dir["payload"]["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["name"].as_str().unwrap().to_owned())
        .collect()
}

#[tokio::test]
async fn cree_un_fichier_et_un_dossier_puis_renvoie_le_dossier_parent() {
    let dir = project();
    let mut socket = open_socket(dir.path()).await;

    let created = request(
        &mut socket,
        "c-1",
        "create_entry",
        json!({ "path": "src/annuler.rs", "kind": "file" }),
    )
    .await;
    assert_eq!(created["type"], "entry_created", "{created}");
    assert_eq!(
        created["payload"],
        json!({ "path": "src/annuler.rs", "kind": "file" })
    );
    let parent = next_event(&mut socket).await;
    assert_eq!(parent["type"], "dir", "{parent}");
    assert_eq!(parent["payload"]["path"], "src");
    assert_eq!(names(&parent), ["a.rs", "annuler.rs", "garde.rs"]);
    let entry = &parent["payload"]["entries"][1];
    assert_eq!(entry["mentors"][0]["id"], "mentor-langage", "{entry}");
    assert_eq!(fs::read(dir.path().join("src/annuler.rs")).unwrap(), b"");

    let created = request(
        &mut socket,
        "c-2",
        "create_entry",
        json!({ "path": "tests", "kind": "dir" }),
    )
    .await;
    assert_eq!(created["payload"]["kind"], "dir");
    let root = next_event(&mut socket).await;
    assert_eq!(root["payload"]["path"], "");
    assert!(names(&root).contains(&"tests".to_owned()));
    assert!(dir.path().join("tests").is_dir());
}

#[tokio::test]
async fn les_refus_de_creation_portent_un_code_et_un_message() {
    let dir = project();
    fs::create_dir_all(dir.path().join("target")).unwrap();
    let mut socket = open_socket(dir.path()).await;
    let cases = [
        ("src/a.rs", "file", "already_exists"),
        ("src", "dir", "already_exists"),
        ("", "file", "bad_message"),
        ("src/", "file", "bad_message"),
        ("src/./b.rs", "file", "bad_message"),
        ("src/..", "file", "path_forbidden"),
        (".git", "dir", "path_forbidden"),
        ("trace.log", "file", "path_forbidden"),
        ("target/x.rs", "file", "path_forbidden"),
        ("absent/x.rs", "file", "not_found"),
    ];
    for (i, (path, kind, code)) in cases.into_iter().enumerate() {
        let id = format!("c-{i}");
        let reply = request(
            &mut socket,
            &id,
            "create_entry",
            json!({ "path": path, "kind": kind }),
        )
        .await;
        assert_eq!(reply["type"], "error", "{path} : {reply}");
        assert_eq!(reply["payload"]["code"], code, "{path} : {reply}");
        assert!(!reply["payload"]["message"].as_str().unwrap().is_empty());
    }
    assert!(!dir.path().join("trace.log").exists());
    assert!(!dir.path().join("absent").exists());
}

/// Dossier des données de l'utilisateur pour ce projet.
fn user_data(dir: &Path) -> PathBuf {
    data_dir(dir).join(project_in(dir).id().as_str())
}

fn remark(id: &str, path: &str, status: &str) -> Value {
    json!({
        "id": id,
        "mentor_id": "mentor-langage",
        "path": path,
        "anchor": { "section_id": "fn:a", "quote": "fn a", "line_hint": 1 },
        "level": "indice",
        "kind": "question",
        "levels_given": ["indice"],
        "status": status,
        "thread": [{ "author": "mentor", "text": "Que fait cette fonction ?", "at": "2026-10-31T10:00:00Z" }],
        "created_at": "2026-10-31T10:00:00Z"
    })
}

/// Remarques, choix forcés et passe d'ouverture pour trois fichiers.
fn seed(dir: &Path) {
    let data = user_data(dir);
    fs::create_dir_all(&data).unwrap();
    let remarks = json!({ "version": 1, "remarks": [
        remark("rm_1", "src/a.rs", "open"),
        remark("rm_2", "src/a.rs", "resolved"),
        remark("rm_3", "src/garde.rs", "open"),
        remark("rm_4", "docs/x.md", "open"),
    ]});
    fs::write(data.join("remarks.json"), remarks.to_string()).unwrap();
    let overrides = json!([
        { "path": "docs/x.md", "section": "", "mentor": "architecte" },
        { "path": "src/a.rs", "section": "fn:a", "mentor": "architecte" },
        { "path": "src/garde.rs", "section": "", "mentor": "architecte" },
    ]);
    fs::write(data.join("overrides.json"), overrides.to_string()).unwrap();
    let opened = json!({ "version": 1, "paths": ["docs/x.md", "src/a.rs", "src/garde.rs"] });
    fs::write(data.join("opened.json"), opened.to_string()).unwrap();
}

/// Chemins encore cités par les données de l'utilisateur.
fn remembered(dir: &Path) -> (Vec<String>, Vec<String>, Vec<String>) {
    let data = user_data(dir);
    let read = |name: &str| -> Value {
        serde_json::from_str(&fs::read_to_string(data.join(name)).unwrap()).unwrap()
    };
    let paths = |items: &Value| -> Vec<String> {
        items
            .as_array()
            .unwrap()
            .iter()
            .map(|i| i["path"].as_str().unwrap().to_owned())
            .collect()
    };
    let opened = read("opened.json")["paths"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p.as_str().unwrap().to_owned())
        .collect();
    (
        paths(&read("remarks.json")["remarks"]),
        paths(&read("overrides.json")),
        opened,
    )
}

#[tokio::test]
async fn supprimer_depuis_hodegos_oublie_remarques_choix_forces_et_passe_d_ouverture() {
    let dir = project();
    seed(dir.path());
    let mut socket = open_socket(dir.path()).await;

    let opened = request(
        &mut socket,
        "c-1",
        "open_file",
        json!({ "path": "src/a.rs" }),
    )
    .await;
    assert_eq!(opened["payload"]["remarks"].as_array().unwrap().len(), 1);

    let deleted = request(
        &mut socket,
        "c-2",
        "delete_entry",
        json!({ "path": "src/a.rs" }),
    )
    .await;
    assert_eq!(deleted["type"], "entry_deleted", "{deleted}");
    assert_eq!(
        deleted["payload"],
        json!({ "path": "src/a.rs", "kind": "file" })
    );
    let parent = next_event(&mut socket).await;
    assert_eq!(parent["type"], "dir");
    assert_eq!(names(&parent), ["garde.rs"]);
    assert!(!dir.path().join("src/a.rs").exists());

    let deleted = request(
        &mut socket,
        "c-3",
        "delete_entry",
        json!({ "path": "docs" }),
    )
    .await;
    assert_eq!(deleted["payload"]["kind"], "dir");
    next_event(&mut socket).await;

    let (remarks, overrides, opened) = remembered(dir.path());
    assert_eq!(remarks, ["src/garde.rs"], "remarques fermées comprises");
    assert_eq!(overrides, ["src/garde.rs"]);
    assert_eq!(opened, ["src/garde.rs"]);

    // L'onglet supprimé ne produit pas de `file_changed` après coup.
    let events = events_within(&mut socket, Duration::from_millis(800)).await;
    assert!(events.is_empty(), "{events:?}");

    // Le fichier revient : il repart sans remarque.
    write(dir.path(), "src/a.rs", "fn a() {}\n");
    let reopened = request(
        &mut socket,
        "c-4",
        "open_file",
        json!({ "path": "src/a.rs" }),
    )
    .await;
    assert_eq!(reopened["payload"]["remarks"], json!([]));
}

#[tokio::test]
async fn une_suppression_hors_de_hodegos_garde_les_remarques() {
    let dir = project();
    seed(dir.path());
    let mut socket = open_socket(dir.path()).await;
    request(
        &mut socket,
        "c-1",
        "open_file",
        json!({ "path": "src/garde.rs" }),
    )
    .await;
    fs::remove_file(dir.path().join("src/garde.rs")).unwrap();
    let changed = next_event(&mut socket).await;
    assert_eq!(changed["type"], "file_changed", "{changed}");
    assert_eq!(changed["payload"]["version"], Value::Null);
    let (remarks, overrides, opened) = remembered(dir.path());
    assert!(remarks.contains(&"src/garde.rs".to_owned()));
    assert!(overrides.contains(&"src/garde.rs".to_owned()));
    assert!(opened.contains(&"src/garde.rs".to_owned()));
}

#[tokio::test]
async fn les_refus_de_suppression() {
    let dir = project();
    write(dir.path(), "target/app", "bin");
    let mut socket = open_socket(dir.path()).await;
    for (i, (path, code)) in [
        ("", "path_forbidden"),
        (".git", "path_forbidden"),
        ("target", "path_forbidden"),
        ("../voisin", "path_forbidden"),
        ("absent.rs", "not_found"),
    ]
    .into_iter()
    .enumerate()
    {
        for kind in ["delete_preview", "delete_entry"] {
            let id = format!("c-{kind}-{i}");
            let reply = request(&mut socket, &id, kind, json!({ "path": path })).await;
            assert_eq!(reply["payload"]["code"], code, "{kind} {path} : {reply}");
        }
    }
    assert!(dir.path().join("target/app").exists());
}

async fn preview(socket: &mut Socket, id: &str, path: &str) -> Value {
    let reply = request(socket, id, "delete_preview", json!({ "path": path })).await;
    assert_eq!(reply["type"], "delete_preview", "{reply}");
    reply["payload"].clone()
}

#[tokio::test]
async fn l_apercu_dit_ce_que_git_peut_restaurer() {
    let dir = project();
    if !git_init(dir.path()) {
        return;
    }
    write(dir.path(), "src/modifie.rs", "fn m() {}\n");
    assert!(git(dir.path(), &["add", "."]));
    assert!(git(dir.path(), &["commit", "-q", "-m", "init"]));
    write(dir.path(), "src/modifie.rs", "fn m() { }\n");
    write(dir.path(), "src/nouveau.rs", "fn n() {}\n");
    write(dir.path(), "src/trace.log", "x");
    let mut socket = open_socket(dir.path()).await;

    let clean = preview(&mut socket, "c-1", "src/a.rs").await;
    assert_eq!(
        clean,
        json!({ "path": "src/a.rs", "kind": "file", "files": 1, "untracked": 0,
                "modified": 0, "ignored": 0, "restorable": true })
    );
    let modified = preview(&mut socket, "c-2", "src/modifie.rs").await;
    assert_eq!(modified["modified"], 1);
    assert_eq!(modified["restorable"], false);
    let untracked = preview(&mut socket, "c-3", "src/nouveau.rs").await;
    assert_eq!(untracked["untracked"], 1);
    assert_eq!(untracked["restorable"], false);
    let folder = preview(&mut socket, "c-4", "src").await;
    assert_eq!(
        folder,
        json!({ "path": "src", "kind": "dir", "files": 5, "untracked": 1,
                "modified": 1, "ignored": 1, "restorable": false })
    );
    assert!(
        dir.path().join("src/nouveau.rs").exists(),
        "l'aperçu ne supprime rien"
    );
}

#[tokio::test]
async fn sans_depot_rien_n_est_restaurable() {
    let dir = project();
    let mut socket = open_socket(dir.path()).await;
    let folder = preview(&mut socket, "c-1", "src").await;
    assert_eq!(folder["files"], 2);
    assert_eq!(folder["untracked"], 2);
    assert_eq!(folder["restorable"], false);
}
