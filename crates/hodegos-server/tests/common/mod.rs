#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, Response, header};
use futures_util::{SinkExt, StreamExt};
use hodegos_core::Project;
use hodegos_llm::{FakeLlmClient, LlmConfig};
use hodegos_mentors::MentorSources;
use hodegos_server::{Llm, Server, ServerOptions};
use serde_json::Value;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::{HeaderValue, StatusCode};
use tokio_tungstenite::tungstenite::{Error, Message};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tower::ServiceExt;

pub type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub fn project_in(dir: &Path) -> Project {
    Project::open(dir).unwrap()
}

/// Mentors livrés et mentors du projet seulement : jamais ceux de l'utilisateur.
pub fn sources_in(dir: &Path) -> MentorSources {
    MentorSources {
        user_dir: None,
        project_dir: Some(dir.join(".hodegos").join("mentors")),
    }
}

/// Sans fournisseur : chaque passe s'arrête sur `provider_error`.
pub fn no_llm() -> Llm {
    Llm {
        client: Err("aucun fournisseur dans les tests".to_owned()),
        config: LlmConfig::default(),
    }
}

/// Faux fournisseur, modèles rapide et fort renseignés.
pub fn fake_llm(fake: Arc<FakeLlmClient>) -> Llm {
    Llm {
        client: Ok(fake),
        config: LlmConfig {
            fast: Some("test/rapide".to_owned()),
            strong: Some("test/fort".to_owned()),
            ..LlmConfig::default()
        },
    }
}

/// Données de l'utilisateur rangées sous `.git/` du projet de test : invisibles dans
/// l'arborescence, effacées avec le dossier temporaire, jamais dans ~/.local/share.
pub fn data_dir(dir: &Path) -> std::path::PathBuf {
    dir.join(".git").join("hodegos-donnees")
}

pub fn options(dir: &Path, port: u16, llm: Llm) -> ServerOptions {
    ServerOptions {
        port,
        mentors: sources_in(dir),
        data_dir: data_dir(dir),
        llm,
        initial_file: None,
    }
}

/// Serveur sur un port libre, pour un projet vide.
pub async fn bind() -> (tempfile::TempDir, Server) {
    let dir = tempfile::tempdir().unwrap();
    let server = Server::bind(project_in(dir.path()), options(dir.path(), 0, no_llm()))
        .await
        .unwrap();
    (dir, server)
}

pub fn token(server: &Server) -> String {
    server
        .access_url()
        .split_once("?token=")
        .expect("URL d'accès sans jeton")
        .1
        .to_owned()
}

pub fn host(server: &Server) -> String {
    format!("127.0.0.1:{}", server.port())
}

pub async fn get(router: Router, host: &str, uri: &str, cookie: Option<&str>) -> Response<Body> {
    let mut request = Request::builder().uri(uri).header(header::HOST, host);
    if let Some(cookie) = cookie {
        request = request.header(header::COOKIE, cookie);
    }
    router
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap()
}

/// Échange le jeton et renvoie le couple `nom=valeur` du cookie de session.
pub async fn session_cookie(server: &Server) -> String {
    let uri = format!("/?token={}", token(server));
    let response = get(server.router(), &host(server), &uri, None).await;
    let set_cookie = response.headers()[header::SET_COOKIE].to_str().unwrap();
    set_cookie.split(';').next().unwrap().to_owned()
}

/// Démarre un vrai serveur sur `dir` et renvoie son port et le cookie de session.
pub async fn start_in(dir: &Path) -> (u16, String) {
    start_with(dir, no_llm()).await
}

pub async fn start_with(dir: &Path, llm: Llm) -> (u16, String) {
    let server = Server::bind(project_in(dir), options(dir, 0, llm))
        .await
        .unwrap();
    let port = server.port();
    let cookie = session_cookie(&server).await;
    tokio::spawn(server.serve(std::future::pending()));
    (port, cookie)
}

pub async fn connect(
    port: u16,
    origin: Option<&str>,
    cookie: Option<&str>,
) -> Result<Socket, Error> {
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

/// Connexion acceptée : bonne Origin, bon cookie ; le `status` initial est consommé.
pub async fn open_socket(dir: &Path) -> Socket {
    open_socket_with(dir, no_llm()).await
}

pub async fn open_socket_with(dir: &Path, llm: Llm) -> Socket {
    let (port, cookie) = start_with(dir, llm).await;
    let origin = format!("http://127.0.0.1:{port}");
    let mut socket = connect(port, Some(&origin), Some(&cookie)).await.unwrap();
    let status = next(&mut socket).await;
    assert_eq!(status["type"], "status", "{status}");
    socket
}

pub fn refused_with(result: Result<Socket, Error>) -> StatusCode {
    match result {
        Err(Error::Http(response)) => response.status(),
        Err(e) => panic!("erreur inattendue : {e}"),
        Ok(_) => panic!("la connexion aurait dû être refusée"),
    }
}

pub async fn send_raw(socket: &mut Socket, text: &str) {
    socket.send(Message::text(text.to_owned())).await.unwrap();
}

/// Prochain message JSON, ou `None` si rien n'arrive pendant `wait`.
pub async fn next_within(socket: &mut Socket, wait: Duration) -> Option<Value> {
    loop {
        let message = tokio::time::timeout(wait, socket.next()).await.ok()?;
        match message.unwrap().unwrap() {
            Message::Text(text) => return Some(serde_json::from_str(text.as_str()).unwrap()),
            Message::Ping(_) | Message::Pong(_) => continue,
            other => panic!("message inattendu : {other:?}"),
        }
    }
}

pub async fn next(socket: &mut Socket) -> Value {
    next_within(socket, Duration::from_secs(5))
        .await
        .expect("aucun message reçu")
}

pub async fn exchange(socket: &mut Socket, message: Value) -> Value {
    send_raw(socket, &message.to_string()).await;
    next(socket).await
}
