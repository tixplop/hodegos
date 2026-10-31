//! Statut git du projet (phase 8a). Lancé avec le `git` de l'utilisateur, donc sa
//! configuration, et `GIT_OPTIONAL_LOCKS=0` : Hodegos ne prend jamais `index.lock`.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use notify_debouncer_full::notify::{EventKind, RecommendedWatcher, RecursiveMode};
use notify_debouncer_full::{DebounceEventResult, Debouncer, RecommendedCache, new_debouncer};

use crate::{FileError, Project, RelPath, Session};

/// Statut d'un fichier, tel que l'arbre l'affiche.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitFileStatus {
    /// `M` : modifié dans l'index ou l'arbre de travail.
    Modified,
    /// `A` : ajouté à l'index (ou copié).
    Added,
    /// `U` : non suivi.
    Untracked,
    /// `C` : en conflit.
    Conflicted,
    /// `R` : renommé.
    Renamed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitEntry {
    pub path: RelPath,
    pub status: GitFileStatus,
}

/// Statut du dépôt, ramené à la racine du projet.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GitStatus {
    /// `None` si `HEAD` est détachée.
    pub branch: Option<String>,
    pub detached: bool,
    /// Fichiers présents avec un statut ; un fichier suivi supprimé n'y est pas.
    pub entries: Vec<GitEntry>,
    /// Tous les changements du projet, fichiers supprimés compris.
    pub changes: usize,
    pub conflicts: usize,
}

/// Dépôt qui contient le projet : la racine du projet peut en être un sous-dossier,
/// ou un worktree (`.git` est alors un fichier).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitRepo {
    /// Dossier git propre à l'arbre de travail : `HEAD`, `index`.
    pub git_dir: PathBuf,
    /// Dossier git commun : `refs/`, `packed-refs`.
    pub common_dir: PathBuf,
    /// Chemin de la racine du projet dans le dépôt, terminé par `/` ; vide à la racine.
    prefix: String,
}

impl Project {
    /// Dépôt du projet ; `None` sans dépôt, ou si `git` est absent.
    pub fn git_repo(&self, session: &Session) -> Result<Option<GitRepo>, FileError> {
        self.check(session)?;
        let output = git(self.root())
            .args([
                "rev-parse",
                "--path-format=absolute",
                "--git-dir",
                "--git-common-dir",
                "--show-prefix",
            ])
            .output();
        let output = match output {
            Ok(output) if output.status.success() => output,
            Ok(_) => return Ok(None),
            Err(err) => {
                tracing::debug!(error = %err, "git introuvable");
                return Ok(None);
            }
        };
        let text = String::from_utf8_lossy(&output.stdout);
        let mut lines = text.lines();
        let (Some(git_dir), Some(common_dir)) = (lines.next(), lines.next()) else {
            return Ok(None);
        };
        Ok(Some(GitRepo {
            git_dir: PathBuf::from(git_dir),
            common_dir: PathBuf::from(common_dir),
            prefix: lines.next().unwrap_or_default().to_owned(),
        }))
    }

    /// Statut du projet ; `None` si `git status` échoue.
    pub fn git_status(
        &self,
        session: &Session,
        repo: &GitRepo,
    ) -> Result<Option<GitStatus>, FileError> {
        self.check(session)?;
        let output = git(self.root())
            .args([
                "status",
                "--porcelain=v2",
                "-z",
                "--branch",
                "--untracked-files=all",
                "--",
                ".",
            ])
            .output();
        match output {
            Ok(output) if output.status.success() => {
                Ok(Some(parse_porcelain_v2(&output.stdout, &repo.prefix)))
            }
            Ok(output) => {
                tracing::debug!(code = ?output.status.code(), "git status a échoué");
                Ok(None)
            }
            Err(err) => {
                tracing::debug!(error = %err, "git introuvable");
                Ok(None)
            }
        }
    }
}

fn git(root: &Path) -> Command {
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(root)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .stdin(Stdio::null())
        .stderr(Stdio::null());
    command
}

/// Lit la sortie de `git status --porcelain=v2 -z --branch` ; `prefix` (terminé par
/// `/`, ou vide) est retiré des chemins, et ce qui n'est pas dessous est ignoré.
pub fn parse_porcelain_v2(output: &[u8], prefix: &str) -> GitStatus {
    let mut status = GitStatus::default();
    let mut deleted = 0;
    let mut tokens = output.split(|byte| *byte == 0);
    while let Some(token) = tokens.next() {
        let line = String::from_utf8_lossy(token);
        let Some((kind, rest)) = line.split_once(' ') else {
            continue;
        };
        let (xy, path, renamed) = match kind {
            "#" => {
                match rest.strip_prefix("branch.head ") {
                    Some("(detached)") => status.detached = true,
                    Some(head) => status.branch = Some(head.to_owned()),
                    None => {}
                }
                continue;
            }
            // 1 XY sub mH mI mW hH hI chemin
            "1" => {
                let fields: Vec<&str> = rest.splitn(8, ' ').collect();
                (fields.first().copied(), fields.get(7).copied(), false)
            }
            // 2 XY sub mH mI mW hH hI Xscore chemin, puis l'ancien chemin
            "2" => {
                tokens.next();
                let fields: Vec<&str> = rest.splitn(9, ' ').collect();
                (fields.first().copied(), fields.get(8).copied(), true)
            }
            // u XY sub m1 m2 m3 mW h1 h2 h3 chemin
            "u" => {
                let fields: Vec<&str> = rest.splitn(10, ' ').collect();
                (Some("UU"), fields.get(9).copied(), false)
            }
            "?" => (Some("??"), Some(rest), false),
            _ => continue,
        };
        let (Some(xy), Some(path)) = (xy, path) else {
            continue;
        };
        let Some(path) = path
            .strip_prefix(prefix)
            .and_then(|p| RelPath::parse(p).ok())
            .filter(|p| !p.is_root())
        else {
            continue;
        };
        let mut codes = xy.chars();
        let (x, y) = (codes.next().unwrap_or('.'), codes.next().unwrap_or('.'));
        let file_status = match kind {
            "u" => GitFileStatus::Conflicted,
            "?" => GitFileStatus::Untracked,
            _ if renamed && (x == 'R' || y == 'R') => GitFileStatus::Renamed,
            _ if renamed => GitFileStatus::Added,
            _ if x == 'D' || y == 'D' => {
                deleted += 1;
                continue;
            }
            _ if x == 'A' => GitFileStatus::Added,
            _ => GitFileStatus::Modified,
        };
        if file_status == GitFileStatus::Conflicted {
            status.conflicts += 1;
        }
        status.entries.push(GitEntry {
            path,
            status: file_status,
        });
    }
    status.changes = status.entries.len() + deleted;
    status
}

/// Délai de regroupement des événements du dossier git.
const DEBOUNCE: Duration = Duration::from_millis(100);

/// Observe `HEAD`, `index`, `packed-refs` et `refs/` (commit, checkout, merge), sans
/// descendre dans les objets.
pub struct GitWatcher {
    _debouncer: Debouncer<RecommendedWatcher, RecommendedCache>,
}

impl GitRepo {
    /// `on_change` est appelé, depuis un fil dédié, quand l'état du dépôt change.
    pub fn watch(&self, on_change: impl Fn() + Send + 'static) -> Result<GitWatcher, FileError> {
        let watch_error = |e: notify_debouncer_full::notify::Error| FileError::Watch(e.to_string());
        let (git_dir, common_dir) = (self.git_dir.clone(), self.common_dir.clone());
        let refs = common_dir.join("refs");
        let filter_refs = refs.clone();
        let mut debouncer = new_debouncer(DEBOUNCE, None, move |result: DebounceEventResult| {
            let Ok(events) = result else {
                return;
            };
            let relevant = events
                .iter()
                .filter(|event| !matches!(event.kind, EventKind::Access(_)))
                .flat_map(|event| event.paths.iter())
                .any(|path| is_state_file(path, &git_dir, &common_dir, &filter_refs));
            if relevant {
                on_change();
            }
        })
        .map_err(watch_error)?;
        debouncer
            .watch(&self.git_dir, RecursiveMode::NonRecursive)
            .map_err(watch_error)?;
        if self.common_dir != self.git_dir {
            debouncer
                .watch(&self.common_dir, RecursiveMode::NonRecursive)
                .map_err(watch_error)?;
        }
        if refs.is_dir() {
            debouncer
                .watch(&refs, RecursiveMode::Recursive)
                .map_err(watch_error)?;
        }
        Ok(GitWatcher {
            _debouncer: debouncer,
        })
    }
}

fn is_state_file(path: &Path, git_dir: &Path, common_dir: &Path, refs: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    if name.ends_with(".lock") {
        return false;
    }
    if path.starts_with(refs) {
        return true;
    }
    let parent = path.parent();
    (parent == Some(git_dir) || parent == Some(common_dir))
        && matches!(name, "HEAD" | "index" | "packed-refs" | "MERGE_HEAD")
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::mpsc;

    use super::*;
    use crate::UserId;

    fn entries(status: &GitStatus) -> Vec<(&str, GitFileStatus)> {
        status
            .entries
            .iter()
            .map(|e| (e.path.as_str(), e.status))
            .collect()
    }

    #[test]
    fn lit_les_statuts_les_renommages_et_les_conflits() {
        let output = [
            "# branch.oid 3f2a9c0d1e4b5a67",
            "# branch.head main",
            "1 .M N... 100644 100644 100644 aaa aaa src/main.rs",
            "1 M. N... 100644 100644 100644 aaa bbb docs/un fichier avec espaces.md",
            "1 A. N... 000000 100644 100644 000 bbb src/nouveau.rs",
            "1 AM N... 000000 100644 100644 000 bbb src/ajoute-modifie.rs",
            "1 .D N... 100644 100644 000000 aaa aaa src/supprime.rs",
            "1 D. N... 100644 000000 000000 aaa 000 src/retire.rs",
            "2 R. N... 100644 100644 100644 aaa aaa R100 src/apres.rs",
            "src/avant.rs",
            "u UU N... 100644 100644 100644 100644 aaa bbb ccc package.json",
            "? notes/brouillon.md",
            "? ligne\nretour.txt",
            "",
        ]
        .join("\0");
        let status = parse_porcelain_v2(output.as_bytes(), "");
        assert_eq!(status.branch.as_deref(), Some("main"));
        assert!(!status.detached);
        assert_eq!(
            entries(&status),
            [
                ("src/main.rs", GitFileStatus::Modified),
                ("docs/un fichier avec espaces.md", GitFileStatus::Modified),
                ("src/nouveau.rs", GitFileStatus::Added),
                ("src/ajoute-modifie.rs", GitFileStatus::Added),
                ("src/apres.rs", GitFileStatus::Renamed),
                ("package.json", GitFileStatus::Conflicted),
                ("notes/brouillon.md", GitFileStatus::Untracked),
                ("ligne\nretour.txt", GitFileStatus::Untracked),
            ]
        );
        assert_eq!(status.changes, 10, "supprimés compris");
        assert_eq!(status.conflicts, 1);
    }

    #[test]
    fn head_detachee_et_prefixe_d_un_sous_dossier() {
        let output = [
            "# branch.oid 3f2a9c0d1e4b5a67",
            "# branch.head (detached)",
            "1 .M N... 100644 100644 100644 aaa aaa app/src/main.rs",
            "1 .M N... 100644 100644 100644 aaa aaa autre/main.rs",
            "? app/note.md",
        ]
        .join("\0");
        let status = parse_porcelain_v2(output.as_bytes(), "app/");
        assert!(status.detached);
        assert_eq!(status.branch, None);
        assert_eq!(
            entries(&status),
            [
                ("src/main.rs", GitFileStatus::Modified),
                ("note.md", GitFileStatus::Untracked),
            ]
        );
        assert_eq!(status.changes, 2);
    }

    /// `git` disponible, sans configuration globale qui gêne les tests.
    fn run_git(dir: &Path, args: &[&str]) -> bool {
        Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(["-c", "user.name=Test", "-c", "user.email=test@example.org"])
            .args([
                "-c",
                "commit.gpgsign=false",
                "-c",
                "init.defaultBranch=main",
            ])
            .args(args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
    }

    fn repo_with_commit() -> Option<tempfile::TempDir> {
        let dir = tempfile::tempdir().unwrap();
        if !run_git(dir.path(), &["init", "-q"]) {
            eprintln!("git absent : test ignoré");
            return None;
        }
        fs::create_dir_all(dir.path().join("app/src")).unwrap();
        fs::write(dir.path().join("app/src/main.rs"), "fn main() {}").unwrap();
        fs::write(dir.path().join("hors.txt"), "x").unwrap();
        assert!(run_git(dir.path(), &["add", "."]));
        assert!(run_git(dir.path(), &["commit", "-q", "-m", "init"]));
        Some(dir)
    }

    #[test]
    fn statut_reel_d_un_projet_dans_un_sous_dossier_du_depot() {
        let Some(dir) = repo_with_commit() else {
            return;
        };
        fs::write(dir.path().join("app/src/main.rs"), "fn main() { }").unwrap();
        fs::write(dir.path().join("app/note.md"), "x").unwrap();
        fs::write(dir.path().join("hors.txt"), "y").unwrap();
        let project = Project::open(&dir.path().join("app")).unwrap();
        let session = project.session(UserId::local());
        let repo = project.git_repo(&session).unwrap().unwrap();
        assert_eq!(
            repo.git_dir,
            dir.path().canonicalize().unwrap().join(".git")
        );
        assert_eq!(repo.prefix, "app/");
        let status = project.git_status(&session, &repo).unwrap().unwrap();
        assert_eq!(status.branch.as_deref(), Some("main"));
        assert_eq!(
            entries(&status),
            [
                ("src/main.rs", GitFileStatus::Modified),
                ("note.md", GitFileStatus::Untracked),
            ]
        );
        assert!(
            !dir.path().join(".git/index.lock").exists(),
            "aucun verrou laissé"
        );
    }

    #[test]
    fn sans_depot_aucun_statut() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join(".git")).unwrap();
        fs::write(dir.path().join(".git/config"), "[core]").unwrap();
        let project = Project::open(dir.path()).unwrap();
        let session = project.session(UserId::local());
        assert_eq!(project.git_repo(&session).unwrap(), None);
    }

    #[test]
    fn un_commit_est_signale_par_l_observateur() {
        let Some(dir) = repo_with_commit() else {
            return;
        };
        let project = Project::open(dir.path()).unwrap();
        let session = project.session(UserId::local());
        let repo = project.git_repo(&session).unwrap().unwrap();
        let (tx, rx) = mpsc::channel();
        let _watcher = repo
            .watch(move || {
                let _ = tx.send(());
            })
            .unwrap();
        fs::write(dir.path().join("app/src/main.rs"), "fn main() { }").unwrap();
        assert!(run_git(dir.path(), &["commit", "-q", "-am", "deux"]));
        assert!(
            rx.recv_timeout(Duration::from_secs(5)).is_ok(),
            "aucun événement après un commit"
        );
    }
}
