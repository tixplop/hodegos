#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::{BufRead, BufReader};
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

const BIN: &str = env!("CARGO_BIN_EXE_hodegos");

/// Arrête le processus même si le test échoue.
struct Running(Child);

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn affiche_l_url_d_acces_et_ecoute_dessus() {
    let project = tempfile::tempdir().unwrap();
    let mut child = Running(
        Command::new(BIN)
            .arg(project.path())
            .args(["--port", "0"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let stdout = child.0.stdout.take().unwrap();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if line.starts_with("http://") {
                let _ = tx.send(line);
                return;
            }
        }
    });
    let url = rx
        .recv_timeout(Duration::from_secs(20))
        .expect("aucune URL affichée");

    let rest = url.strip_prefix("http://127.0.0.1:").unwrap();
    let (port, token) = rest.split_once("/?token=").unwrap();
    let port: u16 = port.parse().unwrap();
    assert_eq!(token.len(), 64);
    assert!(token.bytes().all(|b| b.is_ascii_hexdigit()));
    TcpStream::connect(("127.0.0.1", port)).expect("le serveur n'écoute pas");
}

#[test]
fn refuse_un_port_occupe_avec_un_message_clair() {
    let busy = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = busy.local_addr().unwrap().port().to_string();
    let project = tempfile::tempdir().unwrap();
    let output = Command::new(BIN)
        .arg(project.path())
        .args(["--port", &port])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(&format!("le port {port} est déjà utilisé")),
        "{stderr}"
    );
    assert!(stderr.contains("--port"), "{stderr}");
}

#[test]
fn refuse_un_projet_sous_mnt_avec_un_message_clair() {
    let output = Command::new(BIN)
        .arg("/mnt/c/hodegos-inexistant")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("disque Windows (/mnt/)"), "{stderr}");
    assert!(stderr.contains("~/"), "{stderr}");
}

#[test]
fn refuse_un_dossier_inexistant() {
    let project = tempfile::tempdir().unwrap();
    let output = Command::new(BIN)
        .arg(project.path().join("absent"))
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("introuvable"), "{stderr}");
}
