//! `hodegos eval [dossier]` : rejoue les cas avec les vrais modèles et écrit le rapport
//! (format des cas et seuils dans `evals/README.md`). Manuel : consomme du budget.
//! Sans dossier, les cas intégrés au binaire.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, bail};
use hodegos_llm::{Budget, LlmConfig, OpenRouterClient, load_api_key};
use hodegos_mentors::{MentorSources, Registry};
use hodegos_server::eval::case::{builtin_cases, load_cases};
use hodegos_server::eval::report::{self, Verdict, markdown, summarize, terminal};
use hodegos_server::eval::run::{Models, Spend};
use hodegos_server::eval::{Step, evaluate, models_report_path};
use jiff::Timestamp;

/// Options de `hodegos eval`.
#[derive(Debug, Clone)]
pub struct EvalArgs {
    /// Dossier des cas ; `None` : cas intégrés.
    pub dir: Option<PathBuf>,
    /// Garde les cas dont le nom contient ce texte.
    pub only: Option<String>,
    pub fast: Option<String>,
    pub strong: Option<String>,
    /// Même modèle pour les deux niveaux ; `fast` et `strong` l'emportent.
    pub model: Option<String>,
    pub max_cost: f64,
    /// Dossier des rapports ; par défaut `<dir>/reports`.
    pub out: Option<PathBuf>,
}

/// Joue l'évaluation ; `Ok(true)` si chaque modèle évalué est retenu.
pub async fn command(args: EvalArgs, config_dir: &Path, data_dir: &Path) -> anyhow::Result<bool> {
    let cases = match &args.dir {
        Some(dir) => load_cases(dir, args.only.as_deref())?,
        None => builtin_cases(args.only.as_deref())?,
    };
    let config = LlmConfig::load(config_dir).context("configuration invalide")?;
    let models = Models {
        fast: args
            .fast
            .clone()
            .or_else(|| args.model.clone())
            .or(config.fast.clone()),
        strong: args
            .strong
            .clone()
            .or_else(|| args.model.clone())
            .or(config.strong.clone()),
    };
    if models.fast.is_none() && models.strong.is_none() {
        bail!(
            "aucun modèle : passez --model, --fast ou --strong, ou renseignez {}",
            config_dir.join("config.toml").display()
        );
    }
    let Some(key) = load_api_key(config_dir, std::env::var("OPENROUTER_API_KEY").ok())? else {
        bail!(
            "clé OpenRouter absente : ajoutez-la à {} ou à OPENROUTER_API_KEY",
            config_dir.join("credentials.toml").display()
        );
    };
    let client = OpenRouterClient::new(key, &config.base_url, config.timeout)?;
    // Mentors livrés seulement : les résultats ne dépendent pas de vos surcharges.
    let registry = Registry::load(&MentorSources {
        user_dir: None,
        project_dir: None,
    });
    // L'évaluation est une vraie dépense : elle compte dans le budget du mois, sans
    // être bloquée par son plafond (seul --max-cost l'arrête).
    let budget = Arc::new(Budget::open(
        data_dir.join("usage.json"),
        config.budget.clone(),
    ));
    let mut spend = Spend {
        max_usd: args.max_cost,
        used_usd: 0.0,
        on_cost: Box::new(move |cost| {
            if let Err(err) = budget.record(cost, Timestamp::now()) {
                tracing::error!(error = %err, "coût de l'évaluation non enregistré");
            }
        }),
    };

    println!(
        "Évaluation de {} cas · fast = {} · strong = {} · plafond {}",
        cases.len(),
        models.fast.as_deref().unwrap_or("—"),
        models.strong.as_deref().unwrap_or("—"),
        report::money(args.max_cost)
    );
    let out_dir = match (&args.out, &args.dir) {
        (Some(out), _) => out.clone(),
        (None, Some(dir)) => dir.join("reports"),
        (None, None) => data_dir.join("evals"),
    };
    let path = models_report_path(&out_dir, &models)
        .with_context(|| format!("{} : dossier non créé", out_dir.display()))?;
    let when = jiff::Zoned::now().strftime("%Y-%m-%d %H:%M").to_string();
    // Réécrit après chaque cas : un arrêt en cours laisse un rapport partiel.
    let write = |results: &[_], partial: bool| {
        let text = markdown(&when, results, &summarize(results), partial);
        fs::write(&path, text).with_context(|| format!("{} : rapport non écrit", path.display()))
    };
    let mut written = Ok(());
    let results = evaluate(
        &client,
        &registry,
        &models,
        &cases,
        &mut spend,
        |step| match step {
            Step::Start { index, case } => println!("  [{index}/{}] {}", cases.len(), case.name),
            Step::Done { results } => {
                if written.is_ok() {
                    written = write(results, true);
                }
            }
        },
    )
    .await;
    written?;
    write(&results, false)?;

    let summaries = summarize(&results);
    println!();
    print!("{}", terminal(&results, &summaries));
    println!();
    println!("Rapport : {}", path.display());
    Ok(summaries.iter().all(|s| s.verdict() == Verdict::Retained))
}
