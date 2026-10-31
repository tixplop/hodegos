#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Terminal intégré et confiné (phase 8f). Le protocole et les limites se testent
//! partout avec un faux `bwrap` (qui lance le shell sans confinement) ; le confinement
//! seulement si le vrai `bwrap` fonctionne, sinon ces tests sont ignorés avec un message.

mod common;

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use common::{
    Socket, connect, data_dir, git, next_within, no_llm, project_in, send_raw, session_cookie,
    sources_in, welcome,
};
use hodegos_server::{Bind, BindMode, Server, ServerOptions, TerminalOptions};
use serde_json::{Value, json};

/// `bwrap` réel utilisable ici ? La CI GitHub restreint les espaces de noms.
fn bwrap_works() -> bool {
    let ok = Command::new("bwrap")
        .args([
            "--ro-bind",
            "/",
            "/",
            "--dev",
            "/dev",
            "--unshare-pid",
            "--",
            "true",
        ])
        .output()
        .is_ok_and(|o| o.status.success());
    if !ok {
        eprintln!("bwrap indisponible ici : tests du confinement ignorés");
    }
    ok
}

/// Faux `bwrap` : saute les options jusqu'à `--`, lance le reste dans le projet.
fn fake_bwrap(dir: &Path) -> PathBuf {
    let path = dir.join("faux-bwrap");
    fs::write(
        &path,
        "#!/bin/sh\nwhile [ \"$1\" != \"--\" ]; do shift; done\nshift\nexec \"$@\"\n",
    )
    .unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    path
}

async fn start(project: &Path, terminal: TerminalOptions) -> Socket {
    let options = ServerOptions {
        port: 0,
        mentors: sources_in(project),
        data_dir: data_dir(project),
        llm: no_llm(),
        terminal,
        initial_file: None,
    };
    let server = Server::bind(project_in(project), options).await.unwrap();
    let port = server.port();
    let cookie = session_cookie(&server).await;
    tokio::spawn(server.serve(std::future::pending()));
    let origin = format!("http://127.0.0.1:{port}");
    let mut socket = connect(port, Some(&origin), Some(&cookie)).await.unwrap();
    welcome(&mut socket).await;
    socket
}

fn options(bwrap: PathBuf, home: &Path, binds: Vec<Bind>) -> TerminalOptions {
    TerminalOptions {
        bwrap,
        binds,
        home: home.to_path_buf(),
        shell: "/bin/sh".to_owned(),
        warning: None,
    }
}

async fn send(socket: &mut Socket, kind: &str, id: Option<&str>, payload: Value) {
    let mut message = json!({ "v": 1, "type": kind, "payload": payload });
    if let Some(id) = id {
        message["id"] = json!(id);
    }
    send_raw(socket, &message.to_string()).await;
}

/// Ouvre un terminal ; renvoie son identifiant.
async fn open(socket: &mut Socket) -> u64 {
    send(
        socket,
        "term_open",
        Some("t"),
        json!({ "cols": 100, "rows": 30 }),
    )
    .await;
    loop {
        let message = next_within(socket, Duration::from_secs(5))
            .await
            .expect("term_opened");
        if message["re"] == "t" {
            assert_eq!(message["type"], "term_opened", "{message}");
            return message["payload"]["term_id"].as_u64().unwrap();
        }
    }
}

async fn type_line(socket: &mut Socket, id: u64, line: &str) {
    send(
        socket,
        "term_input",
        None,
        json!({ "term_id": id, "data": STANDARD.encode(format!("{line}\n")) }),
    )
    .await;
}

/// Sortie du terminal jusqu'à `marker` ; les autres messages sont rendus à part.
async fn output_until(socket: &mut Socket, id: u64, marker: &str) -> (String, Vec<Value>) {
    let mut text = Vec::new();
    let mut others = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(15);
    while !String::from_utf8_lossy(&text).contains(marker) {
        let wait = deadline.saturating_duration_since(Instant::now());
        let message = next_within(socket, wait).await.unwrap_or_else(|| {
            panic!(
                "« {marker} » jamais vu : {}",
                String::from_utf8_lossy(&text)
            )
        });
        if message["type"] == "term_output" && message["payload"]["term_id"] == id {
            text.extend(
                STANDARD
                    .decode(message["payload"]["data"].as_str().unwrap())
                    .unwrap(),
            );
        } else {
            others.push(message);
        }
    }
    (String::from_utf8_lossy(&text).into_owned(), others)
}

/// Processus dont la ligne de commande est exactement `args`.
fn running(args: &str) -> bool {
    fs::read_dir("/proc")
        .unwrap()
        .filter_map(Result::ok)
        .any(|entry| {
            fs::read(entry.path().join("cmdline"))
                .map(|c| String::from_utf8_lossy(&c).replace('\0', " ").trim() == args)
                .unwrap_or(false)
        })
}

async fn gone(args: &str) -> bool {
    for _ in 0..30 {
        if !running(args) {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    false
}

#[tokio::test]
async fn sans_bwrap_pas_de_terminal_et_le_message_dit_pourquoi() {
    let project = tempfile::tempdir().unwrap();
    let mut socket = start(
        project.path(),
        options(
            PathBuf::from("/chemin/absent/bwrap"),
            project.path(),
            Vec::new(),
        ),
    )
    .await;
    send(
        &mut socket,
        "term_open",
        Some("t"),
        json!({ "cols": 80, "rows": 24 }),
    )
    .await;
    let reply = next_within(&mut socket, Duration::from_secs(5))
        .await
        .unwrap();
    assert_eq!(reply["type"], "error", "{reply}");
    assert_eq!(reply["payload"]["code"], "terminal_unavailable");
    assert!(
        reply["payload"]["message"]
            .as_str()
            .unwrap()
            .contains("sudo apt install bubblewrap")
    );
}

#[tokio::test]
async fn protocole_limites_et_fin_du_shell() {
    let project = tempfile::tempdir().unwrap();
    let tools = tempfile::tempdir().unwrap();
    let mut socket = start(
        project.path(),
        options(fake_bwrap(tools.path()), project.path(), Vec::new()),
    )
    .await;
    let id = open(&mut socket).await;
    type_line(
        &mut socket,
        id,
        "echo \"pwd=$(pwd) term=$TERM\"; echo FIN-$((40+2))",
    )
    .await;
    let (text, _) = output_until(&mut socket, id, "FIN-42").await;
    let root = fs::canonicalize(project.path()).unwrap();
    assert!(text.contains(&format!("pwd={}", root.display())), "{text}");
    assert!(text.contains("term=xterm-256color"), "{text}");

    // Saisie trop longue, base64 invalide, terminal inconnu.
    let big = STANDARD.encode(vec![b'a'; 64 * 1024 + 1]);
    send(
        &mut socket,
        "term_input",
        Some("g"),
        json!({ "term_id": id, "data": big }),
    )
    .await;
    let reply = next_within(&mut socket, Duration::from_secs(5))
        .await
        .unwrap();
    assert_eq!(reply["payload"]["code"], "bad_message", "{reply}");
    send(
        &mut socket,
        "term_input",
        Some("b"),
        json!({ "term_id": id, "data": "!!" }),
    )
    .await;
    assert_eq!(
        next_within(&mut socket, Duration::from_secs(5))
            .await
            .unwrap()["payload"]["code"],
        "bad_message"
    );
    send(
        &mut socket,
        "term_input",
        Some("i"),
        json!({ "term_id": 99, "data": "" }),
    )
    .await;
    assert_eq!(
        next_within(&mut socket, Duration::from_secs(5))
            .await
            .unwrap()["payload"]["code"],
        "not_found"
    );

    // Quatre terminaux au plus.
    for _ in 0..3 {
        open(&mut socket).await;
    }
    send(
        &mut socket,
        "term_open",
        Some("x"),
        json!({ "cols": 80, "rows": 24 }),
    )
    .await;
    let refused = loop {
        let message = next_within(&mut socket, Duration::from_secs(5))
            .await
            .unwrap();
        if message["re"] == "x" {
            break message;
        }
    };
    assert_eq!(refused["payload"]["code"], "terminal_limit", "{refused}");

    // `exit` : term_exited avec le code.
    type_line(&mut socket, id, "exit 7").await;
    let exited = loop {
        let message = next_within(&mut socket, Duration::from_secs(5))
            .await
            .expect("term_exited");
        if message["type"] == "term_exited" && message["payload"]["term_id"] == id {
            break message;
        }
    };
    assert_eq!(exited["payload"]["code"], 7);
}

#[tokio::test]
async fn le_shell_est_confine_au_projet_et_aux_montages() {
    if !bwrap_works() {
        return;
    }
    let project = tempfile::tempdir().unwrap();
    // $HOME hors de /tmp (monté vide dans le terminal) : /var/tmp n'y est pas monté.
    let home = tempfile::Builder::new().tempdir_in("/var/tmp").unwrap();
    fs::write(home.path().join(".gitconfig"), "[user]\n\tname = Test\n").unwrap();
    fs::create_dir(home.path().join("cache")).unwrap();
    fs::create_dir(home.path().join(".ssh")).unwrap();
    fs::write(home.path().join(".ssh/id_secret"), "secret").unwrap();
    let binds = vec![
        Bind {
            path: "~/.gitconfig".to_owned(),
            mode: BindMode::Ro,
        },
        Bind {
            path: "~/cache".to_owned(),
            mode: BindMode::Rw,
        },
    ];
    let mut socket = start(
        project.path(),
        options(PathBuf::from("bwrap"), home.path(), binds),
    )
    .await;
    let id = open(&mut socket).await;
    let uid = String::from_utf8(Command::new("id").arg("-u").output().unwrap().stdout).unwrap();
    type_line(
        &mut socket,
        id,
        "echo \"pwd=$(pwd) uid=$(id -u)\"; echo \"maison=$(ls -A ~ | tr '\\n' ' ')\"; \
         cd / && echo \"racine=$(ls | tr '\\n' ' ')\"; \
         touch ~/x 2>&1; touch /usr/x 2>&1; echo dedans > ~/cache/f && echo cache-ok; \
         echo projet > \"$OLDPWD/ecrit.txt\" && echo projet-ok; echo FIN-$((40+2))",
    )
    .await;
    let (text, _) = output_until(&mut socket, id, "FIN-42").await;
    let root = fs::canonicalize(project.path()).unwrap();
    assert!(
        text.contains(&format!("pwd={} uid={}", root.display(), uid.trim())),
        "{text}"
    );
    assert!(
        text.contains("maison=.gitconfig cache "),
        "seuls les montages : {text}"
    );
    assert!(
        !text.contains("id_secret") && !text.contains(".ssh"),
        "{text}"
    );
    assert!(
        !text.contains(" root ") && !text.contains(" boot "),
        "{text}"
    );
    assert_eq!(
        text.matches("Read-only file system").count(),
        2,
        "~ et /usr : {text}"
    );
    assert!(
        text.contains("cache-ok") && text.contains("projet-ok"),
        "{text}"
    );
    assert_eq!(
        fs::read_to_string(home.path().join("cache/f")).unwrap(),
        "dedans\n"
    );
    assert!(project.path().join("ecrit.txt").is_file());
}

#[tokio::test]
async fn fermer_l_onglet_ou_la_fenetre_tue_le_shell_et_ses_descendants() {
    if !bwrap_works() {
        return;
    }
    let project = tempfile::tempdir().unwrap();
    let home = tempfile::Builder::new().tempdir_in("/var/tmp").unwrap();
    let mut socket = start(
        project.path(),
        options(PathBuf::from("bwrap"), home.path(), Vec::new()),
    )
    .await;

    let id = open(&mut socket).await;
    type_line(
        &mut socket,
        id,
        "sleep 4242 & setsid sleep 4243 < /dev/null > /dev/null 2>&1 & echo PRET",
    )
    .await;
    output_until(&mut socket, id, "PRET").await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(running("sleep 4242") && running("sleep 4243"));
    send(&mut socket, "term_close", None, json!({ "term_id": id })).await;
    loop {
        let message = next_within(&mut socket, Duration::from_secs(5))
            .await
            .expect("term_exited");
        if message["type"] == "term_exited" {
            break;
        }
    }
    assert!(
        gone("sleep 4242").await && gone("sleep 4243").await,
        "onglet fermé"
    );

    let id = open(&mut socket).await;
    type_line(
        &mut socket,
        id,
        "setsid sleep 4244 < /dev/null > /dev/null 2>&1 & echo PRET",
    )
    .await;
    output_until(&mut socket, id, "PRET").await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(running("sleep 4244"));
    drop(socket);
    assert!(gone("sleep 4244").await, "fenêtre fermée");
}

#[tokio::test]
async fn un_commit_dans_le_terminal_met_le_statut_git_a_jour() {
    if !bwrap_works() {
        return;
    }
    let project = tempfile::tempdir().unwrap();
    if !git(project.path(), &["init", "-q"]) {
        eprintln!("git indisponible : test ignoré");
        return;
    }
    fs::write(project.path().join("a.txt"), "a\n").unwrap();
    let home = tempfile::Builder::new().tempdir_in("/var/tmp").unwrap();
    let mut socket = start(
        project.path(),
        options(PathBuf::from("bwrap"), home.path(), Vec::new()),
    )
    .await;
    let id = open(&mut socket).await;
    type_line(
        &mut socket,
        id,
        "git add a.txt && git -c user.name=t -c user.email=t@t -c commit.gpgsign=false commit -qm a && echo COMMIT-OK",
    )
    .await;
    let (text, mut others) = output_until(&mut socket, id, "COMMIT-OK").await;
    assert!(text.contains("COMMIT-OK"), "{text}");
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if others
            .iter()
            .any(|m| m["type"] == "git_status" && m["payload"]["changes"] == 0)
        {
            break;
        }
        let wait = deadline.saturating_duration_since(Instant::now());
        others.push(
            next_within(&mut socket, wait)
                .await
                .expect("git_status après le commit"),
        );
    }
}
