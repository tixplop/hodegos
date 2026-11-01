//! Matière d'une revue du projet (phase 9c, ADR 0010), calculée sans modèle : fichiers
//! clés d'un mentor sous leur plafond, textes masqués pour l'instantané d'une revue, et
//! diff depuis la précédente. Mêmes exclusions et même masquage que la carte.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::time::SystemTime;

use similar::TextDiff;

use super::extract::FileFacts;
use super::{MapCache, estimate_tokens};
use crate::secrets::mask_secrets;
use crate::sections::sections;
use crate::{Language, Project, RelPath, Session, strip_html_comments};

/// Plafond par défaut des fichiers clés, en jetons estimés.
pub const DEFAULT_KEY_FILES_CAP: usize = 12_000;
/// Fichiers au plus dans le diff d'une revue.
pub const MAX_DIFF_FILES: usize = 20;

/// Fichier clé tel qu'envoyé : texte déjà masqué.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyFile {
    pub path: String,
    /// « entier », « signatures » ou « titres ».
    pub form: String,
    pub text: String,
    pub tokens: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KeyFiles {
    pub files: Vec<KeyFile>,
    pub tokens: usize,
    /// Fichiers clés laissés faute de place, même réduits.
    pub skipped: usize,
}

/// Fichiers clés (`is_key`) parmi ceux de la carte, en entier tant qu'ils tiennent sous
/// `cap` ; au-delà, les moins récemment modifiés sont réduits à leurs signatures ou à
/// leurs titres, ou laissés. Rangés par chemin.
pub fn key_files(
    project: &Project,
    session: &Session,
    cache: &MapCache,
    is_key: &dyn Fn(&str) -> bool,
    cap: usize,
) -> KeyFiles {
    let mut candidates: Vec<(SystemTime, &str, &FileFacts)> = cache
        .facts()
        .filter(|(path, _)| is_key(path))
        .map(|(path, facts)| {
            let modified = fs::metadata(project.root().join(path))
                .and_then(|m| m.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);
            (modified, path, facts)
        })
        .collect();
    // Les plus récemment modifiés d'abord : ils gardent leur texte entier.
    candidates.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(b.1)));
    let mut out = KeyFiles::default();
    for (_, path, facts) in candidates {
        let Some(content) = read(project, session, path) else {
            continue;
        };
        let whole = masked(path, &content);
        let mut options = vec![("entier".to_owned(), whole)];
        if let Some(reduced) = reduce(path, &content, facts) {
            options.push(reduced);
        }
        let fitting = options.into_iter().find_map(|(form, text)| {
            let tokens = estimate_tokens(&text) + estimate_tokens(path) + 8;
            (out.tokens + tokens <= cap).then_some(KeyFile {
                path: path.to_owned(),
                form,
                text,
                tokens,
            })
        });
        match fitting {
            Some(file) => {
                out.tokens += file.tokens;
                out.files.push(file);
            }
            None => out.skipped += 1,
        }
    }
    out.files.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

/// Texte envoyé d'un fichier : secrets masqués, commentaires HTML retirés en Markdown.
fn masked(path: &str, content: &str) -> String {
    let text = if Language::of(path) == Language::Markdown {
        strip_html_comments(content)
    } else {
        content.to_owned()
    };
    mask_secrets(&text).0
}

/// Code : signatures ; document : titres.
fn reduce(path: &str, content: &str, facts: &FileFacts) -> Option<(String, String)> {
    if Language::of(path) == Language::Markdown {
        let titles: Vec<String> = sections(Language::Markdown, &strip_html_comments(content))
            .iter()
            .filter(|s| s.kind == "h")
            .map(|s| format!("{}- {}", "  ".repeat(s.parents.len()), s.name))
            .collect();
        return (!titles.is_empty())
            .then(|| ("titres".to_owned(), mask_secrets(&titles.join("\n")).0));
    }
    (!facts.symbols.is_empty()).then(|| {
        (
            "signatures".to_owned(),
            mask_secrets(&facts.symbols.join("\n")).0,
        )
    })
}

fn read(project: &Project, session: &Session, path: &str) -> Option<String> {
    let rel = RelPath::parse(path).ok()?;
    project.read_file(session, &rel).ok().map(|f| f.content)
}

/// Textes masqués de tous les fichiers de la carte : l'instantané d'une revue.
pub fn masked_texts(
    project: &Project,
    session: &Session,
    cache: &MapCache,
) -> BTreeMap<String, String> {
    cache
        .paths()
        .filter_map(|path| {
            Some((
                path.to_owned(),
                masked(path, &read(project, session, path)?),
            ))
        })
        .collect()
}

/// Diff d'un fichier depuis la revue précédente.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileChange {
    pub path: String,
    /// Diff unifié, deux lignes de contexte.
    pub diff: String,
    /// Lignes ajoutées ou retirées.
    pub changed: usize,
}

/// Fichiers changés entre deux instantanés, les plus modifiés d'abord, `max` au plus,
/// puis rangés par chemin.
pub fn diff_since(
    before: &BTreeMap<String, String>,
    after: &BTreeMap<String, String>,
    max: usize,
) -> Vec<FileChange> {
    let paths: BTreeSet<&String> = before.keys().chain(after.keys()).collect();
    let mut changes: Vec<FileChange> = paths
        .into_iter()
        .filter_map(|path| {
            let old = before.get(path).map_or("", String::as_str);
            let new = after.get(path).map_or("", String::as_str);
            if old == new {
                return None;
            }
            let diff = TextDiff::from_lines(old, new);
            let changed = diff
                .iter_all_changes()
                .filter(|c| c.tag() != similar::ChangeTag::Equal)
                .count();
            let text = diff
                .unified_diff()
                .context_radius(2)
                .header(
                    if before.contains_key(path) {
                        path
                    } else {
                        "/dev/null"
                    },
                    if after.contains_key(path) {
                        path
                    } else {
                        "/dev/null"
                    },
                )
                .to_string();
            Some(FileChange {
                path: path.clone(),
                diff: text,
                changed,
            })
        })
        .collect();
    changes.sort_by(|a, b| b.changed.cmp(&a.changed).then_with(|| a.path.cmp(&b.path)));
    changes.truncate(max);
    changes.sort_by(|a, b| a.path.cmp(&b.path));
    changes
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::time::Duration;

    use super::*;
    use crate::UserId;
    use crate::map::Exclusions;

    fn write(root: &Path, path: &str, content: &str) {
        let path = root.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    fn setup() -> (tempfile::TempDir, Project, Session, MapCache) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(root, ".gitignore", "brouillons/\n");
        write(
            root,
            "docs/adr/0001.md",
            "# Stockage\n\n## Décision\n\nUn seul module.\n<!-- note -->\n",
        );
        write(root, "docs/brouillons/x.md", "# x\n");
        write(root, "brouillons/adr/0002.md", "# ignoré\n");
        write(root, "docs/adr/.env", "TOKEN=abcdefgh123\n");
        write(
            root,
            "src/depot.ts",
            "export function lister(): Emprunt[] {\n  return JSON.parse(lire());\n}\nconst password = \"tres-secret-1\";\n",
        );
        let project = Project::open(root).unwrap();
        let session = project.session(UserId::local());
        let mut cache = MapCache::default();
        cache
            .refresh(&project, &session, &Exclusions::default())
            .unwrap();
        (dir, project, session, cache)
    }

    #[test]
    fn fichiers_cles_entiers_puis_reduits_sous_le_plafond() {
        let (dir, project, session, cache) = setup();
        let is_key =
            |p: &str| p.starts_with("docs/adr/") || p.ends_with(".ts") || p.contains("adr/");
        let all = key_files(&project, &session, &cache, &is_key, DEFAULT_KEY_FILES_CAP);
        let paths: Vec<(&str, &str)> = all
            .files
            .iter()
            .map(|f| (f.path.as_str(), f.form.as_str()))
            .collect();
        assert_eq!(
            paths,
            vec![("docs/adr/0001.md", "entier"), ("src/depot.ts", "entier")]
        );
        let depot = &all.files[1];
        assert!(
            depot.text.contains("JSON.parse"),
            "un fichier clé part en entier"
        );
        assert!(depot.text.contains("[secret masqué]") && !depot.text.contains("tres-secret-1"));
        assert!(!all.files[0].text.contains("note"), "commentaires retirés");

        // Plafond serré : le plus ancien est réduit à ses signatures.
        let old = std::time::SystemTime::now() - Duration::from_secs(3600);
        fs::File::options()
            .write(true)
            .open(dir.path().join("src/depot.ts"))
            .unwrap()
            .set_modified(old)
            .unwrap();
        let whole_adr = all.files[0].tokens;
        let tight = key_files(&project, &session, &cache, &is_key, whole_adr + 20);
        let forms: Vec<(&str, &str)> = tight
            .files
            .iter()
            .map(|f| (f.path.as_str(), f.form.as_str()))
            .collect();
        assert_eq!(
            forms,
            vec![
                ("docs/adr/0001.md", "entier"),
                ("src/depot.ts", "signatures")
            ]
        );
        assert_eq!(tight.files[1].text, "export function lister(): Emprunt[]");
        assert!(tight.tokens <= whole_adr + 20);
        let none = key_files(&project, &session, &cache, &is_key, 5);
        assert_eq!(none.skipped, 2);
    }

    #[test]
    fn diff_depuis_la_revue_precedente() {
        let (dir, project, session, cache) = setup();
        let before = masked_texts(&project, &session, &cache);
        assert!(before.contains_key("src/depot.ts"));
        assert!(
            !before
                .keys()
                .any(|p| p.contains("brouillons/adr") || p.ends_with(".env"))
        );
        assert!(!before["src/depot.ts"].contains("tres-secret-1"));
        assert!(diff_since(&before, &before, MAX_DIFF_FILES).is_empty());

        let mut cache = cache;
        write(
            dir.path(),
            "src/depot.ts",
            "export function lister(): Emprunt[] {\n  return [];\n}\n",
        );
        write(dir.path(), "src/neuf.ts", "export const n = 1;\n");
        cache
            .refresh(&project, &session, &Exclusions::default())
            .unwrap();
        let after = masked_texts(&project, &session, &cache);
        let changes = diff_since(&before, &after, MAX_DIFF_FILES);
        let paths: Vec<&str> = changes.iter().map(|c| c.path.as_str()).collect();
        assert_eq!(paths, vec!["src/depot.ts", "src/neuf.ts"]);
        assert!(changes[0].diff.contains("-  return JSON.parse(lire());"));
        assert!(changes[0].diff.contains("+  return [];"));
        assert!(changes[1].diff.contains("--- /dev/null"));
        let one = diff_since(&before, &after, 1);
        assert_eq!(one[0].path, "src/depot.ts", "le plus modifié");
    }
}
