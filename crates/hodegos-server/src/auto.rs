//! Revues automatiques et réévaluations (phase 9d, ADR 0010) : à l'ouverture du projet
//! (création, ou changements depuis la dernière revue), après des modifications au
//! calme, après un commit si on l'active ; passages cités vérifiés après chaque
//! changement, remarques à réévaluer relues au calme. Le planificateur (`scheduler.rs`)
//! décide ; cette boucle observe, lance et prévient les fenêtres (`project_status`).

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::process::Command;
use std::sync::{Arc, Mutex};

use hodegos_core::map::{diff_since, masked_texts};
use hodegos_core::{RelPath, UserId};
use hodegos_llm::BudgetLevel;
use hodegos_mentors::{Mentor, Mode, ReviewTrigger, Scope};
use jiff::Timestamp;
use tokio::sync::broadcast::error::RecvError;

use crate::AppState;
use crate::project_map;
use crate::protocol::{AnyRemark, PendingReview, ProjectRemark, ServerMessage};
use crate::reevaluate::{self, Outcome};
use crate::review::{self, locate_anchor};
use crate::scheduler::{Candidate, Context, Scheduler};
use crate::snapshots::digest;

#[derive(Default)]
struct Inner {
    /// Réévaluations essayées sans résultat : remarque → mentor ; reprises après la
    /// revue suivante de ce mentor.
    attempted: HashMap<String, String>,
    /// Réévaluations en cours, une par mentor : mentor → remarque.
    reevaluating: HashMap<String, String>,
    /// Dernier `HEAD` vu.
    head: Option<String>,
    /// Dernier `project_status` envoyé.
    last_status: Option<ServerMessage>,
}

#[derive(Default)]
pub(crate) struct AutoState {
    scheduler: Mutex<Scheduler>,
    inner: Mutex<Inner>,
}

fn project_mentors(state: &AppState) -> Vec<Mentor> {
    state
        .mentors
        .registry()
        .active()
        .filter(|m| m.scopes.contains(&Scope::Project))
        .cloned()
        .collect()
}

fn mark_all(state: &AppState, trigger: ReviewTrigger) {
    let now = state.clock.now();
    if let Ok(mut scheduler) = state.auto.scheduler.lock() {
        for mentor in project_mentors(state) {
            scheduler.mark(&mentor.id, trigger, now);
        }
    }
}

/// Pousse les remarques modifiées et l'état du projet.
fn publish(state: &AppState, remarks: Vec<ProjectRemark>) {
    for remark in remarks {
        let _ = state
            .events
            .send(ServerMessage::RemarkUpdate(Box::new(AnyRemark::Project(
                remark,
            ))));
    }
    push_status(state, true);
}

/// Vérifie les passages cités (dans `paths`, ou partout) ; bloquant.
fn check_anchors(state: &AppState, paths: Option<&BTreeSet<String>>) {
    let root = state.project.root().to_path_buf();
    let exists = |path: &str| root.join(path).is_file();
    let locate =
        |path: &str, section: &str, quote: &str| locate_anchor(state, path, section, quote);
    let changed = state.project_store.check_anchors(paths, &exists, &locate);
    if !changed.is_empty() {
        publish(state, changed);
    }
}

/// Changement enregistré (dans Hodegos ou ailleurs), après le recalcul de la carte :
/// le calme repart, les mentors de projet auront à relire, les passages cités des
/// fichiers touchés sont vérifiés. Bloquant.
pub(crate) fn changed(state: &AppState, paths: &[RelPath]) {
    let now = state.clock.now();
    if let Ok(mut scheduler) = state.auto.scheduler.lock() {
        scheduler.changed(now);
    }
    mark_all(state, ReviewTrigger::Change);
    let paths: BTreeSet<String> = paths.iter().map(|p| p.to_string()).collect();
    check_anchors(state, Some(&paths));
}

/// Fichiers supprimés depuis Hodegos : leurs passages cités ont disparu, et c'est une
/// modification (le calme repart).
pub(crate) fn forgotten(state: &AppState, paths: &HashSet<String>) {
    if let Ok(mut scheduler) = state.auto.scheduler.lock() {
        scheduler.changed(state.clock.now());
    }
    let changed = state.project_store.forget(paths);
    if !changed.is_empty() {
        publish(state, changed);
    }
}

/// Une revue est finie : les réévaluations ratées de ses mentors seront retentées.
pub(crate) fn review_finished(state: &AppState, mentor_ids: &[String]) {
    if let Ok(mut inner) = state.auto.inner.lock() {
        inner
            .attempted
            .retain(|_, mentor| !mentor_ids.contains(mentor));
    }
    push_status(state, false);
}

/// `HEAD` du dépôt, `None` sans dépôt ni commit.
fn head(state: &AppState) -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--verify", "-q", "HEAD"])
        .current_dir(state.project.root())
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

/// Ouverture du projet : passages vérifiés partout ; `create` si aucune revue n'a
/// jamais été notée, sinon `open` pour chaque mentor dont les fichiers ont changé
/// depuis sa dernière revue. Bloquant.
fn startup(state: &AppState) {
    check_anchors(state, None);
    let head = head(state);
    if let Ok(mut inner) = state.auto.inner.lock() {
        inner.head = head;
    }
    if state.project_store.never_reviewed() {
        mark_all(state, ReviewTrigger::Create);
        return;
    }
    let session = state.project.session(UserId::local());
    let Some(current) = project_map::with_map(state, |_, _, cache| {
        masked_texts(&state.project, &session, cache)
            .iter()
            .map(|(path, text)| (path.clone(), digest(text)))
            .collect::<BTreeMap<_, _>>()
    }) else {
        return;
    };
    let now = state.clock.now();
    if let Ok(mut scheduler) = state.auto.scheduler.lock() {
        for mentor in project_mentors(state) {
            let changed = state
                .project_store
                .manifest(&mentor.id)
                .is_none_or(|manifest| manifest.files != current);
            if changed {
                scheduler.mark(&mentor.id, ReviewTrigger::Open, now);
            }
        }
    }
}

/// Un mot-clé d'un mentor `strong-signal` figure-t-il dans le diff depuis sa dernière
/// revue ? Bloquant.
fn keyword_hit(state: &AppState, mentor: &Mentor) -> bool {
    let session = state.project.session(UserId::local());
    let Some(now) = project_map::with_map(state, |_, _, cache| {
        masked_texts(&state.project, &session, cache)
    }) else {
        return false;
    };
    let before = state
        .project_store
        .manifest(&mentor.id)
        .map(|m| state.snapshots.load(&m.files))
        .unwrap_or_default();
    diff_since(&before, &now, usize::MAX).iter().any(|change| {
        mentor
            .triggers
            .keywords
            .iter()
            .any(|k| change.diff.to_lowercase().contains(&k.to_lowercase()))
    })
}

fn budget_ok(state: &AppState) -> bool {
    state.llm().client.is_ok()
        && state.budget.state(Timestamp::now()).level != BudgetLevel::Exceeded
}

fn running(state: &AppState) -> HashSet<String> {
    state.reviews.running_ids()
}

/// Un tour de la boucle : revues dues, réévaluations au calme, état publié. Bloquant
/// pour le planificateur ; les appels au modèle partent en tâche de fond.
fn tick(state: &Arc<AppState>) {
    let now = state.clock.now();
    let mentors = project_mentors(state);
    let candidates: Vec<Candidate> = mentors
        .iter()
        .map(|m| Candidate {
            id: m.id.clone(),
            mode: m.mode,
            triggers: m
                .project
                .as_ref()
                .map(|p| p.triggers.clone())
                .unwrap_or_default(),
        })
        .collect();
    let today = now.to_zoned(jiff::tz::TimeZone::UTC).date();
    let (last_review, today_count) = state.project_store.rhythm(today);
    let running = running(state);
    let budget_ok = budget_ok(state);
    let hit = |id: &str| {
        mentors
            .iter()
            .find(|m| m.id == id)
            .is_some_and(|m| m.mode == Mode::StrongSignal && keyword_hit(state, m))
    };
    let options = state.review_options();
    let (due, calm) = {
        let Ok(mut scheduler) = state.auto.scheduler.lock() else {
            return;
        };
        let due = scheduler.due(
            &candidates,
            &Context {
                options: &options,
                budget_ok,
                running: &running,
                last_review: &last_review,
                today: &today_count,
                keyword_hit: &hit,
            },
            now,
        );
        for (id, _) in &due {
            scheduler.clear(id);
        }
        (due, scheduler.calm(&options, now))
    };
    for (id, trigger) in due {
        let state = state.clone();
        tokio::spawn(async move {
            if let Err(refused) =
                review::request(state.clone(), vec![id.clone()], false, trigger.as_str()).await
            {
                tracing::info!(mentor = %id, message = %refused.message, "revue automatique non lancée");
            }
        });
    }

    // Réévaluations : au calme, une à la fois par mentor, hors plafond quotidien.
    if calm && budget_ok {
        let candidates: Vec<(String, String)> = {
            let Ok(inner) = state.auto.inner.lock() else {
                return;
            };
            state
                .project_store
                .to_review()
                .into_iter()
                .filter(|r| !inner.attempted.contains_key(&r.id))
                .filter(|r| !inner.reevaluating.contains_key(&r.mentor_id))
                .filter(|r| !running.contains(&r.mentor_id))
                .map(|r| (r.id, r.mentor_id))
                .collect()
        };
        let mut started: HashSet<String> = HashSet::new();
        for (id, mentor) in candidates {
            if !started.insert(mentor.clone()) {
                continue;
            }
            if let Ok(mut inner) = state.auto.inner.lock() {
                inner.reevaluating.insert(mentor.clone(), id.clone());
            }
            let state = state.clone();
            tokio::spawn(async move {
                let outcome = reevaluate::reevaluate(state.clone(), id.clone()).await;
                if let Ok(mut inner) = state.auto.inner.lock() {
                    inner.reevaluating.remove(&mentor);
                    if outcome != Outcome::Skipped && !matches!(outcome, Outcome::Done(_)) {
                        inner.attempted.insert(id.clone(), mentor.clone());
                    }
                }
                match outcome {
                    Outcome::Done(remark) => publish(&state, vec![*remark]),
                    _ => push_status(&state, true),
                }
                let _ = state.events.send(state.status());
            });
        }
    }
    push_status(state, false);
}

/// État des revues pour les fenêtres.
pub(crate) fn status(state: &AppState) -> ServerMessage {
    let pending = state
        .auto
        .scheduler
        .lock()
        .map(|s| {
            s.pending()
                .iter()
                .map(|(id, p)| PendingReview {
                    mentor_id: id.clone(),
                    trigger: p
                        .triggers
                        .iter()
                        .next()
                        .map_or("change", |t| t.as_str())
                        .to_owned(),
                    at: p.since.to_string(),
                })
                .collect()
        })
        .unwrap_or_default();
    let mut reviewing: Vec<String> = running(state).into_iter().collect();
    if let Ok(inner) = state.auto.inner.lock() {
        reviewing.extend(inner.reevaluating.keys().cloned());
    }
    reviewing.sort();
    reviewing.dedup();
    let (open, to_review) = state.project_store.counts();
    ServerMessage::ProjectStatus {
        pending,
        reviewing,
        open,
        to_review,
    }
}

/// Pousse `project_status` s'il a changé (ou toujours, avec `force`).
fn push_status(state: &AppState, force: bool) {
    let message = status(state);
    let Ok(mut inner) = state.auto.inner.lock() else {
        return;
    };
    if force || inner.last_status.as_ref() != Some(&message) {
        inner.last_status = Some(message.clone());
        let _ = state.events.send(message);
    }
}

/// Boucle des revues automatiques, jusqu'à l'arrêt du serveur.
pub(crate) async fn run(state: Arc<AppState>) {
    let mut git = state.git.refreshed.subscribe();
    let first = state.clone();
    let _ = tokio::task::spawn_blocking(move || {
        // Point de départ : l'état vide que reçoit une fenêtre connectée avant la fin du
        // démarrage ; s'il change (revue en attente), il est poussé.
        let initial = status(&first);
        if let Ok(mut inner) = first.auto.inner.lock() {
            inner.last_status = Some(initial);
        }
        startup(&first);
        push_status(&first, false);
    })
    .await;
    let mut interval = tokio::time::interval(state.review_options().tick);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            _ = interval.tick() => {
                let worker = state.clone();
                let _ = tokio::task::spawn_blocking(move || tick(&worker)).await;
            }
            update = git.recv() => match update {
                Ok(_) | Err(RecvError::Lagged(_)) => {
                    let worker = state.clone();
                    let _ = tokio::task::spawn_blocking(move || {
                        let current = head(&worker);
                        let moved = worker.auto.inner.lock().is_ok_and(|mut inner| {
                            let moved = current.is_some() && inner.head.is_some() && inner.head != current;
                            inner.head = current.clone();
                            moved
                        });
                        if moved {
                            mark_all(&worker, ReviewTrigger::Commit);
                        }
                    })
                    .await;
                }
                Err(RecvError::Closed) => return,
            },
        }
    }
}
