use std::collections::{BTreeSet, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use notify_debouncer_full::notify::{EventKind, RecommendedWatcher, RecursiveMode};
use notify_debouncer_full::{DebounceEventResult, Debouncer, RecommendedCache, new_debouncer};

use crate::{FileError, Project, RelPath, Session};

/// Délai de regroupement des événements du système de fichiers.
const DEBOUNCE: Duration = Duration::from_millis(200);

/// Observe les dossiers des fichiers ouverts et signale les chemins touchés.
///
/// Seuls les dossiers des fichiers suivis sont observés, sans récursion, pour ne pas
/// épuiser les observateurs inotify sur un gros projet (`target/`, `node_modules/`).
pub struct FileWatcher {
    project: Project,
    debouncer: Mutex<Debouncer<RecommendedWatcher, RecommendedCache>>,
    watched: Mutex<HashSet<PathBuf>>,
}

impl Project {
    /// Démarre l'observation ; `on_change` reçoit les chemins touchés, depuis un fil dédié.
    pub fn watch(
        &self,
        on_change: impl Fn(Vec<RelPath>) + Send + 'static,
    ) -> Result<FileWatcher, FileError> {
        let root = self.root().to_path_buf();
        let debouncer =
            new_debouncer(
                DEBOUNCE,
                None,
                move |result: DebounceEventResult| match result {
                    Ok(events) => {
                        let paths: BTreeSet<RelPath> = events
                            .iter()
                            .filter(|event| !matches!(event.kind, EventKind::Access(_)))
                            .flat_map(|event| event.paths.iter())
                            .filter_map(|path| relative(&root, path))
                            .collect();
                        if !paths.is_empty() {
                            on_change(paths.into_iter().collect());
                        }
                    }
                    Err(errors) => {
                        for error in errors {
                            tracing::warn!(%error, "erreur de l'observateur de fichiers");
                        }
                    }
                },
            )
            .map_err(|e| FileError::Watch(e.to_string()))?;
        Ok(FileWatcher {
            project: self.clone(),
            debouncer: Mutex::new(debouncer),
            watched: Mutex::new(HashSet::new()),
        })
    }
}

impl FileWatcher {
    /// Suit `rel` : son dossier est observé jusqu'à l'arrêt de Hodegos.
    pub fn track(&self, session: &Session, rel: &RelPath) -> Result<(), FileError> {
        self.project.check(session)?;
        let dir = self.project.lexical(&rel.parent());
        let poisoned = || FileError::Watch("verrou empoisonné".to_owned());
        let mut watched = self.watched.lock().map_err(|_| poisoned())?;
        if watched.contains(&dir) {
            return Ok(());
        }
        self.debouncer
            .lock()
            .map_err(|_| poisoned())?
            .watch(&dir, RecursiveMode::NonRecursive)
            .map_err(|e| FileError::Watch(e.to_string()))?;
        watched.insert(dir);
        Ok(())
    }
}

fn relative(root: &Path, path: &Path) -> Option<RelPath> {
    let inside = path.strip_prefix(root).ok()?.to_str()?;
    RelPath::parse(inside).ok().filter(|rel| !rel.is_root())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::mpsc;

    use super::*;
    use crate::UserId;

    fn wait_for(rx: &mpsc::Receiver<Vec<RelPath>>, expected: &str) {
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while let Some(left) = deadline.checked_duration_since(std::time::Instant::now()) {
            match rx.recv_timeout(left) {
                Ok(paths) if paths.iter().any(|p| p.as_str() == expected) => return,
                Ok(_) => continue,
                Err(_) => break,
            }
        }
        panic!("aucun événement pour {expected}");
    }

    #[test]
    fn signale_une_modification_et_une_suppression_externes() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src/lib.rs"), "fn a() {}").unwrap();
        let project = Project::open(dir.path()).unwrap();
        let session = project.session(UserId::local());

        let (tx, rx) = mpsc::channel();
        let watcher = project
            .watch(move |paths| {
                let _ = tx.send(paths);
            })
            .unwrap();
        let rel = RelPath::parse("src/lib.rs").unwrap();
        watcher.track(&session, &rel).unwrap();
        watcher.track(&session, &rel).unwrap();

        fs::write(dir.path().join("src/lib.rs"), "fn b() {}").unwrap();
        wait_for(&rx, "src/lib.rs");
        fs::remove_file(dir.path().join("src/lib.rs")).unwrap();
        wait_for(&rx, "src/lib.rs");
    }
}
