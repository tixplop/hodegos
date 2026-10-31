//! Terminal intégré et confiné (phase 8f, ADR 0005) : un pseudo-terminal par terminal,
//! le shell de l'utilisateur dans bubblewrap, ses octets par la WebSocket existante.
//! Sans confinement possible, pas de terminal.

mod sandbox;
mod session;

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub use sandbox::{Bind, BindMode, default_binds, load_binds};
pub(crate) use session::{Event, Session, spawn};

use sandbox::{Sandbox, bwrap_args, environment, probe, resolv_target};
use session::Launch;

/// Terminaux ouverts au plus par fenêtre.
pub(crate) const MAX_PER_WINDOW: usize = 4;
/// Taille maximale d'une saisie (`term_input`), après décodage.
pub(crate) const MAX_INPUT: usize = 64 * 1024;

/// Réglages du terminal.
pub struct TerminalOptions {
    /// Programme `bwrap` (les tests en donnent un absent).
    pub bwrap: PathBuf,
    pub binds: Vec<Bind>,
    pub home: PathBuf,
    /// Shell de l'utilisateur (`$SHELL`, sinon `/bin/bash`).
    pub shell: String,
    /// `[terminal] binds` invalide : montages par défaut, et pourquoi.
    pub warning: Option<String>,
}

impl TerminalOptions {
    /// D'après l'environnement : `$HOME`, `$SHELL`, `bwrap` dans le `PATH`.
    pub fn from_env(binds: Vec<Bind>, warning: Option<String>) -> Self {
        Self {
            bwrap: PathBuf::from("bwrap"),
            binds,
            home: std::env::var_os("HOME").map_or_else(|| PathBuf::from("/"), PathBuf::from),
            shell: std::env::var("SHELL")
                .ok()
                .filter(|s| s.starts_with('/'))
                .unwrap_or_else(|| "/bin/bash".to_owned()),
            warning,
        }
    }
}

/// Terminaux du serveur : réglages et disponibilité du confinement, vérifiée une fois.
pub(crate) struct Terminals {
    options: TerminalOptions,
    available: OnceLock<Result<(), String>>,
}

impl Terminals {
    pub(crate) fn new(options: TerminalOptions) -> Self {
        Self {
            options,
            available: OnceLock::new(),
        }
    }

    /// Confinement utilisable, sinon la raison et le remède.
    pub(crate) fn check(&self) -> Result<(), String> {
        self.available
            .get_or_init(|| {
                let result = probe(&self.options.bwrap);
                if let Err(message) = &result {
                    tracing::warn!(%message, "terminal indisponible");
                }
                result
            })
            .clone()
    }

    /// Nom court du shell, pour l'onglet (`zsh`).
    pub(crate) fn shell_name(&self) -> String {
        Path::new(&self.options.shell)
            .file_name()
            .map_or_else(|| "shell".to_owned(), |n| n.to_string_lossy().into_owned())
    }

    pub(crate) fn warning(&self) -> Option<String> {
        self.options.warning.clone()
    }

    /// `bwrap` et ses arguments pour un terminal dans `project`.
    pub(crate) fn launch(&self, project: &Path) -> Launch {
        let resolv = resolv_target();
        let args = bwrap_args(
            &Sandbox {
                project,
                home: &self.options.home,
                binds: &self.options.binds,
                resolv: resolv.as_deref(),
                shell: &self.options.shell,
            },
            Path::exists,
        );
        // `HOME` suit les montages, même si l'environnement du serveur dit autre chose.
        let mut env = environment(std::env::vars());
        env.retain(|(name, _)| name != "HOME");
        env.push((
            "HOME".to_owned(),
            self.options.home.to_string_lossy().into_owned(),
        ));
        Launch {
            program: self.options.bwrap.clone(),
            args,
            env,
            cwd: project.to_path_buf(),
        }
    }
}
