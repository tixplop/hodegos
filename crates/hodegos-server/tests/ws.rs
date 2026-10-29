#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::{connect, exchange, next, open_socket, refused_with, send_raw, start_in};
use serde_json::json;
use tokio_tungstenite::tungstenite::http::StatusCode;

#[tokio::test]
async fn ping_renvoie_pong() {
    let dir = tempfile::tempdir().unwrap();
    let mut socket = open_socket(dir.path()).await;
    let reply = exchange(&mut socket, json!({ "v": 1, "type": "ping", "id": "c-1" })).await;
    assert_eq!(reply, json!({ "v": 1, "type": "pong", "re": "c-1" }));
}

#[tokio::test]
async fn un_message_invalide_donne_bad_message_sans_couper_la_connexion() {
    let dir = tempfile::tempdir().unwrap();
    let mut socket = open_socket(dir.path()).await;

    send_raw(&mut socket, "pas du json").await;
    let reply = next(&mut socket).await;
    assert_eq!(reply["type"], "error");
    assert_eq!(reply["payload"]["code"], "bad_message");

    let reply = exchange(&mut socket, json!({ "v": 2, "type": "ping", "id": "c-2" })).await;
    assert_eq!(reply["payload"]["code"], "bad_message");
    assert_eq!(reply["re"], "c-2");

    let reply = exchange(&mut socket, json!({ "v": 1, "type": "ping", "id": "c-3" })).await;
    assert_eq!(reply["type"], "pong");
}

#[tokio::test]
async fn refuse_une_origin_etrangere_ou_absente() {
    let dir = tempfile::tempdir().unwrap();
    let (port, cookie) = start_in(dir.path()).await;
    let other_port = format!("http://127.0.0.1:{}", port.wrapping_add(1));
    let localhost = format!("http://localhost:{port}");
    for origin in [
        Some("http://evil.test"),
        Some(other_port.as_str()),
        Some(localhost.as_str()),
        Some("null"),
        None,
    ] {
        let status = refused_with(connect(port, origin, Some(&cookie)).await);
        assert_eq!(status, StatusCode::FORBIDDEN, "Origin {origin:?}");
    }
}

#[tokio::test]
async fn refuse_la_websocket_sans_cookie() {
    let dir = tempfile::tempdir().unwrap();
    let (port, _) = start_in(dir.path()).await;
    let origin = format!("http://127.0.0.1:{port}");
    let status = refused_with(connect(port, Some(&origin), None).await);
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let status = refused_with(connect(port, Some(&origin), Some("hodegos_1=faux")).await);
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
