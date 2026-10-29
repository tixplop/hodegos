//! Mentors côté serveur : registre rechargé à chaud et choix forcés en mémoire.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

use hodegos_mentors::{MentorSources, MentorWatcher, Registry};
use tokio::sync::{broadcast, mpsc};

pub(crate) struct Mentors {
    sources: MentorSources,
    registry: RwLock<Arc<Registry>>,
    watcher: Option<MentorWatcher>,
    /// Signalé à chaque rechargement du registre.
    pub(crate) reloaded: broadcast::Sender<()>,
    /// Titulaires forcés : (chemin, section ou "" pour le fichier) → id du mentor.
    /// Gardés le temps du processus ; la persistance viendra en phase 4.
    overrides: Mutex<HashMap<(String, String), String>>,
}

impl Mentors {
    /// Charge le registre et démarre l'observation ; `changed` reçoit un signal à
    /// chaque modification d'un fichier de mentor, pour appeler [`Mentors::reload`].
    pub(crate) fn start(sources: MentorSources) -> (Self, mpsc::UnboundedReceiver<()>) {
        let (tx, changed) = mpsc::unbounded_channel();
        let watcher = MentorWatcher::start(&sources, move || {
            let _ = tx.send(());
        })
        .map_err(
            |err| tracing::warn!(error = %err, "rechargement à chaud des mentors indisponible"),
        )
        .ok();
        let registry = Registry::load(&sources);
        let (reloaded, _) = broadcast::channel(8);
        let mentors = Self {
            sources,
            registry: RwLock::new(Arc::new(registry)),
            watcher,
            reloaded,
            overrides: Mutex::new(HashMap::new()),
        };
        (mentors, changed)
    }

    pub(crate) fn registry(&self) -> Arc<Registry> {
        match self.registry.read() {
            Ok(registry) => registry.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }

    pub(crate) fn reload(&self) {
        let registry = Arc::new(Registry::load(&self.sources));
        tracing::info!(
            actifs = registry.active().count(),
            erreurs = registry.warnings().len(),
            "mentors rechargés"
        );
        match self.registry.write() {
            Ok(mut current) => *current = registry,
            Err(poisoned) => *poisoned.into_inner() = registry,
        }
        if let Some(watcher) = &self.watcher {
            watcher.rearm();
        }
        let _ = self.reloaded.send(());
    }

    pub(crate) fn forced(&self, path: &str, section: &str) -> Option<String> {
        let overrides = self.overrides.lock().ok()?;
        overrides
            .get(&(path.to_owned(), section.to_owned()))
            .or_else(|| overrides.get(&(path.to_owned(), String::new())))
            .cloned()
    }

    pub(crate) fn set_forced(&self, path: &str, section: &str, mentor: Option<String>) {
        let Ok(mut overrides) = self.overrides.lock() else {
            return;
        };
        let key = (path.to_owned(), section.to_owned());
        match mentor {
            Some(id) => overrides.insert(key, id),
            None => overrides.remove(&key),
        };
    }
}
