//! Évaluation des mentors (format des cas et seuils dans `evals/README.md`) : rejoue
//! les cas avec les vrais modèles et écrit le rapport. Jouée par `hodegos eval` ;
//! consomme du budget.

pub mod case;
pub mod day;
pub mod project;
pub mod report;
pub mod run;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use hodegos_llm::{BoxFuture, LlmClient, LlmError, LlmRequest, LlmResponse, Tokens};
use hodegos_mentors::Registry;

use self::case::Case;
use self::run::{CaseResult, Models, Spend, run_case};

/// Étape de l'évaluation, pour suivre sa progression.
pub enum Step<'a> {
    /// Le cas `index` (à partir de 1) commence.
    Start { index: usize, case: &'a Case },
    /// Un cas vient de finir : résultats obtenus jusqu'ici.
    Done { results: &'a [CaseResult] },
}

/// Compte les jetons des réponses d'un client.
struct Counting<'a> {
    inner: &'a dyn LlmClient,
    tokens: Mutex<Tokens>,
}

impl Counting<'_> {
    fn take(&self) -> Tokens {
        self.tokens
            .lock()
            .map(|mut tokens| std::mem::take(&mut *tokens))
            .unwrap_or_default()
    }
}

impl LlmClient for Counting<'_> {
    fn complete<'b>(
        &'b self,
        request: &'b LlmRequest,
    ) -> BoxFuture<'b, Result<LlmResponse, LlmError>> {
        Box::pin(async move {
            let response = self.inner.complete(request).await;
            if let (Ok(response), Ok(mut tokens)) = (&response, self.tokens.lock()) {
                tokens.add(response.tokens);
            }
            response
        })
    }
}

/// Joue les cas un par un, en annonçant chaque étape à `progress`.
pub async fn evaluate(
    client: &dyn LlmClient,
    registry: &Registry,
    models: &Models,
    cases: &[Case],
    spend: &mut Spend,
    mut progress: impl FnMut(Step<'_>),
) -> Vec<CaseResult> {
    let counting = Counting {
        inner: client,
        tokens: Mutex::new(Tokens::default()),
    };
    let mut results = Vec::new();
    for (index, case) in cases.iter().enumerate() {
        progress(Step::Start {
            index: index + 1,
            case,
        });
        let mut result = run_case(&counting, registry, models, case, spend).await;
        result.tokens = counting.take();
        results.push(result);
        progress(Step::Done { results: &results });
    }
    results
}

/// Nom de fichier sûr tiré des modèles évalués : `vendeur-modele-1`.
pub fn slug(names: &[&str]) -> String {
    let mut names = names.to_vec();
    names.dedup();
    names
        .join("_")
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect()
}

/// `<dossier>/AAAA-MM-JJ-<nom>.md`, sans reprendre le nom d'un rapport existant.
pub fn report_path(dir: &Path, name: &str) -> std::io::Result<PathBuf> {
    fs::create_dir_all(dir)?;
    let date = jiff::Zoned::now().strftime("%Y-%m-%d").to_string();
    let mut path = dir.join(format!("{date}-{name}.md"));
    let mut n = 2;
    while path.exists() {
        path = dir.join(format!("{date}-{name}-{n}.md"));
        n += 1;
    }
    Ok(path)
}

/// Chemin du rapport des modèles `models` dans `dir`.
pub fn models_report_path(dir: &Path, models: &Models) -> std::io::Result<PathBuf> {
    let names: Vec<&str> = [models.fast.as_deref(), models.strong.as_deref()]
        .into_iter()
        .flatten()
        .collect();
    report_path(dir, &slug(&names))
}

#[cfg(test)]
mod tests;
