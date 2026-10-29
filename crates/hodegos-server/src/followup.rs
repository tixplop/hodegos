//! Reprise d'une remarque par son mentor : palier suivant (`hint_next`) ou relance au
//! même palier après une réponse (`remark_reply`). Mêmes exclusions, masquage, budget
//! et garde « pas de solution » que la passe proactive.

use std::sync::Arc;

use hodegos_core::secrets::{exclusion, mask_secrets};
use hodegos_core::{RelPath, Session, sections};
use hodegos_llm::{
    AskError, BudgetLevel, ChatMessage, LlmRequest, Role, Rules, ask_for_reply, reply_schema,
};
use hodegos_mentors::{
    Author, FollowUp, FollowUpKind, Level, ModelTier, PromptInput, Registry, build_prompt,
};
use jiff::Timestamp;

use crate::AppState;
use crate::pass::{context, lines, locate_anchor, model_for, remark_level};
use crate::protocol::{ErrorCode, Remark, ThreadEntry};

/// Demande refusée avant ou pendant l'appel au modèle.
#[derive(Debug, Clone)]
pub(crate) struct Refused {
    pub(crate) code: ErrorCode,
    pub(crate) message: String,
}

impl Refused {
    fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

/// Appel prêt à partir pour une remarque.
pub(crate) struct Job {
    pub(crate) remark_id: String,
    kind: FollowUpKind,
    level: Level,
    /// Texte réel de la section, pour la garde « pas de solution ».
    section_text: String,
    request: LlmRequest,
}

/// Issue d'une reprise : le message ajouté par le mentor (`None` s'il n'a rien
/// ajouté ou si sa réponse a été écartée), ou un refus.
pub(crate) struct Done {
    pub(crate) remark_id: String,
    kind: FollowUpKind,
    level: Level,
    pub(crate) outcome: Result<Option<String>, Refused>,
}

/// Vérifie que la reprise est possible et construit l'appel, sur la version
/// enregistrée du fichier. Au-delà du budget, la demande n'est faite que si
/// l'utilisateur l'a confirmée.
pub(crate) fn prepare(
    state: &AppState,
    registry: &Registry,
    session: &Session,
    remark: &Remark,
    kind: FollowUpKind,
    confirm_over_budget: bool,
) -> Result<Job, Refused> {
    if !remark.is_active() {
        return Err(Refused::new(
            ErrorCode::BadMessage,
            "cette remarque est fermée ou détachée",
        ));
    }
    let current = Level::from_name(&remark.level)
        .ok_or_else(|| Refused::new(ErrorCode::BadMessage, "palier de la remarque inconnu"))?;
    let level = match kind {
        FollowUpKind::NextLevel => current.next().ok_or_else(|| {
            Refused::new(
                ErrorCode::BadMessage,
                "dernier palier atteint : il n'y a rien après l'exemple",
            )
        })?,
        FollowUpKind::Reply => current,
    };
    let mentor = registry.get(&remark.mentor_id).ok_or_else(|| {
        Refused::new(
            ErrorCode::NotFound,
            format!("le mentor {} n'est plus actif", remark.mentor_id),
        )
    })?;
    let path = RelPath::parse(&remark.path)
        .map_err(|err| Refused::new(ErrorCode::PathForbidden, err.to_string()))?;
    let file = state
        .project
        .read_file(session, &path)
        .map_err(|err| Refused::new(ErrorCode::NotFound, err.to_string()))?;
    if exclusion(path.as_str(), &file.content).is_some() {
        return Err(Refused::new(
            ErrorCode::BadMessage,
            "fichier sensible : rien n'est envoyé au modèle",
        ));
    }
    let all_sections = sections(file.language, &file.content);
    let section = all_sections
        .iter()
        .find(|s| s.id == remark.anchor.section_id)
        .filter(|_| locate_anchor(&all_sections, &file.content, &remark.anchor).is_some())
        .ok_or_else(|| {
            Refused::new(
                ErrorCode::BadMessage,
                "le passage cité n'existe plus : la remarque est détachée",
            )
        })?;

    if let Err(message) = &state.llm.client {
        state.set_provider(false, Some(message.clone()));
        return Err(Refused::new(ErrorCode::ProviderError, message.clone()));
    }
    if state.budget.state(Timestamp::now()).level == BudgetLevel::Exceeded && !confirm_over_budget {
        return Err(Refused::new(
            ErrorCode::BudgetExceeded,
            "budget du mois atteint : confirmez pour envoyer cette demande",
        ));
    }
    let Some(model) = model_for(state, mentor) else {
        let tier = match mentor.model {
            ModelTier::Fast => "fast",
            ModelTier::Strong => "strong",
        };
        let message = format!("modèle « {tier} » non configuré dans config.toml");
        state.set_provider(false, Some(message.clone()));
        return Err(Refused::new(ErrorCode::ProviderError, message));
    };

    let (masked, _) = mask_secrets(&file.content);
    let masked_lines: Vec<&str> = masked.split('\n').collect();
    // Une réponse de la personne peut contenir un secret collé par mégarde.
    let masked_thread: Vec<(Author, String)> = remark
        .thread
        .iter()
        .map(|entry| {
            let author = if entry.author == "user" {
                Author::Person
            } else {
                Author::Mentor
            };
            (author, mask_secrets(&entry.text).0)
        })
        .collect();
    let (quote, _) = mask_secrets(&remark.anchor.quote);
    let follow_up = FollowUp {
        kind,
        quote: &quote,
        levels_given: remark
            .levels_given
            .iter()
            .filter_map(|l| Level::from_name(l))
            .collect(),
        entries: masked_thread
            .iter()
            .map(|(author, text)| (*author, text.as_str()))
            .collect(),
    };
    let prompt = build_prompt(&PromptInput {
        mentor,
        level,
        path: path.as_str(),
        language: file.language.as_str(),
        section_id: &section.id,
        section_text: &lines(&masked_lines, section.start_line, section.end_line),
        section_diff: "",
        context: &context(&masked_lines, section),
        follow_up: Some(&follow_up),
    });
    Ok(Job {
        remark_id: remark.id.clone(),
        kind,
        level,
        section_text: section.text(&file.content),
        request: LlmRequest {
            model,
            messages: vec![
                ChatMessage::new(Role::System, prompt.system),
                ChatMessage::new(Role::User, prompt.user),
            ],
            schema_name: "reponse".to_owned(),
            schema: reply_schema(),
        },
    })
}

/// Appelle le mentor, compte le coût et met à jour l'état du fournisseur. Une panne
/// n'est jamais relancée.
pub(crate) async fn run(state: Arc<AppState>, job: Job) -> Done {
    let done = |outcome| Done {
        remark_id: job.remark_id.clone(),
        kind: job.kind,
        level: job.level,
        outcome,
    };
    let client = match &state.llm.client {
        Ok(client) => client.clone(),
        Err(message) => return done(Err(Refused::new(ErrorCode::ProviderError, message.clone()))),
    };
    let rules = Rules {
        section_text: &job.section_text,
        level: remark_level(job.level),
        max_remarks: 1,
    };
    let result = ask_for_reply(client.as_ref(), job.request.clone(), &rules).await;
    let cost = match &result {
        Ok(replied) => replied.cost_usd,
        Err(err) => err.cost_usd(),
    };
    if cost > 0.0 {
        if let Err(err) = state.budget.record(cost, Timestamp::now()) {
            tracing::error!(error = %err, "coût de la demande non enregistré");
        }
    }
    let outcome = match result {
        Ok(replied) => {
            state.set_provider(true, None);
            if replied.reply.is_none() {
                tracing::info!(rejetees = ?replied.rejected, "le mentor n'ajoute rien au fil");
            }
            Ok(replied.reply.map(|r| r.message))
        }
        Err(AskError::Provider { error, .. }) => {
            let message = error.to_string();
            tracing::warn!(%message, "demande abandonnée : erreur du fournisseur");
            state.set_provider(false, Some(message.clone()));
            Err(Refused::new(ErrorCode::ProviderError, message))
        }
        Err(AskError::InvalidResponse { .. }) => Ok(None),
    };
    done(outcome)
}

/// Ajoute au fil le message du mentor, si la remarque est toujours active ; renvoie
/// son état courant.
pub(crate) fn apply(state: &AppState, done: &Done, message: Option<&str>) -> Option<Remark> {
    let Some(message) = message else {
        return state.store.get(&done.remark_id);
    };
    let now = Timestamp::now().to_string();
    state.store.update(&done.remark_id, |remark| {
        if !remark.is_active() {
            tracing::debug!(id = %remark.id, "remarque fermée entre-temps, réponse ignorée");
            return;
        }
        remark.thread.push(ThreadEntry {
            author: "mentor".to_owned(),
            text: message.to_owned(),
            at: now,
        });
        if done.kind == FollowUpKind::NextLevel {
            let level = done.level.as_str().to_owned();
            remark.level = level.clone();
            remark.levels_given.push(level);
        }
    })
}
