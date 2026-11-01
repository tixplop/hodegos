//! Binaire `hodegos` : lance l'IDE sur un projet et affiche l'URL d'accès ;
//! crée un projet (`hodegos new <chemin>`), gère les mentors (`hodegos mentors
//! list`, `hodegos mentors eject <id>`) et évalue les mentors avec les vrais modèles
//! (`hodegos eval`, sur les cas intégrés ou ceux d'un dossier).

mod eval;
mod new;

use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use anyhow::Context;
use clap::{Parser, Subcommand};
use hodegos_core::Project;
use hodegos_llm::{ApiKey, LlmClient, LlmConfig, OpenRouterClient, load_api_key};
use hodegos_mentors::{
    ContextScope, Mentor, MentorSources, Registry, Scope, default_user_dir, eject,
};
use hodegos_server::{
    Connect, DEFAULT_PORT, Llm, Server, ServerOptions, SettingsSource, SystemClock,
    TerminalOptions, client_for, load_binds, load_context, load_review_options,
};
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
    /// Crée un projet et sa vision (docs/vision.md), puis l'ouvre dans l'IDE.
    New {
        /// Dossier à créer, dans le système de fichiers Linux (~/…) ; vide s'il existe.
        chemin: PathBuf,
        /// Ne pas lancer git init.
        #[arg(long)]
        no_git: bool,
        /// Créer le projet sans lancer l'IDE.
        #[arg(long)]
        no_open: bool,
        /// Port d'écoute sur 127.0.0.1 (0 : un port libre choisi par le système).
        #[arg(long, default_value_t = DEFAULT_PORT)]
        port: u16,
    },
    /// Mentors du dossier courant : livrés, personnels et du projet.
    Mentors {
        #[command(subcommand)]
        action: MentorsAction,
    },
    /// Rejoue les cas d'évaluation avec les vrais modèles (consomme du budget).
    Eval {
        /// Dossier des cas (evals/ du dépôt Hodegos) ; par défaut, les cas intégrés.
        dir: Option<PathBuf>,
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
        /// Dossier des rapports (par défaut <dir>/reports, ou ~/.local/share/hodegos/evals
        /// pour les cas intégrés).
        #[arg(long)]
        out: Option<PathBuf>,
        /// Mentors à placer par-dessus les mentors livrés, comme des mentors personnels
        /// (par exemple evals/mentors/ pour évaluer ceux de la phase 9 avant de les livrer).
        #[arg(long)]
        mentors: Option<PathBuf>,
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
    let serves = matches!(
        cli.command,
        None | Some(Command::New { no_open: false, .. })
    );
    let default_level = if !serves {
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
            Some(Command::New {
                chemin,
                no_git,
                no_open,
                port,
            }),
            _,
        ) => new::create(&chemin, !no_git).and_then(|root| {
            if no_open {
                Ok(())
            } else {
                run(root, port, Some(new::VISION.to_owned()))
            }
        }),
        (
            Some(Command::Eval {
                dir,
                only,
                fast,
                strong,
                model,
                max_cost,
                out,
                mentors,
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
            mentors,
        }) {
            // Un modèle non retenu : code de sortie 1, le rapport dit pourquoi.
            Ok(false) => return ExitCode::FAILURE,
            other => other.map(|_| ()),
        },
        (None, Some(projet)) => run(projet, cli.port, None),
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
async fn run(projet: PathBuf, port: u16, initial_file: Option<String>) -> anyhow::Result<()> {
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
        terminal: {
            let (binds, warning) = load_binds(&config_dir);
            if let Some(message) = &warning {
                tracing::warn!(%message, "montages du terminal");
            }
            TerminalOptions::from_env(binds, warning)
        },
        context: {
            let (context, warning) = load_context(&config_dir);
            if let Some(message) = &warning {
                tracing::warn!(%message, "plafonds de contexte");
            }
            context
        },
        review: {
            let (review, warning) = load_review_options(&config_dir);
            if let Some(message) = &warning {
                tracing::warn!(%message, "revues du projet");
            }
            review
        },
        clock: Arc::new(SystemClock),
        initial_file,
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
    println!("Mentors du dépôt   : {}", dir(&sources.project_dir));
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
            "  {:id_width$}  {:name_width$}  {:9}  {:13}  {:38}  {}",
            mentor.id,
            mentor.name,
            mentor.origin.to_string(),
            mentor.mode.as_str(),
            scopes(mentor),
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

/// Portées et contexte d'un mentor (phase 9) : « section, projet · contexte voisinage ».
fn scopes(mentor: &Mentor) -> String {
    let scopes: Vec<&str> = mentor
        .scopes
        .iter()
        .map(|s| match s {
            Scope::Section => "section",
            Scope::Project => "projet",
        })
        .collect();
    let mut line = scopes.join(", ");
    if mentor.scopes.contains(&Scope::Section) {
        line.push_str(match mentor.context {
            ContextScope::Section => " · contexte section",
            ContextScope::Neighborhood => " · contexte voisinage",
            ContextScope::Project => " · contexte projet",
        });
    }
    line
}

/// Client OpenRouter d'après `config.toml` et la clé ; sinon, la raison de son absence,
/// affichée dans la barre d'état. Hodegos démarre dans tous les cas, et les réglages de
/// l'IDE peuvent corriger la configuration ou la clé.
fn llm(config_dir: &Path) -> Llm {
    let connect: Connect = Arc::new(|key: &ApiKey, config: &LlmConfig| {
        OpenRouterClient::new(key.clone(), &config.base_url, config.timeout)
            .map(|client| Arc::new(client) as Arc<dyn LlmClient>)
            .map_err(|err| err.to_string())
    });
    let env_key = std::env::var("OPENROUTER_API_KEY")
        .ok()
        .filter(|k| !k.trim().is_empty());
    let key_from_env = env_key.is_some();
    let (key, key_error) = match load_api_key(config_dir, env_key) {
        Ok(key) => (key, None),
        Err(err) => (None, Some(err.to_string())),
    };
    let (config, client) = match LlmConfig::load(config_dir) {
        Ok(config) => {
            let client = match &key_error {
                Some(message) => Err(message.clone()),
                None => client_for(config_dir, key.as_ref(), &config, &connect),
            };
            (config, client)
        }
        Err(err) => (
            LlmConfig::default(),
            Err(format!("configuration invalide : {err}")),
        ),
    };
    Llm {
        client,
        config,
        settings: Some(SettingsSource {
            config_dir: config_dir.to_path_buf(),
            key,
            key_from_env,
            connect,
        }),
    }
}

#[cfg(test)]
mod tests {
    use hodegos_mentors::{Origin, Parsed, parse};

    use super::*;

    fn mentor(extra: &str) -> Mentor {
        let text = format!("---\nid: m\nname: M\ninitials: MM\nrole: R\n{extra}---\nPosture.\n");
        match parse(&text, Origin::User, "m.md").unwrap() {
            Parsed::Active(m) => *m,
            Parsed::Disabled { .. } => unreachable!(),
        }
    }

    #[test]
    fn portees_et_contexte_dans_la_liste() {
        assert_eq!(scopes(&mentor("")), "section · contexte section");
        assert_eq!(
            scopes(&mentor(
                "scopes: [section, project]\ncontext: neighborhood\n"
            )),
            "section, projet · contexte voisinage"
        );
        assert_eq!(scopes(&mentor("scopes: [project]\n")), "projet");
    }
}
