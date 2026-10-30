//! Binaire `hodegos` : lance l'IDE sur un projet et affiche l'URL d'accès ;
//! gère aussi les mentors (`hodegos mentors list`, `hodegos mentors eject <id>`)
//! et évalue les mentors avec les vrais modèles (`hodegos eval evals/`).

mod eval;

use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use anyhow::Context;
use clap::{Parser, Subcommand};
use hodegos_core::Project;
use hodegos_llm::{LlmClient, LlmConfig, OpenRouterClient, load_api_key};
use hodegos_mentors::{MentorSources, Registry, default_user_dir, eject};
use hodegos_server::{DEFAULT_PORT, Llm, Server, ServerOptions};
use tracing_subscriber::EnvFilter;

/// IDE personnel où des mentors IA vous guident sans écrire à votre place.
#[derive(Debug, Parser)]
#[command(
    name = "hodegos",
    version,
    args_conflicts_with_subcommands = true,
    subcommand_negates_reqs = true
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Dossier du projet à ouvrir, dans le système de fichiers Linux (~/…).
    #[arg(required = true)]
    projet: Option<PathBuf>,

    /// Port d'écoute sur 127.0.0.1 (0 : un port libre choisi par le système).
    #[arg(long, default_value_t = DEFAULT_PORT)]
    port: u16,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Mentors du dossier courant : livrés, personnels et du projet.
    Mentors {
        #[command(subcommand)]
        action: MentorsAction,
    },
    /// Rejoue les cas d'évaluation avec les vrais modèles (consomme du budget).
    Eval {
        /// Dossier des cas : evals/ du dépôt Hodegos.
        #[arg(default_value = "evals")]
        dir: PathBuf,
        /// Seulement les cas dont le nom contient ce texte.
        #[arg(long = "case")]
        only: Option<String>,
        /// Modèle rapide (Mentor du langage), à la place de config.toml.
        #[arg(long)]
        fast: Option<String>,
        /// Modèle fort (Architecte), à la place de config.toml.
        #[arg(long)]
        strong: Option<String>,
        /// Même modèle pour les deux niveaux ; --fast et --strong l'emportent.
        #[arg(long)]
        model: Option<String>,
        /// Dépense maximale de l'évaluation, en dollars.
        #[arg(long, default_value_t = 3.0)]
        max_cost: f64,
        /// Dossier des rapports (par défaut <dir>/reports).
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
enum MentorsAction {
    /// Affiche les mentors actifs, leur origine, et les fichiers en erreur.
    List,
    /// Copie un mentor dans vos mentors personnels, prêt à être modifié.
    Eject {
        /// Identifiant du mentor, par exemple architecte.
        id: String,
        /// Copier dans .hodegos/mentors/ du dossier courant plutôt que dans vos mentors personnels.
        #[arg(long)]
        project: bool,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    // Par défaut : les messages de Hodegos en info, ceux des bibliothèques (hyper,
    // reqwest…) à partir de warn. Les commandes affichent elles-mêmes leurs résultats.
    // RUST_LOG remplace ce réglage (README, « Journaux »).
    let default_level = if cli.command.is_some() {
        "error"
    } else {
        "warn,hodegos=info,hodegos_core=info,hodegos_mentors=info,hodegos_llm=info,hodegos_server=info"
    };
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_level)),
        )
        .with_writer(std::io::stderr)
        .with_ansi(std::io::stderr().is_terminal())
        .init();

    let result = match (cli.command, cli.projet) {
        (Some(Command::Mentors { action }), _) => mentors(action),
        (
            Some(Command::Eval {
                dir,
                only,
                fast,
                strong,
                model,
                max_cost,
                out,
            }),
            _,
        ) => match evaluate(eval::EvalArgs {
            dir,
            only,
            fast,
            strong,
            model,
            max_cost,
            out,
        }) {
            // Un modèle non retenu : code de sortie 1, le rapport dit pourquoi.
            Ok(false) => return ExitCode::FAILURE,
            other => other.map(|_| ()),
        },
        (None, Some(projet)) => run(projet, cli.port),
        (None, None) => Err(anyhow::anyhow!("indiquez le dossier du projet")),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("hodegos : {err:#}");
            ExitCode::FAILURE
        }
    }
}

#[tokio::main]
async fn run(projet: PathBuf, port: u16) -> anyhow::Result<()> {
    let project = Project::open(&projet)?;
    let root = project.root().to_path_buf();
    let config_dir = dirs::config_dir()
        .context("dossier de configuration introuvable")?
        .join("hodegos");
    let data_dir = dirs::data_dir()
        .context("dossier de données introuvable")?
        .join("hodegos");
    let options = ServerOptions {
        port,
        mentors: MentorSources::for_project(&root),
        data_dir,
        llm: llm(&config_dir),
        initial_file: None,
    };
    if let Err(message) = &options.llm.client {
        tracing::warn!(%message, "aucune passe tant que le fournisseur n'est pas configuré");
    }
    let server = Server::bind(project, options)
        .await
        .context("impossible de démarrer le serveur")?;

    println!("Hodegos est prêt pour {}", root.display());
    println!("Ouvrez cette adresse dans votre navigateur (Ctrl+clic) :");
    println!("{}", server.access_url());
    println!("Ctrl+C pour arrêter.");

    server.serve(shutdown_signal()).await?;
    tracing::info!("Hodegos arrêté");
    Ok(())
}

#[tokio::main]
async fn evaluate(args: eval::EvalArgs) -> anyhow::Result<bool> {
    let config_dir = dirs::config_dir()
        .context("dossier de configuration introuvable")?
        .join("hodegos");
    let data_dir = dirs::data_dir()
        .context("dossier de données introuvable")?
        .join("hodegos");
    eval::command(args, &config_dir, &data_dir).await
}

async fn shutdown_signal() {
    if let Err(e) = tokio::signal::ctrl_c().await {
        tracing::error!(error = %e, "impossible d'écouter Ctrl+C");
        std::future::pending::<()>().await;
    }
}

fn mentors(action: MentorsAction) -> anyhow::Result<()> {
    let project = std::env::current_dir().context("dossier courant illisible")?;
    let sources = MentorSources::for_project(&project);
    let registry = Registry::load(&sources);
    match action {
        MentorsAction::List => {
            list(&registry, &sources);
            Ok(())
        }
        MentorsAction::Eject {
            id,
            project: in_project,
        } => {
            let dest = if in_project {
                sources.project_dir.clone()
            } else {
                default_user_dir()
            }
            .context("dossier de destination introuvable")?;
            let path = eject(&registry, &id, &dest)?;
            println!("Copié dans {}", path.display());
            println!("Modifiez ce fichier : il remplace le mentor « {id} » dès la passe suivante.");
            Ok(())
        }
    }
}

fn list(registry: &Registry, sources: &MentorSources) {
    let dir = |d: &Option<PathBuf>| {
        d.as_ref()
            .map_or_else(|| "(aucun)".to_owned(), |d| d.display().to_string())
    };
    println!("Mentors personnels : {}", dir(&sources.user_dir));
    println!("Mentors du projet  : {}", dir(&sources.project_dir));
    println!();

    let mentors: Vec<_> = registry.active().collect();
    let id_width = mentors
        .iter()
        .map(|m| m.id.chars().count())
        .max()
        .unwrap_or(0);
    let name_width = mentors
        .iter()
        .map(|m| m.name.chars().count())
        .max()
        .unwrap_or(0);
    println!("Mentors actifs ({}) :", mentors.len());
    for mentor in &mentors {
        println!(
            "  {:id_width$}  {:name_width$}  {:9}  {:13}  {}",
            mentor.id,
            mentor.name,
            mentor.origin.to_string(),
            mentor.mode.as_str(),
            mentor.source_name,
        );
    }
    if !registry.disabled().is_empty() {
        println!();
        println!("Mentors désactivés :");
        for (id, (origin, file)) in registry.disabled() {
            println!("  {id}  désactivé au niveau {origin} par {file}");
        }
    }
    if !registry.warnings().is_empty() {
        println!();
        println!("Fichiers en erreur, ignorés :");
        for warning in registry.warnings() {
            println!("  {} : {}", warning.file, warning.message);
        }
    }
}

/// Client OpenRouter d'après `config.toml` et la clé ; sinon, la raison de son absence,
/// affichée dans la barre d'état. Hodegos démarre dans tous les cas.
fn llm(config_dir: &Path) -> Llm {
    let config = match LlmConfig::load(config_dir) {
        Ok(config) => config,
        Err(err) => {
            return Llm {
                client: Err(format!("configuration invalide : {err}")),
                config: LlmConfig::default(),
            };
        }
    };
    let client = match load_api_key(config_dir, std::env::var("OPENROUTER_API_KEY").ok()) {
        Err(err) => Err(err.to_string()),
        Ok(None) => Err(format!(
            "clé OpenRouter absente : ajoutez-la à {} ou à OPENROUTER_API_KEY",
            config_dir.join("credentials.toml").display()
        )),
        Ok(Some(_)) if config.fast.is_none() && config.strong.is_none() => Err(format!(
            "modèles non configurés : renseignez fast et strong dans {}",
            config_dir.join("config.toml").display()
        )),
        Ok(Some(key)) => OpenRouterClient::new(key, &config.base_url, config.timeout)
            .map(|client| Arc::new(client) as Arc<dyn LlmClient>)
            .map_err(|err| err.to_string()),
    };
    Llm { client, config }
}
