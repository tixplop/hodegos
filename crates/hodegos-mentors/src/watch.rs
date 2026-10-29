use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use notify_debouncer_full::notify::{EventKind, RecommendedWatcher, RecursiveMode};
use notify_debouncer_full::{DebounceEventResult, Debouncer, RecommendedCache, new_debouncer};

use crate::MentorSources;

const DEBOUNCE: Duration = Duration::from_millis(200);

#[derive(Debug, thiserror::Error)]
#[error("observation des mentors impossible : {0}")]
pub struct WatchError(String);

/// Observe les dossiers de mentors personnels et du projet.
///
/// Le dossier personnel est créé s'il manque. Celui du projet ne l'est jamais : tant
/// qu'il n'existe pas, on observe ses parents pour voir apparaître `.hodegos/mentors/`,
/// et [`MentorWatcher::rearm`] l'ajoute ensuite.
pub struct MentorWatcher {
    debouncer: Mutex<Debouncer<RecommendedWatcher, RecommendedCache>>,
    watched: Mutex<HashSet<PathBuf>>,
    targets: Vec<PathBuf>,
}

impl MentorWatcher {
    /// `on_change` est appelé, depuis un fil dédié, quand un fichier de mentor change.
    pub fn start(
        sources: &MentorSources,
        on_change: impl Fn() + Send + 'static,
    ) -> Result<Self, WatchError> {
        let mut targets = Vec::new();
        let mut relevant = Vec::new();
        if let Some(user) = &sources.user_dir {
            if let Err(err) = fs::create_dir_all(user) {
                tracing::warn!(dir = %user.display(), error = %err, "dossier des mentors personnels non créé");
            }
            targets.push(user.clone());
            relevant.push(user.clone());
        }
        if let Some(project) = &sources.project_dir {
            // <projet>/.hodegos/mentors : on observe aussi <projet>/.hodegos et <projet>.
            let mut ancestors: Vec<PathBuf> =
                project.ancestors().take(3).map(Path::to_path_buf).collect();
            ancestors.reverse();
            targets.extend(ancestors);
            relevant.push(project.parent().unwrap_or(project).to_path_buf());
        }

        let debouncer = new_debouncer(DEBOUNCE, None, move |result: DebounceEventResult| {
            let Ok(events) = result else {
                return;
            };
            let touched = events
                .iter()
                .filter(|event| !matches!(event.kind, EventKind::Access(_)))
                .flat_map(|event| event.paths.iter())
                .any(|path| relevant.iter().any(|dir| path.starts_with(dir)));
            if touched {
                on_change();
            }
        })
        .map_err(|e| WatchError(e.to_string()))?;

        let watcher = Self {
            debouncer: Mutex::new(debouncer),
            watched: Mutex::new(HashSet::new()),
            targets,
        };
        watcher.rearm();
        Ok(watcher)
    }

    /// Observe les dossiers apparus depuis le dernier appel.
    pub fn rearm(&self) {
        let (Ok(mut debouncer), Ok(mut watched)) = (self.debouncer.lock(), self.watched.lock())
        else {
            return;
        };
        watched.retain(|dir| dir.is_dir());
        for dir in &self.targets {
            if watched.contains(dir) || !dir.is_dir() {
                continue;
            }
            match debouncer.watch(dir, RecursiveMode::NonRecursive) {
                Ok(()) => {
                    watched.insert(dir.clone());
                }
                Err(err) => {
                    tracing::warn!(dir = %dir.display(), error = %err, "dossier de mentors non observé");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use super::*;
    use crate::Registry;

    fn wait(rx: &mpsc::Receiver<()>) {
        rx.recv_timeout(Duration::from_secs(5))
            .expect("aucun changement signalé");
        while rx.recv_timeout(Duration::from_millis(400)).is_ok() {}
    }

    #[test]
    fn signale_un_mentor_modifie_ou_ajoute_au_projet() {
        let dir = tempfile::tempdir().unwrap();
        let sources = MentorSources {
            user_dir: Some(dir.path().join("config/mentors")),
            project_dir: Some(dir.path().join("projet/.hodegos/mentors")),
        };
        fs::create_dir_all(dir.path().join("projet")).unwrap();
        let (tx, rx) = mpsc::channel();
        let watcher = MentorWatcher::start(&sources, move || {
            let _ = tx.send(());
        })
        .unwrap();
        assert!(
            sources.user_dir.as_ref().unwrap().is_dir(),
            "dossier personnel créé"
        );
        assert!(
            !sources.project_dir.as_ref().unwrap().exists(),
            "projet intact"
        );

        let text = "---\nid: perso\nname: Perso\ninitials: PE\nrole: R\n---\n";
        fs::write(sources.user_dir.as_ref().unwrap().join("perso.md"), text).unwrap();
        wait(&rx);
        assert!(Registry::load(&sources).get("perso").is_some());

        let project = sources.project_dir.clone().unwrap();
        fs::create_dir_all(&project).unwrap();
        wait(&rx);
        watcher.rearm();
        fs::write(project.join("projet.md"), text.replace("perso", "projet")).unwrap();
        wait(&rx);
        assert!(Registry::load(&sources).get("projet").is_some());
    }
}
