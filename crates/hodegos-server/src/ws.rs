use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::sync::Arc;
use std::time::Instant;

use axum::Extension;
use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use base64::Engine as _;
use hodegos_core::{
    DeletePlan, EntryKind, FileError, GitFileStatus, Language, MovePlan, RelPath, Section, Session,
    UserId, Version, section_at, sections,
};
use hodegos_mentors::{CastRequest, FollowUpKind, Registry, cast};
use jiff::Timestamp;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::mpsc;

use crate::AppState;
use crate::followup;
use crate::mentor_settings::{self, Content};
use crate::pass::{
    self, PassRequest, PassResult, Prepared, Trigger, cast_section, locate_anchor, mentor_ref,
};
use crate::project_map;
use crate::protocol::{
    self, AnyRemark, ClientMessage, DirEntry, ErrorCode, ForcedBy, LastPass, MentorRef,
    MentorWarning, Outgoing, PassOutcome, Remark, SectionInfo, ServerMessage, ThreadEntry,
};
use crate::review;
use crate::settings;
use crate::terminal;

/// Longueur maximale d'une réponse de l'utilisateur.
const MAX_REPLY_CHARS: usize = 2000;
/// Dossiers renvoyés par `list_dirs` au plus.
const MAX_LISTED_DIRS: usize = 5000;

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
    /// Réponses des demandes traitées en tâche de fond (réglages).
    background: mpsc::UnboundedSender<Vec<Outgoing>>,
    /// Dossiers affichés dans l'arbre (listés par `list_dir`) et leurs entrées : un
    /// changement d'entrées, venu du terminal ou d'ailleurs, repousse `dir`.
    listed: HashMap<RelPath, Vec<(String, protocol::EntryKind)>>,
    /// Terminaux de cette fenêtre : ils meurent avec elle.
    terminals: HashMap<u32, terminal::Session>,
    next_terminal: u32,
    /// Sorties des terminaux ; borné : une fenêtre lente suspend leur lecture.
    terminal_events: mpsc::Sender<terminal::Event>,
}

async fn run(mut socket: WebSocket, state: Arc<AppState>, user: UserId) {
    let mut changes = state.changes.subscribe();
    let mut reloaded = state.mentors.reloaded.subscribe();
    let mut git = state.git.updates.subscribe();
    let session = state.project.session(user);
    let (passes, mut finished) = mpsc::unbounded_channel();
    let (followups, mut followed) = mpsc::unbounded_channel();
    let (background, mut answered) = mpsc::unbounded_channel();
    let (terminal_events, mut terminal_output) = mpsc::channel(16);
    let mut events = state.events.subscribe();
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
        background,
        listed: HashMap::new(),
        terminals: HashMap::new(),
        next_terminal: 0,
        terminal_events,
    };
    let mut pending = vec![
        Outgoing::push(connection.state.connection_status()),
        Outgoing::push(ServerMessage::GitStatus(connection.state.git.current())),
        Outgoing::push(crate::auto::status(&connection.state)),
    ];
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
            update = git.recv() => match update {
                Ok(status) => vec![Outgoing::push(ServerMessage::GitStatus(status))],
                Err(RecvError::Lagged(_)) => {
                    vec![Outgoing::push(ServerMessage::GitStatus(connection.state.git.current()))]
                }
                Err(RecvError::Closed) => break,
            },
            Some(result) = finished.recv() => connection.pass_finished(result),
            Some(done) = followed.recv() => connection.followup_finished(done),
            Some(replies) = answered.recv() => replies,
            Some(event) = terminal_output.recv() => connection.terminal_event(event),
            event = events.recv() => match event {
                Ok(message) => vec![Outgoing::push(message)],
                Err(RecvError::Lagged(_)) => vec![
                    Outgoing::push(settings::info(&connection.state)),
                    Outgoing::push(connection.state.status()),
                ],
                Err(RecvError::Closed) => break,
            },
        };
    }
}

impl Connection {
    fn open_terminal(&mut self, re: Option<String>, cols: u16, rows: u16) -> Outgoing {
        let terminals = &self.state.terminals;
        if let Err(message) = terminals.check() {
            return Outgoing::error(re, ErrorCode::TerminalUnavailable, message);
        }
        if self.terminals.len() >= terminal::MAX_PER_WINDOW {
            return Outgoing::error(
                re,
                ErrorCode::TerminalLimit,
                format!("{} terminaux au plus par fenêtre", terminal::MAX_PER_WINDOW),
            );
        }
        self.next_terminal += 1;
        let id = self.next_terminal;
        let launch = terminals.launch(self.state.project.root());
        match terminal::spawn(launch, id, cols, rows, self.terminal_events.clone()) {
            Ok(session) => {
                self.terminals.insert(id, session);
                tracing::info!(terminal = id, "terminal ouvert");
                Outgoing::reply(
                    re,
                    ServerMessage::TermOpened {
                        term_id: id,
                        shell: terminals.shell_name(),
                        warning: terminals.warning(),
                    },
                )
            }
            Err(message) => Outgoing::error(re, ErrorCode::Internal, message),
        }
    }

    fn terminal_input(&mut self, re: Option<String>, id: u32, data: &str) -> Vec<Outgoing> {
        let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(data) else {
            return vec![Outgoing::error(
                re,
                ErrorCode::BadMessage,
                "saisie : base64 attendu",
            )];
        };
        if bytes.len() > terminal::MAX_INPUT {
            return vec![Outgoing::error(
                re,
                ErrorCode::BadMessage,
                "saisie de plus de 64 Kio : envoyez-la en plusieurs fois",
            )];
        }
        let Some(session) = self.terminals.get(&id) else {
            return vec![Outgoing::error(
                re,
                ErrorCode::NotFound,
                format!("terminal {id} fermé"),
            )];
        };
        match session.write(bytes) {
            Ok(()) => Vec::new(),
            Err(message) => vec![Outgoing::error(re, ErrorCode::TerminalLimit, message)],
        }
    }

    fn terminal_event(&mut self, event: terminal::Event) -> Vec<Outgoing> {
        match event {
            terminal::Event::Output { id, data } => {
                vec![Outgoing::push(ServerMessage::TermOutput {
                    term_id: id,
                    data: base64::engine::general_purpose::STANDARD.encode(data),
                })]
            }
            terminal::Event::Exited { id, code } => {
                self.terminals.remove(&id);
                tracing::info!(terminal = id, ?code, "terminal terminé");
                vec![Outgoing::push(ServerMessage::TermExited {
                    term_id: id,
                    code,
                })]
            }
        }
    }

    /// Traite une demande de réglages en tâche de fond (catalogue, évaluation, réseau) ;
    /// la réponse revient par `background`.
    fn in_background<F, Fut>(&self, re: Option<String>, work: F)
    where
        F: FnOnce(Arc<AppState>) -> Fut,
        Fut: Future<Output = Result<ServerMessage, settings::Refused>> + Send + 'static,
    {
        let future = work(self.state.clone());
        let sender = self.background.clone();
        tokio::spawn(async move {
            let _ = sender.send(vec![settings_reply(re, future.await)]);
        });
    }

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
            ClientMessage::ProjectMapGet => {
                self.in_background(re, project_map::get);
                Ok(Vec::new())
            }
            ClientMessage::ProjectMapSet {
                map_cap,
                disabled_blocks,
                exclude,
            } => {
                self.in_background(re, move |state| {
                    project_map::set(state, map_cap, disabled_blocks, exclude)
                });
                Ok(Vec::new())
            }
            ClientMessage::SettingsGet => {
                Ok(vec![Outgoing::reply(re, settings::info(&self.state))])
            }
            ClientMessage::SettingsSet {
                fast,
                strong,
                fast_effort,
                strong_effort,
                budget,
                confirm_eval,
            } => {
                let request = settings::Request {
                    fast,
                    strong,
                    fast_effort,
                    strong_effort,
                    budget,
                    confirm_eval,
                };
                self.in_background(
                    re,
                    |state| async move { settings::set(&state, request).await },
                );
                Ok(Vec::new())
            }
            ClientMessage::ModelCheck { id } => {
                self.in_background(re, |state| async move {
                    settings::model_check(&state, &id).await
                });
                Ok(Vec::new())
            }
            ClientMessage::ApiKeySet { key } => Ok(vec![settings_reply(
                re,
                settings::set_key(&self.state, &key.0),
            )]),
            ClientMessage::MentorsList => Ok(vec![Outgoing::reply(
                re,
                mentor_settings::list(&self.state),
            )]),
            ClientMessage::MentorGet { id, origin } => Ok(vec![mentor_reply(
                re,
                mentor_settings::get(&self.state, &id, origin),
            )]),
            ClientMessage::MentorSave {
                origin,
                id,
                content,
                form,
                body,
            } => {
                let content = match (content, form) {
                    (Some(text), _) => Content::Raw(text),
                    (None, Some(form)) => Content::Form(form, body.unwrap_or_default()),
                    (None, None) => {
                        return Ok(vec![Outgoing::error(
                            re,
                            ErrorCode::BadMessage,
                            "content ou form attendu",
                        )]);
                    }
                };
                Ok(vec![mentor_reply(
                    re,
                    mentor_settings::save(&self.state, origin, &id, content),
                )])
            }
            ClientMessage::MentorEject { id, to } => Ok(vec![mentor_reply(
                re,
                mentor_settings::personalize(&self.state, &id, to),
            )]),
            ClientMessage::MentorDelete { id, origin } => Ok(vec![mentor_reply(
                re,
                mentor_settings::delete(&self.state, &id, origin),
            )]),
            ClientMessage::MentorPrompt { id, content } => Ok(vec![mentor_reply(
                re,
                mentor_settings::prompt(&self.state, &id, content.as_deref()),
            )]),
            ClientMessage::TermOpen { cols, rows } => Ok(vec![self.open_terminal(re, cols, rows)]),
            ClientMessage::TermInput { term_id, data } => {
                Ok(self.terminal_input(re, term_id, &data))
            }
            ClientMessage::TermResize {
                term_id,
                cols,
                rows,
            } => {
                if let Some(session) = self.terminals.get(&term_id) {
                    session.resize(cols, rows);
                }
                Ok(Vec::new())
            }
            ClientMessage::TermClose { term_id } => {
                // Le shell tué, `term_exited` suit.
                if let Some(mut session) = self.terminals.remove(&term_id) {
                    session.kill();
                }
                Ok(Vec::new())
            }
            ClientMessage::ProviderTest => {
                self.in_background(re, |state| async move {
                    Ok(settings::provider_test(&state).await)
                });
                Ok(Vec::new())
            }
            ClientMessage::ListDir { path } => {
                let rel = RelPath::parse(&path)?;
                let listing = self.dir(&rel)?;
                self.remember_listing(&rel, &listing);
                Ok(vec![Outgoing::reply(re, listing)])
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
                    project_remarks: self.state.project_store.citing(rel.as_str()),
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
                skip_pass,
            } => {
                let rel = RelPath::parse(&path)?;
                let base = Version::from_client(base_version);
                let version = project.write_file(&self.session, &rel, &content, &base)?;
                self.state.git.refresh();
                self.state
                    .map
                    .touched(vec![rel.clone()], protocol::MapReason::Save);
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
                if skip_pass {
                    // La passe viendra de l'activation qui suit ; une passe plus ancienne
                    // ne s'applique plus à cette version.
                    *self.generations.entry(rel).or_insert(0) += 1;
                    return Ok(replies);
                }
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
                activate,
                confirm_over_budget,
            } => {
                let rel = RelPath::parse(&path)?;
                if let Some(id) = &mentor_id {
                    if self.state.mentors.registry().get(id).is_none() {
                        return Err(FileError::NotFound(format!("mentor {id}")));
                    }
                }
                // Revenir au choix automatique ne lance rien.
                let activate = activate && mentor_id.is_some();
                if activate
                    && !confirm_over_budget
                    && self.state.budget.state(Timestamp::now()).level
                        == hodegos_llm::BudgetLevel::Exceeded
                {
                    return Ok(vec![Outgoing::error(
                        re,
                        ErrorCode::BudgetExceeded,
                        "budget du mois atteint : confirmez pour demander cette relecture",
                    )]);
                }
                let section = section_id.clone().unwrap_or_default();
                self.state
                    .mentors
                    .set_forced(rel.as_str(), &section, mentor_id.clone());
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
                let mut replies = vec![Outgoing::reply(re, casting)];
                if let (true, Some(mentor_id)) = (activate, &mentor_id) {
                    let target = section_id.or_else(|| {
                        section_at(&self.open[&rel].sections, line).map(|s| s.id.clone())
                    });
                    match target {
                        Some(target) => replies.extend(self.request_pass(&rel, &target, mentor_id)),
                        None => tracing::info!(path = %rel, "aucune section à relire"),
                    }
                }
                Ok(replies)
            }
            // Remarques de projet (phase 9c) : identifiants `rp_`.
            ClientMessage::HintNext {
                remark_id,
                confirm_over_budget,
            } if remark_id.starts_with("rp_") => {
                self.in_background(re, move |state| {
                    review::follow_up(
                        state,
                        remark_id,
                        FollowUpKind::NextLevel,
                        confirm_over_budget,
                    )
                });
                Ok(Vec::new())
            }
            ClientMessage::RemarkReply {
                remark_id, text, ..
            } if remark_id.starts_with("rp_") => Ok(self.project_reply(re, remark_id, &text)),
            ClientMessage::RemarkDismiss { remark_id, reason } if remark_id.starts_with("rp_") => {
                Ok(vec![settings_reply(
                    re,
                    review::dismiss(&self.state, &remark_id, reason.status()),
                )])
            }
            ClientMessage::ReviewEstimate { mentor_ids } => {
                self.in_background(re, move |state| review::estimate(state, mentor_ids));
                Ok(Vec::new())
            }
            ClientMessage::ReviewRequest {
                mentor_ids,
                confirm_over_budget,
            } => {
                self.in_background(re, move |state| {
                    review::request(state, mentor_ids, confirm_over_budget, "request")
                });
                Ok(Vec::new())
            }
            ClientMessage::ProjectRemarksGet => {
                Ok(vec![Outgoing::reply(re, review::remarks(&self.state))])
            }
            ClientMessage::ReviewsGet => {
                Ok(vec![Outgoing::reply(re, review::reviews(&self.state))])
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
            ClientMessage::CreateEntry { path, kind } => {
                if path.split('/').any(|part| part.is_empty() || part == ".") {
                    return Err(FileError::InvalidName(path));
                }
                let rel = RelPath::parse(&path)?;
                let core_kind = match kind {
                    protocol::EntryKind::Dir => EntryKind::Dir,
                    protocol::EntryKind::File => EntryKind::File,
                };
                project.create_entry(&self.session, &rel, core_kind)?;
                self.state.git.refresh();
                Ok(vec![
                    Outgoing::reply(
                        re,
                        ServerMessage::EntryCreated {
                            path: rel.to_string(),
                            kind,
                        },
                    ),
                    Outgoing::push(self.parent_listing(&rel)?),
                ])
            }
            ClientMessage::DeletePreview { path } => {
                let rel = RelPath::parse(&path)?;
                let plan = project.delete_preview(&self.session, &rel)?;
                Ok(vec![Outgoing::reply(re, self.delete_preview(&rel, &plan))])
            }
            ClientMessage::DeleteEntry { path } => {
                let rel = RelPath::parse(&path)?;
                let plan = project.delete_entry(&self.session, &rel)?;
                self.state.git.refresh();
                self.forget(&plan);
                Ok(vec![
                    Outgoing::reply(
                        re,
                        ServerMessage::EntryDeleted {
                            path: rel.to_string(),
                            kind: protocol_kind(plan.kind),
                        },
                    ),
                    Outgoing::push(self.parent_listing(&rel)?),
                ])
            }
            ClientMessage::MoveEntry { from, to_dir } => {
                let from = RelPath::parse(&from)?;
                let to_dir = RelPath::parse(&to_dir)?;
                let state = &self.state;
                let plan = project
                    .move_entry(&self.session, &from, &to_dir, |plan| move_data(state, plan))?;
                self.state.git.refresh();
                let mut replies = vec![Outgoing::reply(
                    re,
                    ServerMessage::EntryMoved {
                        from: plan.from.to_string(),
                        to: plan.to.to_string(),
                        kind: protocol_kind(plan.kind),
                    },
                )];
                replies.extend(self.follow(&plan));
                Ok(replies)
            }
            ClientMessage::ListDirs => {
                let (dirs, truncated) = project.list_all_dirs(&self.session, MAX_LISTED_DIRS)?;
                Ok(vec![Outgoing::reply(
                    re,
                    ServerMessage::Dirs {
                        paths: dirs.iter().map(ToString::to_string).collect(),
                        truncated,
                    },
                )])
            }
        }
    }

    /// Entrées d'un dossier, avec les mentors de chaque fichier.
    fn dir(&self, rel: &RelPath) -> Result<ServerMessage, FileError> {
        let registry = self.state.mentors.registry();
        let entries = self
            .state
            .project
            .list_dir(&self.session, rel)?
            .into_iter()
            .map(|entry| {
                let mentors = match entry.kind {
                    EntryKind::Dir => Vec::new(),
                    EntryKind::File => file_mentors(&registry, rel.join(&entry.name).as_str()),
                };
                DirEntry {
                    name: entry.name,
                    kind: protocol_kind(entry.kind),
                    mentors,
                }
            })
            .collect();
        Ok(ServerMessage::Dir {
            path: rel.to_string(),
            entries,
        })
    }

    /// Ce que la suppression emporterait, d'après le statut git actuel.
    fn delete_preview(&self, rel: &RelPath, plan: &DeletePlan) -> ServerMessage {
        let project = &self.state.project;
        let git = project
            .git_repo(&self.session)
            .ok()
            .flatten()
            .and_then(|repo| project.git_status(&self.session, &repo).ok().flatten());
        let statuses: HashMap<&str, GitFileStatus> = git
            .iter()
            .flat_map(|status| &status.entries)
            .map(|entry| (entry.path.as_str(), entry.status))
            .collect();
        let (mut untracked, mut modified, mut ignored) = (0, 0, 0);
        for file in &plan.files {
            if file.ignored {
                ignored += 1;
                continue;
            }
            match statuses.get(file.path.as_str()) {
                _ if git.is_none() => untracked += 1,
                Some(GitFileStatus::Untracked | GitFileStatus::Added) => untracked += 1,
                Some(_) => modified += 1,
                None => {}
            }
        }
        ServerMessage::DeletePreview {
            path: rel.to_string(),
            kind: protocol_kind(plan.kind),
            files: plan.files.len(),
            untracked,
            modified,
            ignored,
            restorable: git.is_some() && untracked + modified + ignored == 0,
        }
    }

    /// Fichiers supprimés depuis Hodegos : remarques, choix forcés et passe d'ouverture
    /// oubliés (ADR 0006), onglets fermés côté serveur, passes en cours caduques.
    fn forget(&mut self, plan: &DeletePlan) {
        let paths: HashSet<String> = plan.files.iter().map(|f| f.path.to_string()).collect();
        self.state.store.forget(&paths);
        self.state.mentors.forget(&paths);
        crate::auto::forgotten(&self.state, &paths);
        for file in &plan.files {
            if self.open.remove(&file.path).is_some() {
                *self.generations.entry(file.path.clone()).or_insert(0) += 1;
            }
            self.opening.remove(&file.path);
        }
        if self
            .cursor
            .as_ref()
            .is_some_and(|(path, _)| paths.contains(path.as_str()))
        {
            self.cursor = None;
            self.last_casting = None;
        }
    }

    /// Fichiers déplacés depuis Hodegos : fichiers ouverts et curseur suivent le nouveau
    /// chemin, passes en cours caduques, dossiers affichés à jour ; `dir` des deux
    /// parents, puis le casting s'il a changé. L'observateur, qui verra l'ancien chemin
    /// disparaître, ne trouve plus de fichier ouvert à cet endroit : pas de `file_changed`.
    fn follow(&mut self, plan: &MovePlan) -> Vec<Outgoing> {
        for file in &plan.files {
            if let Some(open) = self.open.remove(&file.from) {
                *self.generations.entry(file.from.clone()).or_insert(0) += 1;
                self.open.insert(file.to.clone(), open);
                self.watch(&file.to);
            }
            self.opening.remove(&file.from);
            if let Some((path, line)) = &self.cursor {
                if *path == file.from {
                    self.cursor = Some((file.to.clone(), *line));
                }
            }
        }
        let inside = format!("{}/", plan.from);
        self.listed
            .retain(|dir, _| *dir != plan.from && !dir.as_str().starts_with(&inside));
        let mut replies = Vec::new();
        for rel in [&plan.from, &plan.to] {
            match self.parent_listing(rel) {
                Ok(listing) => replies.push(Outgoing::push(listing)),
                Err(err) => tracing::warn!(path = %rel, error = %err, "dossier non relu"),
            }
        }
        replies.extend(self.casting_if_changed());
        replies
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

    /// Réponse à une remarque de projet : `remark_update` tout de suite, puis la relance
    /// du mentor, poussée à toutes les fenêtres quand elle arrive.
    fn project_reply(&mut self, re: Option<String>, id: String, text: &str) -> Vec<Outgoing> {
        let text = text.trim();
        if text.is_empty() || text.chars().count() > MAX_REPLY_CHARS {
            return vec![Outgoing::error(
                re,
                ErrorCode::BadMessage,
                "réponse vide ou de plus de 2 000 caractères",
            )];
        }
        let remark = match review::reply(&self.state, &id, text) {
            Ok(remark) => remark,
            Err(refused) => return vec![Outgoing::error(re, refused.code, refused.message)],
        };
        let state = self.state.clone();
        tokio::spawn(async move {
            match review::follow_up(state.clone(), id, FollowUpKind::Reply, true).await {
                Ok(update) => {
                    let _ = state.events.send(update);
                }
                Err(refused) => {
                    tracing::info!(message = %refused.message, "réponse enregistrée sans relance du mentor");
                }
            }
        });
        vec![Outgoing::reply(
            re,
            ServerMessage::RemarkUpdate(Box::new(AnyRemark::Project(remark))),
        )]
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

    /// Contenu du dossier parent après une création ou une suppression, retenu pour
    /// que l'observateur ne le repousse pas.
    fn parent_listing(&mut self, rel: &RelPath) -> Result<ServerMessage, FileError> {
        let parent = rel.parent();
        let listing = self.dir(&parent)?;
        self.remember_listing(&parent, &listing);
        Ok(listing)
    }

    /// Retient les entrées d'un dossier affiché et le fait observer.
    fn remember_listing(&mut self, rel: &RelPath, listing: &ServerMessage) {
        let ServerMessage::Dir { entries, .. } = listing else {
            return;
        };
        let names = entries.iter().map(|e| (e.name.clone(), e.kind)).collect();
        if self.listed.insert(rel.clone(), names).is_none() {
            if let Err(err) = self.state.watcher.track_dir(&self.session, rel) {
                tracing::warn!(path = %rel, error = %err, "dossier non observé");
            }
        }
    }

    /// Dossiers affichés dont les entrées ont changé (ajout, suppression, renommage ;
    /// fichiers ignorés exclus) : leur nouveau contenu, poussé en `dir`.
    fn listings_changed(&mut self, paths: &[RelPath]) -> Vec<Outgoing> {
        let mut dirs: Vec<RelPath> = paths
            .iter()
            .flat_map(|path| [path.parent(), path.clone()])
            .filter(|dir| self.listed.contains_key(dir))
            .collect();
        dirs.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        dirs.dedup();
        let mut replies = Vec::new();
        for dir in dirs {
            let Ok(listing) = self.dir(&dir) else {
                // Dossier disparu : son parent, s'il est affiché, le dira.
                self.listed.remove(&dir);
                continue;
            };
            let ServerMessage::Dir { entries, .. } = &listing else {
                continue;
            };
            let names: Vec<(String, protocol::EntryKind)> =
                entries.iter().map(|e| (e.name.clone(), e.kind)).collect();
            if self.listed.get(&dir) != Some(&names) {
                self.listed.insert(dir, names);
                replies.push(Outgoing::push(listing));
            }
        }
        replies
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
        self.begin_pass(&request.path, request.trigger, |state, registry| {
            pass::prepare(state, registry, &request)
        })
    }

    /// Prépare une passe (nouvelle génération pour le fichier) puis la lance en tâche
    /// de fond, ou en donne l'issue tout de suite.
    fn begin_pass(
        &mut self,
        path: &RelPath,
        trigger: Trigger,
        prepare: impl FnOnce(&AppState, &Registry) -> Prepared,
    ) -> Vec<Outgoing> {
        let started = Instant::now();
        let generation = {
            let counter = self.generations.entry(path.clone()).or_insert(0);
            *counter += 1;
            *counter
        };
        let registry = self.state.mentors.registry();
        match prepare(&self.state, &registry) {
            Prepared::Done(outcome) => {
                self.record_pass(
                    path,
                    trigger,
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

    /// Passe demandée en activant `mentor_id` : la section `section_id` de la version
    /// enregistrée, même sans changement.
    fn request_pass(&mut self, rel: &RelPath, section_id: &str, mentor_id: &str) -> Vec<Outgoing> {
        let file = &self.open[rel];
        let request = PassRequest {
            path: rel.clone(),
            language: file.language,
            old: file.content.clone(),
            new: file.content.clone(),
            cursor_line: None,
            trigger: Trigger::Request,
        };
        self.begin_pass(rel, Trigger::Request, |state, registry| {
            match registry.get(mentor_id) {
                Some(mentor) => {
                    pass::prepare_request(state, registry, &request, section_id, mentor)
                }
                None => Prepared::Done(PassOutcome::NoMentor),
            }
        })
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
        let mut replies = self.listings_changed(paths);
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

fn protocol_kind(kind: EntryKind) -> protocol::EntryKind {
    match kind {
        EntryKind::Dir => protocol::EntryKind::Dir,
        EntryKind::File => protocol::EntryKind::File,
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
    Outgoing::reply(
        re,
        ServerMessage::RemarkUpdate(Box::new(AnyRemark::Section(remark))),
    )
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

fn mentor_reply(
    re: Option<String>,
    result: Result<ServerMessage, mentor_settings::Refused>,
) -> Outgoing {
    match result {
        Ok(message) => Outgoing::reply(re, message),
        Err(refused) => Outgoing::reply(
            re,
            ServerMessage::Error {
                code: refused.code,
                message: refused.message,
                version: None,
                errors: refused.errors,
            },
        ),
    }
}

fn settings_reply(
    re: Option<String>,
    result: Result<ServerMessage, settings::Refused>,
) -> Outgoing {
    match result {
        Ok(message) => Outgoing::reply(re, message),
        Err(refused) => Outgoing::error(re, refused.code, refused.message),
    }
}

/// Remarques, passes d'ouverture et choix forcés suivent un déplacement (ADR 0008) ;
/// si un fichier de données ne peut pas être écrit, tout revient à l'état d'avant.
fn move_data(state: &AppState, plan: &MovePlan) -> Result<(), FileError> {
    let moves: HashMap<String, String> = plan
        .files
        .iter()
        .map(|f| (f.from.to_string(), f.to.to_string()))
        .collect();
    let data_error = |source| FileError::Io {
        path: "données du projet".to_owned(),
        source,
    };
    let before = state.store.snapshot();
    let before_project = state.project_store.snapshot();
    let result = state
        .store
        .move_paths(&moves)
        .and_then(|()| state.project_store.move_paths(&moves))
        .and_then(|()| state.mentors.move_paths(&moves));
    if let Err(err) = result {
        // Tout revient à l'état d'avant : le déplacement sera annulé.
        if let Some(before) = before {
            state.store.restore(before);
        }
        if let Some(before) = before_project {
            state.project_store.restore(before);
        }
        return Err(data_error(err));
    }
    Ok(())
}

fn error_reply(re: Option<String>, err: FileError) -> Outgoing {
    let message = err.to_string();
    let code = match err {
        FileError::Forbidden(_) => ErrorCode::PathForbidden,
        FileError::NotFound(_) => ErrorCode::NotFound,
        FileError::AlreadyExists(_) => ErrorCode::AlreadyExists,
        FileError::InvalidName(_) => ErrorCode::BadMessage,
        FileError::InvalidMove(_) => ErrorCode::InvalidMove,
        FileError::NotText(_) => ErrorCode::NotText,
        FileError::Conflict { current, .. } => {
            return Outgoing::reply(
                re,
                ServerMessage::Error {
                    code: ErrorCode::VersionConflict,
                    message,
                    version: Some(current.as_str().to_owned()),
                    errors: Vec::new(),
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
