#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]

use axum::Router;
use axum::body::Body;
use axum::http::{Request, Response, header};
use hodegos_server::Server;
use tower::ServiceExt;

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
