#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Réglages des revues du projet (phase 9e) : `config.toml` réécrit en gardant ses
//! commentaires, bornes refusées sans écriture, réglages appliqués sans redémarrage,
//! dépense du mois des revues automatiques ; portées et bloc `project` d'un mentor
//! enregistrés et validés champ par champ.

mod common;

use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use common::{Socket, next_within};
use hodegos_llm::{FakeLlmClient, LlmConfig};
use hodegos_server::{
    Connect, Llm, ManualClock, ReviewOptions, Server, ServerOptions, SettingsSource,
};
use jiff::Timestamp;
use serde_json::{Value, json};

const CONFIG: &str = "\
# Réglages de Hodegos
[llm]
fast = \"test/rapide\" # le niveau rapide
strong = \"test/fort\"

[project_review]
# Rythme des revues automatiques
calm_secs = 180 # trois minutes
";

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn llm(config_dir: &Path) -> Llm {
    let fake = Arc::new(FakeLlmClient::default());
    let config = LlmConfig::load(config_dir).unwrap();
    let client: Connect = Arc::new(move |_, _| Ok(fake.clone() as Arc<dyn hodegos_llm::LlmClient>));
    Llm {
        client: Err("aucune clé dans ce test".to_owned()),
        config,
        settings: Some(SettingsSource {
            config_dir: config_dir.to_path_buf(),
            key: None,
            key_from_env: false,
            connect: client,
        }),
    }
}

async fn open(dir: &Path, config_dir: &Path, clock: Arc<ManualClock>) -> Socket {
    let options = ServerOptions {
        review: ReviewOptions {
            tick: Duration::from_millis(20),
            ..ReviewOptions::default()
        },
        clock,
        ..common::options(dir, 0, llm(config_dir))
    };
    let server = Server::bind(common::project_in(dir), options)
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
            .expect("réponse");
        if message["re"] == id {
            return (message, seen);
        }
        seen.push(message);
    }
}

fn reviews(payload: Value) -> Value {
    json!({
        "project_review": payload,
        "context": { "map_cap": 6000, "neighborhood_cap": 2000, "key_files_cap": 10000 }
    })
}

#[tokio::test]
async fn reglages_des_revues_ecrits_et_appliques_sans_redemarrage() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join(".git")).unwrap();
    write(dir.path(), "src/a.ts", "export function a() {}\n");
    write(
        dir.path(),
        ".hodegos/mentors/archi.md",
        "---\nid: archi\nname: Archi\ninitials: AR\nrole: R\nscopes: [project]\n---\nRelisez.\n",
    );
    let config = tempfile::tempdir().unwrap();
    fs::write(config.path().join("config.toml"), CONFIG).unwrap();
    let clock = Arc::new(ManualClock::new(
        Timestamp::from_second(1_793_000_000).unwrap(),
    ));
    let mut socket = open(dir.path(), config.path(), clock.clone()).await;

    let (settings, _) = request(&mut socket, "c-1", "settings_get", Value::Null).await;
    let payload = &settings["payload"];
    assert_eq!(
        payload["project_review"],
        json!({ "enabled": true, "triggers": ["create", "open", "change"],
            "calm_secs": 180, "min_interval_secs": 1200, "daily_max": 12 })
    );
    assert_eq!(
        payload["context"],
        json!({ "map_cap": 8000, "neighborhood_cap": 3000, "key_files_cap": 12000 })
    );
    assert_eq!(payload["auto_spend"], json!({ "reviews": 0, "cost": 0.0 }));

    // Hors bornes : refusé, rien n'est écrit.
    let before = fs::read_to_string(config.path().join("config.toml")).unwrap();
    for (id, bad) in [
        (
            "c-2",
            reviews(
                json!({ "enabled": true, "triggers": [], "calm_secs": 999_999,
            "min_interval_secs": 600, "daily_max": 3 }),
            ),
        ),
        (
            "c-3",
            json!({ "project_review": { "enabled": true, "triggers": [], "calm_secs": 60,
            "min_interval_secs": 600, "daily_max": 3 },
            "context": { "map_cap": 10, "neighborhood_cap": 2000, "key_files_cap": 10000 } }),
        ),
    ] {
        let (refused, _) = request(&mut socket, id, "project_review_set", bad).await;
        assert_eq!(refused["payload"]["code"], "invalid_settings", "{refused}");
    }
    assert_eq!(
        fs::read_to_string(config.path().join("config.toml")).unwrap(),
        before
    );

    // Enregistré : commentaires gardés, réglages appliqués, carte recalculée.
    let (saved, seen) = request(
        &mut socket,
        "c-4",
        "project_review_set",
        reviews(
            json!({ "enabled": true, "triggers": ["change", "commit"], "calm_secs": 60,
            "min_interval_secs": 600, "daily_max": 3 }),
        ),
    )
    .await;
    assert_eq!(saved["type"], "settings", "{saved}");
    assert_eq!(saved["payload"]["project_review"]["calm_secs"], 60);
    assert_eq!(saved["payload"]["context"]["map_cap"], 6000);
    let text = fs::read_to_string(config.path().join("config.toml")).unwrap();
    assert!(text.contains("# Réglages de Hodegos"));
    assert!(text.contains("fast = \"test/rapide\" # le niveau rapide"));
    assert!(text.contains("# Rythme des revues automatiques"));
    assert!(text.contains("calm_secs = 60 # trois minutes"), "{text}");
    assert!(text.contains("triggers = [\"change\", \"commit\"]"));
    assert!(text.contains("[context]"));
    assert!(text.contains("map_cap = 6000"));
    let mut pushed = seen;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while !pushed.iter().any(|m| m["type"] == "map_updated") {
        let left = deadline
            .checked_duration_since(tokio::time::Instant::now())
            .expect("map_updated");
        pushed.push(next_within(&mut socket, left).await.expect("map_updated"));
    }
    let map = pushed.iter().find(|m| m["type"] == "map_updated").unwrap();
    assert_eq!(map["payload"]["cap"], 6000);

    // Un déclencheur retiré : la revue `create` en attente (sans fournisseur, elle ne
    // pouvait pas partir) disparaît au tour suivant.
    let (project, more) = request(&mut socket, "c-6", "settings_get", Value::Null).await;
    assert_eq!(
        project["payload"]["project_review"]["triggers"],
        json!(["change", "commit"])
    );
    let emptied = pushed.iter().chain(&more).any(|m| {
        m["type"] == "project_status" && m["payload"]["pending"].as_array().unwrap().is_empty()
    });
    if !emptied {
        loop {
            let message = next_within(&mut socket, Duration::from_secs(5))
                .await
                .expect("project_status");
            if message["type"] == "project_status"
                && message["payload"]["pending"].as_array().unwrap().is_empty()
            {
                break;
            }
        }
    }
}

#[tokio::test]
async fn portees_et_bloc_projet_dans_l_editeur_de_mentors() {
    let dir = tempfile::tempdir().unwrap();
    let config = tempfile::tempdir().unwrap();
    fs::write(config.path().join("config.toml"), CONFIG).unwrap();
    let clock = Arc::new(ManualClock::new(
        Timestamp::from_second(1_793_000_000).unwrap(),
    ));
    let mut socket = open(dir.path(), config.path(), clock).await;
    let form = |scopes: Value, project: Value| {
        json!({ "origin": "project", "id": "archi", "body": "Relisez à l'échelle {scope}.",
            "form": { "id": "archi", "name": "Archi", "initials": "AR", "role": "R",
                "enabled": true, "mode": "proactive", "model": "strong", "priority": 50,
                "entry_level": "indice", "max_remarks": 2, "on_open": false,
                "context": "neighborhood", "scopes": scopes, "project": project,
                "triggers": { "paths": [], "languages": ["typescript"], "headings": [], "keywords": [] } } })
    };

    // Erreurs rendues champ par champ, rien n'est écrit.
    let fields = |refused: &Value| -> Vec<String> {
        assert_eq!(refused["payload"]["code"], "mentor_invalid", "{refused}");
        refused["payload"]["errors"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e["field"].as_str().unwrap().to_owned())
            .collect()
    };
    let (refused, _) = request(
        &mut socket,
        "m-1",
        "mentor_save",
        form(
            json!([]),
            json!({ "key_files": [], "max_remarks": 3, "triggers": ["change"] }),
        ),
    )
    .await;
    assert_eq!(fields(&refused), ["scopes"]);
    let (refused, _) = request(
        &mut socket,
        "m-1b",
        "mentor_save",
        form(
            json!(["project"]),
            json!({ "key_files": ["a["], "max_remarks": 4, "triggers": ["change"] }),
        ),
    )
    .await;
    let fields = fields(&refused);
    assert!(
        fields.contains(&"project.max_remarks".to_owned()),
        "{fields:?}"
    );
    assert!(
        fields.contains(&"project.key_files[0]".to_owned()),
        "{fields:?}"
    );
    assert!(!dir.path().join(".hodegos").exists());

    // Valide : écrit, relu à l'identique, visible dans la liste.
    let (saved, _) = request(
        &mut socket,
        "m-2",
        "mentor_save",
        form(
            json!(["section", "project"]),
            json!({ "key_files": ["docs/architecture/**"], "max_remarks": 3, "triggers": ["change"] }),
        ),
    )
    .await;
    assert_eq!(saved["type"], "mentor_file", "{saved}");
    let text = fs::read_to_string(dir.path().join(".hodegos/mentors/archi.md")).unwrap();
    assert!(text.contains("scopes: [section, project]"), "{text}");
    assert!(text.contains("context: neighborhood"));
    assert!(text.contains("max_remarks: 3"));
    assert_eq!(
        saved["payload"]["form"]["project"]["triggers"],
        json!(["change"])
    );
    assert_eq!(
        saved["payload"]["form"]["scopes"],
        json!(["section", "project"])
    );
    let (list, _) = request(&mut socket, "m-3", "mentors_list", Value::Null).await;
    let archi = list["payload"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["id"] == "archi")
        .unwrap()
        .clone();
    assert_eq!(archi["scopes"], json!(["section", "project"]));
    assert_eq!(archi["context"], "neighborhood");
    assert_eq!(
        archi["project"]["key_files"],
        json!(["docs/architecture/**"])
    );
}
