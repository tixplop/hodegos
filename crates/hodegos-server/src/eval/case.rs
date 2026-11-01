//! Cas d'évaluation : un dossier par cas sous `evals/cases/`, avec `case.toml` (format
//! dans `evals/README.md`). Un cas de passe porte son fichier source à son chemin
//! réel ; un cas de revue, de voisinage ou de réévaluation (phase 9f) désigne un
//! projet d'exemple de `evals/projects/`, et son dossier `after/` les fichiers qui le
//! modifient. Les cas et les projets du dépôt sont intégrés au binaire : l'IDE et
//! `hodegos eval` sans argument les jouent.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use hodegos_core::{Language, RelPath};
use hodegos_llm::About;
use hodegos_mentors::Level;
use rust_embed::Embed;
use serde::Deserialize;

/// Cas livrés, intégrés au binaire.
#[derive(Embed)]
#[folder = "../../evals/cases"]
struct Builtin;

/// Projets d'exemple livrés, intégrés au binaire.
#[derive(Embed)]
#[folder = "../../evals/projects"]
struct BuiltinProjects;

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

/// Fichiers d'un projet, par chemin relatif.
pub type Files = BTreeMap<String, String>;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CaseFile {
    title: String,
    #[serde(default)]
    kind: KindName,
    project: Option<String>,
    file: Option<String>,
    section: Option<String>,
    expected_mentor: Option<String>,
    mentor: Option<String>,
    levels: Option<Vec<String>>,
    level: Option<String>,
    about: Option<About>,
    message: Option<String>,
    #[serde(default)]
    quotes: Vec<QuoteFile>,
    verdict: Option<Verdict>,
    expect: Expect,
}

#[derive(Deserialize, Default, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum KindName {
    #[default]
    Pass,
    Review,
    Neighborhood,
    Reevaluation,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct QuoteFile {
    path: String,
    quote: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Expect {
    #[serde(default)]
    must_mention: Vec<String>,
    must_not: MustNot,
    #[serde(default)]
    cited_files: Vec<String>,
    about: Option<About>,
}

/// Motifs interdits : une liste, ou une liste par fichier (revue).
#[derive(Deserialize)]
#[serde(untagged)]
enum MustNot {
    All(Vec<String>),
    ByFile(BTreeMap<String, Vec<String>>),
}

/// Verdict attendu d'une réévaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Keep,
    Withdraw,
}

impl Verdict {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Keep => "keep",
            Self::Withdraw => "withdraw",
        }
    }
}

/// Projet d'exemple et fichiers qui le modifient.
#[derive(Debug, Clone)]
pub struct Sample {
    /// Nom du dossier sous `evals/projects/`.
    pub name: String,
    pub files: Arc<Files>,
    /// Fichiers de `after/`, qui remplacent ou s'ajoutent à ceux du projet.
    pub after: Files,
}

/// Ce qui distingue un cas d'un autre.
#[derive(Debug, Clone)]
pub enum Kind {
    /// Passe sur une section, puis « Palier suivant » (phases 6 et 8).
    Pass,
    /// Passe sur une section d'un projet, jouée avec le contexte `section` puis
    /// `neighborhood` du titulaire.
    Neighborhood(Sample),
    /// Revue d'un mentor de projet ; sans `after/`, première revue.
    Review(Box<ReviewCase>),
    /// Réévaluation d'une remarque de projet dont un passage a disparu.
    Reevaluation(Box<ReevaluationCase>),
}

#[derive(Debug, Clone)]
pub struct ReviewCase {
    pub sample: Sample,
    /// La bonne remarque cite au moins un de ces fichiers (aucun : pas d'attente).
    pub cited_files: Vec<String>,
    pub about: Option<About>,
    /// Motifs interdits par fichier ; cherchés dans tous les messages.
    pub must_not_by_file: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct ReevaluationCase {
    pub sample: Sample,
    pub about: About,
    /// Remarque d'origine.
    pub message: String,
    /// Passages cités : (fichier, citation).
    pub quotes: Vec<(String, String)>,
    pub verdict: Verdict,
}

/// Cas prêt à jouer.
#[derive(Debug, Clone)]
pub struct Case {
    /// Nom du dossier, par exemple `01-dates-par-reference`.
    pub name: String,
    pub title: String,
    /// Chemin relatif du fichier, tel que le voit le casting (passe et voisinage ;
    /// vide sinon).
    pub file: String,
    pub language: Language,
    pub content: String,
    pub section: String,
    /// Titulaire attendu du casting ; pour une revue ou une réévaluation, le mentor joué.
    pub expected_mentor: String,
    /// Paliers consécutifs, joués dans l'ordre comme « Palier suivant » ; un seul
    /// pour le voisinage et la réévaluation, aucun pour une revue (palier d'entrée
    /// du mentor).
    pub levels: Vec<Level>,
    pub must_mention: Vec<String>,
    /// Tous les motifs interdits (ceux de chaque fichier pour une revue).
    pub must_not: Vec<String>,
    pub kind: Kind,
}

impl Case {
    pub fn kind_name(&self) -> &'static str {
        match self.kind {
            Kind::Pass => "pass",
            Kind::Neighborhood(_) => "neighborhood",
            Kind::Review(_) => "review",
            Kind::Reevaluation(_) => "reevaluation",
        }
    }

    /// Appels au modèle quand tout se passe bien.
    pub fn calls(&self) -> usize {
        match self.kind {
            Kind::Pass => self.levels.len(),
            Kind::Neighborhood(_) => 2,
            Kind::Review(_) | Kind::Reevaluation(_) => 1,
        }
    }
}

/// Cas de `root/cases/` (ou de `root` s'il n'a pas de sous-dossier `cases`), par
/// ordre de nom ; `only` garde les cas dont le nom contient ce texte. Les projets
/// d'exemple sont dans `projects/`, à côté de `cases/`.
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
    let mut projects = HashMap::new();
    let cases = dirs
        .iter()
        .filter(|path| only.is_none_or(|only| dir_name(path).contains(only)))
        .map(|path| load_case_with(path, &mut projects))
        .collect::<Result<Vec<_>, _>>()?;
    if cases.is_empty() {
        return Err(CaseError::Empty(dir));
    }
    Ok(cases)
}

/// Cas intégrés au binaire, par ordre de nom ; `only` comme pour [`load_cases`].
pub fn builtin_cases(only: Option<&str>) -> Result<Vec<Case>, CaseError> {
    let names: BTreeSet<String> = Builtin::iter()
        .filter_map(|path| path.strip_suffix("/case.toml").map(str::to_owned))
        .filter(|name| only.is_none_or(|only| name.contains(only)))
        .collect();
    let mut projects: HashMap<String, Arc<Files>> = HashMap::new();
    let cases = names
        .iter()
        .map(|name| {
            let origin = Path::new("cas intégrés").join(name);
            let read = |rel: &str| {
                let file =
                    Builtin::get(&format!("{name}/{rel}")).ok_or_else(|| "absent".to_owned())?;
                String::from_utf8(file.data.into_owned()).map_err(|e| e.to_string())
            };
            let after = || {
                let prefix = format!("{name}/after/");
                embedded_files(Builtin::iter(), &prefix, |p| {
                    Builtin::get(p).map(|f| f.data)
                })
            };
            let project = |project: &str| -> Result<Arc<Files>, String> {
                if let Some(files) = projects.get(project) {
                    return Ok(files.clone());
                }
                let files = embedded_files(BuiltinProjects::iter(), &format!("{project}/"), |p| {
                    BuiltinProjects::get(p).map(|f| f.data)
                })?;
                if files.is_empty() {
                    return Err(format!("projet d'exemple « {project} » introuvable"));
                }
                let files = Arc::new(files);
                projects.insert(project.to_owned(), files.clone());
                Ok(files)
            };
            parse_case(name, &origin, read, after, project)
        })
        .collect::<Result<Vec<_>, _>>()?;
    if cases.is_empty() {
        return Err(CaseError::Empty(PathBuf::from("cas intégrés")));
    }
    Ok(cases)
}

/// Fichiers intégrés sous `prefix`, par chemin relatif à `prefix`.
fn embedded_files<I, D>(
    paths: I,
    prefix: &str,
    get: impl Fn(&str) -> Option<D>,
) -> Result<Files, String>
where
    I: Iterator<Item = std::borrow::Cow<'static, str>>,
    D: Into<Vec<u8>>,
{
    let mut files = Files::new();
    for path in paths {
        let Some(rel) = path.strip_prefix(prefix) else {
            continue;
        };
        let data = get(&path).ok_or_else(|| format!("{path} absent"))?;
        let text = String::from_utf8(data.into()).map_err(|e| format!("{path} : {e}"))?;
        files.insert(rel.to_owned(), text);
    }
    Ok(files)
}

pub fn load_case(dir: &Path) -> Result<Case, CaseError> {
    load_case_with(dir, &mut HashMap::new())
}

fn load_case_with(
    dir: &Path,
    projects: &mut HashMap<String, Arc<Files>>,
) -> Result<Case, CaseError> {
    // `evals/cases/<cas>` → `evals/projects/<projet>`.
    let projects_dir = dir
        .parent()
        .and_then(Path::parent)
        .map(|evals| evals.join("projects"));
    parse_case(
        &dir_name(dir),
        dir,
        |rel| fs::read_to_string(dir.join(rel)).map_err(|e| e.to_string()),
        || read_tree(&dir.join("after")),
        |name| {
            if let Some(files) = projects.get(name) {
                return Ok(files.clone());
            }
            let Some(root) = &projects_dir else {
                return Err(format!("projet d'exemple « {name} » introuvable"));
            };
            let files = read_tree(&root.join(name))?;
            if files.is_empty() {
                return Err(format!(
                    "projet d'exemple « {name} » introuvable dans {}",
                    root.display()
                ));
            }
            let files = Arc::new(files);
            projects.insert(name.to_owned(), files.clone());
            Ok(files)
        },
    )
}

/// Fichiers texte sous `root`, par chemin relatif ; vide si le dossier n'existe pas.
fn read_tree(root: &Path) -> Result<Files, String> {
    let mut files = Files::new();
    if !root.is_dir() {
        return Ok(files);
    }
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = fs::read_dir(&dir).map_err(|e| format!("{} : {e}", dir.display()))?;
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let rel = path
                .strip_prefix(root)
                .map_err(|e| e.to_string())?
                .to_string_lossy()
                .replace('\\', "/");
            let text =
                fs::read_to_string(&path).map_err(|e| format!("{} : {e}", path.display()))?;
            files.insert(rel, text);
        }
    }
    Ok(files)
}

/// Lit le cas `name` ; `read` donne un fichier du cas par son chemin relatif, `after`
/// les fichiers de son dossier `after/`, `project` les fichiers d'un projet d'exemple ;
/// `origin` désigne le cas dans les erreurs.
fn parse_case(
    name: &str,
    origin: &Path,
    read: impl Fn(&str) -> Result<String, String>,
    after: impl FnOnce() -> Result<Files, String>,
    project: impl FnOnce(&str) -> Result<Arc<Files>, String>,
) -> Result<Case, CaseError> {
    let invalid = |message: String| CaseError::Invalid(origin.to_path_buf(), message);
    let text = read("case.toml").map_err(|e| CaseError::Read(origin.to_path_buf(), e))?;
    let file: CaseFile =
        toml::from_str(&text).map_err(|e| CaseError::Read(origin.to_path_buf(), e.to_string()))?;
    let required = |value: Option<String>, field: &str| {
        value.ok_or_else(|| invalid(format!("{field} manque")))
    };
    let level_of = |name: &str| {
        Level::from_name(name).ok_or_else(|| invalid(format!("palier inconnu : {name}")))
    };
    let (must_not, must_not_by_file) = match file.expect.must_not {
        MustNot::All(all) => (all, BTreeMap::new()),
        MustNot::ByFile(by_file) => (by_file.values().flatten().cloned().collect(), by_file),
    };
    if must_not.is_empty() && file.kind != KindName::Reevaluation {
        return Err(invalid("expect.must_not est vide".to_owned()));
    }
    if file.expect.must_mention.is_empty() && file.kind != KindName::Reevaluation {
        return Err(invalid("expect.must_mention est vide".to_owned()));
    }
    if !must_not_by_file.is_empty() && file.kind != KindName::Review {
        return Err(invalid(
            "expect.must_not par fichier : seulement pour une revue".to_owned(),
        ));
    }
    let mut case = Case {
        name: name.to_owned(),
        title: file.title,
        file: String::new(),
        language: Language::Markdown,
        content: String::new(),
        section: String::new(),
        expected_mentor: String::new(),
        levels: Vec::new(),
        must_mention: file.expect.must_mention,
        must_not,
        kind: Kind::Pass,
    };
    let sample = |project: Result<Arc<Files>, String>, after: Files| {
        Ok::<_, CaseError>(Sample {
            name: String::new(),
            files: project.map_err(invalid)?,
            after,
        })
    };

    match file.kind {
        KindName::Pass | KindName::Neighborhood => {
            let rel = RelPath::parse(&required(file.file, "file")?)
                .map_err(|e| invalid(format!("file : {e}")))?;
            case.file = rel.to_string();
            case.language = Language::of(rel.as_str());
            case.section = required(file.section, "section")?;
            case.expected_mentor = required(file.expected_mentor, "expected_mentor")?;
            if file.kind == KindName::Pass {
                case.content = read(rel.as_str())
                    .map_err(|e| invalid(format!("fichier {rel} illisible : {e}")))?;
                case.levels = file
                    .levels
                    .unwrap_or_default()
                    .iter()
                    .map(|name| level_of(name))
                    .collect::<Result<Vec<_>, _>>()?;
                check_levels(&case.levels).map_err(|m| invalid(m.to_owned()))?;
            } else {
                let name = required(file.project, "project")?;
                let mut sample = sample(project(&name), Files::new())?;
                sample.name = name;
                case.content =
                    sample.files.get(rel.as_str()).cloned().ok_or_else(|| {
                        invalid(format!("{rel} absent du projet {}", sample.name))
                    })?;
                case.levels = vec![level_of(&required(file.level, "level")?)?];
                case.kind = Kind::Neighborhood(sample);
            }
        }
        KindName::Review => {
            let name = required(file.project, "project")?;
            let mut sample = sample(project(&name), after().map_err(invalid)?)?;
            sample.name = name;
            case.expected_mentor = required(file.mentor, "mentor")?;
            case.kind = Kind::Review(Box::new(ReviewCase {
                sample,
                cited_files: file.expect.cited_files,
                about: file.expect.about,
                must_not_by_file,
            }));
        }
        KindName::Reevaluation => {
            let name = required(file.project, "project")?;
            let after = after().map_err(invalid)?;
            if after.is_empty() {
                return Err(invalid(
                    "une réévaluation a besoin d'un dossier after/".to_owned(),
                ));
            }
            let mut sample = sample(project(&name), after)?;
            sample.name = name;
            case.expected_mentor = required(file.mentor, "mentor")?;
            case.levels = vec![level_of(&required(file.level, "level")?)?];
            if file.quotes.is_empty() {
                return Err(invalid("quotes est vide".to_owned()));
            }
            case.kind = Kind::Reevaluation(Box::new(ReevaluationCase {
                sample,
                about: file.about.unwrap_or(About::Project),
                message: required(file.message, "message")?,
                quotes: file.quotes.into_iter().map(|q| (q.path, q.quote)).collect(),
                verdict: file
                    .verdict
                    .ok_or_else(|| invalid("verdict manque".to_owned()))?,
            }));
        }
    }
    Ok(case)
}

/// Les paliers se jouent comme « Palier suivant » : ils doivent se suivre.
fn check_levels(levels: &[Level]) -> Result<(), &'static str> {
    let Some(first) = levels.first() else {
        return Err("levels est vide");
    };
    let mut expected = Some(*first);
    for level in levels {
        if Some(*level) != expected {
            return Err(
                "levels doit suivre l'ordre indice, question, piste, exemple sans en sauter",
            );
        }
        expected = level.next();
    }
    Ok(())
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
