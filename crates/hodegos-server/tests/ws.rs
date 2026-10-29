#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::session_cookie;
use futures_util::{SinkExt, StreamExt};
use hodegos_server::Server;
use serde_json::{Value, json};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::{HeaderValue, StatusCode};
use tokio_tungstenite::tungstenite::{Error, Message};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// Démarre un vrai serveur et renvoie son port et le cookie de session.
async fn start() -> (u16, String) {
    let server = Server::bind(0).await.unwrap();
    let port = server.port();
    let cookie = session_cookie(&server).await;
    tokio::spawn(server.serve(std::future::pending()));
    (port, cookie)
}

async fn connect(port: u16, origin: Option<&str>, cookie: Option<&str>) -> Result<Socket, Error> {
    let mut request = format!("ws://127.0.0.1:{port}/ws")
        .into_client_request()
        .unwrap();
    if let Some(origin) = origin {
        request
            .headers_mut()
            .insert("Origin", HeaderValue::from_str(origin).unwrap());
    }
    if let Some(cookie) = cookie {
        request
            .headers_mut()
            .insert("Cookie", HeaderValue::from_str(cookie).unwrap());
    }
    tokio_tungstenite::connect_async(request)
        .await
        .map(|(socket, _)| socket)
}

fn refused_with(result: Result<Socket, Error>) -> StatusCode {
    match result {
        Err(Error::Http(response)) => response.status(),
        Err(e) => panic!("erreur inattendue : {e}"),
        Ok(_) => panic!("la connexion aurait dû être refusée"),
    }
}

async fn exchange(socket: &mut Socket, message: Value) -> Value {
    socket
        .send(Message::text(message.to_string()))
        .await
        .unwrap();
    loop {
        match socket.next().await.unwrap().unwrap() {
            Message::Text(text) => return serde_json::from_str(text.as_str()).unwrap(),
            Message::Ping(_) | Message::Pong(_) => continue,
            other => panic!("message inattendu : {other:?}"),
        }
    }
}

#[tokio::test]
async fn ping_renvoie_pong() {
    let (port, cookie) = start().await;
    let origin = format!("http://127.0.0.1:{port}");
    let mut socket = connect(port, Some(&origin), Some(&cookie)).await.unwrap();
    let reply = exchange(&mut socket, json!({ "v": 1, "type": "ping", "id": "c-1" })).await;
    assert_eq!(reply, json!({ "v": 1, "type": "pong", "re": "c-1" }));
}

#[tokio::test]
async fn un_message_invalide_donne_bad_message_sans_couper_la_connexion() {
    let (port, cookie) = start().await;
    let origin = format!("http://127.0.0.1:{port}");
    let mut socket = connect(port, Some(&origin), Some(&cookie)).await.unwrap();

    socket.send(Message::text("pas du json")).await.unwrap();
    let Message::Text(text) = socket.next().await.unwrap().unwrap() else {
        panic!("réponse texte attendue");
    };
    let reply: Value = serde_json::from_str(text.as_str()).unwrap();
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
    let (port, cookie) = start().await;
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
    let (port, _) = start().await;
    let origin = format!("http://127.0.0.1:{port}");
    let status = refused_with(connect(port, Some(&origin), None).await);
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let status = refused_with(connect(port, Some(&origin), Some("hodegos_1=faux")).await);
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
