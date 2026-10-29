//! Binaire `hodegos` : lance l'IDE sur un projet et affiche l'URL d'accès ;
//! gère aussi les mentors (`hodegos mentors list`, `hodegos mentors eject <id>`).

use std::io::IsTerminal;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Context;
use clap::{Parser, Subcommand};
use hodegos_core::Project;
use hodegos_mentors::{MentorSources, Registry, default_user_dir, eject};
use hodegos_server::{DEFAULT_PORT, Server};
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
    // Les commandes de mentors affichent elles-mêmes les fichiers en erreur.
    let default_level = if cli.command.is_some() {
        "error"
    } else {
        "info"
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
    let sources = MentorSources::for_project(&root);
    let server = Server::bind(project, sources, port)
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
