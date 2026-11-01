//! Réévaluation d'une remarque de projet dont un passage a disparu (phase 9d,
//! ADR 0010) : son mentor la relit à l'échelle du projet et la maintient avec de
//! nouvelles citations (`keep`) ou la lève en disant pourquoi (`withdraw`).

use std::sync::Arc;

use hodegos_core::Language;
use hodegos_core::secrets::mask_secrets;
use hodegos_core::strip_html_comments;
use hodegos_llm::{
    AskError, BudgetLevel, ChatMessage, LlmRequest, ProjectRules, Reevaluated, Role,
    ask_for_reevaluation, reevaluation_schema,
};
use hodegos_mentors::{
    Author, FileText, FollowUp, FollowUpKind, Level, ReevaluationInput, Scope, reevaluation_prompt,
};
use jiff::Timestamp;

use crate::AppState;
use crate::pass::{model_for, remark_level};
use crate::protocol::{ProjectAnchor, ProjectRemark, ThreadEntry};
use crate::review::{key_material, locate};

/// Issue d'une réévaluation.
#[derive(Debug, PartialEq)]
pub(crate) enum Outcome {
    /// Remarque maintenue ou levée : la voici à jour.
    Done(Box<ProjectRemark>),
    /// Rien n'a changé (verdict écarté, réponse illisible, panne) : elle reste à
    /// réévaluer, à la revue suivante du mentor.
    Retry,
    /// Rien n'est parti (mentor inactif, budget, fournisseur).
    Skipped,
}

/// Réévalue la remarque `id` (statut `to_review`).
pub(crate) async fn reevaluate(state: Arc<AppState>, id: String) -> Outcome {
    let Some(remark) = state
        .project_store
        .get(&id)
        .filter(|r| r.status == "to_review")
    else {
        return Outcome::Skipped;
    };
    let registry = state.mentors.registry();
    let Some(mentor) = registry
        .get(&remark.mentor_id)
        .filter(|m| m.scopes.contains(&Scope::Project))
        .cloned()
    else {
        return Outcome::Skipped;
    };
    let Ok(client) = state.llm().client else {
        return Outcome::Skipped;
    };
    if state.budget.state(Timestamp::now()).level == BudgetLevel::Exceeded {
        return Outcome::Skipped;
    }
    let Some((model, effort)) = model_for(&state, &mentor) else {
        return Outcome::Skipped;
    };

    let worker = state.clone();
    let anchors = remark.anchors.clone();
    let prepared_mentor = mentor.clone();
    let prepared = tokio::task::spawn_blocking(move || {
        let (map, keys) = key_material(&worker, &prepared_mentor)?;
        let cited: Vec<(String, String, String)> = anchors
            .iter()
            .filter(|a| a.status == "ok")
            .filter_map(|a| {
                let (section, text, _) = locate(&worker, &a.path, &a.quote)?;
                let text = if Language::of(&a.path) == Language::Markdown {
                    strip_html_comments(&text)
                } else {
                    text
                };
                Some((
                    a.path.clone(),
                    format!("section {}", section.id),
                    mask_secrets(&text).0,
                ))
            })
            .collect();
        Some((map, keys, cited))
    })
    .await
    .ok()
    .flatten();
    let Some((map, keys, cited)) = prepared else {
        return Outcome::Retry;
    };

    let gone: Vec<(String, String)> = remark
        .anchors
        .iter()
        .filter(|a| a.status == "gone")
        .map(|a| (a.path.clone(), mask_secrets(&a.quote).0))
        .collect();
    let entries: Vec<(Author, String)> = remark
        .thread
        .iter()
        .map(|e| {
            let author = if e.author == "user" {
                Author::Person
            } else {
                Author::Mentor
            };
            (author, mask_secrets(&e.text).0)
        })
        .collect();
    let level = Level::from_name(&remark.level).unwrap_or(Level::Indice);
    let quote = gone
        .iter()
        .map(|(_, q)| q.as_str())
        .collect::<Vec<_>>()
        .join(" · ");
    let follow_up = FollowUp {
        kind: FollowUpKind::Reply,
        quote: &quote,
        levels_given: remark
            .levels_given
            .iter()
            .filter_map(|l| Level::from_name(l))
            .collect(),
        entries: entries.iter().map(|(a, t)| (*a, t.as_str())).collect(),
    };
    let prompt = reevaluation_prompt(&ReevaluationInput {
        mentor: &mentor,
        level,
        map: &map,
        key_files: keys
            .files
            .iter()
            .map(|f| FileText {
                path: &f.path,
                form: &f.form,
                text: &f.text,
            })
            .collect(),
        cited: cited
            .iter()
            .map(|(path, form, text)| FileText { path, form, text })
            .collect(),
        gone: gone.iter().map(|(p, q)| (p.as_str(), q.as_str())).collect(),
        follow_up: &follow_up,
    });
    let request = LlmRequest {
        model,
        messages: vec![
            ChatMessage::new(Role::System, prompt.system),
            ChatMessage::new(Role::User, prompt.user),
        ],
        schema_name: "reevaluation".to_owned(),
        schema: reevaluation_schema(),
        effort,
    };
    let mut sent: Vec<(String, String)> = keys
        .files
        .iter()
        .map(|f| (f.path.clone(), f.text.clone()))
        .collect();
    sent.extend(cited.iter().map(|(p, _, t)| (p.clone(), t.clone())));
    let sent_refs: Vec<(&str, &str)> = sent.iter().map(|(p, t)| (p.as_str(), t.as_str())).collect();
    let finder = |path: &str, quote: &str| locate(&state, path, quote).map(|(_, text, _)| text);
    let rules = ProjectRules {
        level: remark_level(level),
        max_remarks: 1,
        sent: &sent_refs,
        map: &map,
        locate: &finder,
    };
    let result = ask_for_reevaluation(client.as_ref(), request, &rules).await;
    let cost = match &result {
        Ok((_, cost)) => *cost,
        Err(err) => err.cost_usd(),
    };
    if cost > 0.0
        && let Err(err) = state.budget.record(cost, Timestamp::now())
    {
        tracing::error!(error = %err, "coût de la réévaluation non enregistré");
    }
    let verdict = match result {
        Ok((Ok(verdict), _)) => {
            state.set_provider(true, None);
            verdict
        }
        Ok((Err(reason), _)) => {
            tracing::info!(remarque = %id, reason, "verdict de réévaluation écarté");
            return Outcome::Retry;
        }
        Err(AskError::Provider { error, .. }) => {
            state.set_provider(false, Some(error.to_string()));
            return Outcome::Retry;
        }
        Err(AskError::InvalidResponse { .. }) => return Outcome::Retry,
    };
    let now = state.clock.now().to_string();
    let anchors: Vec<ProjectAnchor> = match &verdict {
        Reevaluated::Keep(quotes) => quotes
            .iter()
            .filter_map(|q| {
                let (section, _, line) = locate(&state, &q.path, &q.quote)?;
                Some(ProjectAnchor {
                    path: q.path.clone(),
                    section_id: section.id,
                    section_name: section.name,
                    quote: q.quote.clone(),
                    line_hint: line,
                    status: "ok".to_owned(),
                })
            })
            .collect(),
        Reevaluated::Withdraw(_) => Vec::new(),
    };
    tracing::info!(
        remarque = %id,
        verdict = if matches!(verdict, Reevaluated::Keep(_)) { "keep" } else { "withdraw" },
        cout_usd = cost,
        "réévaluation d'une remarque de projet"
    );
    let updated = state.project_store.update(&id, |remark| {
        if remark.status != "to_review" {
            return;
        }
        match verdict {
            Reevaluated::Keep(_) => {
                remark.anchors = anchors;
                let answered = remark.thread.last().is_some_and(|e| e.author == "user");
                remark.status = if answered { "answered" } else { "open" }.to_owned();
            }
            Reevaluated::Withdraw(reason) => {
                remark.thread.push(ThreadEntry {
                    author: "mentor".to_owned(),
                    text: reason,
                    at: now,
                });
                remark.status = "resolved".to_owned();
            }
        }
    });
    match updated {
        Some(remark) => Outcome::Done(Box::new(remark)),
        None => Outcome::Skipped,
    }
}
