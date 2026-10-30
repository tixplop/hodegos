//! Serveur local de Hodegos : fichiers statiques, WebSocket, jeton de lancement.
//!
//! Seul crate qui connaît HTTP et la WebSocket. Il n'écoute que sur `127.0.0.1`
//! et n'accepte que la fenêtre ouverte avec le jeton de lancement.

mod assets;
mod auth;
mod followup;
mod guard;
mod mentors;
mod pass;
pub mod protocol;
mod secret;
mod store;
mod ws;

use std::future::Future;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::routing::get;
use hodegos_core::{FileError, FileWatcher, Project, RelPath};
use hodegos_llm::{Budget, LlmClient, LlmConfig};
use hodegos_mentors::MentorSources;
use jiff::Timestamp;
use tokio::net::TcpListener;
use tokio::sync::broadcast;

pub use auth::{Authenticator, LaunchTokenAuthenticator};

use crate::mentors::Mentors;
use crate::protocol::{BudgetLevel, LastPass, ServerMessage};
use crate::store::Store;

/// Port par défaut. Il est fixe : sous WSL2 en mode NAT, un port tiré au hasard peut
/// tomber dans une plage réservée par Windows et rester injoignable depuis le navigateur.
pub const DEFAULT_PORT: u16 = 7420;

#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    #[error(
        "le port {0} est déjà utilisé sur 127.0.0.1 (une autre instance de Hodegos ?) ; \
         choisissez-en un autre avec --port"
    )]
    PortInUse(u16),
    #[error("impossible d'ouvrir le port {port} sur 127.0.0.1 : {source}")]
    Bind {
        port: u16,
        #[source]
        source: std::io::Error,
    },
    #[error("le serveur s'est arrêté sur une erreur : {0}")]
    Serve(#[source] std::io::Error),
    #[error("impossible de générer un secret aléatoire : {0}")]
    Random(String),
    #[error("impossible d'observer les fichiers du projet : {0}")]
    Watch(#[source] FileError),
}

/// Accès au modèle : un client prêt, ou la raison de son absence (clé absente…).
pub struct Llm {
    pub client: Result<Arc<dyn LlmClient>, String>,
    pub config: LlmConfig,
}

/// Réglages du serveur.
pub struct ServerOptions {
    /// Port sur 127.0.0.1 ; `0` laisse le système choisir (tests).
    pub port: u16,
    /// Mentors personnels et du projet, en plus des mentors livrés.
    pub mentors: MentorSources,
    /// Données de l'utilisateur : `$XDG_DATA_HOME/hodegos` (budget, historique).
    pub data_dir: PathBuf,
    pub llm: Llm,
    /// Fichier que le front ouvre à la connexion (`hodegos new` : `docs/vision.md`).
    pub initial_file: Option<String>,
}

/// Chemins touchés sur le disque, diffusés à toutes les connexions.
pub(crate) type Changes = Arc<Vec<RelPath>>;

/// État partagé par toutes les requêtes.
struct AppState {
    port: u16,
    auth: Arc<dyn Authenticator>,
    project: Project,
    watcher: FileWatcher,
    changes: broadcast::Sender<Changes>,
    mentors: Mentors,
    llm: Llm,
    budget: Budget,
    store: Store,
    /// Fournisseur joignable, et sinon pourquoi.
    provider: Mutex<(bool, Option<String>)>,
    last_pass: Mutex<Option<LastPass>>,
    initial_file: Option<String>,
}

impl AppState {
    fn canonical_host(&self) -> String {
        format!("127.0.0.1:{}", self.port)
    }

    fn canonical_origin(&self) -> String {
        format!("http://{}", self.canonical_host())
    }

    fn cookie_name(&self) -> String {
        format!("hodegos_{}", self.port)
    }

    fn set_provider(&self, ok: bool, message: Option<String>) {
        if let Ok(mut provider) = self.provider.lock() {
            *provider = (ok, message);
        }
    }

    fn set_last_pass(&self, last: LastPass) {
        if let Ok(mut last_pass) = self.last_pass.lock() {
            *last_pass = Some(last);
        }
    }

    /// État du fournisseur, du budget et de la dernière passe.
    fn status(&self) -> ServerMessage {
        self.status_with(None)
    }

    /// Premier `status` d'une connexion : il porte aussi le fichier à ouvrir.
    fn connection_status(&self) -> ServerMessage {
        self.status_with(self.initial_file.clone())
    }

    fn status_with(&self, initial_file: Option<String>) -> ServerMessage {
        let (provider_ok, provider_message) = self
            .provider
            .lock()
            .map(|p| p.clone())
            .unwrap_or((false, None));
        let budget = self.budget.state(Timestamp::now());
        ServerMessage::Status {
            provider_ok,
            provider_message,
            budget_used: budget.used_usd,
            budget_cap: budget.cap_usd,
            budget_level: match budget.level {
                hodegos_llm::BudgetLevel::Ok => BudgetLevel::Ok,
                hodegos_llm::BudgetLevel::Warning => BudgetLevel::Warning,
                hodegos_llm::BudgetLevel::Exceeded => BudgetLevel::Exceeded,
            },
            last_pass: self.last_pass.lock().ok().and_then(|l| l.clone()),
            initial_file,
        }
    }
}

/// Serveur lié à un port libre de `127.0.0.1`, prêt à servir.
pub struct Server {
    listener: TcpListener,
    state: Arc<AppState>,
    access_url: String,
}

impl Server {
    /// Ouvre le port demandé sur `127.0.0.1` pour `project` et génère le jeton de lancement.
    pub async fn bind(project: Project, options: ServerOptions) -> Result<Self, ServerError> {
        let ServerOptions {
            port,
            mentors: sources,
            data_dir,
            llm,
            initial_file,
        } = options;
        let bind_error = |source: std::io::Error| match source.kind() {
            std::io::ErrorKind::AddrInUse => ServerError::PortInUse(port),
            _ => ServerError::Bind { port, source },
        };
        let listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, port)))
            .await
            .map_err(bind_error)?;
        let port = listener.local_addr().map_err(bind_error)?.port();
        let auth = LaunchTokenAuthenticator::generate()?;
        let access_url = format!("http://127.0.0.1:{port}/?token={}", auth.launch_token());
        let (changes, _) = broadcast::channel(64);
        let sender = changes.clone();
        let watcher = project
            .watch(move |paths| {
                let _ = sender.send(Arc::new(paths));
            })
            .map_err(ServerError::Watch)?;
        let project_dir = data_dir.join(project.id().as_str());
        let (mentors, mut mentor_changes) =
            Mentors::start(sources, Some(project_dir.join("overrides.json")));
        let store = Store::open(Some(project_dir), project.root());
        let budget = Budget::open(data_dir.join("usage.json"), llm.config.budget.clone());
        let provider = match &llm.client {
            Ok(_) => (true, None),
            Err(message) => (false, Some(message.clone())),
        };
        let state = Arc::new(AppState {
            port,
            auth: Arc::new(auth),
            project,
            watcher,
            changes,
            mentors,
            llm,
            budget,
            store,
            provider: Mutex::new(provider),
            last_pass: Mutex::new(None),
            initial_file,
        });
        let reload_state = state.clone();
        tokio::spawn(async move {
            while mentor_changes.recv().await.is_some() {
                reload_state.mentors.reload();
            }
        });
        Ok(Self {
            listener,
            state,
            access_url,
        })
    }

    pub fn port(&self) -> u16 {
        self.state.port
    }

    /// URL d'accès à afficher dans le terminal ; elle contient le jeton.
    pub fn access_url(&self) -> &str {
        &self.access_url
    }

    /// Routes et contrôles de sécurité, sans la boucle d'écoute.
    pub fn router(&self) -> Router {
        Router::new()
            .route("/ws", get(ws::upgrade))
            .fallback(assets::serve)
            .layer(axum::middleware::from_fn_with_state(
                self.state.clone(),
                guard::guard,
            ))
            .with_state(self.state.clone())
    }

    /// Sert les requêtes jusqu'à ce que `shutdown` se termine.
    pub async fn serve(
        self,
        shutdown: impl Future<Output = ()> + Send + 'static,
    ) -> Result<(), ServerError> {
        let router = self.router();
        tracing::info!(port = self.state.port, "serveur à l'écoute sur 127.0.0.1");
        axum::serve(self.listener, router)
            .with_graceful_shutdown(shutdown)
            .await
            .map_err(ServerError::Serve)
    }
}
