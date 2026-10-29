//! Contrôles appliqués à toutes les requêtes, dans l'ordre : redirection de
//! `localhost`, vérification de `Host`, échange du jeton, cookie de session.

use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::header::{self, HeaderMap, HeaderValue};
use axum::http::{StatusCode, Uri};
use axum::middleware::Next;
use axum::response::{Html, IntoResponse, Response};

use crate::AppState;

const UNAUTHORIZED_PAGE: &str = "<!doctype html><html lang=\"fr\"><meta charset=\"utf-8\">\
<title>Hodegos</title><p>Accès refusé. Ouvrez l'URL affichée dans le terminal \
au lancement de Hodegos.</p></html>";

pub(crate) async fn guard(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Response {
    let mut response = check(&state, request, next).await;
    add_security_headers(&state, response.headers_mut());
    response
}

async fn check(state: &AppState, request: Request, next: Next) -> Response {
    let host = request
        .headers()
        .get(header::HOST)
        .and_then(|h| h.to_str().ok())
        .unwrap_or_default()
        .to_ascii_lowercase();

    if host == format!("localhost:{}", state.port) {
        let target = format!(
            "{}{}",
            state.canonical_origin(),
            path_and_query(request.uri())
        );
        return redirect(StatusCode::TEMPORARY_REDIRECT, &target);
    }
    if host != state.canonical_host() {
        tracing::warn!(host, "requête refusée : en-tête Host étranger");
        return (StatusCode::MISDIRECTED_REQUEST, "Hôte refusé.").into_response();
    }

    if let Some(token) = query_param(request.uri(), "token") {
        let Some(session) = state.auth.exchange(token) else {
            tracing::warn!("requête refusée : jeton invalide");
            return unauthorized();
        };
        let mut response = redirect(StatusCode::SEE_OTHER, request.uri().path());
        let cookie = format!(
            "{}={session}; HttpOnly; SameSite=Strict; Path=/",
            state.cookie_name()
        );
        match HeaderValue::from_str(&cookie) {
            Ok(value) => {
                response.headers_mut().insert(header::SET_COOKIE, value);
            }
            Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
        return response;
    }

    let authenticated = session_cookie(request.headers(), &state.cookie_name())
        .and_then(|session| state.auth.authenticate(session))
        .is_some();
    if !authenticated {
        return unauthorized();
    }
    next.run(request).await
}

fn path_and_query(uri: &Uri) -> &str {
    uri.path_and_query().map_or("/", |pq| pq.as_str())
}

fn query_param<'a>(uri: &'a Uri, name: &str) -> Option<&'a str> {
    uri.query()?
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find_map(|(key, value)| (key == name).then_some(value))
}

fn session_cookie<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find_map(|(key, value)| (key == name).then_some(value))
}

fn redirect(status: StatusCode, location: &str) -> Response {
    match HeaderValue::from_str(location) {
        Ok(value) => (status, [(header::LOCATION, value)]).into_response(),
        Err(_) => StatusCode::BAD_REQUEST.into_response(),
    }
}

fn unauthorized() -> Response {
    (StatusCode::UNAUTHORIZED, Html(UNAUTHORIZED_PAGE)).into_response()
}

fn add_security_headers(state: &AppState, headers: &mut HeaderMap) {
    let csp = format!(
        "default-src 'none'; script-src 'self'; style-src 'self'; font-src 'self'; \
         img-src 'self'; connect-src ws://{}; base-uri 'none'; form-action 'none'; \
         frame-ancestors 'none'",
        state.canonical_host()
    );
    if let Ok(value) = HeaderValue::from_str(&csp) {
        headers.insert(header::CONTENT_SECURITY_POLICY, value);
    }
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
}
