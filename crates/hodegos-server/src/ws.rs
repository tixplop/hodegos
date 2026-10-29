use std::collections::HashMap;
use std::sync::Arc;

use axum::Extension;
use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use hodegos_core::{EntryKind, FileError, RelPath, Session, UserId, Version};
use tokio::sync::broadcast::error::RecvError;

use crate::AppState;
use crate::protocol::{self, ClientMessage, DirEntry, ErrorCode, Outgoing, ServerMessage};

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

/// Une fenêtre connectée : sa session et la version connue de chaque fichier ouvert.
struct Connection {
    state: Arc<AppState>,
    session: Session,
    open: HashMap<RelPath, Option<Version>>,
}

async fn run(mut socket: WebSocket, state: Arc<AppState>, user: UserId) {
    let mut changes = state.changes.subscribe();
    let session = state.project.session(user);
    let mut connection = Connection {
        state,
        session,
        open: HashMap::new(),
    };
    loop {
        let replies = tokio::select! {
            message = socket.recv() => match message {
                Some(Ok(Message::Text(text))) => vec![connection.handle_text(text.as_str())],
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
        };
        for reply in replies {
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
    }
}

impl Connection {
    fn handle_text(&mut self, text: &str) -> Outgoing {
        let (re, message) = match protocol::parse(text) {
            Ok(parsed) => parsed,
            Err(err) => return err.into(),
        };
        match self.handle(message) {
            Ok(reply) => Outgoing::reply(re, reply),
            Err(err) => error_reply(re, err),
        }
    }

    fn handle(&mut self, message: ClientMessage) -> Result<ServerMessage, FileError> {
        let project = &self.state.project;
        match message {
            ClientMessage::Ping => Ok(ServerMessage::Pong),
            ClientMessage::ListDir { path } => {
                let rel = RelPath::parse(&path)?;
                let entries = project
                    .list_dir(&self.session, &rel)?
                    .into_iter()
                    .map(|entry| DirEntry {
                        name: entry.name,
                        kind: match entry.kind {
                            EntryKind::Dir => protocol::EntryKind::Dir,
                            EntryKind::File => protocol::EntryKind::File,
                        },
                        mentors: Vec::new(),
                    })
                    .collect();
                Ok(ServerMessage::Dir {
                    path: rel.to_string(),
                    entries,
                })
            }
            ClientMessage::OpenFile { path } => {
                let rel = RelPath::parse(&path)?;
                let file = project.read_file(&self.session, &rel)?;
                self.track(&rel, file.version.clone());
                Ok(ServerMessage::File {
                    path: rel.to_string(),
                    content: file.content,
                    version: file.version.as_str().to_owned(),
                    language: file.language.as_str().to_owned(),
                    sections: Vec::new(),
                    remarks: Vec::new(),
                })
            }
            ClientMessage::Save {
                path,
                content,
                base_version,
            } => {
                let rel = RelPath::parse(&path)?;
                let base = Version::from_client(base_version);
                let version = project.write_file(&self.session, &rel, &content, &base)?;
                self.track(&rel, version.clone());
                Ok(ServerMessage::Saved {
                    path: rel.to_string(),
                    version: version.as_str().to_owned(),
                })
            }
        }
    }

    /// Retient la version connue du fichier et fait observer son dossier.
    fn track(&mut self, rel: &RelPath, version: Version) {
        if let Err(err) = self.state.watcher.track(&self.session, rel) {
            tracing::warn!(path = %rel, error = %err, "modifications externes non suivies");
        }
        self.open.insert(rel.clone(), Some(version));
    }

    /// Compare la version sur disque des fichiers ouverts touchés à celle connue.
    fn changed(&mut self, paths: &[RelPath]) -> Vec<Outgoing> {
        let mut replies = Vec::new();
        for path in paths {
            let Some(known) = self.open.get_mut(path) else {
                continue;
            };
            let current = self
                .state
                .project
                .file_version(&self.session, path)
                .unwrap_or_else(|err| {
                    tracing::debug!(%path, error = %err, "fichier devenu illisible");
                    None
                });
            if current != *known {
                replies.push(Outgoing::push(ServerMessage::FileChanged {
                    path: path.to_string(),
                    version: current.as_ref().map(|v| v.as_str().to_owned()),
                }));
                *known = current;
            }
        }
        replies
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
