//! Revues du projet (phase 9c, ADR 0010) : estimation, revue demandée par un ou
//! plusieurs mentors de projet, remarques de projet et leurs reprises (palier suivant,
//! relance, fermeture). Une revue ne part que sur demande en 9c ; les déclencheurs
//! automatiques viennent en 9d.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use hodegos_core::map::{
    FileChange, KeyFiles, MAX_DIFF_FILES, diff_since, estimate_tokens, key_files, masked_texts,
};
use hodegos_core::secrets::mask_secrets;
use hodegos_core::{Language, RelPath, Section, UserId, section_at, sections, strip_html_comments};
use hodegos_llm::{
    AskError, BudgetLevel, ChatMessage, LlmRequest, ProjectRemarkDraft, ProjectRules, Role, Rules,
    ask_for_project_remarks, ask_for_reply, project_remarks_schema, reply_schema,
};
use hodegos_mentors::{
    Author, FileDiff, FileText, FollowUp, FollowUpKind, Level, Mentor, OpenRemark,
    ProjectFollowUpInput, ReviewInput, Scope, project_follow_up_prompt, review_prompt,
};
use jiff::Timestamp;
use tokio::task::JoinSet;

use crate::AppState;
use crate::pass::{model_for, remark_level};
use crate::project_map;
use crate::project_store::{LastCost, Manifest};
use crate::protocol::{
    AnyRemark, ContextItem, ErrorCode, MentorEstimate, ProjectAnchor, ProjectMentorInfo,
    ProjectRemark, ReviewInfo, ReviewMentorInfo, ServerMessage, ThreadEntry,
};
use crate::settings::{Refused, model_info};

/// Durée supposée d'une revue sans mesure précédente.
const DEFAULT_DURATION_MS: u64 = 20_000;
/// Jetons de réponse supposés, pour estimer le coût d'après le prix du modèle.
const ANSWER_TOKENS: f64 = 800.0;

/// Revues en cours et reprises de remarques de projet.
#[derive(Default)]
pub(crate) struct ReviewState {
    /// Mentor → génération de sa revue en cours ; une nouvelle demande la rend caduque.
    running: Mutex<HashMap<String, u64>>,
    generation: AtomicU64,
    /// Remarques de projet qui attendent leur mentor.
    busy: Mutex<HashSet<String>>,
}

impl ReviewState {
    /// Mentors dont une revue est en cours.
    pub(crate) fn running_ids(&self) -> HashSet<String> {
        self.running
            .lock()
            .map(|r| r.keys().cloned().collect())
            .unwrap_or_default()
    }
}

fn refused(code: ErrorCode, message: impl Into<String>) -> Refused {
    Refused {
        code,
        message: message.into(),
    }
}

/// Ce qui part pour un mentor, prêt à être envoyé.
struct Job {
    mentor: Mentor,
    /// `project.max_remarks` du mentor.
    max_remarks: usize,
    request: LlmRequest,
    tokens: usize,
    /// Textes envoyés par fichier (fichiers clés, diff).
    sent: Vec<(String, String)>,
    map: String,
    /// Instantané relu, noté si la revue aboutit.
    now: BTreeMap<String, String>,
    items: Vec<ContextItem>,
    detail: String,
}

/// Mentors de projet actifs désignés par `ids`.
fn mentors(state: &AppState, ids: &[String]) -> Result<Vec<Mentor>, Refused> {
    if ids.is_empty() {
        return Err(refused(ErrorCode::BadMessage, "aucun mentor choisi"));
    }
    let registry = state.mentors.registry();
    ids.iter()
        .map(|id| {
            registry
                .get(id)
                .filter(|m| m.scopes.contains(&Scope::Project))
                .cloned()
                .ok_or_else(|| {
                    refused(
                        ErrorCode::NotFound,
                        format!("« {id} » n'est pas un mentor de projet actif"),
                    )
                })
        })
        .collect()
}

fn files(count: usize, one: &str, many: &str) -> String {
    if count == 1 {
        format!("1 {one}")
    } else {
        format!("{count} {many}")
    }
}

/// Prépare la revue d'un mentor : carte, fichiers clés, diff depuis sa dernière revue,
/// ses remarques ouvertes. Bloquant.
fn prepare(state: &AppState, mentor: Mentor) -> Result<Job, Refused> {
    let session = state.project.session(UserId::local());
    let material = project_map::with_map(state, |map, map_tokens, cache| {
        let keys = key_files(
            &state.project,
            &session,
            cache,
            &|path| mentor.is_key_file(path),
            state.map.key_files_cap,
        );
        (
            map.to_owned(),
            map_tokens,
            keys,
            masked_texts(&state.project, &session, cache),
        )
    });
    let Some((map, map_tokens, keys, now)) = material else {
        return Err(refused(ErrorCode::Internal, "carte du projet indisponible"));
    };
    let diff: Vec<FileChange> = match state.project_store.manifest(&mentor.id) {
        Some(manifest) => diff_since(&state.snapshots.load(&manifest.files), &now, MAX_DIFF_FILES),
        None => Vec::new(),
    };
    let first = state.project_store.manifest(&mentor.id).is_none();
    let open = state.project_store.open_of(&mentor.id);
    let Some((model, effort)) = model_for(state, &mentor) else {
        return Err(refused(
            ErrorCode::ProviderError,
            format!(
                "modèle « {} » non configuré dans config.toml",
                mentor.model.as_str()
            ),
        ));
    };
    let prompt = review_prompt(&ReviewInput {
        mentor: &mentor,
        level: mentor.entry_level,
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
        diff: diff
            .iter()
            .map(|d| FileDiff {
                path: &d.path,
                diff: &d.diff,
            })
            .collect(),
        first,
        open_remarks: open
            .iter()
            .map(|r| OpenRemark {
                message: r.thread.first().map_or("", |e| e.text.as_str()),
                quotes: r
                    .anchors
                    .iter()
                    .map(|a| (a.path.as_str(), a.quote.as_str()))
                    .collect(),
            })
            .collect(),
    });
    let tokens = estimate_tokens(&prompt.system) + estimate_tokens(&prompt.user);
    let items = items(map_tokens, &keys, &diff, first, open.len());
    let detail = format!(
        "carte ({map_tokens} jetons), {}{}",
        files(keys.files.len(), "fichier clé", "fichiers clés"),
        if first {
            ", première revue".to_owned()
        } else if diff.is_empty() {
            ", aucun changement depuis la dernière revue".to_owned()
        } else {
            format!(", diff de {}", files(diff.len(), "fichier", "fichiers"))
        }
    );
    let mut sent: Vec<(String, String)> = keys
        .files
        .iter()
        .map(|f| (f.path.clone(), f.text.clone()))
        .collect();
    sent.extend(diff.iter().map(|d| (d.path.clone(), d.diff.clone())));
    let max_remarks = mentor.project.as_ref().map_or(3, |p| p.max_remarks);
    Ok(Job {
        request: LlmRequest {
            model,
            messages: vec![
                ChatMessage::new(Role::System, prompt.system),
                ChatMessage::new(Role::User, prompt.user),
            ],
            schema_name: "revue".to_owned(),
            schema: project_remarks_schema(),
            effort,
        },
        tokens,
        sent,
        map,
        now,
        items,
        detail,
        max_remarks: usize::try_from(max_remarks).unwrap_or(3),
        mentor,
    })
}

/// Ce qui est parti, bloc par bloc, sans les contenus.
fn items(
    map_tokens: usize,
    keys: &KeyFiles,
    diff: &[FileChange],
    first: bool,
    open: usize,
) -> Vec<ContextItem> {
    let mut items = vec![ContextItem {
        block: "map".to_owned(),
        detail: "Carte du projet".to_owned(),
        tokens: map_tokens,
    }];
    let mut detail = format!(
        "Fichiers clés : {}",
        files(keys.files.len(), "fichier", "fichiers")
    );
    if keys.skipped > 0 {
        detail.push_str(&format!(
            ", {} de plus laissés faute de place",
            keys.skipped
        ));
    }
    items.push(ContextItem {
        block: "key_files".to_owned(),
        detail,
        tokens: keys.tokens,
    });
    for file in &keys.files {
        items.push(ContextItem {
            block: "key_file".to_owned(),
            detail: format!("{} — {}", file.path, file.form),
            tokens: file.tokens,
        });
    }
    items.push(ContextItem {
        block: "diff".to_owned(),
        detail: if first {
            "Première revue : aucun diff".to_owned()
        } else if diff.is_empty() {
            "Aucun changement depuis la dernière revue".to_owned()
        } else {
            format!(
                "Diff depuis la dernière revue : {}",
                files(diff.len(), "fichier", "fichiers")
            )
        },
        tokens: diff.iter().map(|d| estimate_tokens(&d.diff)).sum(),
    });
    items.push(ContextItem {
        block: "open_remarks".to_owned(),
        detail: if open == 0 {
            "Aucune remarque ouverte à rappeler".to_owned()
        } else {
            format!(
                "Remarques ouvertes rappelées : {}",
                files(open, "remarque", "remarques")
            )
        },
        tokens: 0,
    });
    items
}

/// Carte et fichiers clés d'un mentor ; bloquant.
pub(crate) fn key_material(state: &AppState, mentor: &Mentor) -> Option<(String, KeyFiles)> {
    let session = state.project.session(UserId::local());
    project_map::with_map(state, |map, _, cache| {
        let keys = key_files(
            &state.project,
            &session,
            cache,
            &|path| mentor.is_key_file(path),
            state.map.key_files_cap,
        );
        (map.to_owned(), keys)
    })
}

/// Passage cité toujours présent : la section `section_id` existe encore dans le fichier
/// enregistré et contient `quote`. Renvoie sa ligne et le nom de la section.
pub(crate) fn locate_anchor(
    state: &AppState,
    path: &str,
    section_id: &str,
    quote: &str,
) -> Option<(usize, String)> {
    let rel = RelPath::parse(path).ok()?;
    let session = state.project.session(UserId::local());
    let content = state.project.read_file(&session, &rel).ok()?.content;
    let all = sections(Language::of(path), &content);
    let section = all.iter().find(|s| s.id == section_id)?;
    let text = section.text(&content);
    let offset = text.find(quote)?;
    Some((
        section.start_line + text[..offset].matches('\n').count(),
        section.name.clone(),
    ))
}

/// Section du fichier enregistré `path` qui contient `quote`, son texte et sa ligne.
pub(crate) fn locate(
    state: &AppState,
    path: &str,
    quote: &str,
) -> Option<(Section, String, usize)> {
    let rel = RelPath::parse(path).ok()?;
    let session = state.project.session(UserId::local());
    let content = state.project.read_file(&session, &rel).ok()?.content;
    let line = if quote.is_empty() {
        1
    } else {
        content[..content.find(quote)?].matches('\n').count() + 1
    };
    let all = sections(Language::of(path), &content);
    let section = section_at(&all, line)?.clone();
    let text = section.text(&content);
    Some((section, text, line))
}

/* ---------- Estimation ---------- */

/// `review_estimate` : d'après la dernière revue du mentor, sinon le prix du modèle.
pub(crate) async fn estimate(
    state: Arc<AppState>,
    mentor_ids: Vec<String>,
) -> Result<ServerMessage, Refused> {
    let chosen = mentors(&state, &mentor_ids)?;
    let worker = state.clone();
    let jobs = tokio::task::spawn_blocking(move || {
        chosen
            .into_iter()
            .map(|m| prepare(&worker, m))
            .collect::<Result<Vec<_>, _>>()
    })
    .await
    .map_err(|_| refused(ErrorCode::Internal, "estimation interrompue"))??;
    let mut mentors = Vec::new();
    for job in &jobs {
        let (cost, duration_ms, basis) = match state.project_store.last_cost(&job.mentor.id) {
            Some(LastCost {
                tokens,
                cost,
                duration_ms,
            }) if tokens > 0 => (
                Some(cost * job.tokens as f64 / tokens as f64),
                duration_ms,
                "last",
            ),
            _ => match model_info(&state, &job.request.model).await.ok().flatten() {
                Some(info) => (
                    Some(
                        job.tokens as f64 * info.prompt_price
                            + ANSWER_TOKENS * info.completion_price,
                    ),
                    DEFAULT_DURATION_MS,
                    "price",
                ),
                None => (None, DEFAULT_DURATION_MS, "unknown"),
            },
        };
        mentors.push(MentorEstimate {
            id: job.mentor.id.clone(),
            tokens: job.tokens,
            cost,
            duration_ms,
            basis: basis.to_owned(),
            detail: job.detail.clone(),
        });
    }
    let total_cost = mentors.iter().map(|m| m.cost).sum::<Option<f64>>();
    let duration_ms = mentors.iter().map(|m| m.duration_ms).max().unwrap_or(0);
    let budget = state.budget.state(Timestamp::now());
    Ok(ServerMessage::ReviewEstimated {
        mentors,
        total_cost,
        duration_ms,
        budget_left: (budget.cap_usd - budget.used_usd).max(0.0),
    })
}

/* ---------- Revue ---------- */

/// `review_request` : vérifie le fournisseur et le budget, prépare chaque mentor, puis
/// lance la revue en tâche de fond ; `review_started` est renvoyé aussitôt.
pub(crate) async fn request(
    state: Arc<AppState>,
    mentor_ids: Vec<String>,
    confirm_over_budget: bool,
    trigger: &'static str,
) -> Result<ServerMessage, Refused> {
    let chosen = mentors(&state, &mentor_ids)?;
    if let Err(message) = state.llm().client {
        state.set_provider(false, Some(message.clone()));
        return Err(refused(ErrorCode::ProviderError, message));
    }
    if state.budget.state(Timestamp::now()).level == BudgetLevel::Exceeded && !confirm_over_budget {
        return Err(refused(
            ErrorCode::BudgetExceeded,
            "budget du mois atteint : confirmez pour lancer cette revue",
        ));
    }
    let review_id = format!("rv_{}", ulid::Ulid::generate());
    let mut generations = HashMap::new();
    if let Ok(mut running) = state.reviews.running.lock() {
        for mentor in &chosen {
            let generation = state.reviews.generation.fetch_add(1, Ordering::SeqCst) + 1;
            running.insert(mentor.id.clone(), generation);
            generations.insert(mentor.id.clone(), generation);
        }
    }
    let worker = state.clone();
    let jobs = tokio::task::spawn_blocking(move || {
        chosen
            .into_iter()
            .map(|m| prepare(&worker, m))
            .collect::<Result<Vec<_>, _>>()
    })
    .await
    .map_err(|_| refused(ErrorCode::Internal, "revue interrompue"))?;
    let jobs = match jobs {
        Ok(jobs) => jobs,
        Err(err) => {
            release(&state, &generations);
            return Err(err);
        }
    };
    let started = ServerMessage::ReviewStarted {
        review_id: review_id.clone(),
        trigger: trigger.to_owned(),
        mentor_ids: jobs.iter().map(|j| j.mentor.id.clone()).collect(),
    };
    let _ = state.events.send(started.clone());
    tokio::spawn(run(state, review_id, trigger, jobs, generations));
    Ok(started)
}

/// Oublie les revues en cours de ces générations (les plus récentes restent).
fn release(state: &AppState, generations: &HashMap<String, u64>) {
    if let Ok(mut running) = state.reviews.running.lock() {
        running.retain(|id, generation| generations.get(id) != Some(generation));
    }
}

fn current(state: &AppState, mentor_id: &str, generation: u64) -> bool {
    state
        .reviews
        .running
        .lock()
        .is_ok_and(|running| running.get(mentor_id) == Some(&generation))
}

struct Outcome {
    mentor_id: String,
    outcome: &'static str,
    remarks: Vec<ProjectRemark>,
    cost: f64,
    duration_ms: u64,
    tokens: usize,
    now: BTreeMap<String, String>,
}

async fn run(
    state: Arc<AppState>,
    review_id: String,
    trigger: &'static str,
    jobs: Vec<Job>,
    generations: HashMap<String, u64>,
) {
    let started = Instant::now();
    let Ok(client) = state.llm().client else {
        release(&state, &generations);
        return;
    };
    let mut tasks = JoinSet::new();
    for job in jobs {
        let client = client.clone();
        let state = state.clone();
        let review_id = review_id.clone();
        let generation = generations.get(&job.mentor.id).copied().unwrap_or_default();
        tasks.spawn(async move {
            let began = Instant::now();
            let sent: Vec<(&str, &str)> = job
                .sent
                .iter()
                .map(|(p, t)| (p.as_str(), t.as_str()))
                .collect();
            let finder =
                |path: &str, quote: &str| locate(&state, path, quote).map(|(_, text, _)| text);
            let rules = ProjectRules {
                level: remark_level(job.mentor.entry_level),
                max_remarks: job.max_remarks,
                sent: &sent,
                map: &job.map,
                locate: &finder,
            };
            let result =
                ask_for_project_remarks(client.as_ref(), job.request.clone(), &rules).await;
            let duration_ms = u64::try_from(began.elapsed().as_millis()).unwrap_or(u64::MAX);
            let (outcome, drafts, cost) = match result {
                Ok(asked) => ("ok", asked.remarks, asked.cost_usd),
                Err(err) => {
                    let cost = err.cost_usd();
                    match err {
                        AskError::Provider { error, .. } => {
                            state.set_provider(false, Some(error.to_string()));
                            ("provider_error", Vec::new(), cost)
                        }
                        AskError::InvalidResponse { .. } => ("invalid_response", Vec::new(), cost),
                    }
                }
            };
            let (outcome, drafts) =
                if outcome == "ok" && !current(&state, &job.mentor.id, generation) {
                    // Une revue plus récente du même mentor a été demandée : coût compté,
                    // résultat ignoré.
                    ("stale", Vec::new())
                } else {
                    (outcome, drafts)
                };
            if outcome == "ok" {
                state.set_provider(true, None);
            }
            let now = state.clock.now().to_string();
            let remarks = drafts
                .into_iter()
                .map(|draft| remark(&state, &job, &review_id, draft, &now))
                .collect();
            Outcome {
                mentor_id: job.mentor.id.clone(),
                outcome,
                remarks,
                cost,
                duration_ms,
                tokens: job.tokens,
                now: job.now,
            }
        });
    }
    let mut outcomes = Vec::new();
    while let Some(Ok(outcome)) = tasks.join_next().await {
        outcomes.push(outcome);
    }
    release(&state, &generations);
    let cost: f64 = outcomes.iter().map(|o| o.cost).sum();
    if cost > 0.0
        && let Err(err) = state.budget.record(cost, Timestamp::now())
    {
        tracing::error!(error = %err, "coût de la revue non enregistré");
    }
    let items: Vec<ProjectRemark> = outcomes.iter().flat_map(|o| o.remarks.clone()).collect();
    state.project_store.add(&items);
    let at = state.clock.now().to_string();
    let mut manifests = BTreeMap::new();
    let mut costs = BTreeMap::new();
    for outcome in outcomes.iter().filter(|o| o.outcome == "ok") {
        manifests.insert(
            outcome.mentor_id.clone(),
            Manifest {
                at: at.clone(),
                files: state.snapshots.store(&outcome.now),
            },
        );
        costs.insert(
            outcome.mentor_id.clone(),
            LastCost {
                tokens: outcome.tokens,
                cost: outcome.cost,
                duration_ms: outcome.duration_ms,
            },
        );
    }
    let duration_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    let info = ReviewInfo {
        id: review_id.clone(),
        at,
        trigger: trigger.to_owned(),
        mentors: outcomes
            .iter()
            .map(|o| ReviewMentorInfo {
                id: o.mentor_id.clone(),
                outcome: o.outcome.to_owned(),
                remarks: o.remarks.len(),
                cost: o.cost,
            })
            .collect(),
        cost,
        duration_ms,
    };
    let keep = state.project_store.record(info, manifests, costs);
    state.snapshots.collect(&keep);
    let mentor_ids: Vec<String> = outcomes.iter().map(|o| o.mentor_id.clone()).collect();
    crate::auto::review_finished(&state, &mentor_ids);
    tracing::info!(
        revue = %review_id,
        declencheur = trigger,
        remarques = items.len(),
        duree_ms = duration_ms,
        cout_usd = cost,
        "revue du projet"
    );
    let _ = state.events.send(ServerMessage::ProjectRemarks {
        review_id: Some(review_id.clone()),
        items,
    });
    let _ = state.events.send(ServerMessage::ReviewDone {
        review_id,
        outcomes: outcomes
            .iter()
            .map(|o| (o.mentor_id.clone(), o.outcome.to_owned()))
            .collect(),
        cost,
        duration_ms,
    });
    let _ = state.events.send(state.status());
}

/// Remarque de projet complète, ancrée sur chacune de ses citations.
fn remark(
    state: &AppState,
    job: &Job,
    review_id: &str,
    draft: ProjectRemarkDraft,
    now: &str,
) -> ProjectRemark {
    let anchors = draft
        .quotes
        .iter()
        .filter_map(|q| {
            let (section, _, line) = locate(state, &q.path, &q.quote)?;
            Some(ProjectAnchor {
                path: q.path.clone(),
                section_id: section.id,
                section_name: section.name,
                quote: q.quote.clone(),
                line_hint: line,
                status: "ok".to_owned(),
            })
        })
        .collect();
    let level = draft.level.as_str().to_owned();
    ProjectRemark {
        id: format!("rp_{}", ulid::Ulid::generate()),
        scope: "project".to_owned(),
        mentor_id: job.mentor.id.clone(),
        about: draft.about.as_str().to_owned(),
        anchors,
        level: level.clone(),
        kind: draft.kind.as_str().to_owned(),
        levels_given: vec![level],
        status: "open".to_owned(),
        thread: vec![ThreadEntry {
            author: "mentor".to_owned(),
            text: draft.message,
            at: now.to_owned(),
        }],
        context: job.items.clone(),
        review_id: review_id.to_owned(),
        created_at: now.to_owned(),
    }
}

/* ---------- Vue Projet ---------- */

/// `reviews_get` : mentors de projet et historique.
pub(crate) fn reviews(state: &AppState) -> ServerMessage {
    let registry = state.mentors.registry();
    let running = state
        .reviews
        .running
        .lock()
        .map(|r| r.keys().cloned().collect::<HashSet<_>>())
        .unwrap_or_default();
    let mut mentors: Vec<ProjectMentorInfo> = registry
        .active()
        .filter(|m| m.scopes.contains(&Scope::Project))
        .map(|m| ProjectMentorInfo {
            id: m.id.clone(),
            name: m.name.clone(),
            initials: m.initials.clone(),
            model: m.model,
            triggers: m
                .project
                .as_ref()
                .map(|p| p.triggers.iter().map(|t| t.as_str().to_owned()).collect())
                .unwrap_or_default(),
            last_review_at: state.project_store.manifest(&m.id).map(|man| man.at),
            reviewing: running.contains(&m.id),
        })
        .collect();
    mentors.sort_by(|a, b| a.name.cmp(&b.name));
    ServerMessage::Reviews {
        mentors,
        items: state.project_store.reviews(),
    }
}

pub(crate) fn remarks(state: &AppState) -> ServerMessage {
    ServerMessage::ProjectRemarks {
        review_id: None,
        items: state.project_store.visible(),
    }
}

/* ---------- Reprises d'une remarque de projet ---------- */

/// `remark_dismiss` sur une remarque de projet.
pub(crate) fn dismiss(state: &AppState, id: &str, status: &str) -> Result<ServerMessage, Refused> {
    state
        .project_store
        .update(id, |remark| {
            if remark.is_visible() {
                remark.status = status.to_owned();
            }
        })
        .map(|r| ServerMessage::RemarkUpdate(Box::new(AnyRemark::Project(r))))
        .ok_or_else(|| refused(ErrorCode::NotFound, format!("remarque {id}")))
}

/// `remark_reply` : la réponse est notée tout de suite ; la relance suit.
pub(crate) fn reply(state: &AppState, id: &str, text: &str) -> Result<ProjectRemark, Refused> {
    let remark = state
        .project_store
        .get(id)
        .ok_or_else(|| refused(ErrorCode::NotFound, format!("remarque {id}")))?;
    if !remark.is_active() {
        return Err(refused(ErrorCode::BadMessage, "cette remarque est fermée"));
    }
    let now = state.clock.now().to_string();
    state
        .project_store
        .update(id, |remark| {
            remark.thread.push(ThreadEntry {
                author: "user".to_owned(),
                text: text.to_owned(),
                at: now,
            });
            remark.status = "answered".to_owned();
        })
        .ok_or_else(|| refused(ErrorCode::NotFound, format!("remarque {id}")))
}

/// Palier suivant ou relance d'une remarque de projet : carte, fichiers cités, fil.
pub(crate) async fn follow_up(
    state: Arc<AppState>,
    id: String,
    kind: FollowUpKind,
    confirm_over_budget: bool,
) -> Result<ServerMessage, Refused> {
    {
        let mut busy = state
            .reviews
            .busy
            .lock()
            .map_err(|_| refused(ErrorCode::Internal, "état des reprises illisible"))?;
        if !busy.insert(id.clone()) {
            return Err(refused(
                ErrorCode::BadMessage,
                "le mentor répond déjà à cette remarque",
            ));
        }
    }
    let result = follow_up_inner(&state, &id, kind, confirm_over_budget).await;
    if let Ok(mut busy) = state.reviews.busy.lock() {
        busy.remove(&id);
    }
    let _ = state.events.send(state.status());
    result.map(|r| ServerMessage::RemarkUpdate(Box::new(AnyRemark::Project(r))))
}

async fn follow_up_inner(
    state: &Arc<AppState>,
    id: &str,
    kind: FollowUpKind,
    confirm_over_budget: bool,
) -> Result<ProjectRemark, Refused> {
    let remark = state
        .project_store
        .get(id)
        .ok_or_else(|| refused(ErrorCode::NotFound, format!("remarque {id}")))?;
    if !remark.is_active() {
        return Err(refused(ErrorCode::BadMessage, "cette remarque est fermée"));
    }
    let current = Level::from_name(&remark.level).unwrap_or(Level::Indice);
    let level = match kind {
        FollowUpKind::NextLevel => current
            .next()
            .ok_or_else(|| refused(ErrorCode::BadMessage, "dernier palier déjà atteint"))?,
        FollowUpKind::Reply => current,
    };
    if level == Level::Exemple && remark.anchors.is_empty() {
        return Err(refused(
            ErrorCode::BadMessage,
            "sans passage cité, le dernier palier est la piste",
        ));
    }
    let registry = state.mentors.registry();
    let mentor = registry
        .get(&remark.mentor_id)
        .filter(|m| m.scopes.contains(&Scope::Project))
        .cloned()
        .ok_or_else(|| refused(ErrorCode::NotFound, "ce mentor de projet n'est plus actif"))?;
    let client = state.llm().client.map_err(|message| {
        state.set_provider(false, Some(message.clone()));
        refused(ErrorCode::ProviderError, message)
    })?;
    if state.budget.state(Timestamp::now()).level == BudgetLevel::Exceeded && !confirm_over_budget {
        return Err(refused(
            ErrorCode::BudgetExceeded,
            "budget du mois atteint : confirmez pour envoyer cette demande",
        ));
    }
    let (model, effort) = model_for(state, &mentor).ok_or_else(|| {
        refused(
            ErrorCode::ProviderError,
            format!(
                "modèle « {} » non configuré dans config.toml",
                mentor.model.as_str()
            ),
        )
    })?;

    let worker = state.clone();
    let anchors = remark.anchors.clone();
    let (map, cited) = tokio::task::spawn_blocking(move || {
        let map = project_map::with_map(&worker, |map, _, _| map.to_owned()).unwrap_or_default();
        let cited: Vec<(String, String, String)> = anchors
            .iter()
            .filter_map(|a| {
                let (section, text, _) = locate(&worker, &a.path, &a.quote)?;
                Some((a.path.clone(), format!("section {}", section.id), text))
            })
            .collect();
        (map, cited)
    })
    .await
    .map_err(|_| refused(ErrorCode::Internal, "reprise interrompue"))?;
    // Ce qui part est masqué ; la garde juge sur le texte réel des sections citées.
    let masked: Vec<(String, String, String)> = cited
        .iter()
        .map(|(path, form, text)| {
            let text = if Language::of(path) == Language::Markdown {
                strip_html_comments(text)
            } else {
                text.clone()
            };
            (path.clone(), form.clone(), mask_secrets(&text).0)
        })
        .collect();
    let quotes: Vec<String> = remark.anchors.iter().map(|a| a.quote.clone()).collect();
    let quote = mask_secrets(&quotes.join(" · ")).0;
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
    let follow_up = FollowUp {
        kind,
        quote: &quote,
        levels_given: remark
            .levels_given
            .iter()
            .filter_map(|l| Level::from_name(l))
            .collect(),
        entries: entries.iter().map(|(a, t)| (*a, t.as_str())).collect(),
    };
    let prompt = project_follow_up_prompt(&ProjectFollowUpInput {
        mentor: &mentor,
        level,
        map: &map,
        cited: masked
            .iter()
            .map(|(path, form, text)| FileText { path, form, text })
            .collect(),
        follow_up: &follow_up,
    });
    let request = LlmRequest {
        model,
        messages: vec![
            ChatMessage::new(Role::System, prompt.system),
            ChatMessage::new(Role::User, prompt.user),
        ],
        schema_name: "reponse".to_owned(),
        schema: reply_schema(),
        effort,
    };
    let section_text = cited
        .iter()
        .map(|(_, _, text)| text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let rules = Rules {
        section_text: &section_text,
        level: remark_level(level),
        max_remarks: 1,
    };
    let result = ask_for_reply(client.as_ref(), request, &rules).await;
    let cost = match &result {
        Ok(replied) => replied.cost_usd,
        Err(err) => err.cost_usd(),
    };
    if cost > 0.0
        && let Err(err) = state.budget.record(cost, Timestamp::now())
    {
        tracing::error!(error = %err, "coût de la reprise non enregistré");
    }
    let message = match result {
        Ok(replied) => {
            state.set_provider(true, None);
            replied.reply.map(|r| r.message)
        }
        Err(AskError::Provider { error, .. }) => {
            state.set_provider(false, Some(error.to_string()));
            return Err(refused(ErrorCode::ProviderError, error.to_string()));
        }
        Err(AskError::InvalidResponse { .. }) => None,
    };
    tracing::info!(remarque = %id, palier = level.as_str(), ajoute = message.is_some(), cout_usd = cost, "reprise d'une remarque de projet");
    let Some(message) = message else {
        return state
            .project_store
            .get(id)
            .ok_or_else(|| refused(ErrorCode::NotFound, format!("remarque {id}")));
    };
    let now = state.clock.now().to_string();
    state
        .project_store
        .update(id, |remark| {
            if !remark.is_active() {
                return;
            }
            remark.thread.push(ThreadEntry {
                author: "mentor".to_owned(),
                text: message,
                at: now,
            });
            if kind == FollowUpKind::NextLevel {
                let level = level.as_str().to_owned();
                remark.level = level.clone();
                remark.levels_given.push(level);
            }
        })
        .ok_or_else(|| refused(ErrorCode::NotFound, format!("remarque {id}")))
}
