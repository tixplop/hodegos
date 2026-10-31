#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Réglages des mentors (phase 8e) : personnaliser, éditer, créer, désactiver, revenir
//! au livré ; mêmes erreurs que le chargeur ; rechargement à chaud.

mod common;

use std::fs;
use std::path::Path;
use std::time::Duration;

use common::{
    Socket, connect, data_dir, exchange, next, next_within, no_llm, project_in, session_cookie,
    welcome,
};
use hodegos_mentors::{MentorSources, Origin, Parsed, parse};
use hodegos_server::{Server, ServerOptions};
use serde_json::{Value, json};

const ADR: &str = "docs/adr/0001-stockage.md";

struct Setup {
    project: tempfile::TempDir,
    user: tempfile::TempDir,
}

impl Setup {
    fn project_mentors(&self) -> std::path::PathBuf {
        self.project.path().join(".hodegos").join("mentors")
    }
}

async fn setup() -> (Setup, Socket) {
    let setup = Setup {
        project: tempfile::tempdir().unwrap(),
        user: tempfile::tempdir().unwrap(),
    };
    let adr = setup.project.path().join(ADR);
    fs::create_dir_all(adr.parent().unwrap()).unwrap();
    fs::write(&adr, "# ADR 0001\n\n## Décision\n\nTout en mémoire.\n").unwrap();
    let options = ServerOptions {
        port: 0,
        mentors: MentorSources {
            user_dir: Some(setup.user.path().join("mentors")),
            project_dir: Some(setup.project_mentors()),
        },
        data_dir: data_dir(setup.project.path()),
        llm: no_llm(),
        initial_file: None,
    };
    let server = Server::bind(project_in(setup.project.path()), options)
        .await
        .unwrap();
    let port = server.port();
    let cookie = session_cookie(&server).await;
    tokio::spawn(server.serve(std::future::pending()));
    let origin = format!("http://127.0.0.1:{port}");
    let mut socket = connect(port, Some(&origin), Some(&cookie)).await.unwrap();
    welcome(&mut socket).await;
    (setup, socket)
}

async fn request(socket: &mut Socket, kind: &str, payload: Value) -> Value {
    let mut reply = exchange(
        socket,
        json!({ "v": 1, "type": kind, "id": "m", "payload": payload }),
    )
    .await;
    while reply["re"] != "m" {
        reply = next(socket).await;
    }
    reply
}

/// Messages spontanés reçus jusqu'au silence.
async fn drain(socket: &mut Socket) -> Vec<Value> {
    let mut messages = Vec::new();
    while let Some(message) = next_within(socket, Duration::from_millis(600)).await {
        messages.push(message);
    }
    messages
}

fn item<'a>(list: &'a Value, id: &str) -> &'a Value {
    list["payload"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["id"] == id)
        .unwrap_or_else(|| panic!("{id} absent : {list}"))
}

#[tokio::test]
async fn un_mentor_livre_se_lit_et_se_personnalise_sans_etre_ecrit() {
    let (setup, mut socket) = setup().await;
    let list = request(&mut socket, "mentors_list", Value::Null).await;
    assert_eq!(list["type"], "mentors", "{list}");
    let architecte = item(&list, "architecte");
    assert_eq!(architecte["origin"], "builtin");
    assert_eq!(architecte["model"], "strong");
    assert_eq!(architecte["enabled"], true);

    let file = request(
        &mut socket,
        "mentor_get",
        json!({ "id": "architecte", "origin": "builtin" }),
    )
    .await;
    assert_eq!(file["type"], "mentor_file", "{file}");
    assert_eq!(file["payload"]["editable"], false);
    assert_eq!(file["payload"]["form"]["initials"], "AR");
    assert!(
        file["payload"]["body"]
            .as_str()
            .unwrap()
            .starts_with("Vous accompagnez")
    );

    let refused = request(
        &mut socket,
        "mentor_save",
        json!({ "origin": "builtin", "id": "architecte", "content": file["payload"]["content"] }),
    )
    .await;
    assert_eq!(refused["type"], "error");
    assert_eq!(refused["payload"]["code"], "invalid_settings");

    // Personnaliser : copie dans les mentors personnels, rechargement à chaud.
    let ejected = request(
        &mut socket,
        "mentor_eject",
        json!({ "id": "architecte", "to": "user" }),
    )
    .await;
    assert_eq!(ejected["payload"]["origin"], "user", "{ejected}");
    assert_eq!(ejected["payload"]["editable"], true);
    assert!(setup.user.path().join("mentors/architecte.md").is_file());
    let pushed = drain(&mut socket).await;
    assert!(
        pushed.iter().any(|m| m["type"] == "mentors_changed"),
        "{pushed:?}"
    );
    let list = request(&mut socket, "mentors_list", Value::Null).await;
    assert_eq!(item(&list, "architecte")["origin"], "user");
    assert_eq!(item(&list, "architecte")["overrides"], json!(["builtin"]));
    assert!(!setup.project_mentors().exists(), "rien dans le projet");
}

#[tokio::test]
async fn editer_par_le_formulaire_recharge_et_change_le_casting() {
    let (setup, mut socket) = setup().await;
    request(
        &mut socket,
        "mentor_eject",
        json!({ "id": "architecte", "to": "user" }),
    )
    .await;
    drain(&mut socket).await;
    let file = request(
        &mut socket,
        "mentor_get",
        json!({ "id": "architecte", "origin": "user" }),
    )
    .await;
    let mut form = file["payload"]["form"].clone();
    form["name"] = json!("Architecte exigeant");
    form["priority"] = json!(95);

    // Le fichier ouvert reçoit le nouveau titulaire après le rechargement.
    let opened = request(&mut socket, "open_file", json!({ "path": ADR })).await;
    assert_eq!(opened["type"], "file", "{opened}");
    drain(&mut socket).await;
    let saved = request(
        &mut socket,
        "mentor_save",
        json!({ "origin": "user", "id": "architecte", "form": form, "body": "Posture courte." }),
    )
    .await;
    assert_eq!(saved["type"], "mentor_file", "{saved}");
    assert_eq!(saved["payload"]["form"]["name"], "Architecte exigeant");
    let text = fs::read_to_string(setup.user.path().join("mentors/architecte.md")).unwrap();
    assert!(text.contains("name: Architecte exigeant"), "{text}");
    assert!(text.ends_with("---\nPosture courte.\n"), "{text}");
    let pushed = drain(&mut socket).await;
    assert!(
        pushed.iter().any(|m| m["type"] == "mentors_changed"),
        "{pushed:?}"
    );
    let casting = pushed
        .iter()
        .find(|m| m["type"] == "casting")
        .expect("casting recalculé");
    assert_eq!(casting["payload"]["titular"]["name"], "Architecte exigeant");

    let prompt = request(&mut socket, "mentor_prompt", json!({ "id": "architecte" })).await;
    let text = prompt["payload"]["text"].as_str().unwrap();
    assert!(
        text.starts_with("Vous êtes Architecte exigeant, un mentor dans Hodegos"),
        "{text}"
    );
    assert!(text.ends_with("Posture courte."), "{text}");
}

#[tokio::test]
async fn un_refus_donne_les_erreurs_du_chargeur_et_n_ecrit_rien() {
    let (setup, mut socket) = setup().await;
    let content = "---\nid: testeur\nname: Testeur\ninitials: T\nrole: Testeur\npriority: 300\nmode: bavard\ntriggers:\n  paths: [\"a[\"]\n---\nPosture.\n";
    let refused = request(
        &mut socket,
        "mentor_save",
        json!({ "origin": "project", "id": "testeur", "content": content }),
    )
    .await;
    assert_eq!(refused["payload"]["code"], "mentor_invalid", "{refused}");
    let fields: Vec<&str> = refused["payload"]["errors"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["field"].as_str().unwrap())
        .collect();
    assert_eq!(
        fields,
        ["initials", "priority", "mode", "triggers.paths[0]"]
    );
    // Le message est exactement celui du chargeur.
    let loader = parse(content, Origin::Project, "testeur.md").unwrap_err().0;
    assert_eq!(refused["payload"]["message"], loader);
    assert!(
        !setup.project_mentors().exists(),
        ".hodegos/mentors/ non créé"
    );

    // Un fichier en erreur, écrit à la main, est listé avec la même raison.
    fs::create_dir_all(setup.project_mentors()).unwrap();
    fs::write(setup.project_mentors().join("testeur.md"), content).unwrap();
    let list = request(&mut socket, "mentors_list", Value::Null).await;
    let error = &list["payload"]["errors"][0];
    assert_eq!(error["id"], "testeur");
    assert_eq!(error["origin"], "project");
    assert_eq!(
        error["errors"][0]["message"],
        "initials « T » : deux lettres attendues"
    );
    let file = request(
        &mut socket,
        "mentor_get",
        json!({ "id": "testeur", "origin": "project" }),
    )
    .await;
    assert_eq!(file["payload"]["form"], Value::Null);
    assert_eq!(file["payload"]["errors"].as_array().unwrap().len(), 4);

    // L'identifiant ne change pas à l'enregistrement.
    let renamed = request(
        &mut socket,
        "mentor_save",
        json!({ "origin": "user", "id": "architecte",
            "content": "---\nid: autre\nenabled: false\n---\n" }),
    )
    .await;
    assert_eq!(renamed["payload"]["errors"][0]["field"], "id", "{renamed}");
}

#[tokio::test]
async fn creer_un_mentor_de_projet_cree_le_dossier_a_l_enregistrement() {
    let (setup, mut socket) = setup().await;
    assert!(!setup.project_mentors().exists());
    let content = "---\nid: testeur\nname: Testeur\ninitials: TE\nrole: Testeur logiciel\nmode: on-demand\n---\nVous relisez les tests en {language}.\n";
    let saved = request(
        &mut socket,
        "mentor_save",
        json!({ "origin": "project", "id": "testeur", "content": content }),
    )
    .await;
    assert_eq!(saved["type"], "mentor_file", "{saved}");
    assert_eq!(
        fs::read_to_string(setup.project_mentors().join("testeur.md")).unwrap(),
        content
    );
    let list = request(&mut socket, "mentors_list", Value::Null).await;
    assert_eq!(item(&list, "testeur")["origin"], "project");
    assert_eq!(item(&list, "testeur")["mode"], "on-demand");

    // Prompt d'un brouillon : posture avec {language} remplacé.
    let prompt = request(
        &mut socket,
        "mentor_prompt",
        json!({ "id": "testeur", "content": content.replace("TE", "TS") }),
    )
    .await;
    assert!(
        prompt["payload"]["text"]
            .as_str()
            .unwrap()
            .ends_with("Vous relisez les tests en rust.")
    );
}

#[tokio::test]
async fn desactiver_un_mentor_livre_puis_revenir_au_livre() {
    let (setup, mut socket) = setup().await;
    let disabled = request(
        &mut socket,
        "mentor_save",
        json!({ "origin": "user", "id": "mentor-langage",
            "content": hodegos_mentors::minimal_override("mentor-langage") }),
    )
    .await;
    assert_eq!(disabled["type"], "mentor_file", "{disabled}");
    assert_eq!(
        disabled["payload"]["form"],
        Value::Null,
        "surcharge minimale"
    );
    let list = request(&mut socket, "mentors_list", Value::Null).await;
    let langage = item(&list, "mentor-langage");
    assert_eq!(langage["enabled"], false);
    assert_eq!(
        langage["name"], "Mentor du langage",
        "nom du mentor désactivé"
    );
    assert_eq!(langage["origin"], "user");

    let refused = request(
        &mut socket,
        "mentor_delete",
        json!({ "id": "mentor-langage", "origin": "builtin" }),
    )
    .await;
    assert_eq!(refused["payload"]["code"], "invalid_settings");
    let back = request(
        &mut socket,
        "mentor_delete",
        json!({ "id": "mentor-langage", "origin": "user" }),
    )
    .await;
    assert_eq!(back["type"], "mentors", "{back}");
    assert_eq!(item(&back, "mentor-langage")["enabled"], true);
    assert_eq!(item(&back, "mentor-langage")["origin"], "builtin");
    assert!(!setup.user.path().join("mentors/mentor-langage.md").exists());

    // Désactiver sa propre copie garde le mentor complet, réactivable.
    request(
        &mut socket,
        "mentor_eject",
        json!({ "id": "architecte", "to": "project" }),
    )
    .await;
    let file = request(
        &mut socket,
        "mentor_get",
        json!({ "id": "architecte", "origin": "project" }),
    )
    .await;
    let mut form = file["payload"]["form"].clone();
    form["enabled"] = json!(false);
    let off = request(
        &mut socket,
        "mentor_save",
        json!({ "origin": "project", "id": "architecte", "form": form, "body": file["payload"]["body"] }),
    )
    .await;
    assert_eq!(off["payload"]["form"]["enabled"], false, "{off}");
    let text = fs::read_to_string(setup.project_mentors().join("architecte.md")).unwrap();
    assert!(matches!(
        parse(&text, Origin::Project, "a.md"),
        Ok(Parsed::Disabled { .. })
    ));
    assert!(Path::new(&setup.project_mentors()).is_dir());
}

#[tokio::test]
async fn un_formulaire_hors_limites_est_refuse_champ_par_champ() {
    let (setup, mut socket) = setup().await;
    request(
        &mut socket,
        "mentor_eject",
        json!({ "id": "architecte", "to": "user" }),
    )
    .await;
    let file = request(
        &mut socket,
        "mentor_get",
        json!({ "id": "architecte", "origin": "user" }),
    )
    .await;
    let before = fs::read_to_string(setup.user.path().join("mentors/architecte.md")).unwrap();
    let mut form = file["payload"]["form"].clone();
    form["initials"] = json!("T");
    form["priority"] = json!(300);
    let refused = request(
        &mut socket,
        "mentor_save",
        json!({ "origin": "user", "id": "architecte", "form": form, "body": "Posture." }),
    )
    .await;
    assert_eq!(refused["payload"]["code"], "mentor_invalid", "{refused}");
    let fields: Vec<&str> = refused["payload"]["errors"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["field"].as_str().unwrap())
        .collect();
    assert_eq!(fields, ["initials", "priority"]);
    assert_eq!(
        fs::read_to_string(setup.user.path().join("mentors/architecte.md")).unwrap(),
        before
    );
}
