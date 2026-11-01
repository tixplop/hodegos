#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Sous-phase 8g : déplacer depuis l'arbre ; remarques, choix forcés et passe
//! d'ouverture suivent le nouveau chemin (ADR 0008).

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use common::{Socket, data_dir, exchange, next, next_within, open_socket, project_in};
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
    fs::create_dir(root.join("lib")).unwrap();
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

/// Tous les messages reçus pendant `wait`.
async fn within(socket: &mut Socket, wait: Duration) -> Vec<Value> {
    let deadline = tokio::time::Instant::now() + wait;
    let mut messages = Vec::new();
    while let Some(left) = deadline.checked_duration_since(tokio::time::Instant::now()) {
        match next_within(socket, left).await {
            Some(message) => messages.push(message),
            None => break,
        }
    }
    messages
}

fn of_type<'a>(messages: &'a [Value], kind: &str) -> Vec<&'a Value> {
    messages.iter().filter(|m| m["type"] == kind).collect()
}

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

/// Remarques, choix forcés et passe d'ouverture pour trois fichiers, plus une remarque
/// gardée pour un fichier supprimé hors de Hodegos.
fn seed(dir: &Path) {
    let data = user_data(dir);
    fs::create_dir_all(&data).unwrap();
    let remarks = json!({ "version": 1, "remarks": [
        remark("rm_1", "src/a.rs", "open"),
        remark("rm_2", "src/a.rs", "resolved"),
        remark("rm_3", "src/garde.rs", "open"),
        remark("rm_4", "docs/x.md", "open"),
        remark("rm_5", "src/parti.rs", "open"),
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

fn read_data(dir: &Path, name: &str) -> Value {
    serde_json::from_str(&fs::read_to_string(user_data(dir).join(name)).unwrap()).unwrap()
}

fn paths(items: &Value) -> Vec<String> {
    items
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["path"].as_str().unwrap().to_owned())
        .collect()
}

#[tokio::test]
async fn deplacer_un_dossier_fait_suivre_remarques_choix_forces_et_passes() {
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
    let version = opened["payload"]["version"].as_str().unwrap().to_owned();
    within(&mut socket, Duration::from_millis(300)).await;

    let moved = request(
        &mut socket,
        "c-2",
        "move_entry",
        json!({ "from": "src", "to_dir": "lib" }),
    )
    .await;
    assert_eq!(moved["type"], "entry_moved", "{moved}");
    assert_eq!(
        moved["payload"],
        json!({ "from": "src", "to": "lib/src", "kind": "dir" })
    );
    assert!(dir.path().join("lib/src/a.rs").is_file());
    assert!(!dir.path().join("src").exists());

    // Les deux parents, le casting au nouveau chemin, puis plus aucun `file_changed`
    // ni `dir` : l'observateur ne prend pas le déplacement pour une suppression.
    let messages = within(&mut socket, Duration::from_millis(1200)).await;
    let dirs: Vec<&str> = of_type(&messages, "dir")
        .iter()
        .map(|d| d["payload"]["path"].as_str().unwrap())
        .collect();
    assert_eq!(dirs, ["", "lib"], "{messages:?}");
    let castings = of_type(&messages, "casting");
    assert_eq!(castings.len(), 1, "{messages:?}");
    assert_eq!(castings[0]["payload"]["path"], "lib/src/a.rs");
    assert_eq!(castings[0]["payload"]["titular"]["id"], "architecte");
    assert!(
        of_type(&messages, "file_changed").is_empty(),
        "{messages:?}"
    );

    assert_eq!(
        paths(&read_data(dir.path(), "remarks.json")["remarks"]),
        [
            "lib/src/a.rs",
            "lib/src/a.rs",
            "lib/src/garde.rs",
            "docs/x.md",
            "src/parti.rs"
        ],
        "un fichier absent garde ses remarques sous l'ancien chemin"
    );
    assert_eq!(
        paths(&read_data(dir.path(), "overrides.json")),
        ["docs/x.md", "lib/src/a.rs", "lib/src/garde.rs"]
    );
    assert_eq!(
        read_data(dir.path(), "opened.json")["paths"],
        json!(["docs/x.md", "lib/src/a.rs", "lib/src/garde.rs"])
    );

    // L'onglet suit : enregistrer au nouveau chemin avec la version d'avant.
    let saved = request(
        &mut socket,
        "c-3",
        "save",
        json!({ "path": "lib/src/a.rs", "content": "fn a() { }\n", "base_version": version, "skip_pass": true }),
    )
    .await;
    assert_eq!(saved["type"], "saved", "{saved}");

    let reopened = request(
        &mut socket,
        "c-4",
        "open_file",
        json!({ "path": "lib/src/a.rs" }),
    )
    .await;
    let remarks = reopened["payload"]["remarks"].as_array().unwrap();
    assert_eq!(remarks.len(), 1, "{reopened}");
    assert_eq!(remarks[0]["id"], "rm_1");
    assert_eq!(remarks[0]["path"], "lib/src/a.rs");

    // Retour en glissant en sens inverse.
    let back = request(
        &mut socket,
        "c-5",
        "move_entry",
        json!({ "from": "lib/src", "to_dir": "" }),
    )
    .await;
    assert_eq!(back["payload"]["to"], "src", "{back}");
    assert_eq!(
        read_data(dir.path(), "opened.json")["paths"],
        json!(["docs/x.md", "src/a.rs", "src/garde.rs"])
    );
}

#[tokio::test]
async fn un_fichier_de_donnees_non_ecrit_annule_le_deplacement() {
    let dir = project();
    seed(dir.path());
    // `overrides.json` remplacé par un dossier : il ne peut plus être écrit.
    let overrides = user_data(dir.path()).join("overrides.json");
    fs::remove_file(&overrides).unwrap();
    fs::create_dir(&overrides).unwrap();
    let remarks_before = read_data(dir.path(), "remarks.json");
    let opened_before = read_data(dir.path(), "opened.json");

    let mut socket = open_socket(dir.path()).await;
    request(
        &mut socket,
        "c-1",
        "open_file",
        json!({ "path": "src/a.rs" }),
    )
    .await;
    let casting = request(
        &mut socket,
        "c-2",
        "cast_override",
        json!({ "path": "src/a.rs", "section_id": null, "mentor_id": "architecte" }),
    )
    .await;
    assert_eq!(casting["type"], "casting", "{casting}");

    let refused = request(
        &mut socket,
        "c-3",
        "move_entry",
        json!({ "from": "src/a.rs", "to_dir": "docs" }),
    )
    .await;
    assert_eq!(refused["type"], "error", "{refused}");
    assert_eq!(refused["payload"]["code"], "internal");
    assert!(dir.path().join("src/a.rs").is_file());
    assert!(!dir.path().join("docs/a.rs").exists());
    assert_eq!(read_data(dir.path(), "remarks.json"), remarks_before);
    assert_eq!(read_data(dir.path(), "opened.json"), opened_before);

    // Rien n'a changé : le fichier se rouvre avec sa remarque.
    let reopened = request(
        &mut socket,
        "c-4",
        "open_file",
        json!({ "path": "src/a.rs" }),
    )
    .await;
    assert_eq!(reopened["payload"]["remarks"][0]["id"], "rm_1");
}

#[tokio::test]
async fn les_refus_de_deplacement() {
    let dir = project();
    write(dir.path(), "target/app", "bin");
    write(dir.path(), "lib/a.rs", "pris");
    let mut socket = open_socket(dir.path()).await;
    for (i, (from, to_dir, code)) in [
        ("", "lib", "path_forbidden"),
        (".git", "lib", "path_forbidden"),
        ("target", "lib", "path_forbidden"),
        ("src/a.rs", "target", "path_forbidden"),
        ("../voisin", "lib", "path_forbidden"),
        ("src/a.rs", "../voisin", "path_forbidden"),
        ("absent.rs", "lib", "not_found"),
        ("src/a.rs", "docs/x.md", "not_found"),
        ("src", "src", "invalid_move"),
        ("src/a.rs", "src", "invalid_move"),
        ("src/a.rs", "lib", "already_exists"),
    ]
    .into_iter()
    .enumerate()
    {
        let id = format!("c-{i}");
        let payload = json!({ "from": from, "to_dir": to_dir });
        let reply = request(&mut socket, &id, "move_entry", payload).await;
        assert_eq!(
            reply["payload"]["code"], code,
            "{from} → {to_dir} : {reply}"
        );
    }
    assert_eq!(
        fs::read_to_string(dir.path().join("lib/a.rs")).unwrap(),
        "pris"
    );
    assert!(dir.path().join("src/a.rs").is_file());
}

#[tokio::test]
async fn list_dirs_donne_les_dossiers_du_projet() {
    let dir = project();
    write(dir.path(), "target/app", "bin");
    write(dir.path(), "docs/adr/0001.md", "# ADR");
    let mut socket = open_socket(dir.path()).await;
    let reply = request(&mut socket, "c-1", "list_dirs", Value::Null).await;
    assert_eq!(reply["type"], "dirs", "{reply}");
    assert_eq!(
        reply["payload"],
        json!({ "paths": ["docs", "docs/adr", "lib", "src"], "truncated": false })
    );
}
