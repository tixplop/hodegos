#![allow(clippy::unwrap_used, clippy::expect_used)]

//! `hodegos new` : création du projet et de sa vision, sans lancer l'IDE
//! (`--no-open`), avec des dossiers de configuration et de données temporaires.

use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::mpsc;
use std::time::Duration;

const BIN: &str = env!("CARGO_BIN_EXE_hodegos");

struct Sandbox {
    dir: tempfile::TempDir,
}

impl Sandbox {
    fn new() -> Self {
        Self {
            dir: tempfile::tempdir().unwrap(),
        }
    }

    fn path(&self, rel: &str) -> PathBuf {
        self.dir.path().join(rel)
    }

    fn command(&self, args: &[&str]) -> Command {
        let mut command = Command::new(BIN);
        command
            .args(args)
            .env("XDG_CONFIG_HOME", self.path("config"))
            .env("XDG_DATA_HOME", self.path("donnees"))
            .env_remove("OPENROUTER_API_KEY");
        command
    }

    fn new_project(&self, extra: &[&str]) -> Output {
        let target = self.path("essai");
        let mut args = vec!["new", target.to_str().unwrap(), "--no-open"];
        args.extend_from_slice(extra);
        self.command(&args).output().unwrap()
    }

    /// Rien n'est écrit dans ~/.config ni ~/.local/share par la création.
    fn nothing_outside(&self) {
        assert!(!self.path("config").exists());
        assert!(!self.path("donnees").exists());
    }
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

fn vision(root: &Path) -> String {
    fs::read_to_string(root.join("docs/vision.md")).unwrap()
}

#[test]
fn cree_le_projet_et_sa_vision_avec_le_nom_et_la_date() {
    let sandbox = Sandbox::new();
    let output = sandbox.new_project(&["--no-git"]);
    assert!(output.status.success(), "{}", stderr(&output));
    let root = sandbox.path("essai");
    let text = vision(&root);
    let today = jiff::Zoned::now().date().to_string();
    assert!(
        text.starts_with(&format!("# essai — vision\n\nDémarré le {today}\n")),
        "{text}"
    );
    assert!(!text.contains("{project_name}") && !text.contains("{date}"));
    assert!(text.contains("## Première version"));
    assert!(!root.join(".git").exists(), "--no-git");
    assert_eq!(fs::read_dir(&root).unwrap().count(), 1, "seulement docs/");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("docs/vision.md"), "{stdout}");
    sandbox.nothing_outside();
}

#[test]
fn git_init_sans_commit() {
    if !git_available() {
        eprintln!("git absent : test ignoré");
        return;
    }
    let sandbox = Sandbox::new();
    let output = sandbox.new_project(&[]);
    assert!(output.status.success(), "{}", stderr(&output));
    let root = sandbox.path("essai");
    assert!(root.join(".git").is_dir());
    let log = Command::new("git")
        .args(["rev-parse", "--verify", "HEAD"])
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(!log.status.success(), "aucun commit");
    sandbox.nothing_outside();
}

#[test]
fn sans_git_un_avertissement_et_pas_d_echec() {
    let sandbox = Sandbox::new();
    let target = sandbox.path("essai");
    let output = sandbox
        .command(&["new", target.to_str().unwrap(), "--no-open"])
        .env("PATH", sandbox.path("vide"))
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        stderr(&output).contains("git introuvable"),
        "{}",
        stderr(&output)
    );
    assert!(target.join("docs/vision.md").is_file());
    assert!(!target.join(".git").exists());
}

#[test]
fn accepte_un_dossier_vide_et_refuse_un_dossier_non_vide() {
    let sandbox = Sandbox::new();
    let root = sandbox.path("essai");
    fs::create_dir_all(&root).unwrap();
    let output = sandbox.new_project(&["--no-git"]);
    assert!(output.status.success(), "{}", stderr(&output));
    let before = vision(&root);

    fs::write(root.join("docs/vision.md"), "ma vision").unwrap();
    let output = sandbox.new_project(&["--no-git"]);
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("n'est pas vide"),
        "{}",
        stderr(&output)
    );
    assert_eq!(vision(&root), "ma vision", "rien n'est écrasé");
    assert!(before.contains("## Idée"));
}

#[test]
fn refuse_un_chemin_sous_mnt_avec_le_message_du_lancement() {
    let output = Command::new(BIN)
        .args(["new", "/mnt/c/hodegos-inexistant/essai", "--no-open"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = stderr(&output);
    assert!(stderr.contains("disque Windows (/mnt/)"), "{stderr}");
    assert!(stderr.contains("~/"), "{stderr}");
    assert!(!Path::new("/mnt/c/hodegos-inexistant").exists());
}

/// Sans `--no-open`, l'IDE démarre sur le projet créé, comme `hodegos <chemin>`.
#[test]
fn sans_no_open_l_ide_demarre_sur_le_projet() {
    let sandbox = Sandbox::new();
    let target = sandbox.path("essai");
    let mut child = sandbox
        .command(&["new", target.to_str().unwrap(), "--no-git", "--port", "0"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let stdout = child.stdout.take().unwrap();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if line.starts_with("http://") {
                let _ = tx.send(line);
                return;
            }
        }
    });
    let url = rx.recv_timeout(Duration::from_secs(20));
    let _ = child.kill();
    let _ = child.wait();
    assert!(
        url.expect("aucune URL affichée")
            .starts_with("http://127.0.0.1:")
    );
    assert!(target.join("docs/vision.md").is_file());
}
