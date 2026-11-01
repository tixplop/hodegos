#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Revues automatiques (phase 9d), avec `FakeLlmClient` et une horloge simulée : à la
//! création, à l'ouverture après un changement fait hors de Hodegos, après des
//! modifications une fois au calme ; intervalle, plafond quotidien, budget, commit.

mod common;

use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use common::{Socket, fake_llm, next_within};
use hodegos_llm::{BudgetConfig, FakeLlmClient, LlmError, LlmResponse};
use hodegos_mentors::ReviewTrigger;
use hodegos_server::{Llm, ManualClock, ReviewOptions, Server, ServerOptions};
use jiff::Timestamp;
use serde_json::{Value, json};

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::create_dir(root.join(".git")).unwrap();
    write(
        root,
        "src/annuler.ts",
        "export function annuler(id: string): void {}\n",
    );
    write(
        root,
        ".hodegos/mentors/archi.md",
        "---\nid: archi\nname: Archi\ninitials: AR\nrole: Architecte de test\nmodel: strong\nscopes: [project]\nproject:\n  key_files: [\"src/**\"]\n  triggers: [create, open, change, commit]\n---\nRelisez.\n",
    );
    dir
}

fn empty_reviews(count: usize) -> Arc<FakeLlmClient> {
    Arc::new(FakeLlmClient::new((0..count).map(|_| {
        Ok::<_, LlmError>(LlmResponse {
            content: json!({ "remarks": [] }).to_string(),
            cost_usd: 0.001,
            ..Default::default()
        })
    })))
}

fn start() -> Timestamp {
    Timestamp::from_second(1_793_000_000).unwrap()
}

fn options(review: ReviewOptions) -> ReviewOptions {
    ReviewOptions {
        tick: Duration::from_millis(20),
        ..review
    }
}

async fn open(dir: &Path, llm: Llm, clock: Arc<ManualClock>, review: ReviewOptions) -> Socket {
    let options = ServerOptions {
        review: options(review),
        clock,
        ..common::options(dir, 0, llm)
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

/// Comme `open`, mais garde le `project_status` d'accueil.
async fn open_seeing(
    dir: &Path,
    llm: Llm,
    clock: Arc<ManualClock>,
    review: ReviewOptions,
) -> (Socket, Value) {
    let options = ServerOptions {
        review: options(review),
        clock,
        ..common::options(dir, 0, llm)
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
    assert_eq!(common::next(&mut socket).await["type"], "status");
    assert_eq!(common::next(&mut socket).await["type"], "git_status");
    let project = common::next(&mut socket).await;
    assert_eq!(project["type"], "project_status");
    (socket, project["payload"].clone())
}

/// Déclencheur de la prochaine revue lancée, après sa fin ; `None` si aucune ne part
/// pendant `wait`.
async fn next_review(socket: &mut Socket, wait: Duration) -> Option<String> {
    let deadline = tokio::time::Instant::now() + wait;
    let mut trigger = None;
    loop {
        let left = deadline.checked_duration_since(tokio::time::Instant::now())?;
        let message = next_within(socket, left).await?;
        match message["type"].as_str() {
            Some("review_started") => {
                trigger = message["payload"]["trigger"].as_str().map(str::to_owned)
            }
            Some("review_done") if trigger.is_some() => return trigger,
            _ => {}
        }
    }
}

/// Prochain `project_status` avec au moins une revue en attente.
async fn status(socket: &mut Socket) -> Value {
    loop {
        let message = next_within(socket, Duration::from_secs(5))
            .await
            .expect("project_status");
        if message["type"] == "project_status"
            && !message["payload"]["pending"].as_array().unwrap().is_empty()
        {
            return message["payload"].clone();
        }
    }
}

const QUIET: Duration = Duration::from_millis(400);
const SOON: Duration = Duration::from_secs(5);

#[tokio::test]
async fn creation_puis_ouverture_apres_un_changement_fait_ailleurs() {
    let dir = project();
    let fake = empty_reviews(4);
    let clock = Arc::new(ManualClock::new(start()));
    let mut socket = open(
        dir.path(),
        fake_llm(fake.clone()),
        clock.clone(),
        ReviewOptions::default(),
    )
    .await;
    assert_eq!(
        next_review(&mut socket, SOON).await.as_deref(),
        Some("create")
    );
    assert_eq!(fake.requests().len(), 1);
    drop(socket);

    // Relancé sans changement : aucune revue.
    let later = Arc::new(ManualClock::new(start()));
    later.advance(Duration::from_secs(3600));
    let mut again = open(
        dir.path(),
        fake_llm(fake.clone()),
        later.clone(),
        ReviewOptions::default(),
    )
    .await;
    assert_eq!(next_review(&mut again, QUIET).await, None);
    assert_eq!(fake.requests().len(), 1);
    drop(again);

    // Modifié hors de Hodegos, puis relancé : ouverture ; l'intervalle compte.
    write(
        dir.path(),
        "src/annuler.ts",
        "export function annuler(id: string): boolean { return true; }\n",
    );
    let soon = Arc::new(ManualClock::new(start()));
    soon.advance(Duration::from_secs(60));
    let (mut early, welcome) = open_seeing(
        dir.path(),
        fake_llm(fake.clone()),
        soon.clone(),
        ReviewOptions::default(),
    )
    .await;
    // Revue en attente : dans l'accueil si le démarrage était fini, sinon poussée ensuite.
    let pending = if welcome["pending"].as_array().unwrap().is_empty() {
        status(&mut early).await
    } else {
        welcome
    };
    assert_eq!(pending["pending"][0]["trigger"], "open", "{pending}");
    assert_eq!(
        next_review(&mut early, QUIET).await,
        None,
        "20 min depuis la création"
    );
    soon.advance(Duration::from_secs(1200));
    assert_eq!(next_review(&mut early, SOON).await.as_deref(), Some("open"));
}

#[tokio::test]
async fn modifications_au_calme_intervalle_plafond_quotidien_et_budget() {
    let dir = project();
    let fake = empty_reviews(4);
    let clock = Arc::new(ManualClock::new(start()));
    let changes_only = ReviewOptions {
        triggers: vec![ReviewTrigger::Change],
        daily_max: 2,
        ..ReviewOptions::default()
    };
    let mut socket = open(
        dir.path(),
        fake_llm(fake.clone()),
        clock.clone(),
        changes_only.clone(),
    )
    .await;
    assert_eq!(
        next_review(&mut socket, QUIET).await,
        None,
        "création non retenue"
    );

    write(
        dir.path(),
        "src/annuler.ts",
        "export function annuler(): void { /* 1 */ }\n",
    );
    let waiting = status(&mut socket).await;
    assert_eq!(waiting["pending"][0]["trigger"], "change", "{waiting}");
    assert_eq!(
        next_review(&mut socket, QUIET).await,
        None,
        "pas encore au calme"
    );
    clock.advance(Duration::from_secs(181));
    assert_eq!(
        next_review(&mut socket, SOON).await.as_deref(),
        Some("change")
    );

    // Deuxième modification : au calme, mais l'intervalle de 20 min n'est pas passé.
    write(
        dir.path(),
        "src/annuler.ts",
        "export function annuler(): void { /* 2 */ }\n",
    );
    status(&mut socket).await;
    clock.advance(Duration::from_secs(181));
    assert_eq!(next_review(&mut socket, QUIET).await, None, "intervalle");
    clock.advance(Duration::from_secs(1200));
    assert_eq!(
        next_review(&mut socket, SOON).await.as_deref(),
        Some("change")
    );

    // Plafond quotidien (2) atteint : plus rien aujourd'hui, même au calme et reposé.
    write(
        dir.path(),
        "src/annuler.ts",
        "export function annuler(): void { /* 3 */ }\n",
    );
    status(&mut socket).await;
    clock.advance(Duration::from_secs(1500));
    assert_eq!(
        next_review(&mut socket, QUIET).await,
        None,
        "plafond quotidien"
    );
    assert_eq!(fake.requests().len(), 2);
    // Le lendemain (UTC), la revue en attente part.
    clock.advance(Duration::from_secs(86_400));
    assert_eq!(
        next_review(&mut socket, SOON).await.as_deref(),
        Some("change")
    );

    // Budget atteint : plus rien d'automatique.
    let other = project();
    let mut broke = fake_llm(empty_reviews(1));
    broke.config.budget = BudgetConfig {
        monthly_cap_usd: 0.0,
        warn_at: 0.8,
    };
    let quiet = Arc::new(ManualClock::new(start()));
    let mut socket = open(other.path(), broke, quiet.clone(), ReviewOptions::default()).await;
    quiet.advance(Duration::from_secs(4000));
    assert_eq!(
        next_review(&mut socket, QUIET).await,
        None,
        "budget atteint"
    );
}

/// Projet d'exemple dans un vrai dépôt git, avec un premier commit ; `None` sans git.
fn committed() -> Option<tempfile::TempDir> {
    let dir = project();
    fs::remove_dir_all(dir.path().join(".git")).unwrap();
    let ok = common::git_init(dir.path())
        && common::git(dir.path(), &["add", "-A"])
        && common::git(dir.path(), &["commit", "-qm", "début"]);
    ok.then_some(dir)
}

fn commit(dir: &Path, name: &str) {
    write(dir, name, "x\n");
    assert!(common::git(dir, &["add", "-A"]));
    assert!(common::git(dir, &["commit", "-qm", name]));
}

#[tokio::test]
async fn commit_desactive_par_defaut_puis_active() {
    let Some(dir) = committed() else {
        return;
    };
    let fake = empty_reviews(4);
    let clock = Arc::new(ManualClock::new(start()));
    let defaults = ReviewOptions {
        min_interval: Duration::ZERO,
        ..ReviewOptions::default()
    };
    let mut socket = open(
        dir.path(),
        fake_llm(fake.clone()),
        clock.clone(),
        defaults.clone(),
    )
    .await;
    assert_eq!(
        next_review(&mut socket, SOON).await.as_deref(),
        Some("create")
    );
    commit(dir.path(), "notes.md");
    // Le commit modifie aussi un fichier : seule une revue `change` attend le calme.
    let waiting = status(&mut socket).await;
    assert!(
        waiting["pending"]
            .as_array()
            .unwrap()
            .iter()
            .all(|p| p["trigger"] == "change"),
        "commit désactivé par défaut : {waiting}"
    );
    drop(socket);

    let Some(other) = committed() else {
        return;
    };
    let commits = ReviewOptions {
        triggers: vec![ReviewTrigger::Commit],
        ..defaults
    };
    let mut socket = open(other.path(), fake_llm(fake.clone()), clock.clone(), commits).await;
    assert_eq!(next_review(&mut socket, QUIET).await, None);
    commit(other.path(), "notes.md");
    assert_eq!(
        next_review(&mut socket, SOON).await.as_deref(),
        Some("commit")
    );
}
