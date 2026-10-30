use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;

use axum::Extension;
use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use hodegos_core::{
    EntryKind, FileError, Language, RelPath, Section, Session, UserId, Version, section_at,
    sections,
};
use hodegos_mentors::{CastRequest, FollowUpKind, Registry, cast};
use jiff::Timestamp;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::mpsc;

use crate::AppState;
use crate::followup;
use crate::pass::{
    self, PassRequest, PassResult, Prepared, Trigger, cast_section, locate_anchor, mentor_ref,
};
use crate::protocol::{
    self, ClientMessage, DirEntry, ErrorCode, ForcedBy, LastPass, MentorRef, MentorWarning,
    Outgoing, PassOutcome, Remark, SectionInfo, ServerMessage, ThreadEntry,
};

/// Longueur maximale d'une réponse de l'utilisateur.
const MAX_REPLY_CHARS: usize = 2000;

/// Reprise terminée, avec l'`id` de la demande à laquelle elle répond (`hint_next`),
/// ou `None` pour la relance qui suit une réponse.
struct FollowUpDone {
    re: Option<String>,
    done: followup::Done,
}

/// Mise à niveau WebSocket : le cookie est déjà vérifié par la garde, on vérifie ici `Origin`.
pub(crate) async fn upgrade(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<UserId>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Response {
    let origin = headers
        .get(header::ORIGIN)
        .and_then(|o| o.to_str().ok())
        .unwrap_or_default();
    if origin != state.canonical_origin() {
        tracing::warn!(origin, "WebSocket refusée : Origin étrangère");
        return (StatusCode::FORBIDDEN, "Origine refusée.").into_response();
    }
    ws.on_upgrade(move |socket| run(socket, state, user))
}

/// Fichier ouvert dans la fenêtre, tel qu'il est sur le disque.
struct OpenFile {
    /// `None` si le fichier n'existe plus.
    version: Option<Version>,
    content: String,
    language: Language,
    sections: Vec<Section>,
}

impl OpenFile {
    fn new(version: Version, content: String, language: Language) -> Self {
        let sections = sections(language, &content);
        Self {
            version: Some(version),
            content,
            language,
            sections,
        }
    }
}

/// Une fenêtre connectée : sa session, ses fichiers ouverts, son curseur et le
/// dernier casting envoyé.
struct Connection {
    state: Arc<AppState>,
    session: Session,
    open: HashMap<RelPath, OpenFile>,
    cursor: Option<(RelPath, usize)>,
    last_casting: Option<ServerMessage>,
    /// Les passes terminées reviennent par ce canal.
    passes: mpsc::UnboundedSender<PassResult>,
    /// Numéro de la dernière passe lancée par fichier : un résultat plus ancien est ignoré.
    generations: HashMap<RelPath, u64>,
    /// Les reprises terminées (palier suivant, relance) reviennent par ce canal.
    followups: mpsc::UnboundedSender<FollowUpDone>,
    /// Remarques dont une reprise est en cours : une seule à la fois.
    busy: HashSet<String>,
    /// Fichiers dont la passe d'ouverture est en cours : une seule à la fois.
    opening: HashSet<RelPath>,
}

async fn run(mut socket: WebSocket, state: Arc<AppState>, user: UserId) {
    let mut changes = state.changes.subscribe();
    let mut reloaded = state.mentors.reloaded.subscribe();
    let session = state.project.session(user);
    let (passes, mut finished) = mpsc::unbounded_channel();
    let (followups, mut followed) = mpsc::unbounded_channel();
    let mut connection = Connection {
        state,
        session,
        open: HashMap::new(),
        cursor: None,
        last_casting: None,
        passes,
        generations: HashMap::new(),
        followups,
        busy: HashSet::new(),
        opening: HashSet::new(),
    };
    let mut pending = vec![Outgoing::push(connection.state.connection_status())];
    pending.extend(connection.mentor_warnings(false));
    loop {
        for reply in pending.drain(..) {
            let text = match serde_json::to_string(&reply) {
                Ok(text) => text,
                Err(e) => {
                    tracing::error!(error = %e, "réponse impossible à sérialiser");
                    continue;
                }
            };
            if socket.send(Message::Text(text.into())).await.is_err() {
                return;
            }
        }
        pending = tokio::select! {
            message = socket.recv() => match message {
                Some(Ok(Message::Text(text))) => connection.handle_text(text.as_str()),
                Some(Ok(Message::Close(_)) | Err(_)) | None => break,
                Some(Ok(_)) => continue,
            },
            changed = changes.recv() => match changed {
                Ok(paths) => connection.changed(&paths),
                Err(RecvError::Lagged(_)) => {
                    let all: Vec<RelPath> = connection.open.keys().cloned().collect();
                    connection.changed(&all)
                }
                Err(RecvError::Closed) => break,
            },
            signal = reloaded.recv() => match signal {
                Ok(()) | Err(RecvError::Lagged(_)) => connection.mentor_warnings(true),
                Err(RecvError::Closed) => break,
            },
            Some(result) = finished.recv() => connection.pass_finished(result),
            Some(done) = followed.recv() => connection.followup_finished(done),
        };
    }
}

impl Connection {
    fn handle_text(&mut self, text: &str) -> Vec<Outgoing> {
        let (re, message) = match protocol::parse(text) {
            Ok(parsed) => parsed,
            Err(err) => return vec![err.into()],
        };
        match self.handle(re.clone(), message) {
            Ok(replies) => replies,
            Err(err) => vec![error_reply(re, err)],
        }
    }

    /// Réponse à la demande, suivie des messages spontanés qu'elle entraîne.
    fn handle(
        &mut self,
        re: Option<String>,
        message: ClientMessage,
    ) -> Result<Vec<Outgoing>, FileError> {
        let project = &self.state.project;
        match message {
            ClientMessage::Ping => Ok(vec![Outgoing::reply(re, ServerMessage::Pong)]),
            ClientMessage::ListDir { path } => {
                let rel = RelPath::parse(&path)?;
                let registry = self.state.mentors.registry();
                let entries = project
                    .list_dir(&self.session, &rel)?
                    .into_iter()
                    .map(|entry| {
                        let (kind, mentors) = match entry.kind {
                            EntryKind::Dir => (protocol::EntryKind::Dir, Vec::new()),
                            EntryKind::File => (
                                protocol::EntryKind::File,
                                file_mentors(&registry, rel.join(&entry.name).as_str()),
                            ),
                        };
                        DirEntry {
                            name: entry.name,
                            kind,
                            mentors,
                        }
                    })
                    .collect();
                Ok(vec![Outgoing::reply(
                    re,
                    ServerMessage::Dir {
                        path: rel.to_string(),
                        entries,
                    },
                )])
            }
            ClientMessage::OpenFile { path } => {
                let rel = RelPath::parse(&path)?;
                let file = project.read_file(&self.session, &rel)?;
                self.watch(&rel);
                let open = OpenFile::new(file.version.clone(), file.content.clone(), file.language);
                let content = file.content.clone();
                // Le fichier a pu changer pendant que Hodegos était arrêté ; les remarques
                // détachées partent avec `file`, sans `remark_update`.
                self.state.store.reanchor(rel.as_str(), |anchor| {
                    locate_anchor(&open.sections, &open.content, anchor)
                });
                let reply = ServerMessage::File {
                    path: rel.to_string(),
                    content: file.content,
                    version: file.version.as_str().to_owned(),
                    language: file.language.as_str().to_owned(),
                    sections: open.sections.iter().map(section_info).collect(),
                    remarks: self.state.store.visible_remarks(rel.as_str()),
                };
                self.open.insert(rel.clone(), open);
                self.cursor = Some((rel.clone(), 1));
                // Toujours un casting à l'ouverture, même identique au précédent.
                self.last_casting = None;
                let mut replies = vec![Outgoing::reply(re, reply)];
                replies.extend(self.casting_if_changed());
                replies.extend(self.open_pass(rel, file.language, content));
                Ok(replies)
            }
            ClientMessage::Save {
                path,
                content,
                base_version,
            } => {
                let rel = RelPath::parse(&path)?;
                let base = Version::from_client(base_version);
                let version = project.write_file(&self.session, &rel, &content, &base)?;
                self.watch(&rel);
                let language = Language::of(rel.as_str());
                let old = self
                    .open
                    .get(&rel)
                    .map(|f| f.content.clone())
                    .unwrap_or_default();
                self.open.insert(
                    rel.clone(),
                    OpenFile::new(version.clone(), content.clone(), language),
                );
                let mut replies = vec![Outgoing::reply(
                    re,
                    ServerMessage::Saved {
                        path: rel.to_string(),
                        version: version.as_str().to_owned(),
                    },
                )];
                replies.extend(self.reanchor(&rel));
                replies.extend(self.casting_if_changed());
                replies.extend(self.start_pass(PassRequest {
                    path: rel,
                    language,
                    old,
                    new: content,
                    cursor_line: self.cursor.as_ref().map(|(_, line)| *line),
                    trigger: Trigger::Save,
                }));
                Ok(replies)
            }
            ClientMessage::Cursor { path, line, .. } => {
                let Ok(rel) = RelPath::parse(&path) else {
                    return Ok(Vec::new());
                };
                if !self.open.contains_key(&rel) {
                    return Ok(Vec::new());
                }
                self.cursor = Some((rel, line.max(1)));
                Ok(self.casting_if_changed().into_iter().collect())
            }
            ClientMessage::CastOverride {
                path,
                section_id,
                mentor_id,
            } => {
                let rel = RelPath::parse(&path)?;
                if let Some(id) = &mentor_id {
                    if self.state.mentors.registry().get(id).is_none() {
                        return Err(FileError::NotFound(format!("mentor {id}")));
                    }
                }
                let section = section_id.unwrap_or_default();
                self.state
                    .mentors
                    .set_forced(rel.as_str(), &section, mentor_id);
                if !self.open.contains_key(&rel) {
                    return Err(FileError::NotFound(rel.to_string()));
                }
                let line = match &self.cursor {
                    Some((path, line)) if *path == rel => *line,
                    _ => self.open[&rel]
                        .sections
                        .iter()
                        .find(|s| s.id == section)
                        .map_or(1, |s| s.start_line),
                };
                self.cursor = Some((rel.clone(), line));
                let casting = self.casting(&rel, line);
                self.last_casting = Some(casting.clone());
                Ok(vec![Outgoing::reply(re, casting)])
            }
            ClientMessage::HintNext {
                remark_id,
                confirm_over_budget,
            } => Ok(self.hint_next(re, &remark_id, confirm_over_budget)),
            ClientMessage::RemarkReply {
                remark_id,
                text,
                confirm_over_budget,
            } => Ok(self.remark_reply(re, &remark_id, &text, confirm_over_budget)),
            ClientMessage::RemarkDismiss { remark_id, reason } => {
                let remark = self
                    .state
                    .store
                    .update(&remark_id, |remark| {
                        if remark.is_visible() {
                            remark.status = reason.status().to_owned();
                        }
                    })
                    .ok_or_else(|| FileError::NotFound(format!("remarque {remark_id}")))?;
                Ok(vec![remark_update(re, remark)])
            }
        }
    }

    /// Palier suivant : la réponse (`remark_update` ou `error`) part quand le mentor
    /// a répondu.
    fn hint_next(&mut self, re: Option<String>, id: &str, confirm: bool) -> Vec<Outgoing> {
        let Some(remark) = self.state.store.get(id) else {
            return vec![Outgoing::error(
                re,
                ErrorCode::NotFound,
                format!("remarque {id}"),
            )];
        };
        match self.start_followup(re.clone(), &remark, FollowUpKind::NextLevel, confirm) {
            Ok(()) => Vec::new(),
            Err(refused) => vec![
                Outgoing::error(re, refused.code, refused.message),
                Outgoing::push(self.state.status()),
            ],
        }
    }

    /// Réponse de l'utilisateur : `remark_update` tout de suite avec son message, puis
    /// un second `remark_update` quand la relance est finie (inchangé si le mentor
    /// n'ajoute rien ou ne peut pas être joint). Au-delà du budget, rien n'est
    /// enregistré sans confirmation.
    fn remark_reply(
        &mut self,
        re: Option<String>,
        id: &str,
        text: &str,
        confirm: bool,
    ) -> Vec<Outgoing> {
        let text = text.trim();
        if text.is_empty() || text.chars().count() > MAX_REPLY_CHARS {
            return vec![Outgoing::error(
                re,
                ErrorCode::BadMessage,
                "réponse vide ou de plus de 2 000 caractères",
            )];
        }
        let Some(remark) = self.state.store.get(id) else {
            return vec![Outgoing::error(
                re,
                ErrorCode::NotFound,
                format!("remarque {id}"),
            )];
        };
        if !remark.is_active() {
            return vec![Outgoing::error(
                re,
                ErrorCode::BadMessage,
                "cette remarque est fermée ou détachée",
            )];
        }
        if self.busy.contains(id) {
            return vec![busy_error(re)];
        }
        let over_budget =
            self.state.budget.state(Timestamp::now()).level == hodegos_llm::BudgetLevel::Exceeded;
        if over_budget && !confirm {
            return vec![Outgoing::error(
                re,
                ErrorCode::BudgetExceeded,
                "budget du mois atteint : confirmez pour envoyer cette réponse",
            )];
        }
        let now = Timestamp::now().to_string();
        let Some(remark) = self.state.store.update(id, |remark| {
            remark.thread.push(ThreadEntry {
                author: "user".to_owned(),
                text: text.to_owned(),
                at: now,
            });
            remark.status = "answered".to_owned();
        }) else {
            return vec![Outgoing::error(
                re,
                ErrorCode::NotFound,
                format!("remarque {id}"),
            )];
        };
        let mut replies = vec![remark_update(re, remark.clone())];
        if let Err(refused) = self.start_followup(None, &remark, FollowUpKind::Reply, true) {
            tracing::info!(message = %refused.message, "réponse enregistrée sans relance du mentor");
            replies.push(remark_update(None, remark));
            replies.push(Outgoing::push(self.state.status()));
        }
        replies
    }

    /// Lance une reprise en tâche de fond ; son issue revient par `followups`.
    fn start_followup(
        &mut self,
        re: Option<String>,
        remark: &Remark,
        kind: FollowUpKind,
        confirm: bool,
    ) -> Result<(), followup::Refused> {
        if self.busy.contains(&remark.id) {
            return Err(followup::Refused {
                code: ErrorCode::BadMessage,
                message: BUSY.to_owned(),
            });
        }
        let registry = self.state.mentors.registry();
        let job = followup::prepare(&self.state, &registry, &self.session, remark, kind, confirm)?;
        self.busy.insert(job.remark_id.clone());
        let state = self.state.clone();
        let sender = self.followups.clone();
        tokio::spawn(async move {
            let done = followup::run(state, job).await;
            let _ = sender.send(FollowUpDone { re, done });
        });
        Ok(())
    }

    /// Message du mentor ajouté au fil (ou remarque inchangée), puis nouvel état.
    fn followup_finished(&mut self, finished: FollowUpDone) -> Vec<Outgoing> {
        let FollowUpDone { re, done } = finished;
        self.busy.remove(&done.remark_id);
        let first = match (&done.outcome, re) {
            (Err(refused), Some(re)) => Some(Outgoing::error(
                Some(re),
                refused.code,
                refused.message.clone(),
            )),
            (Err(_), None) => self
                .state
                .store
                .get(&done.remark_id)
                .map(|remark| remark_update(None, remark)),
            (Ok(message), re) => followup::apply(&self.state, &done, message.as_deref())
                .map(|remark| remark_update(re, remark)),
        };
        first
            .into_iter()
            .chain([Outgoing::push(self.state.status())])
            .collect()
    }

    /// Recale les remarques du fichier sur sa version enregistrée ; `remark_update`
    /// pour chaque remarque détachée.
    fn reanchor(&self, rel: &RelPath) -> Vec<Outgoing> {
        let Some(file) = self.open.get(rel) else {
            return Vec::new();
        };
        self.state
            .store
            .reanchor(rel.as_str(), |anchor| {
                locate_anchor(&file.sections, &file.content, anchor)
            })
            .into_iter()
            .map(|remark| remark_update(None, remark))
            .collect()
    }

    /// Fait observer le dossier du fichier.
    fn watch(&self, rel: &RelPath) {
        if let Err(err) = self.state.watcher.track(&self.session, rel) {
            tracing::warn!(path = %rel, error = %err, "modifications externes non suivies");
        }
    }

    /// Casting de la section sous le curseur, s'il diffère du dernier envoyé.
    fn casting_if_changed(&mut self) -> Option<Outgoing> {
        let (path, line) = self.cursor.clone()?;
        if !self.open.contains_key(&path) {
            return None;
        }
        let casting = self.casting(&path, line);
        if self.last_casting.as_ref() == Some(&casting) {
            return None;
        }
        self.last_casting = Some(casting.clone());
        Some(Outgoing::push(casting))
    }

    fn casting(&self, path: &RelPath, line: usize) -> ServerMessage {
        let registry = self.state.mentors.registry();
        let file = &self.open[path];
        let section = section_at(&file.sections, line);
        let section_id = section.map(|s| s.id.clone());
        let casting = cast_section(
            &self.state,
            &registry,
            path,
            file.language,
            &file.content,
            section,
        );
        let refs = |ids: &[String]| -> Vec<MentorRef> {
            ids.iter()
                .filter_map(|id| mentor_ref(&registry, id))
                .collect()
        };
        ServerMessage::Casting {
            path: path.to_string(),
            section_id,
            section_name: section.map(|s| s.name.clone()).filter(|n| !n.is_empty()),
            titular: casting
                .titular
                .as_deref()
                .and_then(|id| mentor_ref(&registry, id)),
            consultants: refs(&casting.consultants),
            available: refs(&casting.available),
            reasons: casting.reasons,
            forced: casting.forced.map(|by| match by {
                hodegos_mentors::ForcedBy::User => ForcedBy::User,
                hodegos_mentors::ForcedBy::Frontmatter => ForcedBy::Frontmatter,
            }),
        }
    }

    /// `mentors_changed` (toujours après un rechargement, à la connexion seulement
    /// s'il y a des fichiers en erreur), puis le casting s'il a changé.
    fn mentor_warnings(&mut self, reloaded: bool) -> Vec<Outgoing> {
        let registry = self.state.mentors.registry();
        let warnings: Vec<MentorWarning> = registry
            .warnings()
            .iter()
            .map(|w| MentorWarning {
                file: w.file.clone(),
                message: w.message.clone(),
            })
            .collect();
        let mut replies = Vec::new();
        if reloaded || !warnings.is_empty() {
            replies.push(Outgoing::push(ServerMessage::MentorsChanged { warnings }));
        }
        replies.extend(self.casting_if_changed());
        replies
    }

    /// Passe d'ouverture, si elle est due et qu'aucune n'est déjà en cours pour ce fichier.
    fn open_pass(&mut self, rel: RelPath, language: Language, content: String) -> Vec<Outgoing> {
        let registry = self.state.mentors.registry();
        if self.opening.contains(&rel)
            || !pass::wants_open_pass(&self.state, &registry, &rel, language, &content)
        {
            return Vec::new();
        }
        let replies = self.start_pass(PassRequest {
            path: rel.clone(),
            language,
            old: String::new(),
            new: content,
            cursor_line: None,
            trigger: Trigger::Open,
        });
        if replies
            .iter()
            .any(|r| matches!(r.message, ServerMessage::PassStarted { .. }))
        {
            self.opening.insert(rel);
        }
        replies
    }

    /// Lance la passe qui suit un enregistrement ou une ouverture, ou en donne l'issue
    /// tout de suite.
    fn start_pass(&mut self, request: PassRequest) -> Vec<Outgoing> {
        let started = Instant::now();
        let generation = {
            let counter = self.generations.entry(request.path.clone()).or_insert(0);
            *counter += 1;
            *counter
        };
        let registry = self.state.mentors.registry();
        match pass::prepare(&self.state, &registry, &request) {
            Prepared::Done(outcome) => {
                self.record_pass(
                    &request.path,
                    request.trigger,
                    outcome,
                    0,
                    started.elapsed().as_millis(),
                    0.0,
                );
                vec![Outgoing::push(self.state.status())]
            }
            Prepared::Run(run) => {
                let started_message = Outgoing::push(run.started.clone());
                let state = self.state.clone();
                let sender = self.passes.clone();
                tokio::spawn(async move {
                    let result = pass::run(state, run, generation, started).await;
                    let _ = sender.send(result);
                });
                vec![started_message]
            }
        }
    }

    /// Remarques d'une passe terminée, persistées puis envoyées, et nouvel état.
    fn pass_finished(&mut self, result: PassResult) -> Vec<Outgoing> {
        if result.trigger == Trigger::Open {
            self.opening.remove(&result.path);
        }
        let current = self.generations.get(&result.path) == Some(&result.generation);
        if !current {
            tracing::debug!(path = %result.path, "passe dépassée par un enregistrement plus récent");
            return vec![Outgoing::push(self.state.status())];
        }
        self.state.store.add(&result.remarks);
        // Une passe d'ouverture aboutie, même sans remarque, ne se refait pas ; après
        // une panne ou un budget atteint, elle sera retentée à la prochaine ouverture.
        if result.trigger == Trigger::Open && result.outcome == PassOutcome::Ok {
            self.state.store.mark_opened(result.path.as_str());
        }
        self.record_pass(
            &result.path,
            result.trigger,
            result.outcome,
            result.remarks.len(),
            u128::from(result.duration_ms),
            result.cost_usd,
        );
        vec![
            Outgoing::push(ServerMessage::Remarks {
                path: result.path.to_string(),
                items: result.remarks,
            }),
            Outgoing::push(self.state.status()),
        ]
    }

    /// Dernière passe, pour la barre d'état, et une ligne de journal par passe.
    fn record_pass(
        &self,
        path: &RelPath,
        trigger: Trigger,
        outcome: PassOutcome,
        remarks: usize,
        duration_ms: u128,
        cost_usd: f64,
    ) {
        tracing::info!(
            %path,
            declencheur = trigger.as_str(),
            issue = outcome.as_str(),
            remarques = remarks,
            duree_ms = u64::try_from(duration_ms).unwrap_or(u64::MAX),
            cout_usd = cost_usd,
            "passe"
        );
        self.state.set_last_pass(LastPass {
            at: Timestamp::now().to_string(),
            path: path.to_string(),
            outcome,
            remarks,
            duration_ms: u64::try_from(duration_ms).unwrap_or(u64::MAX),
        });
    }

    /// Relit les fichiers ouverts touchés ; `file_changed` si leur version a changé.
    fn changed(&mut self, paths: &[RelPath]) -> Vec<Outgoing> {
        let mut replies = Vec::new();
        for path in paths {
            let Some(known) = self.open.get(path).map(|f| f.version.clone()) else {
                continue;
            };
            let current = match self.state.project.read_file(&self.session, path) {
                Ok(file) => Some(file),
                Err(err) => {
                    tracing::debug!(%path, error = %err, "fichier devenu illisible");
                    None
                }
            };
            let version = current.as_ref().map(|f| f.version.clone());
            if version == known {
                continue;
            }
            replies.push(Outgoing::push(ServerMessage::FileChanged {
                path: path.to_string(),
                version: version.as_ref().map(|v| v.as_str().to_owned()),
            }));
            match current {
                Some(file) => {
                    self.open.insert(
                        path.clone(),
                        OpenFile::new(file.version, file.content, file.language),
                    );
                    replies.extend(self.reanchor(path));
                }
                None => {
                    if let Some(open) = self.open.get_mut(path) {
                        open.version = None;
                    }
                }
            }
        }
        replies.extend(self.casting_if_changed());
        replies
    }
}

/// Titulaire et consultant d'un fichier, sur son chemin et son langage seulement.
fn file_mentors(registry: &Registry, path: &str) -> Vec<MentorRef> {
    let casting = cast(
        registry,
        &CastRequest {
            path,
            language: Language::of(path).as_str(),
            ..CastRequest::default()
        },
    );
    casting
        .titular
        .iter()
        .chain(casting.consultants.iter())
        .filter_map(|id| mentor_ref(registry, id))
        .collect()
}

const BUSY: &str = "une demande est déjà en cours pour cette remarque";

fn busy_error(re: Option<String>) -> Outgoing {
    Outgoing::error(re, ErrorCode::BadMessage, BUSY)
}

fn remark_update(re: Option<String>, remark: Remark) -> Outgoing {
    Outgoing::reply(re, ServerMessage::RemarkUpdate(Box::new(remark)))
}

fn section_info(section: &Section) -> SectionInfo {
    SectionInfo {
        id: section.id.clone(),
        kind: section.kind.clone(),
        name: section.name.clone(),
        start_line: section.start_line,
        end_line: section.end_line,
        parents: section.parents.clone(),
    }
}

fn error_reply(re: Option<String>, err: FileError) -> Outgoing {
    let message = err.to_string();
    let code = match err {
        FileError::Forbidden(_) => ErrorCode::PathForbidden,
        FileError::NotFound(_) => ErrorCode::NotFound,
        FileError::NotText(_) => ErrorCode::NotText,
        FileError::Conflict { current, .. } => {
            return Outgoing::reply(
                re,
                ServerMessage::Error {
                    code: ErrorCode::VersionConflict,
                    message,
                    version: Some(current.as_str().to_owned()),
                },
            );
        }
        FileError::WrongSession | FileError::Watch(_) | FileError::Io { .. } => {
            tracing::error!(error = %message, "erreur inattendue");
            return Outgoing::error(
                re,
                ErrorCode::Internal,
                "erreur inattendue, voir le journal du serveur",
            );
        }
    };
    Outgoing::error(re, code, message)
}
