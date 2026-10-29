//! Binaire `hodegos` : lance l'IDE sur un projet et affiche l'URL d'accès.

use std::io::IsTerminal;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Context;
use clap::Parser;
use hodegos_core::Project;
use hodegos_server::{DEFAULT_PORT, Server};
use tracing_subscriber::EnvFilter;

/// IDE personnel où des mentors IA vous guident sans écrire à votre place.
#[derive(Debug, Parser)]
#[command(name = "hodegos", version)]
struct Cli {
    /// Dossier du projet à ouvrir, dans le système de fichiers Linux (~/…).
    projet: PathBuf,

    /// Port d'écoute sur 127.0.0.1 (0 : un port libre choisi par le système).
    #[arg(long, default_value_t = DEFAULT_PORT)]
    port: u16,
}

fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .with_ansi(std::io::stderr().is_terminal())
        .init();

    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("hodegos : {err:#}");
            ExitCode::FAILURE
        }
    }
}

#[tokio::main]
async fn run(cli: Cli) -> anyhow::Result<()> {
    let project = Project::open(&cli.projet)?;
    let root = project.root().to_path_buf();
    let server = Server::bind(project, cli.port)
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
