//! Cas d'évaluation : un dossier par cas sous `evals/cases/`, avec le fichier source
//! à son chemin réel et `case.toml` (format dans `evals/README.md`).

use std::fs;
use std::path::{Path, PathBuf};

use hodegos_core::{Language, RelPath};
use hodegos_mentors::Level;
use serde::Deserialize;

#[derive(Debug, thiserror::Error)]
pub enum CaseError {
    #[error("{0} : dossier illisible : {1}")]
    Dir(PathBuf, std::io::Error),
    #[error("{0} : case.toml illisible : {1}")]
    Read(PathBuf, String),
    #[error("{0} : {1}")]
    Invalid(PathBuf, String),
    #[error("aucun cas dans {0}")]
    Empty(PathBuf),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CaseFile {
    title: String,
    file: String,
    section: String,
    expected_mentor: String,
    levels: Vec<String>,
    expect: Expect,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Expect {
    must_mention: Vec<String>,
    must_not: Vec<String>,
}

/// Cas prêt à jouer.
#[derive(Debug, Clone)]
pub struct Case {
    /// Nom du dossier, par exemple `01-dates-par-reference`.
    pub name: String,
    pub title: String,
    /// Chemin relatif du fichier, tel que le voit le casting.
    pub file: String,
    pub language: Language,
    pub content: String,
    pub section: String,
    pub expected_mentor: String,
    /// Paliers consécutifs, joués dans l'ordre comme « Palier suivant ».
    pub levels: Vec<Level>,
    pub must_mention: Vec<String>,
    pub must_not: Vec<String>,
}

/// Cas de `root/cases/` (ou de `root` s'il n'a pas de sous-dossier `cases`), par
/// ordre de nom ; `only` garde les cas dont le nom contient ce texte.
pub fn load_cases(root: &Path, only: Option<&str>) -> Result<Vec<Case>, CaseError> {
    let dir = if root.join("cases").is_dir() {
        root.join("cases")
    } else {
        root.to_path_buf()
    };
    let mut dirs: Vec<PathBuf> = fs::read_dir(&dir)
        .map_err(|e| CaseError::Dir(dir.clone(), e))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.join("case.toml").is_file())
        .collect();
    dirs.sort();
    let cases = dirs
        .iter()
        .filter(|path| only.is_none_or(|only| dir_name(path).contains(only)))
        .map(|path| load_case(path))
        .collect::<Result<Vec<_>, _>>()?;
    if cases.is_empty() {
        return Err(CaseError::Empty(dir));
    }
    Ok(cases)
}

pub fn load_case(dir: &Path) -> Result<Case, CaseError> {
    let invalid = |message: String| CaseError::Invalid(dir.to_path_buf(), message);
    let text = fs::read_to_string(dir.join("case.toml"))
        .map_err(|e| CaseError::Read(dir.to_path_buf(), e.to_string()))?;
    let file: CaseFile =
        toml::from_str(&text).map_err(|e| CaseError::Read(dir.to_path_buf(), e.to_string()))?;

    let rel = RelPath::parse(&file.file).map_err(|e| invalid(format!("file : {e}")))?;
    let content = fs::read_to_string(dir.join(rel.as_str()))
        .map_err(|e| invalid(format!("fichier {} illisible : {e}", rel)))?;
    let levels = file
        .levels
        .iter()
        .map(|name| {
            Level::from_name(name).ok_or_else(|| invalid(format!("palier inconnu : {name}")))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let Some(first) = levels.first() else {
        return Err(invalid("levels est vide".to_owned()));
    };
    // Les paliers se jouent comme « Palier suivant » : ils doivent se suivre.
    let mut expected = Some(*first);
    for level in &levels {
        if Some(*level) != expected {
            return Err(invalid(
                "levels doit suivre l'ordre indice, question, piste, exemple sans en sauter"
                    .to_owned(),
            ));
        }
        expected = level.next();
    }
    if file.expect.must_mention.is_empty() {
        return Err(invalid("expect.must_mention est vide".to_owned()));
    }
    if file.expect.must_not.is_empty() {
        return Err(invalid("expect.must_not est vide".to_owned()));
    }
    Ok(Case {
        name: dir_name(dir),
        title: file.title,
        language: Language::of(rel.as_str()),
        file: rel.to_string(),
        content,
        section: file.section,
        expected_mentor: file.expected_mentor,
        levels,
        must_mention: file.expect.must_mention,
        must_not: file.expect.must_not,
    })
}

fn dir_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_case(root: &Path, name: &str, toml: &str) -> PathBuf {
        let dir = root.join("cases").join(name);
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::write(dir.join("src/a.py"), "def a():\n    return 1\n").unwrap();
        fs::write(dir.join("case.toml"), toml).unwrap();
        dir
    }

    const VALID: &str = r#"
title = "Cas"
file = "src/a.py"
section = "fn:a"
expected_mentor = "mentor-langage"
levels = ["question", "piste"]

[expect]
must_mention = ["retour"]
must_not = ["return 2"]
"#;

    #[test]
    fn lit_un_cas_valide() {
        let root = tempfile::tempdir().unwrap();
        write_case(root.path(), "01-a", VALID);
        let cases = load_cases(root.path(), None).unwrap();
        assert_eq!(cases.len(), 1);
        let case = &cases[0];
        assert_eq!(case.name, "01-a");
        assert_eq!(case.language, Language::Python);
        assert_eq!(case.levels, [Level::Question, Level::Piste]);
        assert!(case.content.starts_with("def a()"));
    }

    #[test]
    fn filtre_par_nom() {
        let root = tempfile::tempdir().unwrap();
        write_case(root.path(), "01-a", VALID);
        write_case(root.path(), "02-b", VALID);
        let cases = load_cases(root.path(), Some("02")).unwrap();
        assert_eq!(cases.len(), 1);
        assert_eq!(cases[0].name, "02-b");
        assert!(matches!(
            load_cases(root.path(), Some("zz")),
            Err(CaseError::Empty(_))
        ));
    }

    #[test]
    fn refuse_un_cas_incomplet_ou_incoherent() {
        let root = tempfile::tempdir().unwrap();
        for (toml, message) in [
            (VALID.replace("title = \"Cas\"\n", ""), "missing field"),
            (VALID.replace("\"piste\"", "\"solution\""), "palier inconnu"),
            (
                VALID.replace("[\"question\", \"piste\"]", "[\"indice\", \"piste\"]"),
                "sans en sauter",
            ),
            (
                VALID.replace("[\"question\", \"piste\"]", "[]"),
                "levels est vide",
            ),
            (VALID.replace("[\"retour\"]", "[]"), "must_mention est vide"),
            (VALID.replace("src/a.py", "src/absent.py"), "illisible"),
            (VALID.replace("src/a.py", "../a.py"), "file"),
        ] {
            let dir = write_case(root.path(), "x", &toml);
            let err = load_case(&dir).unwrap_err().to_string();
            assert!(err.contains(message), "{message} : {err}");
        }
    }
}
