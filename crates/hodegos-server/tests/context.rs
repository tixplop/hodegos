#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Contexte des mentors de section (phase 9b), avec `FakeLlmClient` : un mentor
//! `context: neighborhood` reçoit le voisinage, `project` aussi la carte, `section`
//! rien de plus ; les renvois (`see_also`) sont filtrés et complétés ; chaque remarque
//! garde la liste de ce qui est parti, sans les contenus.

mod common;

use std::fs;
use std::path::Path;
use std::sync::Arc;

use common::{Socket, data_dir, exchange, fake_llm, next, open_socket_with, project_in};
use hodegos_llm::{FakeLlmClient, LlmError, LlmResponse};
use serde_json::{Value, json};

const PATH: &str = "src/emprunts/annuler.ts";
const ANNULER: &str = "\
import { lister, enregistrer } from \"../stockage/depot\";

export function annuler(id: string): void {
  const emprunts = lister().filter((e) => e.id !== id);
  enregistrer(emprunts);
}
";

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

/// Projet d'exemple et mentor du dépôt `voisin`, de contexte `context`.
fn project(context: Option<&str>) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(root, ".gitignore", "cache/\n");
    write(root, PATH, ANNULER);
    write(
        root,
        "src/stockage/depot.ts",
        "export function lister(): Emprunt[] {\n  return JSON.parse(lire());\n}\nexport function enregistrer(e: Emprunt[]): void {\n  ecrire(JSON.stringify(e));\n}\n",
    );
    write(
        root,
        "src/emprunts/reserver.ts",
        "export function reserver(objet: string): boolean {\n  return true;\n}\n",
    );
    write(root, "src/emprunts/.env", "API_KEY=abcdef1234567\n");
    write(
        root,
        "cache/espion.ts",
        "import { annuler } from \"../src/emprunts/annuler\";\nexport function espion() {}\n",
    );
    write(
        root,
        "docs/notes.md",
        "# Notes\n\n## Annulation\n\nVoir src/emprunts/annuler.ts : on relit tout.\npassword = \"tres-secret-99\"\n",
    );
    let context = context
        .map(|c| format!("context: {c}\n"))
        .unwrap_or_default();
    write(
        root,
        ".hodegos/mentors/voisin.md",
        &format!(
            "---\nid: voisin\nname: Voisin\ninitials: VO\nrole: Mentor de test\nmode: proactive\nmodel: fast\npriority: 100\n{context}triggers:\n  languages: [typescript]\n---\nPosture du voisin.\n"
        ),
    );
    dir
}

fn answer(see_also: Option<Vec<Value>>) -> Result<LlmResponse, LlmError> {
    let mut remark = json!({ "quote": "lister().filter", "level": "indice", "kind": "question",
        "message": "D'où viennent les emprunts relus ici ?", "confidence": 0.9 });
    if let Some(see_also) = see_also {
        remark["see_also"] = json!(see_also);
    }
    Ok(LlmResponse {
        content: json!({ "remarks": [remark] }).to_string(),
        cost_usd: 0.001,
        ..Default::default()
    })
}

fn reply() -> Result<LlmResponse, LlmError> {
    Ok(LlmResponse {
        content: json!({ "replies": [{ "level": "question",
            "message": "Que fait enregistrer de ce qu'il reçoit ?", "confidence": 0.8 }] })
        .to_string(),
        cost_usd: 0.001,
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

/// Ouvre, modifie et enregistre `annuler.ts` ; renvoie la remarque produite.
async fn remark_after_save(socket: &mut Socket) -> Value {
    let file = request(socket, "open_file", json!({ "path": PATH })).await;
    assert_eq!(file["type"], "file", "{file}");
    let casting = next(socket).await;
    assert_eq!(casting["type"], "casting");
    let saved = request(
        socket,
        "save",
        json!({ "path": PATH, "content": ANNULER.replace("e.id !== id", "e.id != id"),
            "base_version": file["payload"]["version"] }),
    )
    .await;
    assert_eq!(saved["type"], "saved", "{saved}");
    loop {
        let message = next(socket).await;
        if message["type"] == "remarks" {
            return message["payload"]["items"][0].clone();
        }
        assert_ne!(message["type"], "status", "aucune remarque : {message}");
    }
}

fn blocks(remark: &Value) -> Vec<String> {
    remark["context"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["block"].as_str().unwrap().to_owned())
        .collect()
}

#[tokio::test]
async fn un_mentor_de_voisinage_recoit_le_voisinage_et_renvoie_a_d_autres_fichiers() {
    let dir = project(Some("neighborhood"));
    let fake = Arc::new(FakeLlmClient::new([answer(Some(vec![
        json!({ "path": "src/stockage/depot.ts", "quote": "lister(): Emprunt[]" }),
        json!({ "path": "src/autre.ts", "quote": "" }),
        json!({ "path": "src/stockage/depot.ts", "quote": "JSON.parse" }),
        json!({ "path": "docs/notes.md", "quote": "on relit tout" }),
    ]))]));
    let mut socket = open_socket_with(dir.path(), fake_llm(fake.clone())).await;
    let remark = remark_after_save(&mut socket).await;

    // Ce qui est parti : voisinage en signatures, sans ignoré, secret ni corps.
    let sent = fake.requests()[0].clone();
    let system = &sent.messages[0].content;
    let user = &sent.messages[1].content;
    assert!(system.contains("Le voisinage et la carte servent à comprendre"));
    assert!(system.contains("Champ see_also"));
    assert!(user.contains(
        "<voisinage>\n<fichier_voisin chemin=\"src/stockage/depot.ts\" relation=\"importé par ce fichier\">\nexport function lister(): Emprunt[]\nexport function enregistrer(e: Emprunt[]): void\n</fichier_voisin>"
    ), "{user}");
    assert!(
        user.contains("<fichier_voisin chemin=\"docs/notes.md\" relation=\"cite ce fichier\">")
    );
    assert!(user.contains(
        "<fichier_voisin chemin=\"src/emprunts/reserver.ts\" relation=\"même dossier\">"
    ));
    let voisinage = user.find("<voisinage>").unwrap();
    assert!(user.find("</section>").unwrap() < voisinage);
    assert!(voisinage < user.find("<diff>").unwrap());
    for absent in [
        "JSON.parse",
        "cache/espion",
        "espion()",
        ".env",
        "abcdef1234567",
        "tres-secret-99",
        "<carte_du_projet>",
    ] {
        assert!(!user.contains(absent), "{absent} envoyé :\n{user}");
    }
    assert!(user.contains("[secret masqué]"));
    assert!(
        sent.schema["properties"]["remarks"]["items"]["required"]
            .as_array()
            .unwrap()
            .contains(&json!("see_also"))
    );

    // Renvois : fichier non envoyé et citation hors du texte envoyé retirés.
    assert_eq!(
        remark["see_also"],
        json!([
            { "path": "src/stockage/depot.ts", "quote": "lister(): Emprunt[]",
              "section_id": "fn:lister", "section_name": "lister", "line_hint": 1 },
            { "path": "docs/notes.md", "quote": "on relit tout",
              "section_id": "h:annulation", "section_name": "Annulation", "line_hint": 5 }
        ])
    );
    assert_eq!(
        remark["anchor"]["section_id"], "fn:annuler",
        "ancrée dans le fichier relu"
    );

    // Contexte de cette passe : blocs et jetons, jamais les contenus.
    assert_eq!(
        blocks(&remark),
        [
            "section",
            "neighborhood",
            "neighbor",
            "neighbor",
            "neighbor",
            "map"
        ]
    );
    for item in remark["context"].as_array().unwrap() {
        let keys: Vec<&String> = item.as_object().unwrap().keys().collect();
        assert_eq!(keys, ["block", "detail", "tokens"], "{item}");
        let detail = item["detail"].as_str().unwrap();
        assert!(
            !detail.contains("export function") && !detail.contains("relit"),
            "{detail}"
        );
    }
    assert_eq!(
        remark["context"][2]["detail"],
        "src/stockage/depot.ts — importé par ce fichier, signatures"
    );
    assert_eq!(
        remark["context"][5]["detail"],
        "Carte du projet non envoyée (contexte : voisinage)"
    );
    assert!(remark["context"][1]["tokens"].as_u64().unwrap() > 0);

    // Persisté avec la remarque.
    let id = project_in(dir.path()).id().as_str().to_owned();
    let stored: Value = serde_json::from_str(
        &fs::read_to_string(data_dir(dir.path()).join(id).join("remarks.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(stored["remarks"][0]["see_also"], remark["see_also"]);
    assert_eq!(stored["remarks"][0]["context"], remark["context"]);

    // Le palier suivant reçoit le même contexte, sans demande de renvois.
    fake.push(reply());
    let update = request(
        &mut socket,
        "hint_next",
        json!({ "remark_id": remark["id"] }),
    )
    .await;
    assert_eq!(update["type"], "remark_update", "{update}");
    assert_eq!(update["payload"]["see_also"], remark["see_also"]);
    let followed = fake.requests().last().unwrap().clone();
    assert!(followed.messages[1].content.contains("<voisinage>"));
    assert!(
        followed.messages[0]
            .content
            .contains("Le voisinage et la carte")
    );
    assert!(!followed.messages[0].content.contains("Champ see_also"));
}

#[tokio::test]
async fn un_mentor_de_contexte_projet_recoit_aussi_la_carte() {
    let dir = project(Some("project"));
    let fake = Arc::new(FakeLlmClient::new([answer(Some(vec![]))]));
    let mut socket = open_socket_with(dir.path(), fake_llm(fake.clone())).await;
    let remark = remark_after_save(&mut socket).await;
    let user = fake.requests()[0].messages[1].content.clone();
    assert!(user.contains("<voisinage>"));
    let map = user.find("<carte_du_projet>\n# ").expect("carte envoyée");
    assert!(map < user.find("<diff>").unwrap());
    assert!(user.contains("carte du projet (calculée par Hodegos)"));
    assert!(!user[map..].contains("cache/"), "ignoré absent de la carte");
    let last = remark["context"]
        .as_array()
        .unwrap()
        .last()
        .unwrap()
        .clone();
    assert_eq!(last["block"], "map");
    assert_eq!(last["detail"], "Carte du projet");
    assert!(last["tokens"].as_u64().unwrap() > 0);
    assert!(
        remark.get("see_also").is_none(),
        "aucun renvoi : champ absent"
    );
}

#[tokio::test]
async fn un_mentor_de_section_garde_son_prompt_et_ne_renvoie_a_rien() {
    let dir = project(None);
    let fake = Arc::new(FakeLlmClient::new([answer(Some(vec![
        json!({ "path": "src/stockage/depot.ts", "quote": "" }),
    ]))]));
    let mut socket = open_socket_with(dir.path(), fake_llm(fake.clone())).await;
    let remark = remark_after_save(&mut socket).await;
    let sent = fake.requests()[0].clone();
    assert!(!sent.messages[0].content.contains("Contexte élargi"));
    assert!(!sent.messages[1].content.contains("<voisinage>"));
    assert!(sent.messages[1].content.contains("</section>\n<diff>"));
    assert!(sent.schema["properties"]["remarks"]["items"]["properties"]["see_also"].is_null());
    assert!(remark.get("see_also").is_none());
    assert_eq!(blocks(&remark), ["section", "neighborhood"]);
    assert_eq!(remark["context"][1]["tokens"], 0);
}
