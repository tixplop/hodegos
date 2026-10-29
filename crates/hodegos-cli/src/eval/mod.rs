//! `hodegos eval evals/` : rejoue les cas avec les vrais modèles et écrit le rapport
//! (format des cas et seuils dans `evals/README.md`). Manuel : consomme du budget.

pub mod case;
pub mod report;
pub mod run;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, bail};
use hodegos_llm::{Budget, LlmClient, LlmConfig, OpenRouterClient, load_api_key};
use hodegos_mentors::{MentorSources, Registry};
use jiff::Timestamp;

use self::case::{Case, load_cases};
use self::report::{Verdict, markdown, summarize, terminal};
use self::run::{CaseResult, Models, Spend, run_case};

/// Options de `hodegos eval`.
#[derive(Debug, Clone)]
pub struct EvalArgs {
    pub dir: PathBuf,
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
    let cases = load_cases(&args.dir, args.only.as_deref())?;
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
    let results = evaluate(
        &client,
        &registry,
        &models,
        &cases,
        &mut spend,
        |done, case| {
            println!("  [{done}/{}] {}", cases.len(), case.name);
        },
    )
    .await;

    let summaries = summarize(&results);
    let when = jiff::Zoned::now().strftime("%Y-%m-%d %H:%M").to_string();
    let out_dir = args.out.clone().unwrap_or_else(|| args.dir.join("reports"));
    let path = write_report(&out_dir, &models, &markdown(&when, &results, &summaries))?;
    println!();
    print!("{}", terminal(&results, &summaries));
    println!();
    println!("Rapport : {}", path.display());
    Ok(summaries.iter().all(|s| s.verdict() == Verdict::Retained))
}

/// Joue les cas un par un ; `progress` est appelé avant chaque cas.
pub async fn evaluate(
    client: &dyn LlmClient,
    registry: &Registry,
    models: &Models,
    cases: &[Case],
    spend: &mut Spend,
    mut progress: impl FnMut(usize, &Case),
) -> Vec<CaseResult> {
    let mut results = Vec::new();
    for (index, case) in cases.iter().enumerate() {
        progress(index + 1, case);
        results.push(run_case(client, registry, models, case, spend).await);
    }
    results
}

/// `<dossier>/AAAA-MM-JJ-<modèles>.md`, sans écraser un rapport du même jour.
fn write_report(dir: &Path, models: &Models, text: &str) -> anyhow::Result<PathBuf> {
    fs::create_dir_all(dir).with_context(|| format!("{} : dossier non créé", dir.display()))?;
    let mut names: Vec<&str> = [models.fast.as_deref(), models.strong.as_deref()]
        .into_iter()
        .flatten()
        .collect();
    names.dedup();
    let slug: String = names
        .join("_")
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let date = jiff::Zoned::now().strftime("%Y-%m-%d").to_string();
    let mut path = dir.join(format!("{date}-{slug}.md"));
    let mut n = 2;
    while path.exists() {
        path = dir.join(format!("{date}-{slug}-{n}.md"));
        n += 1;
    }
    fs::write(&path, text).with_context(|| format!("{} : rapport non écrit", path.display()))?;
    Ok(path)
}

#[cfg(test)]
mod tests;
