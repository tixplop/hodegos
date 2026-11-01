#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Carte du projet (phase 9a) : telle qu'envoyée, sans fichier ignoré ni secret ;
//! réglages propres au projet appliqués aussitôt et écrits hors du dépôt ; mise à jour
//! après un enregistrement et après une modification faite ailleurs.

mod common;

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::time::Duration;

use common::{
    Socket, connect, data_dir, next_within, open_socket, project_in, send_raw, start_in, welcome,
};
use serde_json::{Value, json};

fn write(root: &Path, path: &str, content: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::create_dir(root.join(".git")).unwrap();
    write(root, ".gitignore", "cible/\n");
    write(root, "cible/genere.rs", "pub fn genere() {}\n");
    write(root, ".env", "API_KEY=abcdef1234567\n");
    write(
        root,
        "docs/vision.md",
        "# Troc\n\n## Idée\n<!-- consigne -->\nPrêter entre voisins.\npassword = \"tres-secret-1\"\n",
    );
    write(
        root,
        "src/lib.rs",
        "pub fn reserver(id: u32) -> bool {\n    id > 0\n}\n",
    );
    write(root, "src/profond/b.rs", "pub fn b() {}\n");
    write(root, "data/emprunts.json", "[]\n");
    dir
}

/// Réponse à la demande `id`, en sautant les messages spontanés.
async fn request(socket: &mut Socket, id: &str, kind: &str, payload: Value) -> Value {
    request_seeing(socket, id, kind, payload).await.0
}

/// Réponse à la demande `id`, et les messages spontanés reçus avant elle.
async fn request_seeing(
    socket: &mut Socket,
    id: &str,
    kind: &str,
    payload: Value,
) -> (Value, Vec<Value>) {
    send_raw(
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

async fn map(socket: &mut Socket, id: &str) -> Value {
    let reply = request(socket, id, "project_map_get", Value::Null).await;
    assert_eq!(reply["type"], "project_map", "{reply}");
    reply["payload"].clone()
}

/// Prochain `map_updated`, ou `None` si rien n'arrive pendant `wait`.
async fn map_updated(socket: &mut Socket, wait: Duration) -> Option<Value> {
    let deadline = std::time::Instant::now() + wait;
    loop {
        let left = deadline.saturating_duration_since(std::time::Instant::now());
        let message = next_within(socket, left).await?;
        if message["type"] == "map_updated" {
            assert!(message.get("re").is_none(), "spontané : {message}");
            return Some(message["payload"].clone());
        }
    }
}

fn text(map: &Value) -> &str {
    map["text"].as_str().unwrap()
}

fn files_of(root: &Path) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.file_name().unwrap() == ".git" {
                continue;
            }
            if path.is_dir() {
                stack.push(path);
            } else {
                out.insert(path.strip_prefix(root).unwrap().display().to_string());
            }
        }
    }
    out
}

#[tokio::test]
async fn la_carte_est_telle_qu_envoyee_sous_son_plafond_sans_ignores_ni_secrets() {
    let dir = project();
    let mut socket = open_socket(dir.path()).await;
    let map = map(&mut socket, "c-1").await;

    let ids: Vec<&str> = map["blocks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["id"].as_str().unwrap())
        .collect();
    assert_eq!(
        ids,
        [
            "header",
            "vision",
            "decisions",
            "tree",
            "readme",
            "dependencies",
            "symbols",
            "tests"
        ]
    );
    assert_eq!(map["cap"], 8000);
    assert_eq!(map["default_cap"], 8000);
    assert_eq!(map["reason"], "open");
    assert!(map["tokens"].as_u64().unwrap() <= 8000);
    assert_eq!(
        map["blocks"][7]["enabled"], false,
        "Tests désactivé par défaut"
    );

    let text = text(&map);
    assert!(text.contains("Prêter entre voisins."), "{text}");
    assert!(text.contains("[secret masqué]"));
    assert!(text.contains("pub fn reserver(id: u32) -> bool"));
    for absent in [
        "tres-secret-1",
        "consigne",
        ".env",
        "abcdef1234567",
        "cible",
        "genere",
        "id > 0",
    ] {
        assert!(!text.contains(absent), "{absent} dans la carte :\n{text}");
    }
    // Le poids annoncé est celui du texte, à 4 caractères par jeton.
    assert_eq!(
        map["tokens"].as_u64().unwrap() as usize,
        text.chars().count().div_ceil(4)
    );
}

#[tokio::test]
async fn les_reglages_changent_la_carte_aussitot_sans_rien_ecrire_dans_le_projet() {
    let dir = project();
    let before = files_of(dir.path());
    let mut socket = open_socket(dir.path()).await;
    assert!(text(&map(&mut socket, "c-1").await).contains("data/ : emprunts.json"));

    let (reply, seen) = request_seeing(
        &mut socket,
        "c-2",
        "project_map_set",
        json!({ "map_cap": 4000, "disabled_blocks": ["symbols"], "exclude": ["data/**"] }),
    )
    .await;
    assert_eq!(reply["type"], "project_map", "{reply}");
    let set = &reply["payload"];
    assert_eq!(set["cap"], 4000);
    assert_eq!(set["reason"], "settings");
    assert_eq!(set["exclude"], json!(["data/**"]));
    assert_eq!(set["blocks"][6]["enabled"], false);
    assert_eq!(set["blocks"][7]["enabled"], true);
    let text = text(set);
    assert!(!text.contains("emprunts.json"), "{text}");
    assert!(!text.contains("## Symboles"));

    // Toutes les fenêtres sont prévenues.
    let pushed = match seen.iter().find(|m| m["type"] == "map_updated") {
        Some(message) => message["payload"].clone(),
        None => map_updated(&mut socket, Duration::from_secs(2))
            .await
            .expect("map_updated"),
    };
    assert_eq!(pushed["reason"], "settings");
    assert_eq!(pushed["cap"], 4000);

    // Réglage personnel, sous le dossier de données ; rien dans le projet.
    let id = project_in(dir.path()).id().as_str().to_owned();
    let saved: Value = serde_json::from_str(
        &fs::read_to_string(data_dir(dir.path()).join(&id).join("context.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        saved,
        json!({ "map_cap": 4000, "disabled_blocks": ["symbols"], "exclude": ["data/**"] })
    );
    assert_eq!(files_of(dir.path()), before);
    assert!(
        data_dir(dir.path()).join(&id).join("map.json").exists(),
        "map.json persistée"
    );

    // Hors bornes ou motif invalide : refusé, rien ne change.
    for (payload, id) in [
        (
            json!({ "map_cap": 10, "disabled_blocks": [], "exclude": [] }),
            "c-3",
        ),
        (
            json!({ "map_cap": 4000, "disabled_blocks": [], "exclude": ["../x"] }),
            "c-4",
        ),
        (
            json!({ "map_cap": 4000, "disabled_blocks": ["header"], "exclude": [] }),
            "c-5",
        ),
    ] {
        let refused = request(&mut socket, id, "project_map_set", payload).await;
        assert_eq!(refused["type"], "error", "{refused}");
        assert_eq!(refused["payload"]["code"], "invalid_settings");
    }
    assert_eq!(map(&mut socket, "c-6").await["cap"], 4000);
}

#[tokio::test]
async fn un_enregistrement_dans_hodegos_met_la_carte_a_jour() {
    let dir = project();
    let mut socket = open_socket(dir.path()).await;
    map(&mut socket, "c-1").await;
    let opened = request(
        &mut socket,
        "c-2",
        "open_file",
        json!({ "path": "src/lib.rs" }),
    )
    .await;
    let version = opened["payload"]["version"].as_str().unwrap().to_owned();
    let saved = request(
        &mut socket,
        "c-3",
        "save",
        json!({ "path": "src/lib.rs", "content": "pub fn annuler(id: u32) {}\n",
            "base_version": version }),
    )
    .await;
    assert_eq!(saved["type"], "saved", "{saved}");

    let pushed = map_updated(&mut socket, Duration::from_secs(5))
        .await
        .expect("map_updated après l'enregistrement");
    assert_eq!(pushed["reason"], "save");
    let text = map(&mut socket, "c-4").await["text"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(text.contains("pub fn annuler(id: u32)"), "{text}");
    assert!(!text.contains("reserver"));
}

#[tokio::test]
async fn une_modification_faite_ailleurs_met_la_carte_a_jour() {
    let dir = project();
    let root = dir.path();
    let (port, cookie) = start_in(root).await;
    let origin = format!("http://127.0.0.1:{port}");
    let mut socket = connect(port, Some(&origin), Some(&cookie)).await.unwrap();
    welcome(&mut socket).await;
    map(&mut socket, "c-1").await;

    // Dans un dossier jamais ouvert ni affiché.
    write(root, "src/profond/b.rs", "pub fn b_modifiee() {}\n");
    let pushed = map_updated(&mut socket, Duration::from_secs(5))
        .await
        .expect("map_updated après une modification externe");
    assert_eq!(pushed["reason"], "external");
    assert!(text(&map(&mut socket, "c-2").await).contains("pub fn b_modifiee()"));

    // Un dossier créé ailleurs est suivi à son tour.
    write(root, "nouveau/x.rs", "pub fn x() {}\n");
    map_updated(&mut socket, Duration::from_secs(5))
        .await
        .expect("map_updated après un dossier créé");
    assert!(text(&map(&mut socket, "c-3").await).contains("nouveau/ : x.rs"));
    write(root, "nouveau/x.rs", "pub fn x_modifiee() {}\n");
    map_updated(&mut socket, Duration::from_secs(5))
        .await
        .expect("map_updated dans le nouveau dossier");
    assert!(text(&map(&mut socket, "c-4").await).contains("pub fn x_modifiee()"));

    // Un fichier ignoré par git ne recalcule rien.
    write(root, "cible/autre.rs", "pub fn autre() {}\n");
    assert_eq!(
        map_updated(&mut socket, Duration::from_millis(1500)).await,
        None
    );

    // Au redémarrage, la carte relue depuis map.json est la même.
    let first = text(&map(&mut socket, "c-5").await).to_owned();
    let mut again = open_socket(root).await;
    assert_eq!(text(&map(&mut again, "c-6").await), first);
}
