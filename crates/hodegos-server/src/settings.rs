//! Réglages personnels (phase 8d) : modèles et effort par niveau, clé d'API, budget.
//! Un niveau dont le modèle ou l'effort change est réévalué sur les cas intégrés,
//! après confirmation du coût estimé (ADR 0007). Rien de tout cela ne va dans le projet.

use std::collections::HashMap;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use hodegos_llm::{
    ApiKey, BudgetConfig, Effort, LlmConfig, ModelInfo, Settings, check_key, fetch_models,
    write_api_key, write_settings,
};
use hodegos_mentors::{MentorSources, ModelTier, Registry};
use jiff::Timestamp;
use serde::Deserialize;

use crate::eval::case::{Case, builtin_cases};
use crate::eval::report::{Verdict, markdown, summarize};
use crate::eval::run::{CaseResult, Models, Spend, first_prompt};
use crate::eval::{Step, evaluate, report_path, slug};
use crate::protocol::{
    BudgetInfo, BudgetSettings, ErrorCode, EstimateBasis, EvalOrigin, EvalResult, KeySource,
    LevelEvals, LevelSetting, ServerMessage, SettingsInfo, Tier,
};
use crate::{AppState, Connect, LlmNow, SettingsSource, client_for};

/// Dépense maximale d'une évaluation lancée depuis l'IDE, en dollars.
pub(crate) const EVAL_MAX_COST: f64 = 3.0;

/// Catalogue relu au-delà de cet âge quand un modèle y manque.
const CATALOG_MAX_AGE: Duration = Duration::from_secs(600);

/// Jetons de sortie supposés par appel, quand aucun passage n'a été mesuré.
const ASSUMED_COMPLETION_TOKENS: u64 = 250;

/// Durée supposée d'un appel, quand aucun passage du niveau n'est connu.
const ASSUMED_CALL_MS: u64 = 4000;

/// Résumé des choix livrés (ADR 0003).
const BUILTIN_RESULTS: &str = include_str!("../../../evals/livres.toml");

/// Refus d'une demande de réglage, renvoyé en `error`.
#[derive(Debug)]
pub(crate) struct Refused {
    pub(crate) code: ErrorCode,
    pub(crate) message: String,
}

fn refused(code: ErrorCode, message: impl Into<String>) -> Refused {
    Refused {
        code,
        message: message.into(),
    }
}

/// Où écrire, et la clé lue (jamais renvoyée au front, jamais journalisée).
pub(crate) struct Source {
    config_dir: PathBuf,
    key: Mutex<Option<ApiKey>>,
    key_from_env: bool,
    connect: Connect,
}

impl Source {
    pub(crate) fn new(source: SettingsSource) -> Self {
        Self {
            config_dir: source.config_dir,
            key: Mutex::new(source.key),
            key_from_env: source.key_from_env,
            connect: source.connect,
        }
    }

    fn key(&self) -> Option<ApiKey> {
        self.key.lock().ok().and_then(|k| k.clone())
    }
}

/// Catalogue des modèles d'OpenRouter, gardé en mémoire.
#[derive(Default)]
pub(crate) struct Catalog {
    models: Mutex<Option<(Instant, HashMap<String, ModelInfo>)>>,
}

impl Catalog {
    pub(crate) fn known(&self, id: &str) -> Option<ModelInfo> {
        self.models
            .lock()
            .ok()?
            .as_ref()
            .and_then(|(_, models)| models.get(id).cloned())
    }

    fn fresh(&self) -> bool {
        self.models
            .lock()
            .ok()
            .and_then(|m| m.as_ref().map(|(at, _)| at.elapsed() < CATALOG_MAX_AGE))
            .unwrap_or(false)
    }

    fn store(&self, models: Vec<ModelInfo>) {
        if let Ok(mut current) = self.models.lock() {
            *current = Some((
                Instant::now(),
                models.into_iter().map(|m| (m.id.clone(), m)).collect(),
            ));
        }
    }
}

/// Relit le catalogue public (sans clé).
pub(crate) async fn refresh_catalog(state: &AppState) -> Result<(), String> {
    let config = state.llm().config;
    match fetch_models(&config.base_url, config.timeout).await {
        Ok(models) => {
            state.catalog.store(models);
            Ok(())
        }
        Err(err) => {
            tracing::warn!(error = %err, "catalogue des modèles injoignable");
            Err(format!("catalogue des modèles injoignable : {err}"))
        }
    }
}

/// Fiche d'un modèle ; `None` s'il n'existe pas chez OpenRouter.
pub(crate) async fn model_info(state: &AppState, id: &str) -> Result<Option<ModelInfo>, String> {
    if let Some(info) = state.catalog.known(id) {
        return Ok(Some(info));
    }
    if !state.catalog.fresh() {
        refresh_catalog(state).await?;
    }
    Ok(state.catalog.known(id))
}

/// Réponse à `model_check`.
pub(crate) async fn model_check(state: &AppState, id: &str) -> Result<ServerMessage, Refused> {
    let info = model_info(state, id.trim())
        .await
        .map_err(|message| refused(ErrorCode::ProviderError, message))?;
    Ok(match info {
        Some(info) => ServerMessage::ModelInfo {
            id: info.id,
            exists: true,
            structured_outputs: info.structured_outputs,
            reasoning: info.reasoning,
            prompt_price: info.prompt_price,
            completion_price: info.completion_price,
        },
        None => ServerMessage::ModelInfo {
            id: id.trim().to_owned(),
            exists: false,
            structured_outputs: false,
            reasoning: false,
            prompt_price: 0.0,
            completion_price: 0.0,
        },
    })
}

fn tier_of(tier: Tier) -> ModelTier {
    match tier {
        Tier::Fast => ModelTier::Fast,
        Tier::Strong => ModelTier::Strong,
    }
}

fn level_setting(config: &LlmConfig, tier: Tier) -> Option<LevelSetting> {
    let (model, effort) = match tier {
        Tier::Fast => (config.fast.clone(), config.fast_effort),
        Tier::Strong => (config.strong.clone(), config.strong_effort),
    };
    model.map(|model| LevelSetting { model, effort })
}

/* ---------- Résultats connus ---------- */

#[derive(Deserialize)]
struct BuiltinFile {
    result: Vec<EvalResult>,
}

fn builtin_results() -> Vec<EvalResult> {
    toml::from_str::<BuiltinFile>(BUILTIN_RESULTS)
        .map(|f| f.result)
        .unwrap_or_else(|err| {
            tracing::error!(error = %err, "evals/livres.toml illisible");
            Vec::new()
        })
}

fn evals_dir(state: &AppState) -> PathBuf {
    state.data_dir.join("evals")
}

fn results_path(state: &AppState) -> PathBuf {
    evals_dir(state).join("results.json")
}

/// Évaluations faites depuis l'IDE, de la plus ancienne à la plus récente.
fn ide_results(state: &AppState) -> Vec<EvalResult> {
    fs::read_to_string(results_path(state))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn save_result(state: &AppState, result: &EvalResult) -> std::io::Result<()> {
    let mut all = ide_results(state);
    all.push(result.clone());
    let path = results_path(state);
    let dir = evals_dir(state);
    fs::create_dir_all(&dir)?;
    let mut file = tempfile::NamedTempFile::new_in(&dir)?;
    file.write_all(serde_json::to_string_pretty(&all)?.as_bytes())?;
    file.persist(path).map_err(|e| e.error)?;
    Ok(())
}

/// Dernier résultat connu d'un réglage : évaluation de l'IDE, sinon choix livrés.
fn known_result(state: &AppState, tier: Tier, setting: &LevelSetting) -> Option<EvalResult> {
    let matches =
        |r: &EvalResult| r.level == tier && r.model == setting.model && r.effort == setting.effort;
    ide_results(state)
        .into_iter()
        .rev()
        .find(|r| matches(r) && !r.partial)
        .or_else(|| builtin_results().into_iter().find(matches))
}

/* ---------- Message `settings` ---------- */

pub(crate) fn info(state: &AppState) -> ServerMessage {
    let config = state.llm().config;
    let budget = state.budget.state(Timestamp::now());
    let budget_config = state.budget.config();
    let (key_source, key_hint) = match &state.settings {
        Some(source) => match source.key() {
            Some(key) if source.key_from_env => (KeySource::Env, Some(key.hint())),
            Some(key) => (KeySource::File, Some(key.hint())),
            None => (KeySource::None, None),
        },
        None => (KeySource::None, None),
    };
    let last = |tier| level_setting(&config, tier).and_then(|s| known_result(state, tier, &s));
    ServerMessage::Settings(Box::new(SettingsInfo {
        fast: config.fast.clone(),
        strong: config.strong.clone(),
        fast_effort: config.fast_effort,
        strong_effort: config.strong_effort,
        budget: BudgetInfo {
            monthly_cap_usd: budget_config.monthly_cap_usd,
            warn_at: budget_config.warn_at,
            used_usd: budget.used_usd,
        },
        key_source,
        key_hint,
        editable: state.settings.is_some(),
        config_dir: state
            .settings
            .as_ref()
            .map(|s| s.config_dir.display().to_string()),
        last_eval: LevelEvals {
            fast: last(Tier::Fast),
            strong: last(Tier::Strong),
        },
        eval_running: state.eval_running.lock().ok().and_then(|r| *r),
    }))
}

/// Diffuse les réglages et l'état à toutes les connexions.
fn broadcast(state: &AppState) {
    let _ = state.events.send(info(state));
    let _ = state.events.send(state.status());
}

/* ---------- `settings_set` ---------- */

/// Demande de réglages, telle que reçue.
pub(crate) struct Request {
    pub(crate) fast: String,
    pub(crate) strong: String,
    pub(crate) fast_effort: Effort,
    pub(crate) strong_effort: Effort,
    pub(crate) budget: BudgetSettings,
    pub(crate) confirm_eval: bool,
}

/// Applique les réglages, ou renvoie `eval_required` si un niveau doit être réévalué
/// et que la personne n'a pas encore confirmé.
pub(crate) async fn set(state: &Arc<AppState>, request: Request) -> Result<ServerMessage, Refused> {
    let Some(source) = &state.settings else {
        return Err(refused(
            ErrorCode::InvalidSettings,
            "réglages en lecture seule",
        ));
    };
    let fast = request.fast.trim().to_owned();
    let strong = request.strong.trim().to_owned();
    if fast.is_empty() || strong.is_empty() {
        return Err(refused(
            ErrorCode::InvalidSettings,
            "renseignez un modèle pour chaque niveau",
        ));
    }
    let BudgetSettings {
        monthly_cap_usd,
        warn_at,
    } = request.budget;
    if !monthly_cap_usd.is_finite() || monthly_cap_usd < 0.0 || !(0.0..=1.0).contains(&warn_at) {
        return Err(refused(
            ErrorCode::InvalidSettings,
            "budget : plafond positif, seuil d'avertissement entre 0 et 100 %",
        ));
    }

    let current = state.llm().config;
    let wanted = [
        (
            Tier::Fast,
            LevelSetting {
                model: fast.clone(),
                effort: request.fast_effort,
            },
        ),
        (
            Tier::Strong,
            LevelSetting {
                model: strong.clone(),
                effort: request.strong_effort,
            },
        ),
    ];
    let changed: Vec<(Tier, LevelSetting, Option<LevelSetting>)> = wanted
        .into_iter()
        .filter_map(|(tier, setting)| {
            let before = level_setting(&current, tier);
            (before.as_ref() != Some(&setting)).then_some((tier, setting, before))
        })
        .collect();

    let mut infos = HashMap::new();
    for (_, setting, _) in &changed {
        let info = model_info(state, &setting.model)
            .await
            .map_err(|message| refused(ErrorCode::ProviderError, message))?
            .ok_or_else(|| {
                refused(
                    ErrorCode::ModelRefused,
                    format!("{} : modèle inconnu d'OpenRouter", setting.model),
                )
            })?;
        if !info.structured_outputs {
            return Err(refused(
                ErrorCode::ModelRefused,
                format!(
                    "{} n'accepte pas les sorties structurées : Hodegos ne peut pas valider ses remarques",
                    setting.model
                ),
            ));
        }
        if setting.effort != Effort::Default && !info.reasoning {
            return Err(refused(
                ErrorCode::ModelRefused,
                format!(
                    "{} ne propose pas de réglage d'effort : gardez « Par défaut »",
                    setting.model
                ),
            ));
        }
        infos.insert(setting.model.clone(), info);
    }

    // Un réglage déjà évalué et retenu (retour à l'ancien réglage) ne l'est pas de nouveau.
    let to_eval: Vec<&(Tier, LevelSetting, Option<LevelSetting>)> = changed
        .iter()
        .filter(|(tier, setting, _)| !known_result(state, *tier, setting).is_some_and(|r| r.passed))
        .collect();
    if to_eval.len() > 1 {
        return Err(refused(
            ErrorCode::InvalidSettings,
            "changez un niveau à la fois : chacun demande sa propre évaluation",
        ));
    }
    if let Some((tier, setting, _)) = to_eval.first() {
        if state.eval_running.lock().ok().and_then(|r| *r).is_some() {
            return Err(refused(
                ErrorCode::EvalRunning,
                "une évaluation est déjà en cours ; attendez sa fin",
            ));
        }
        if source.key().is_none() {
            return Err(refused(
                ErrorCode::ProviderError,
                "clé OpenRouter absente : l'évaluation ne peut pas tourner",
            ));
        }
        if !request.confirm_eval {
            let info = infos.get(&setting.model);
            return Ok(estimate(state, *tier, setting, info));
        }
    }

    let config = LlmConfig {
        fast: Some(fast),
        strong: Some(strong),
        fast_effort: request.fast_effort,
        strong_effort: request.strong_effort,
        budget: BudgetConfig {
            monthly_cap_usd,
            warn_at,
        },
        ..current
    };
    write_settings(
        &source.config_dir,
        &Settings {
            fast: request.fast.trim().to_owned(),
            strong: request.strong.trim().to_owned(),
            fast_effort: request.fast_effort,
            strong_effort: request.strong_effort,
            budget: config.budget.clone(),
        },
    )
    .map_err(|err| refused(ErrorCode::Internal, err.to_string()))?;
    apply(state, source, config);
    tracing::info!("réglages des modèles enregistrés");

    if let Some((tier, setting, before)) = to_eval.first() {
        if let Ok(mut running) = state.eval_running.lock() {
            *running = Some(*tier);
        }
        let task_state = state.clone();
        let (tier, setting, before) = (*tier, setting.clone(), before.clone());
        tokio::spawn(async move { run_eval(task_state, tier, setting, before).await });
    }
    broadcast(state);
    Ok(info(state))
}

/// Remplace la configuration en cours et refait le client.
fn apply(state: &AppState, source: &Source, config: LlmConfig) {
    let client = client_for(
        &source.config_dir,
        source.key().as_ref(),
        &config,
        &source.connect,
    );
    match &client {
        Ok(_) => state.set_provider(true, None),
        Err(message) => state.set_provider(false, Some(message.clone())),
    }
    state.budget.set_config(config.budget.clone());
    if let Ok(mut llm) = state.llm.write() {
        *llm = LlmNow { client, config };
    }
}

/* ---------- Estimation ---------- */

/// Cas intégrés joués par un niveau, avec la taille de leur premier prompt.
fn level_cases(registry: &Registry, tier: Tier) -> Vec<(Case, usize)> {
    builtin_cases(None)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|case| {
            let (case_tier, chars) = first_prompt(registry, &case)?;
            (case_tier == tier_of(tier)).then_some((case, chars))
        })
        .collect()
}

fn builtin_registry() -> Registry {
    // Mentors livrés seulement : les résultats ne dépendent pas des surcharges.
    Registry::load(&MentorSources {
        user_dir: None,
        project_dir: None,
    })
}

fn estimate(
    state: &AppState,
    tier: Tier,
    setting: &LevelSetting,
    info: Option<&ModelInfo>,
) -> ServerMessage {
    let registry = builtin_registry();
    let cases = level_cases(&registry, tier);
    let calls: u64 = cases.iter().map(|(c, _)| c.levels.len() as u64).sum();
    let price = |prompt: u64, completion: u64| {
        info.map_or(0.0, |i| {
            prompt as f64 * i.prompt_price + completion as f64 * i.completion_price
        })
    };
    let ide = ide_results(state);
    let builtin = builtin_results();
    let (estimated_cost, estimated_duration_ms, basis) =
        if let Some(same) = known_result(state, tier, setting) {
            (same.cost, same.duration_ms, EstimateBasis::Same)
        } else if let Some(last) = ide
            .iter()
            .rev()
            .find(|r| r.level == tier && r.prompt_tokens > 0)
        {
            (
                price(last.prompt_tokens, last.completion_tokens),
                last.duration_ms,
                EstimateBasis::Tokens,
            )
        } else {
            // Environ quatre caractères par jeton ; chaque palier renvoie le contexte.
            let prompt: u64 = cases
                .iter()
                .map(|(c, chars)| (*chars as u64 / 4) * c.levels.len() as u64)
                .sum();
            let duration = builtin
                .iter()
                .chain(ide.iter())
                .rev()
                .find(|r| r.level == tier)
                .map_or(calls * ASSUMED_CALL_MS, |r| r.duration_ms);
            (
                price(prompt, calls * ASSUMED_COMPLETION_TOKENS),
                duration,
                EstimateBasis::Prompts,
            )
        };
    ServerMessage::EvalRequired {
        level: tier,
        cases: cases.len(),
        estimated_cost,
        estimated_duration_ms,
        max_cost: EVAL_MAX_COST,
        basis,
    }
}

/* ---------- Évaluation ---------- */

/// Joue les cas du niveau avec le nouveau réglage ; les passes continuent pendant ce
/// temps. Le rapport est réécrit après chaque cas : un arrêt laisse un rapport partiel.
async fn run_eval(
    state: Arc<AppState>,
    tier: Tier,
    setting: LevelSetting,
    previous: Option<LevelSetting>,
) {
    let started = Instant::now();
    let registry = builtin_registry();
    let cases: Vec<Case> = level_cases(&registry, tier)
        .into_iter()
        .map(|(case, _)| case)
        .collect();
    let total = cases.len();
    let mut models = Models::default();
    match tier {
        Tier::Fast => {
            models.fast = Some(setting.model.clone());
            models.fast_effort = setting.effort;
        }
        Tier::Strong => {
            models.strong = Some(setting.model.clone());
            models.strong_effort = setting.effort;
        }
    }
    let budget_state = state.clone();
    let mut spend = Spend {
        max_usd: EVAL_MAX_COST,
        used_usd: 0.0,
        // La dépense compte dans le mois, sans être bloquée par son plafond.
        on_cost: Box::new(move |cost| {
            if let Err(err) = budget_state.budget.record(cost, Timestamp::now()) {
                tracing::error!(error = %err, "coût de l'évaluation non enregistré");
            }
        }),
    };
    let report = report_path(
        &evals_dir(&state),
        &slug(&[setting.model.as_str(), setting.effort.as_str()]),
    );
    let when = jiff::Zoned::now().strftime("%Y-%m-%d %H:%M").to_string();
    let write = |path: &Path, results: &[CaseResult], partial: bool| {
        let text = markdown(&when, results, &summarize(results), partial);
        if let Err(err) = fs::write(path, text) {
            tracing::error!(error = %err, "rapport d'évaluation non écrit");
        }
    };

    let results = match (&state.llm().client, &report) {
        (Ok(client), Ok(path)) => {
            let progress_state = state.clone();
            evaluate(
                client.as_ref(),
                &registry,
                &models,
                &cases,
                &mut spend,
                |step| {
                    let (done, cost) = match step {
                        Step::Start { index, .. } => (index - 1, None),
                        Step::Done { results } => {
                            write(path, results, true);
                            (
                                results.len(),
                                Some(results.iter().map(CaseResult::cost_usd).sum()),
                            )
                        }
                    };
                    let _ = progress_state.events.send(ServerMessage::EvalProgress {
                        level: tier,
                        done,
                        total,
                        cost: cost.unwrap_or(0.0),
                    });
                },
            )
            .await
        }
        (Err(message), _) => {
            tracing::warn!(%message, "évaluation impossible");
            Vec::new()
        }
        (_, Err(err)) => {
            tracing::error!(error = %err, "dossier des évaluations non créé");
            Vec::new()
        }
    };
    if let Ok(path) = &report {
        write(path, &results, results.len() < total);
    }

    let summaries = summarize(&results);
    let summary = summaries.first();
    let result = EvalResult {
        level: tier,
        model: setting.model.clone(),
        effort: setting.effort,
        date: jiff::Zoned::now().strftime("%Y-%m-%d").to_string(),
        passed: results.len() == total
            && summaries.len() == 1
            && summary.is_some_and(|s| s.verdict() == Verdict::Retained),
        notion: summary.map_or(0, |s| s.mentions),
        cases: total,
        divulged: summary.map_or(0, |s| s.leaks_drafts),
        cost: results.iter().map(CaseResult::cost_usd).sum(),
        duration_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        median_ms: summary.map_or(0, |s| s.median_ms),
        prompt_tokens: summary.map_or(0, |s| s.tokens.prompt),
        completion_tokens: summary.map_or(0, |s| s.tokens.completion),
        report: report.ok().map(|p| p.display().to_string()),
        origin: EvalOrigin::Ide,
        partial: results.len() < total,
        previous,
    };
    if let Err(err) = save_result(&state, &result) {
        tracing::error!(error = %err, "résultat d'évaluation non enregistré");
    }
    if let Ok(mut running) = state.eval_running.lock() {
        *running = None;
    }
    tracing::info!(
        model = %result.model,
        effort = result.effort.as_str(),
        passed = result.passed,
        "évaluation terminée"
    );
    let _ = state.events.send(ServerMessage::EvalDone(Box::new(result)));
    broadcast(&state);
}

/* ---------- Clé d'API ---------- */

pub(crate) fn set_key(state: &AppState, key: &str) -> Result<ServerMessage, Refused> {
    let Some(source) = &state.settings else {
        return Err(refused(
            ErrorCode::InvalidSettings,
            "réglages en lecture seule",
        ));
    };
    if source.key_from_env {
        return Err(refused(
            ErrorCode::KeyFromEnv,
            "OPENROUTER_API_KEY est définie : elle prime sur credentials.toml",
        ));
    }
    let key = key.trim();
    if key.is_empty() || key.chars().any(char::is_whitespace) {
        return Err(refused(ErrorCode::InvalidSettings, "clé vide ou invalide"));
    }
    let key = ApiKey::new(key);
    write_api_key(&source.config_dir, &key)
        .map_err(|err| refused(ErrorCode::Internal, err.to_string()))?;
    if let Ok(mut current) = source.key.lock() {
        *current = Some(key);
    }
    apply(state, source, state.llm().config);
    tracing::info!("clé OpenRouter enregistrée");
    broadcast(state);
    Ok(info(state))
}

/// Réponse à `provider_test` : `GET /key`, sans génération ni coût.
pub(crate) async fn provider_test(state: &AppState) -> ServerMessage {
    let key = state.settings.as_ref().and_then(Source::key);
    let Some(key) = key else {
        return ServerMessage::ProviderTestResult {
            ok: false,
            message: "Aucune clé enregistrée.".to_owned(),
        };
    };
    let config = state.llm().config;
    match check_key(&config.base_url, &key, config.timeout).await {
        Ok(()) => ServerMessage::ProviderTestResult {
            ok: true,
            message: "Connexion réussie.".to_owned(),
        },
        Err(err) => ServerMessage::ProviderTestResult {
            ok: false,
            message: err.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_resume_des_choix_livres_se_lit() {
        let results = builtin_results();
        assert_eq!(results.len(), 2);
        assert!(
            results
                .iter()
                .all(|r| r.passed && r.origin == EvalOrigin::Builtin)
        );
        assert_eq!(results[0].level, Tier::Fast);
        assert_eq!(results[1].level, Tier::Strong);
    }

    #[test]
    fn chaque_niveau_a_ses_cas() {
        let registry = builtin_registry();
        let fast = level_cases(&registry, Tier::Fast);
        let strong = level_cases(&registry, Tier::Strong);
        assert_eq!(
            fast.len() + strong.len(),
            builtin_cases(None).unwrap().len()
        );
        assert!(
            fast.len() >= 15 && strong.len() >= 5,
            "{} / {}",
            fast.len(),
            strong.len()
        );
        assert!(fast.iter().all(|(_, chars)| *chars > 500));
    }
}
