//! Serveur local de Hodegos : fichiers statiques, WebSocket, jeton de lancement.
//!
//! Seul crate qui connaît HTTP et la WebSocket. Il n'écoute que sur `127.0.0.1`
//! et n'accepte que la fenêtre ouverte avec le jeton de lancement.

mod assets;
mod auth;
pub mod eval;
mod followup;
mod git;
mod guard;
mod mentor_settings;
mod mentors;
mod pass;
pub mod protocol;
mod secret;
mod settings;
mod store;
mod terminal;
mod ws;

use std::future::Future;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};

use axum::Router;
use axum::routing::get;
use hodegos_core::{FileError, FileWatcher, Project, RelPath, UserId};
use hodegos_llm::{ApiKey, Budget, Effort, LlmClient, LlmConfig};
use hodegos_mentors::{MentorSources, ModelTier};
use jiff::Timestamp;
use tokio::net::TcpListener;
use tokio::sync::{Notify, broadcast};

pub use auth::{Authenticator, LaunchTokenAuthenticator};
pub use terminal::{Bind, BindMode, TerminalOptions, default_binds, load_binds};

use crate::git::GitTracker;
use crate::mentors::Mentors;
use crate::protocol::{BudgetLevel, LastPass, ServerMessage, Tier};
use crate::settings::Catalog;
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
    /// Réglages modifiables depuis l'IDE ; `None` : lecture seule.
    pub settings: Option<SettingsSource>,
}

/// Crée le client du fournisseur pour une clé et une configuration (les tests
/// fournissent le faux client).
pub type Connect =
    Arc<dyn Fn(&ApiKey, &LlmConfig) -> Result<Arc<dyn LlmClient>, String> + Send + Sync>;

/// Où écrire les réglages, et de quoi refaire le client quand ils changent.
pub struct SettingsSource {
    /// `~/.config/hodegos` : `config.toml` et `credentials.toml`.
    pub config_dir: PathBuf,
    pub key: Option<ApiKey>,
    /// La clé vient de `OPENROUTER_API_KEY` : elle prime, les réglages ne l'écrivent pas.
    pub key_from_env: bool,
    pub connect: Connect,
}

/// Client d'après la clé et la configuration ; sinon, la raison de son absence,
/// affichée dans la barre d'état.
pub fn client_for(
    config_dir: &Path,
    key: Option<&ApiKey>,
    config: &LlmConfig,
    connect: &Connect,
) -> Result<Arc<dyn LlmClient>, String> {
    match key {
        None => Err(format!(
            "clé OpenRouter absente : ajoutez-la dans les réglages, à {} ou à OPENROUTER_API_KEY",
            config_dir.join("credentials.toml").display()
        )),
        Some(_) if config.fast.is_none() && config.strong.is_none() => Err(format!(
            "modèles non configurés : renseignez-les dans les réglages ou dans {}",
            config_dir.join("config.toml").display()
        )),
        Some(key) => connect(key, config),
    }
}

/// Accès au modèle en cours : les réglages le remplacent, chaque passe en lit une copie.
pub(crate) struct LlmNow {
    pub(crate) client: Result<Arc<dyn LlmClient>, String>,
    pub(crate) config: LlmConfig,
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
    /// Terminal intégré et confiné (phase 8f).
    pub terminal: TerminalOptions,
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
    /// Statut git, recalculé après chaque changement.
    git: Arc<GitTracker>,
    mentors: Mentors,
    llm: RwLock<LlmNow>,
    /// Réglages modifiables, et la clé lue (jamais renvoyée au front).
    settings: Option<settings::Source>,
    /// Catalogue des modèles d'OpenRouter, lu à la demande.
    catalog: Catalog,
    /// Évaluation en cours (une à la fois).
    eval_running: Mutex<Option<Tier>>,
    /// Messages pour toutes les connexions : réglages, évaluation, état.
    events: broadcast::Sender<ServerMessage>,
    data_dir: PathBuf,
    terminals: terminal::Terminals,
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

    /// Copie du client et de la configuration en cours.
    fn llm(&self) -> LlmNow {
        match self.llm.read() {
            Ok(llm) => LlmNow {
                client: llm.client.clone(),
                config: llm.config.clone(),
            },
            Err(_) => LlmNow {
                client: Err("état du fournisseur illisible".to_owned()),
                config: LlmConfig::default(),
            },
        }
    }

    /// Modèle et effort d'un niveau. L'effort n'est retenu que si le catalogue
    /// confirme que le modèle accepte `reasoning` ; sinon rien n'est envoyé.
    fn model_for_tier(&self, tier: ModelTier) -> Option<(String, Effort)> {
        let config = self.llm().config;
        let (model, effort) = match tier {
            ModelTier::Fast => (config.fast?, config.fast_effort),
            ModelTier::Strong => (config.strong?, config.strong_effort),
        };
        let effort = if effort != Effort::Default
            && self.catalog.known(&model).is_some_and(|m| m.reasoning)
        {
            effort
        } else {
            Effort::Default
        };
        Some((model, effort))
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
            terminal,
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
        let refresh = Arc::new(Notify::new());
        let git_refresh = refresh.clone();
        let watcher = project
            .watch(move |paths| {
                let _ = sender.send(Arc::new(paths));
                git_refresh.notify_one();
            })
            .map_err(ServerError::Watch)?;
        let git = GitTracker::new(project.clone(), project.session(UserId::local()), refresh);
        let first = git.clone();
        if let Err(err) = tokio::task::spawn_blocking(move || first.init()).await {
            tracing::warn!(error = %err, "statut git initial non calculé");
        }
        tokio::spawn(git.clone().run());
        let project_dir = data_dir.join(project.id().as_str());
        let (mentors, mut mentor_changes) =
            Mentors::start(sources, Some(project_dir.join("overrides.json")));
        let store = Store::open(Some(project_dir), project.root());
        let budget = Budget::open(data_dir.join("usage.json"), llm.config.budget.clone());
        let provider = match &llm.client {
            Ok(_) => (true, None),
            Err(message) => (false, Some(message.clone())),
        };
        let needs_catalog = llm.config.fast_effort != Effort::Default
            || llm.config.strong_effort != Effort::Default;
        let (events, _) = broadcast::channel(64);
        let state = Arc::new(AppState {
            port,
            auth: Arc::new(auth),
            project,
            watcher,
            changes,
            git,
            mentors,
            catalog: Catalog::default(),
            settings: llm.settings.map(settings::Source::new),
            llm: RwLock::new(LlmNow {
                client: llm.client,
                config: llm.config,
            }),
            eval_running: Mutex::new(None),
            events,
            data_dir,
            terminals: terminal::Terminals::new(terminal),
            budget,
            store,
            provider: Mutex::new(provider),
            last_pass: Mutex::new(None),
            initial_file,
        });
        // Un effort choisi n'est envoyé qu'une fois le catalogue lu.
        if needs_catalog {
            let catalog_state = state.clone();
            tokio::spawn(async move { settings::refresh_catalog(&catalog_state).await });
        }
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
