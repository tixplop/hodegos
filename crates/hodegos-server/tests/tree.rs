#![allow(clippy::unwrap_used, clippy::expect_used)]

//! L'arbre suit le disque (phase 8f) : un dossier affiché (listé par `list_dir`) dont
//! les entrées changent hors de Hodegos (terminal, autre outil) est repoussé en `dir`.
//! Une simple modification de contenu, ou un fichier ignoré par git, ne repousse rien.

mod common;

use std::fs;
use std::time::Duration;

use common::{Socket, exchange, git, next_within, open_socket};
use serde_json::{Value, json};

async fn list(socket: &mut Socket, path: &str) -> Value {
    let reply = exchange(
        socket,
        json!({ "v": 1, "type": "list_dir", "id": path, "payload": { "path": path } }),
    )
    .await;
    assert_eq!(reply["type"], "dir", "{reply}");
    reply
}

/// Prochain `dir` poussé, ou `None` si rien n'arrive pendant `wait`.
async fn pushed_dir(socket: &mut Socket, wait: Duration) -> Option<Value> {
    let deadline = std::time::Instant::now() + wait;
    loop {
        let left = deadline.saturating_duration_since(std::time::Instant::now());
        let message = next_within(socket, left).await?;
        if message["type"] == "dir" {
            return Some(message);
        }
    }
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
async fn un_fichier_cree_hors_de_hodegos_apparait_dans_l_arbre() {
    let dir = tempfile::tempdir().unwrap();
    git(dir.path(), &["init", "-q"]);
    fs::write(dir.path().join(".gitignore"), "cible/\n").unwrap();
    fs::create_dir(dir.path().join("src")).unwrap();
    fs::write(dir.path().join("src/a.rs"), "fn a() {}\n").unwrap();
    let mut socket = open_socket(dir.path()).await;
    list(&mut socket, "").await;
    list(&mut socket, "src").await;

    // Nouveau fichier à la racine : `dir` de la racine, avec lui.
    fs::write(dir.path().join("nouveau.txt"), "x\n").unwrap();
    let root = pushed_dir(&mut socket, Duration::from_secs(5))
        .await
        .expect("dir de la racine");
    assert_eq!(root["payload"]["path"], "");
    assert!(root.get("re").is_none(), "spontané : {root}");
    assert!(names(&root).contains(&"nouveau.txt".to_owned()), "{root}");

    // Dans un sous-dossier déplié.
    fs::write(dir.path().join("src/b.rs"), "fn b() {}\n").unwrap();
    let src = pushed_dir(&mut socket, Duration::from_secs(5))
        .await
        .expect("dir de src");
    assert_eq!(src["payload"]["path"], "src");
    assert_eq!(names(&src), ["a.rs", "b.rs"]);

    // Contenu modifié, dossier ignoré par git : rien.
    fs::write(dir.path().join("src/a.rs"), "fn a() { 1; }\n").unwrap();
    fs::create_dir(dir.path().join("cible")).unwrap();
    assert_eq!(
        pushed_dir(&mut socket, Duration::from_millis(1200)).await,
        None
    );

    // Suppression.
    fs::remove_file(dir.path().join("nouveau.txt")).unwrap();
    let root = pushed_dir(&mut socket, Duration::from_secs(5))
        .await
        .expect("dir après suppression");
    assert!(!names(&root).contains(&"nouveau.txt".to_owned()), "{root}");
}

#[tokio::test]
async fn un_dossier_non_affiche_n_est_pas_repousse() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("replie")).unwrap();
    let mut socket = open_socket(dir.path()).await;
    list(&mut socket, "").await;
    fs::write(dir.path().join("replie/x.txt"), "x\n").unwrap();
    assert_eq!(
        pushed_dir(&mut socket, Duration::from_millis(1200)).await,
        None
    );
    // Le dossier racine reste suivi.
    fs::write(dir.path().join("y.txt"), "y\n").unwrap();
    assert!(
        pushed_dir(&mut socket, Duration::from_secs(5))
            .await
            .is_some()
    );
}
