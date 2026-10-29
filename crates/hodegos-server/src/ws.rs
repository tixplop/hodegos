use std::collections::HashMap;
use std::sync::Arc;

use axum::Extension;
use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use hodegos_core::{
    EntryKind, FileError, Language, RelPath, Section, Session, UserId, Version, section_at,
    sections,
};
use hodegos_mentors::{CastRequest, Forced, Registry, cast, frontmatter_mentor};
use tokio::sync::broadcast::error::RecvError;

use crate::AppState;
use crate::protocol::{
    self, ClientMessage, DirEntry, ErrorCode, ForcedBy, MentorRef, MentorWarning, Outgoing,
    SectionInfo, ServerMessage,
};

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
}

async fn run(mut socket: WebSocket, state: Arc<AppState>, user: UserId) {
    let mut changes = state.changes.subscribe();
    let mut reloaded = state.mentors.reloaded.subscribe();
    let session = state.project.session(user);
    let mut connection = Connection {
        state,
        session,
        open: HashMap::new(),
        cursor: None,
        last_casting: None,
    };
    let mut pending = connection.mentor_warnings(false);
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
                let reply = ServerMessage::File {
                    path: rel.to_string(),
                    content: file.content,
                    version: file.version.as_str().to_owned(),
                    language: file.language.as_str().to_owned(),
                    sections: open.sections.iter().map(section_info).collect(),
                    remarks: Vec::new(),
                };
                self.open.insert(rel.clone(), open);
                self.cursor = Some((rel, 1));
                // Toujours un casting à l'ouverture, même identique au précédent.
                self.last_casting = None;
                let mut replies = vec![Outgoing::reply(re, reply)];
                replies.extend(self.casting_if_changed());
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
                self.open.insert(
                    rel.clone(),
                    OpenFile::new(version.clone(), content, language),
                );
                let mut replies = vec![Outgoing::reply(
                    re,
                    ServerMessage::Saved {
                        path: rel.to_string(),
                        version: version.as_str().to_owned(),
                    },
                )];
                replies.extend(self.casting_if_changed());
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
        }
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
        let text = section.map(|s| s.text(&file.content)).unwrap_or_default();
        let mut headings = Vec::new();
        if file.language == Language::Markdown {
            if let Some(section) = section.filter(|s| s.kind == "h") {
                headings.push(section.name.as_str());
            }
            headings.extend(
                section
                    .iter()
                    .flat_map(|s| s.parents.iter().map(String::as_str)),
            );
        }
        let section_id = section.map(|s| s.id.clone());
        let forced = self
            .state
            .mentors
            .forced(path.as_str(), section_id.as_deref().unwrap_or_default())
            .map(Forced::User)
            .or_else(|| {
                (file.language == Language::Markdown)
                    .then(|| frontmatter_mentor(&file.content))
                    .flatten()
                    .map(Forced::Frontmatter)
            });
        let casting = cast(
            &registry,
            &CastRequest {
                path: path.as_str(),
                language: file.language.as_str(),
                headings,
                text: &text,
                forced,
            },
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

fn mentor_ref(registry: &Registry, id: &str) -> Option<MentorRef> {
    registry.get(id).map(|m| MentorRef {
        id: m.id.clone(),
        name: m.name.clone(),
        initials: m.initials.clone(),
        role: m.role.clone(),
    })
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
