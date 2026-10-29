//! Déroulé d'un cas : casting, passe au premier palier, puis « Palier suivant » sur la
//! remarque la plus confiante, avec le fil réel. Mêmes prompts, exclusions, masquage
//! et validation que l'IDE.

use std::time::Instant;

use hodegos_core::secrets::exclusion;
use hodegos_core::{changed_sections, sections};
use hodegos_llm::{
    AskError, ChatMessage, LlmClient, LlmRequest, RemarkKind, RemarkLevel, Role, Rules,
    ask_for_remarks, ask_for_reply, remarks_schema, reply_schema, validate, validate_replies,
};
use hodegos_mentors::{
    Author, CastRequest, FollowUpKind, Forced, Level, ModelTier, Registry, SectionRequest, Thread,
    cast, frontmatter_mentor, section_headings, section_prompt,
};

use super::case::Case;

/// Modèles utilisés pour chaque niveau.
#[derive(Debug, Clone, Default)]
pub struct Models {
    pub fast: Option<String>,
    pub strong: Option<String>,
}

impl Models {
    pub fn get(&self, tier: ModelTier) -> Option<&str> {
        match tier {
            ModelTier::Fast => self.fast.as_deref(),
            ModelTier::Strong => self.strong.as_deref(),
        }
    }
}

pub fn tier_name(tier: ModelTier) -> &'static str {
    match tier {
        ModelTier::Fast => "fast",
        ModelTier::Strong => "strong",
    }
}

/// Ce qu'a donné un palier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Au moins un message affiché.
    Shown,
    /// Le modèle n'a rien proposé, ou tout a été écarté.
    Nothing,
    /// JSON invalide après la relance unique.
    InvalidResponse,
    ProviderError(String),
    /// Le palier précédent n'a rien donné : rien à faire monter.
    NotReached,
    /// `--max-cost` atteint avant ce palier.
    BudgetStopped,
}

/// Message proposé par le modèle, et s'il a été affiché ou écarté (avec le motif).
#[derive(Debug, Clone, PartialEq)]
pub struct Draft {
    pub message: String,
    pub quote: Option<String>,
    pub rejected: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LevelResult {
    pub level: Level,
    pub outcome: Outcome,
    /// Messages affichés (après validation).
    pub shown: Vec<String>,
    /// Tout ce que le modèle a proposé.
    pub drafts: Vec<Draft>,
    /// Brouillons dont la citation figure dans la section (première passe seulement).
    pub valid_quotes: Option<(usize, usize)>,
    /// Brouillons au palier demandé, sur le total.
    pub right_level: (usize, usize),
    pub mentions: bool,
    /// Motifs `must_not` trouvés dans les messages affichés, puis dans les brouillons.
    pub leaks_shown: Vec<String>,
    pub leaks_drafts: Vec<String>,
    pub cost_usd: f64,
    pub duration_ms: u64,
}

impl LevelResult {
    fn skipped(level: Level, outcome: Outcome) -> Self {
        Self {
            level,
            outcome,
            shown: Vec::new(),
            drafts: Vec::new(),
            valid_quotes: None,
            right_level: (0, 0),
            mentions: false,
            leaks_shown: Vec::new(),
            leaks_drafts: Vec::new(),
            cost_usd: 0.0,
            duration_ms: 0,
        }
    }

    /// Un appel est parti vers le modèle.
    pub fn called(&self) -> bool {
        !matches!(self.outcome, Outcome::NotReached | Outcome::BudgetStopped)
    }
}

#[derive(Debug, Clone)]
pub struct CaseResult {
    pub name: String,
    pub title: String,
    pub file: String,
    pub expected_mentor: String,
    /// Titulaire retenu par le casting.
    pub mentor: Option<String>,
    pub tier: Option<ModelTier>,
    pub model: Option<String>,
    /// Le cas n'a pas pu être joué (section introuvable, aucun mentor…).
    pub error: Option<String>,
    pub levels: Vec<LevelResult>,
}

impl CaseResult {
    pub fn casting_ok(&self) -> bool {
        self.mentor.as_deref() == Some(self.expected_mentor.as_str())
    }

    /// Un palier au moins a fait émerger la notion.
    pub fn mentions(&self) -> bool {
        self.levels.iter().any(|l| l.mentions)
    }

    pub fn leaks_drafts(&self) -> usize {
        self.levels.iter().map(|l| l.leaks_drafts.len()).sum()
    }

    pub fn leaks_shown(&self) -> usize {
        self.levels.iter().map(|l| l.leaks_shown.len()).sum()
    }

    pub fn cost_usd(&self) -> f64 {
        self.levels.iter().map(|l| l.cost_usd).sum()
    }

    /// Joué en entier : ni erreur, ni panne, ni arrêt sur le budget.
    pub fn complete(&self) -> bool {
        self.error.is_none()
            && self.levels.iter().all(|l| {
                !matches!(
                    l.outcome,
                    Outcome::ProviderError(_) | Outcome::BudgetStopped
                )
            })
    }
}

/// Coût cumulé de l'évaluation et plafond fixé par `--max-cost`.
pub struct Spend {
    pub max_usd: f64,
    pub used_usd: f64,
    /// Appelé après chaque appel avec son coût (cumul dans `usage.json`).
    pub on_cost: Box<dyn FnMut(f64) + Send>,
}

impl Spend {
    fn add(&mut self, cost: f64) {
        self.used_usd += cost;
        if cost > 0.0 {
            (self.on_cost)(cost);
        }
    }

    fn exhausted(&self) -> bool {
        self.used_usd >= self.max_usd
    }
}

pub async fn run_case(
    client: &dyn LlmClient,
    registry: &Registry,
    models: &Models,
    case: &Case,
    spend: &mut Spend,
) -> CaseResult {
    let mut result = CaseResult {
        name: case.name.clone(),
        title: case.title.clone(),
        file: case.file.clone(),
        expected_mentor: case.expected_mentor.clone(),
        mentor: None,
        tier: None,
        model: None,
        error: None,
        levels: Vec::new(),
    };
    let fail = |mut result: CaseResult, message: String| {
        result.error = Some(message);
        result
    };

    let all = sections(case.language, &case.content);
    let Some(section) = all.iter().find(|s| s.id == case.section) else {
        let ids: Vec<&str> = all.iter().map(|s| s.id.as_str()).collect();
        return fail(
            result,
            format!(
                "section {} introuvable (sections : {})",
                case.section,
                ids.join(", ")
            ),
        );
    };
    if exclusion(&case.file, &case.content).is_some() {
        return fail(result, "fichier sensible : rien n'est envoyé".to_owned());
    }
    let text = section.text(&case.content);
    let casting = cast(
        registry,
        &CastRequest {
            path: &case.file,
            language: case.language.as_str(),
            headings: section_headings(case.language, Some(section)),
            text: &text,
            forced: (case.language == hodegos_core::Language::Markdown)
                .then(|| frontmatter_mentor(&case.content))
                .flatten()
                .map(Forced::Frontmatter),
        },
    );
    result.mentor = casting.titular.clone();
    let Some(mentor) = casting.titular.as_deref().and_then(|id| registry.get(id)) else {
        return fail(result, "aucun mentor pour cette section".to_owned());
    };
    result.tier = Some(mentor.model);
    let Some(model) = models.get(mentor.model) else {
        return fail(
            result,
            format!("modèle « {} » non configuré", tier_name(mentor.model)),
        );
    };
    result.model = Some(model.to_owned());

    // Le fichier est traité comme s'il venait d'être créé : toute la section en ajout.
    let diff = changed_sections(case.language, "", &case.content)
        .into_iter()
        .find(|c| c.section.id == section.id)
        .map(|c| c.diff)
        .unwrap_or_default();

    let mut thread: Vec<String> = Vec::new();
    let mut quote = String::new();
    let mut given: Vec<Level> = Vec::new();
    for (index, &level) in case.levels.iter().enumerate() {
        if spend.exhausted() {
            result
                .levels
                .push(LevelResult::skipped(level, Outcome::BudgetStopped));
            continue;
        }
        if index > 0 && thread.is_empty() {
            result
                .levels
                .push(LevelResult::skipped(level, Outcome::NotReached));
            continue;
        }
        let entries: Vec<(Author, &str)> = thread
            .iter()
            .map(|text| (Author::Mentor, text.as_str()))
            .collect();
        let follow_up = Thread {
            kind: FollowUpKind::NextLevel,
            quote: &quote,
            levels_given: given.clone(),
            entries,
        };
        let prompt = section_prompt(&SectionRequest {
            mentor,
            level,
            path: &case.file,
            language: case.language.as_str(),
            content: &case.content,
            section,
            diff: if index == 0 { &diff } else { "" },
            thread: (index > 0).then_some(&follow_up),
        });
        let (schema_name, schema) = if index == 0 {
            ("remarques", remarks_schema())
        } else {
            ("reponse", reply_schema())
        };
        let request = LlmRequest {
            model: model.to_owned(),
            messages: vec![
                ChatMessage::new(Role::System, prompt.system),
                ChatMessage::new(Role::User, prompt.user),
            ],
            schema_name: schema_name.to_owned(),
            schema,
        };
        let rules = Rules {
            section_text: &text,
            level: remark_level(level),
            max_remarks: if index == 0 {
                usize::from(mentor.max_remarks)
            } else {
                1
            },
        };

        let started = Instant::now();
        let mut level_result = LevelResult::skipped(level, Outcome::Nothing);
        if index == 0 {
            match ask_for_remarks(client, request, &rules).await {
                Ok(asked) => {
                    level_result.cost_usd = asked.cost_usd;
                    level_result.drafts = asked
                        .drafts
                        .iter()
                        .map(|d| Draft {
                            message: d.message.clone(),
                            quote: Some(d.quote.clone()),
                            rejected: (!asked.remarks.contains(d)).then(|| {
                                validate(vec![d.clone()], &rules)
                                    .rejected
                                    .first()
                                    .copied()
                                    .unwrap_or("plafond de remarques")
                            }),
                        })
                        .collect();
                    let valid = asked
                        .drafts
                        .iter()
                        .filter(|d| {
                            (d.kind == RemarkKind::NextStep && d.quote.is_empty())
                                || (!d.quote.is_empty() && text.contains(&d.quote))
                        })
                        .count();
                    level_result.valid_quotes = Some((valid, asked.drafts.len()));
                    level_result.right_level = (
                        asked
                            .drafts
                            .iter()
                            .filter(|d| d.level == rules.level)
                            .count(),
                        asked.drafts.len(),
                    );
                    level_result.shown = asked.remarks.iter().map(|r| r.message.clone()).collect();
                    if let Some(best) = asked.remarks.first() {
                        quote = best.quote.clone();
                        thread.push(best.message.clone());
                        given.push(level);
                    }
                }
                Err(err) => fill_error(&mut level_result, err),
            }
        } else {
            match ask_for_reply(client, request, &rules).await {
                Ok(replied) => {
                    level_result.cost_usd = replied.cost_usd;
                    level_result.drafts = replied
                        .drafts
                        .iter()
                        .map(|d| Draft {
                            message: d.message.clone(),
                            quote: None,
                            rejected: (replied.reply.as_ref() != Some(d)).then(|| {
                                validate_replies(vec![d.clone()], &rules)
                                    .1
                                    .first()
                                    .copied()
                                    .unwrap_or("plafond de remarques")
                            }),
                        })
                        .collect();
                    level_result.right_level = (
                        replied
                            .drafts
                            .iter()
                            .filter(|d| d.level == rules.level)
                            .count(),
                        replied.drafts.len(),
                    );
                    level_result.shown = replied.reply.iter().map(|r| r.message.clone()).collect();
                    match &replied.reply {
                        Some(reply) => {
                            thread.push(reply.message.clone());
                            given.push(level);
                        }
                        // Rien à ce palier : les suivants ne seront pas atteints.
                        None => thread.clear(),
                    }
                }
                Err(err) => fill_error(&mut level_result, err),
            }
        }
        level_result.duration_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
        if level_result.outcome == Outcome::Nothing && !level_result.shown.is_empty() {
            level_result.outcome = Outcome::Shown;
        }
        if matches!(
            level_result.outcome,
            Outcome::InvalidResponse | Outcome::ProviderError(_)
        ) {
            thread.clear();
        }
        level_result.mentions = level_result
            .shown
            .iter()
            .any(|m| matches_any(m, &case.must_mention).is_some());
        level_result.leaks_shown = leaks(
            level_result.shown.iter().map(String::as_str),
            &case.must_not,
        );
        level_result.leaks_drafts = leaks(
            level_result.drafts.iter().map(|d| d.message.as_str()),
            &case.must_not,
        );
        spend.add(level_result.cost_usd);
        result.levels.push(level_result);
    }
    result
}

fn fill_error(level_result: &mut LevelResult, err: AskError) {
    level_result.cost_usd = err.cost_usd();
    level_result.outcome = match err {
        AskError::Provider { error, .. } => Outcome::ProviderError(error.to_string()),
        AskError::InvalidResponse { .. } => Outcome::InvalidResponse,
    };
}

fn remark_level(level: Level) -> RemarkLevel {
    match level {
        Level::Indice => RemarkLevel::Indice,
        Level::Question => RemarkLevel::Question,
        Level::Piste => RemarkLevel::Piste,
        Level::Exemple => RemarkLevel::Exemple,
    }
}

/// Premier motif présent dans `text`, sans tenir compte de la casse.
pub fn matches_any<'a>(text: &str, patterns: &'a [String]) -> Option<&'a str> {
    let text = text.to_lowercase();
    patterns
        .iter()
        .find(|p| text.contains(&p.to_lowercase()))
        .map(String::as_str)
}

/// Motifs `must_not` présents dans au moins un des messages, sans doublon.
fn leaks<'a>(messages: impl Iterator<Item = &'a str>, must_not: &[String]) -> Vec<String> {
    let all: Vec<String> = messages.map(str::to_lowercase).collect();
    must_not
        .iter()
        .filter(|p| all.iter().any(|m| m.contains(&p.to_lowercase())))
        .cloned()
        .collect()
}
