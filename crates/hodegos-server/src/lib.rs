//! Serveur local de Hodegos : fichiers statiques, WebSocket, jeton de lancement.
//!
//! Seul crate qui connaît HTTP et la WebSocket. Il n'écoute que sur `127.0.0.1`
//! et n'accepte que la fenêtre ouverte avec le jeton de lancement.

mod assets;
mod auth;
mod guard;
mod mentors;
pub mod protocol;
mod secret;
mod ws;

use std::future::Future;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;

use axum::Router;
use axum::routing::get;
use hodegos_core::{FileError, FileWatcher, Project, RelPath};
use hodegos_mentors::MentorSources;
use tokio::net::TcpListener;
use tokio::sync::broadcast;

pub use auth::{Authenticator, LaunchTokenAuthenticator};

use crate::mentors::Mentors;

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
}

/// Serveur lié à un port libre de `127.0.0.1`, prêt à servir.
pub struct Server {
    listener: TcpListener,
    state: Arc<AppState>,
    access_url: String,
}

impl Server {
    /// Ouvre `port` sur `127.0.0.1` pour `project` et génère le jeton de lancement.
    /// Les mentors sont lus depuis `sources`, en plus des mentors livrés.
    ///
    /// `0` laisse le système choisir un port libre (utile aux tests).
    pub async fn bind(
        project: Project,
        sources: MentorSources,
        port: u16,
    ) -> Result<Self, ServerError> {
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
        let (mentors, mut mentor_changes) = Mentors::start(sources);
        let state = Arc::new(AppState {
            port,
            auth: Arc::new(auth),
            project,
            watcher,
            changes,
            mentors,
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
