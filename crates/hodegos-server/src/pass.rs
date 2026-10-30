//! Passe proactive après un enregistrement, ou à la première ouverture d'un fichier
//! dont un titulaire a `on_open` : sections, casting, exclusion des secrets, budget,
//! appel au titulaire, validation et ancrage des remarques.

use std::sync::Arc;
use std::time::Instant;

use hodegos_core::secrets::exclusion;
use hodegos_core::{Language, RelPath, Section, changed_sections, sections};
use hodegos_llm::{
    AskError, BudgetLevel, ChatMessage, LlmRequest, RemarkDraft, RemarkLevel, Role, Rules,
    ask_for_remarks, remarks_schema,
};
use hodegos_mentors::{
    CastRequest, Casting, Forced, Level, Mentor, ModelTier, Registry, SectionRequest, cast,
    frontmatter_mentor, section_headings, section_prompt,
};
use jiff::Timestamp;
use tokio::task::JoinSet;

use crate::AppState;
use crate::protocol::{Anchor, MentorRef, PassOutcome, Remark, ServerMessage, ThreadEntry};

/// Sections soumises aux mentors par passe, au plus.
const MAX_SECTIONS: usize = 2;
/// Sections soumises par une passe d'ouverture, au plus.
const MAX_OPEN_SECTIONS: usize = 8;
/// Remarques gardées par passe, au plus, toutes sections confondues.
const MAX_REMARKS: usize = 2;

/// Ce qui déclenche une passe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Trigger {
    /// Enregistrement : les sections modifiées, deux au plus.
    Save,
    /// Première ouverture : toutes les sections dont le titulaire a `on_open`, dans
    /// l'ordre du fichier, le fichier étant traité comme créé.
    Open,
}

impl Trigger {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Save => "enregistrement",
            Self::Open => "ouverture",
        }
    }
}

/// Ce qu'un enregistrement ou une ouverture apporte à la passe.
pub(crate) struct PassRequest {
    pub(crate) path: RelPath,
    pub(crate) language: Language,
    /// Contenu précédent ; vide pour une passe d'ouverture.
    pub(crate) old: String,
    pub(crate) new: String,
    pub(crate) cursor_line: Option<usize>,
    pub(crate) trigger: Trigger,
}

/// Appel à préparer pour une section.
struct Job {
    mentor_id: String,
    section: Section,
    /// Texte réel de la section : chaque citation doit s'y trouver.
    section_text: String,
    level: RemarkLevel,
    max_remarks: usize,
    request: LlmRequest,
}

/// Passe prête à partir, ou terminée sans appel au modèle.
pub(crate) enum Prepared {
    Done(PassOutcome),
    Run(Box<Run>),
}

pub(crate) struct Run {
    pub(crate) started: ServerMessage,
    path: RelPath,
    trigger: Trigger,
    jobs: Vec<Job>,
}

/// Résultat d'une passe, renvoyé à la connexion qui l'a lancée.
pub(crate) struct PassResult {
    pub(crate) path: RelPath,
    pub(crate) trigger: Trigger,
    pub(crate) generation: u64,
    pub(crate) remarks: Vec<Remark>,
    pub(crate) outcome: PassOutcome,
    pub(crate) duration_ms: u64,
    pub(crate) cost_usd: f64,
}

/// Casting d'une section : choix forcé de l'utilisateur, sinon frontmatter, sinon score.
pub(crate) fn cast_section(
    state: &AppState,
    registry: &Registry,
    path: &RelPath,
    language: Language,
    content: &str,
    section: Option<&Section>,
) -> Casting {
    let text = section.map(|s| s.text(content)).unwrap_or_default();
    let section_id = section.map_or("", |s| s.id.as_str());
    let forced = state
        .mentors
        .forced(path.as_str(), section_id)
        .map(Forced::User)
        .or_else(|| {
            (language == Language::Markdown)
                .then(|| frontmatter_mentor(content))
                .flatten()
                .map(Forced::Frontmatter)
        });
    cast(
        registry,
        &CastRequest {
            path: path.as_str(),
            language: language.as_str(),
            headings: section_headings(language, section),
            text: &text,
            forced,
        },
    )
}

pub(crate) fn mentor_ref(registry: &Registry, id: &str) -> Option<MentorRef> {
    registry.get(id).map(|m| MentorRef {
        id: m.id.clone(),
        name: m.name.clone(),
        initials: m.initials.clone(),
        role: m.role.clone(),
    })
}

/// Une passe d'ouverture est due : un titulaire d'au moins une section a `on_open`,
/// aucune remarque n'a encore été faite sur le fichier, et aucune passe d'ouverture
/// n'y a abouti.
pub(crate) fn wants_open_pass(
    state: &AppState,
    registry: &Registry,
    path: &RelPath,
    language: Language,
    content: &str,
) -> bool {
    if state.store.has_remarks(path.as_str()) || state.store.was_opened(path.as_str()) {
        return false;
    }
    sections(language, content)
        .iter()
        .any(|section| open_titular(state, registry, path, language, content, section).is_some())
}

/// Titulaire de la section, s'il intervient à l'ouverture.
fn open_titular(
    state: &AppState,
    registry: &Registry,
    path: &RelPath,
    language: Language,
    content: &str,
    section: &Section,
) -> Option<Mentor> {
    let casting = cast_section(state, registry, path, language, content, Some(section));
    registry
        .get(casting.titular.as_deref()?)
        .filter(|m| m.on_open)
        .cloned()
}

/// Décide, sans appel au modèle, si la passe a lieu et prépare ses appels.
pub(crate) fn prepare(state: &AppState, registry: &Registry, request: &PassRequest) -> Prepared {
    let path = request.path.as_str();
    if exclusion(path, &request.new).is_some() {
        tracing::info!(path, "fichier sensible : rien n'est envoyé au modèle");
        return Prepared::Done(PassOutcome::Excluded);
    }
    let mut changes = changed_sections(request.language, &request.old, &request.new);
    if changes.is_empty() {
        return Prepared::Done(PassOutcome::NoChange);
    }

    let cast_changes: Vec<_> = match request.trigger {
        Trigger::Save => {
            // La section du curseur d'abord, puis les plus modifiées.
            changes.sort_by_key(|c| {
                let at_cursor = request
                    .cursor_line
                    .is_some_and(|line| c.section.contains(line));
                (!at_cursor, std::cmp::Reverse(c.changed_lines))
            });
            changes
                .into_iter()
                .filter_map(|change| {
                    let casting = cast_section(
                        state,
                        registry,
                        &request.path,
                        request.language,
                        &request.new,
                        Some(&change.section),
                    );
                    let mentor = registry.get(casting.titular.as_deref()?)?.clone();
                    Some((change, mentor))
                })
                .take(MAX_SECTIONS)
                .collect()
        }
        Trigger::Open => changes
            .into_iter()
            .filter_map(|change| {
                let mentor = open_titular(
                    state,
                    registry,
                    &request.path,
                    request.language,
                    &request.new,
                    &change.section,
                )?;
                Some((change, mentor))
            })
            .take(MAX_OPEN_SECTIONS)
            .collect(),
    };
    if cast_changes.is_empty() {
        return Prepared::Done(PassOutcome::NoMentor);
    }

    if let Err(message) = &state.llm.client {
        state.set_provider(false, Some(message.clone()));
        return Prepared::Done(PassOutcome::ProviderError);
    }
    if state.budget.state(Timestamp::now()).level == BudgetLevel::Exceeded {
        return Prepared::Done(PassOutcome::BudgetExceeded);
    }

    let mut jobs = Vec::new();
    for (change, mentor) in cast_changes {
        let Some(model) = model_for(state, &mentor) else {
            let tier = match mentor.model {
                ModelTier::Fast => "fast",
                ModelTier::Strong => "strong",
            };
            state.set_provider(
                false,
                Some(format!("modèle « {tier} » non configuré dans config.toml")),
            );
            return Prepared::Done(PassOutcome::ProviderError);
        };
        let section = change.section;
        let prompt = section_prompt(&SectionRequest {
            mentor: &mentor,
            level: mentor.entry_level,
            path,
            language: request.language.as_str(),
            content: &request.new,
            section: &section,
            diff: &change.diff,
            thread: None,
        });
        jobs.push(Job {
            mentor_id: mentor.id.clone(),
            section_text: section.text(&request.new),
            level: remark_level(mentor.entry_level),
            max_remarks: usize::from(mentor.max_remarks),
            request: LlmRequest {
                model,
                messages: vec![
                    ChatMessage::new(Role::System, prompt.system),
                    ChatMessage::new(Role::User, prompt.user),
                ],
                schema_name: "remarques".to_owned(),
                schema: remarks_schema(),
            },
            section,
        });
    }

    let started = ServerMessage::PassStarted {
        path: request.path.to_string(),
        sections: jobs.iter().map(|j| j.section.id.clone()).collect(),
        mentors: jobs
            .iter()
            .filter_map(|j| mentor_ref(registry, &j.mentor_id))
            .collect(),
    };
    Prepared::Run(Box::new(Run {
        started,
        path: request.path.clone(),
        trigger: request.trigger,
        jobs,
    }))
}

pub(crate) fn model_for(state: &AppState, mentor: &Mentor) -> Option<String> {
    match mentor.model {
        ModelTier::Fast => state.llm.config.fast.clone(),
        ModelTier::Strong => state.llm.config.strong.clone(),
    }
}

pub(crate) fn remark_level(level: Level) -> RemarkLevel {
    match level {
        Level::Indice => RemarkLevel::Indice,
        Level::Question => RemarkLevel::Question,
        Level::Piste => RemarkLevel::Piste,
        Level::Exemple => RemarkLevel::Exemple,
    }
}

/// Appelle les mentors en parallèle, compte le coût, garde les meilleures remarques.
pub(crate) async fn run(
    state: Arc<AppState>,
    run: Box<Run>,
    generation: u64,
    started: Instant,
) -> PassResult {
    let Ok(client) = state.llm.client.clone() else {
        return finish(
            run.path,
            run.trigger,
            generation,
            Vec::new(),
            PassOutcome::ProviderError,
            0.0,
            started,
        );
    };
    let mut tasks = JoinSet::new();
    for job in run.jobs {
        let client = client.clone();
        tasks.spawn(async move {
            let rules = Rules {
                section_text: &job.section_text,
                level: job.level,
                max_remarks: job.max_remarks,
            };
            let result = ask_for_remarks(client.as_ref(), job.request.clone(), &rules).await;
            (job.mentor_id, job.section, job.section_text, result)
        });
    }

    let mut found: Vec<(String, Section, String, RemarkDraft)> = Vec::new();
    let mut provider_error = None;
    let mut invalid = 0;
    let mut answered = 0;
    let mut cost = 0.0;
    while let Some(joined) = tasks.join_next().await {
        let Ok((mentor_id, section, text, result)) = joined else {
            continue;
        };
        match result {
            Ok(asked) => {
                cost += asked.cost_usd;
                answered += 1;
                for remark in asked.remarks {
                    found.push((mentor_id.clone(), section.clone(), text.clone(), remark));
                }
            }
            Err(err) => {
                cost += err.cost_usd();
                match err {
                    AskError::Provider { error, .. } => provider_error = Some(error.to_string()),
                    AskError::InvalidResponse { .. } => invalid += 1,
                }
            }
        }
    }
    if cost > 0.0 {
        if let Err(err) = state.budget.record(cost, Timestamp::now()) {
            tracing::error!(error = %err, "coût de la passe non enregistré");
        }
    }
    match &provider_error {
        Some(message) => {
            tracing::warn!(%message, "passe abandonnée : erreur du fournisseur");
            state.set_provider(false, Some(message.clone()));
        }
        None if answered > 0 => state.set_provider(true, None),
        None => {}
    }

    found.sort_by(|a, b| b.3.confidence.total_cmp(&a.3.confidence));
    found.truncate(MAX_REMARKS);
    let now = Timestamp::now().to_string();
    let remarks = found
        .into_iter()
        .map(|(mentor_id, section, text, draft)| {
            anchor(&run.path, mentor_id, &section, &text, draft, &now)
        })
        .collect();

    let outcome = if provider_error.is_some() {
        PassOutcome::ProviderError
    } else if answered == 0 && invalid > 0 {
        PassOutcome::InvalidResponse
    } else {
        PassOutcome::Ok
    };
    finish(
        run.path,
        run.trigger,
        generation,
        remarks,
        outcome,
        cost,
        started,
    )
}

fn finish(
    path: RelPath,
    trigger: Trigger,
    generation: u64,
    remarks: Vec<Remark>,
    outcome: PassOutcome,
    cost_usd: f64,
    started: Instant,
) -> PassResult {
    PassResult {
        path,
        trigger,
        generation,
        remarks,
        outcome,
        cost_usd,
        duration_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
    }
}

/// Ligne (à partir de 1) de `quote` dans la section ; début de la section pour une
/// citation vide, `None` si la citation n'y est plus.
fn line_of(section: &Section, section_text: &str, quote: &str) -> Option<usize> {
    if quote.is_empty() {
        return Some(section.start_line);
    }
    section_text
        .find(quote)
        .map(|index| section.start_line + section_text[..index].matches('\n').count())
}

/// Ligne de l'ancre dans la version enregistrée, ou `None` si sa section ou sa
/// citation n'existe plus : la remarque est alors détachée.
pub(crate) fn locate_anchor(sections: &[Section], content: &str, anchor: &Anchor) -> Option<usize> {
    let section = sections.iter().find(|s| s.id == anchor.section_id)?;
    line_of(section, &section.text(content), &anchor.quote)
}

/// Remarque complète, ancrée sur sa section et sa citation.
fn anchor(
    path: &RelPath,
    mentor_id: String,
    section: &Section,
    section_text: &str,
    draft: RemarkDraft,
    now: &str,
) -> Remark {
    let line_hint = line_of(section, section_text, &draft.quote).unwrap_or(section.start_line);
    let level = draft.level.as_str().to_owned();
    Remark {
        id: format!("rm_{}", ulid::Ulid::generate()),
        mentor_id,
        path: path.to_string(),
        anchor: Anchor {
            section_id: section.id.clone(),
            quote: draft.quote,
            line_hint,
        },
        level: level.clone(),
        kind: draft.kind.as_str().to_owned(),
        levels_given: vec![level],
        status: "open".to_owned(),
        thread: vec![ThreadEntry {
            author: "mentor".to_owned(),
            text: draft.message,
            at: now.to_owned(),
        }],
        created_at: now.to_owned(),
    }
}
