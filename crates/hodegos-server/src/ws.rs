use std::sync::Arc;

use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};

use crate::AppState;
use crate::protocol;

/// Mise à niveau WebSocket : le cookie est déjà vérifié par la garde, on vérifie ici `Origin`.
pub(crate) async fn upgrade(
    State(state): State<Arc<AppState>>,
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
    ws.on_upgrade(session)
}

async fn session(mut socket: WebSocket) {
    while let Some(Ok(message)) = socket.recv().await {
        let reply = match message {
            Message::Text(text) => protocol::handle_text(text.as_str()),
            Message::Close(_) => break,
            _ => continue,
        };
        let text = match serde_json::to_string(&reply) {
            Ok(text) => text,
            Err(e) => {
                tracing::error!(error = %e, "réponse impossible à sérialiser");
                continue;
            }
        };
        if socket.send(Message::Text(text.into())).await.is_err() {
            break;
        }
    }
}
