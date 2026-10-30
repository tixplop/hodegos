#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::fs;
use std::path::Path;
use std::time::Duration;

use common::{Socket, exchange, next, next_within, open_socket, send_raw};
use serde_json::{Value, json};

const EVALS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../evals/cases");

fn copy(from: &str, root: &Path, to: &str) {
    let dest = root.join(to);
    fs::create_dir_all(dest.parent().unwrap()).unwrap();
    fs::copy(Path::new(EVALS).join(from), dest).unwrap();
}

/// Projet avec les deux fichiers des cas d'évaluation.
fn sample() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    copy(
        "01-dates-par-reference/src/emprunts/reserver.ts",
        dir.path(),
        "src/emprunts/reserver.ts",
    );
    copy(
        "02-adr-sans-alternatives/docs/architecture/0001-stockage.md",
        dir.path(),
        "docs/architecture/0001-stockage.md",
    );
    dir
}

async fn request(socket: &mut Socket, kind: &str, payload: Value) -> Value {
    exchange(
        socket,
        json!({ "v": 1, "type": kind, "id": "c", "payload": payload }),
    )
    .await
}

/// `open_file` puis le `casting` spontané qui suit.
async fn open(socket: &mut Socket, path: &str) -> (Value, Value) {
    let file = request(socket, "open_file", json!({ "path": path })).await;
    assert_eq!(file["type"], "file", "{file}");
    let casting = next(socket).await;
    assert_eq!(casting["type"], "casting", "{casting}");
    (file, casting)
}

async fn cursor(socket: &mut Socket, path: &str, line: usize) {
    send_raw(
        socket,
        &json!({ "v": 1, "type": "cursor", "payload": { "path": path, "line": line, "col": 1 } })
            .to_string(),
    )
    .await;
}

fn titular(casting: &Value) -> &str {
    casting["payload"]["titular"]["id"]
        .as_str()
        .unwrap_or("aucun")
}

#[tokio::test]
async fn le_fichier_porte_ses_sections() {
    let dir = sample();
    let mut socket = open_socket(dir.path()).await;
    let (file, _) = open(&mut socket, "src/emprunts/reserver.ts").await;
    let sections: Vec<(&str, u64, u64)> = file["payload"]["sections"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| {
            (
                s["id"].as_str().unwrap(),
                s["start_line"].as_u64().unwrap(),
                s["end_line"].as_u64().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        sections,
        [("top:1", 1, 1), ("fn:reserver", 3, 13), ("top:2", 15, 15)]
    );
}

#[tokio::test]
async fn deplacer_le_curseur_d_une_fonction_a_un_titre_d_adr_change_le_titulaire() {
    let dir = sample();
    let mut socket = open_socket(dir.path()).await;

    open(&mut socket, "src/emprunts/reserver.ts").await;
    cursor(&mut socket, "src/emprunts/reserver.ts", 7).await;
    let casting = next(&mut socket).await;
    let payload = &casting["payload"];
    assert_eq!(payload["section_id"], "fn:reserver");
    assert_eq!(payload["section_name"], "reserver");
    assert_eq!(titular(&casting), "mentor-langage");
    assert_eq!(payload["titular"]["initials"], "ML");
    assert_eq!(
        payload["reasons"]["mentor-langage"],
        json!(["langage typescript"])
    );
    assert_eq!(payload["available"][0]["id"], "architecte");
    assert_eq!(payload["forced"], Value::Null);

    cursor(&mut socket, "src/emprunts/reserver.ts", 9).await;
    let same = next_within(&mut socket, Duration::from_millis(800)).await;
    assert_eq!(same, None, "même section : aucun casting");

    open(&mut socket, "docs/architecture/0001-stockage.md").await;
    cursor(&mut socket, "docs/architecture/0001-stockage.md", 13).await;
    let casting = next(&mut socket).await;
    assert_eq!(casting["payload"]["section_id"], "h:decision");
    assert_eq!(titular(&casting), "architecte");
    let reasons = &casting["payload"]["reasons"]["architecte"];
    for reason in [
        "chemin docs/architecture/**",
        "langage markdown",
        "titre « Décision »",
    ] {
        assert!(
            reasons.as_array().unwrap().contains(&json!(reason)),
            "{reasons}"
        );
    }

    cursor(&mut socket, "src/emprunts/reserver.ts", 4).await;
    assert_eq!(titular(&next(&mut socket).await), "mentor-langage");
}

#[tokio::test]
async fn aucun_mentor_pour_un_fichier_sans_langage_connu() {
    let dir = sample();
    fs::write(dir.path().join("Cargo.toml"), "[package]\n").unwrap();
    let mut socket = open_socket(dir.path()).await;
    let (_, casting) = open(&mut socket, "Cargo.toml").await;
    assert_eq!(casting["payload"]["titular"], Value::Null);
    assert_eq!(casting["payload"]["consultants"], json!([]));
    assert_eq!(casting["payload"]["available"].as_array().unwrap().len(), 3);
}

#[tokio::test]
async fn choix_force_puis_retour_au_choix_automatique() {
    let dir = sample();
    let mut socket = open_socket(dir.path()).await;
    open(&mut socket, "src/emprunts/reserver.ts").await;
    cursor(&mut socket, "src/emprunts/reserver.ts", 7).await;
    next(&mut socket).await;

    let forced = request(
        &mut socket,
        "cast_override",
        json!({ "path": "src/emprunts/reserver.ts", "section_id": "fn:reserver", "mentor_id": "architecte" }),
    )
    .await;
    assert_eq!(forced["type"], "casting", "{forced}");
    assert_eq!(forced["re"], "c");
    assert_eq!(titular(&forced), "architecte");
    assert_eq!(forced["payload"]["forced"], "user");
    assert_eq!(
        forced["payload"]["reasons"]["architecte"],
        json!(["choisi par vous"])
    );

    let automatic = request(
        &mut socket,
        "cast_override",
        json!({ "path": "src/emprunts/reserver.ts", "section_id": "fn:reserver", "mentor_id": null }),
    )
    .await;
    assert_eq!(titular(&automatic), "mentor-langage");
    assert_eq!(automatic["payload"]["forced"], Value::Null);

    let unknown = request(
        &mut socket,
        "cast_override",
        json!({ "path": "src/emprunts/reserver.ts", "mentor_id": "inconnu" }),
    )
    .await;
    assert_eq!(unknown["payload"]["code"], "not_found", "{unknown}");
}

#[tokio::test]
async fn le_frontmatter_mentors_force_le_titulaire_du_document() {
    let dir = sample();
    fs::write(
        dir.path().join("notes.md"),
        "---\nmentors: [mentor-langage]\n---\n# Notes\n\nTexte.\n",
    )
    .unwrap();
    let mut socket = open_socket(dir.path()).await;
    let (_, casting) = open(&mut socket, "notes.md").await;
    assert_eq!(titular(&casting), "mentor-langage");
    assert_eq!(casting["payload"]["forced"], "frontmatter");
}

#[tokio::test]
async fn l_arborescence_porte_les_mentors_de_chaque_fichier() {
    let dir = sample();
    let mut socket = open_socket(dir.path()).await;
    let src = request(&mut socket, "list_dir", json!({ "path": "src/emprunts" })).await;
    assert_eq!(src["payload"]["entries"][0]["mentors"][0]["initials"], "ML");
    let docs = request(
        &mut socket,
        "list_dir",
        json!({ "path": "docs/architecture" }),
    )
    .await;
    assert_eq!(
        docs["payload"]["entries"][0]["mentors"][0]["id"],
        "architecte"
    );
    let root = request(&mut socket, "list_dir", json!({ "path": "" })).await;
    assert_eq!(
        root["payload"]["entries"][0]["mentors"],
        json!([]),
        "pas de mentor pour un dossier"
    );
}

#[tokio::test]
async fn un_mentor_du_projet_s_applique_sans_redemarrage() {
    let dir = sample();
    let mut socket = open_socket(dir.path()).await;
    open(&mut socket, "src/emprunts/reserver.ts").await;
    cursor(&mut socket, "src/emprunts/reserver.ts", 7).await;
    assert_eq!(titular(&next(&mut socket).await), "mentor-langage");

    let mentors = dir.path().join(".hodegos/mentors");
    fs::create_dir_all(&mentors).unwrap();
    fs::write(mentors.join("casse.md"), "---\nid: casse\n---\n").unwrap();
    let changed = next(&mut socket).await;
    assert_eq!(changed["type"], "mentors_changed", "{changed}");
    let warnings = changed["payload"]["warnings"].as_array().unwrap();
    assert_eq!(warnings.len(), 1, "{changed}");
    assert!(warnings[0]["file"].as_str().unwrap().ends_with("casse.md"));
    assert!(
        warnings[0]["message"]
            .as_str()
            .unwrap()
            .contains("name manquant")
    );

    fs::remove_file(mentors.join("casse.md")).unwrap();
    fs::write(
        mentors.join("testeur.md"),
        "---\nid: testeur\nname: Testeur\ninitials: TE\nrole: Testeur\npriority: 90\ntriggers:\n  paths: [\"src/**\"]\n  languages: [typescript]\n---\nPosture.\n",
    )
    .unwrap();
    // Plusieurs rechargements peuvent se suivre : on attend le casting du testeur.
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        let message = next(&mut socket).await;
        if message["type"] == "casting" {
            assert_eq!(titular(&message), "testeur", "{message}");
            // 40 points : sous le seuil de 70, le Mentor du langage reste disponible.
            assert_eq!(message["payload"]["consultants"], json!([]));
            let available = message["payload"]["available"].as_array().unwrap();
            assert!(
                available.iter().any(|m| m["id"] == "mentor-langage"),
                "{message}"
            );
            break;
        }
        assert_eq!(message["type"], "mentors_changed", "{message}");
        assert!(
            std::time::Instant::now() < deadline,
            "aucun nouveau casting"
        );
    }
}

#[tokio::test]
async fn un_enregistrement_recalcule_les_sections() {
    let dir = sample();
    let mut socket = open_socket(dir.path()).await;
    let (file, _) = open(&mut socket, "src/emprunts/reserver.ts").await;
    let content = file["payload"]["content"].as_str().unwrap();
    let edited = format!("export function premiere() {{}}\n\n{content}");
    let saved = request(
        &mut socket,
        "save",
        json!({ "path": "src/emprunts/reserver.ts", "content": edited,
                "base_version": file["payload"]["version"] }),
    )
    .await;
    assert_eq!(saved["type"], "saved");
    let casting = next(&mut socket).await;
    assert_eq!(casting["payload"]["section_id"], "fn:premiere", "{casting}");
}
