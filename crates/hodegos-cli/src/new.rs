//! `hodegos new <chemin>` : crée le dossier d'un projet et sa vision, `docs/vision.md`,
//! à partir du gabarit intégré au binaire. Rien n'est écrit hors du projet.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::Context;
use hodegos_core::Project;

/// Vision du projet, relative à sa racine ; le Mentor produit la suit.
pub const VISION: &str = "docs/vision.md";

const TEMPLATE: &str = include_str!("../../../templates/vision.md");

/// Crée le projet ; `git init` sauf demande contraire, sans commit. Renvoie la racine.
pub fn create(path: &Path, git: bool) -> anyhow::Result<PathBuf> {
    let root = Project::create(path)?;
    let name = root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let today = jiff::Zoned::now().date().to_string();
    let vision = root.join(VISION);
    if let Some(docs) = vision.parent() {
        std::fs::create_dir_all(docs)
            .with_context(|| format!("impossible de créer {}", docs.display()))?;
    }
    std::fs::write(&vision, render(&name, &today))
        .with_context(|| format!("impossible d'écrire {}", vision.display()))?;
    println!("Projet créé : {}", root.display());
    println!("  {VISION} : à remplir, le Mentor produit vous pose les questions de cadrage.");
    if git {
        git_init(&root);
    }
    Ok(root)
}

/// Gabarit avec le nom du projet et la date du jour (AAAA-MM-JJ).
fn render(project_name: &str, date: &str) -> String {
    TEMPLATE
        .replace("{project_name}", project_name)
        .replace("{date}", date)
}

/// `git init` sans commit ; l'absence de git n'est qu'un avertissement.
fn git_init(root: &Path) {
    let status = Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    match status {
        Ok(status) if status.success() => println!("  dépôt git initialisé, sans commit."),
        Ok(status) => {
            eprintln!("avertissement : git init a échoué ({status}) ; dépôt non initialisé")
        }
        Err(err) => eprintln!("avertissement : git introuvable ({err}) ; dépôt non initialisé"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_gabarit_recoit_le_nom_et_la_date() {
        let text = render("essai", "2026-10-30");
        assert!(
            text.starts_with("# essai — vision\n\nDémarré le 2026-10-30\n"),
            "{text}"
        );
        assert!(!text.contains('{'), "{text}");
        for heading in [
            "Idée",
            "Pour qui",
            "Ce que je veux apprendre",
            "Contraintes",
            "Première version",
        ] {
            assert!(text.contains(&format!("\n## {heading}\n")), "{heading}");
        }
    }
}
