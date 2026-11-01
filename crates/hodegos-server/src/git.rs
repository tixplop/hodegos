//! Statut git poussé aux fenêtres (phase 8a) : recalculé après un regroupement de
//! 300 ms quand quelque chose a pu le changer, et diffusé seulement s'il a changé.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use hodegos_core::{GitFileStatus, GitRepo, GitStatus, GitWatcher, Project, Session};
use tokio::sync::{Notify, broadcast};

use crate::protocol::{GitLetter, GitStatusEntry, GitStatusInfo};

/// Délai de regroupement des demandes de rafraîchissement.
const DEBOUNCE: Duration = Duration::from_millis(300);

/// Statut tel qu'il part dans `git_status` ; `None` sans dépôt.
pub(crate) type GitUpdate = Option<GitStatusInfo>;

pub(crate) struct GitTracker {
    project: Project,
    session: Session,
    refresh: Arc<Notify>,
    current: Mutex<GitUpdate>,
    pub(crate) updates: broadcast::Sender<GitUpdate>,
    /// Chaque recalcul, même sans changement du statut (un commit fait depuis un arbre
    /// propre change `HEAD` sans changer le statut) : revues automatiques (phase 9d).
    pub(crate) refreshed: broadcast::Sender<()>,
    /// Dépôt observé (`HEAD`, `index`, `refs/`) ; suit le dépôt s'il apparaît ou change.
    watcher: Mutex<Option<(GitRepo, GitWatcher)>>,
}

impl GitTracker {
    /// `refresh` est aussi signalé par l'observateur des fichiers du projet.
    pub(crate) fn new(project: Project, session: Session, refresh: Arc<Notify>) -> Arc<Self> {
        let (updates, _) = broadcast::channel(16);
        let (refreshed, _) = broadcast::channel(4);
        Arc::new(Self {
            refreshed,
            project,
            session,
            refresh,
            current: Mutex::new(None),
            updates,
            watcher: Mutex::new(None),
        })
    }

    /// Premier calcul, avant d'accepter des connexions.
    pub(crate) fn init(&self) {
        let status = self.compute();
        if let Ok(mut current) = self.current.lock() {
            *current = status;
        }
    }

    pub(crate) fn current(&self) -> GitUpdate {
        self.current.lock().ok().and_then(|c| c.clone())
    }

    /// Demande un nouveau calcul, regroupé avec les demandes voisines.
    pub(crate) fn refresh(&self) {
        self.refresh.notify_one();
    }

    /// Boucle de rafraîchissement, jusqu'à l'arrêt du serveur.
    pub(crate) async fn run(self: Arc<Self>) {
        loop {
            self.refresh.notified().await;
            tokio::time::sleep(DEBOUNCE).await;
            let tracker = self.clone();
            let Ok(status) = tokio::task::spawn_blocking(move || tracker.compute()).await else {
                continue;
            };
            let changed = match self.current.lock() {
                Ok(mut current) if *current != status => {
                    *current = status.clone();
                    true
                }
                _ => false,
            };
            if changed {
                let _ = self.updates.send(status);
            }
            let _ = self.refreshed.send(());
        }
    }

    fn compute(&self) -> GitUpdate {
        let repo = self.project.git_repo(&self.session).ok().flatten();
        self.watch(repo.as_ref());
        let status = self
            .project
            .git_status(&self.session, repo.as_ref()?)
            .ok()
            .flatten()?;
        Some(info(status))
    }

    fn watch(&self, repo: Option<&GitRepo>) {
        let Ok(mut watcher) = self.watcher.lock() else {
            return;
        };
        if watcher.as_ref().map(|(watched, _)| watched) == repo {
            return;
        }
        *watcher = repo.and_then(|repo| {
            let refresh = self.refresh.clone();
            match repo.watch(move || refresh.notify_one()) {
                Ok(git_watcher) => Some((repo.clone(), git_watcher)),
                Err(err) => {
                    tracing::warn!(error = %err, "changements du dépôt git non suivis");
                    None
                }
            }
        });
    }
}

fn info(status: GitStatus) -> GitStatusInfo {
    GitStatusInfo {
        branch: status.branch,
        detached: status.detached,
        entries: status
            .entries
            .into_iter()
            .map(|entry| GitStatusEntry {
                path: entry.path.to_string(),
                status: letter(entry.status),
            })
            .collect(),
        changes: status.changes,
        conflicts: status.conflicts,
    }
}

fn letter(status: GitFileStatus) -> GitLetter {
    match status {
        GitFileStatus::Modified => GitLetter::M,
        GitFileStatus::Added => GitLetter::A,
        GitFileStatus::Untracked => GitLetter::U,
        GitFileStatus::Conflicted => GitLetter::C,
        GitFileStatus::Renamed => GitLetter::R,
    }
}
