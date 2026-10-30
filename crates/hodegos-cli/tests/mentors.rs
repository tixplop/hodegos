#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

const BIN: &str = env!("CARGO_BIN_EXE_hodegos");

/// Dossier de configuration et projet temporaires, jamais ceux de l'utilisateur.
struct Sandbox {
    _dir: tempfile::TempDir,
    config: std::path::PathBuf,
    project: std::path::PathBuf,
}

impl Sandbox {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("config");
        let project = dir.path().join("projet");
        fs::create_dir_all(&project).unwrap();
        Self {
            _dir: dir,
            config,
            project,
        }
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(BIN)
            .args(args)
            .current_dir(&self.project)
            .env("XDG_CONFIG_HOME", &self.config)
            .env("XDG_DATA_HOME", self.config.join("donnees"))
            .output()
            .unwrap()
    }

    fn user_dir(&self) -> std::path::PathBuf {
        self.config.join("hodegos/mentors")
    }
}

fn stdout(output: &Output) -> String {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn line_of<'a>(text: &'a str, id: &str) -> &'a str {
    text.lines()
        .find(|l| l.trim_start().starts_with(&format!("{id} ")))
        .unwrap_or_else(|| panic!("{id} absent :\n{text}"))
}

#[test]
fn list_affiche_les_mentors_livres_avec_leur_origine() {
    let sandbox = Sandbox::new();
    let text = stdout(&sandbox.run(&["mentors", "list"]));
    assert!(text.contains("Mentors actifs (3)"), "{text}");
    for id in ["architecte", "mentor-langage", "mentor-produit"] {
        let line = line_of(&text, id);
        assert!(
            line.contains("livré") && line.contains("proactive"),
            "{line}"
        );
    }
    assert!(!text.contains("en erreur"), "{text}");
}

#[test]
fn list_signale_les_fichiers_en_erreur_et_les_desactivations() {
    let sandbox = Sandbox::new();
    fs::create_dir_all(sandbox.user_dir()).unwrap();
    fs::write(sandbox.user_dir().join("casse.md"), "---\nid: casse\n---\n").unwrap();
    let project_dir = sandbox.project.join(".hodegos/mentors");
    fs::create_dir_all(&project_dir).unwrap();
    fs::write(
        project_dir.join("off.md"),
        "---\nid: mentor-langage\nenabled: false\n---\n",
    )
    .unwrap();

    let text = stdout(&sandbox.run(&["mentors", "list"]));
    assert!(text.contains("Mentors actifs (2)"), "{text}");
    assert!(text.contains("Mentors désactivés"), "{text}");
    assert!(
        line_of(&text, "mentor-langage").contains("projet"),
        "{text}"
    );
    assert!(text.contains("Fichiers en erreur"), "{text}");
    assert!(text.contains("casse.md : champ name manquant"), "{text}");
}

#[test]
fn eject_copie_le_mentor_livre_puis_refuse_d_ecraser() {
    let sandbox = Sandbox::new();
    let text = stdout(&sandbox.run(&["mentors", "eject", "architecte"]));
    let copy = sandbox.user_dir().join("architecte.md");
    assert!(text.contains(&copy.display().to_string()), "{text}");
    assert_eq!(
        fs::read_to_string(&copy).unwrap(),
        include_str!("../../../mentors/architecte.md")
    );
    assert!(
        line_of(&stdout(&sandbox.run(&["mentors", "list"])), "architecte").contains("personnel")
    );

    let again = sandbox.run(&["mentors", "eject", "architecte"]);
    assert!(!again.status.success());
    assert!(String::from_utf8_lossy(&again.stderr).contains("existe déjà"));
}

#[test]
fn eject_project_copie_dans_le_projet() {
    let sandbox = Sandbox::new();
    stdout(&sandbox.run(&["mentors", "eject", "mentor-langage", "--project"]));
    assert!(Path::new(&sandbox.project.join(".hodegos/mentors/mentor-langage.md")).is_file());
    assert!(!sandbox.user_dir().join("mentor-langage.md").exists());
    let text = stdout(&sandbox.run(&["mentors", "list"]));
    assert!(
        line_of(&text, "mentor-langage").contains("projet"),
        "{text}"
    );
}

#[test]
fn eject_refuse_un_mentor_inconnu() {
    let sandbox = Sandbox::new();
    let output = sandbox.run(&["mentors", "eject", "inconnu"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("aucun mentor actif"));
}
