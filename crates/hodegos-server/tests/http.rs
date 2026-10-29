#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use axum::http::{StatusCode, header};
use common::{bind, get, host, project_in, session_cookie, sources_in, token};
use hodegos_server::{Server, ServerError};
use http_body_util::BodyExt;

#[tokio::test]
async fn l_url_d_acces_vise_127_0_0_1_avec_un_jeton_de_256_bits() {
    let (_dir, server) = bind().await;
    let prefix = format!("http://127.0.0.1:{}/?token=", server.port());
    assert!(server.access_url().starts_with(&prefix));
    let token = token(&server);
    assert_eq!(token.len(), 64);
    assert!(token.bytes().all(|b| b.is_ascii_hexdigit()));
}

#[tokio::test]
async fn ecoute_sur_le_port_demande_et_refuse_un_port_occupe() {
    let (dir, server) = bind().await;
    let port = server.port();
    assert!(
        server
            .access_url()
            .starts_with(&format!("http://127.0.0.1:{port}/"))
    );

    let err = Server::bind(project_in(dir.path()), sources_in(dir.path()), port)
        .await
        .err()
        .expect("le port est déjà pris");
    assert!(
        matches!(err, ServerError::PortInUse(p) if p == port),
        "{err:?}"
    );
    assert!(err.to_string().contains("--port"), "{err}");

    drop(server);
    assert_eq!(
        Server::bind(project_in(dir.path()), sources_in(dir.path()), port)
            .await
            .unwrap()
            .port(),
        port
    );
}

#[tokio::test]
async fn refuse_sans_jeton_ni_cookie() {
    let (_dir, server) = bind().await;
    for uri in ["/", "/index.html", "/styles/tokens.css", "/ws"] {
        let response = get(server.router(), &host(&server), uri, None).await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "{uri}");
    }
}

#[tokio::test]
async fn refuse_un_mauvais_jeton_ou_un_mauvais_cookie() {
    let (_dir, server) = bind().await;
    let response = get(server.router(), &host(&server), "/?token=faux", None).await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let forged = format!("hodegos_{}={}", server.port(), token(&server));
    let response = get(server.router(), &host(&server), "/", Some(&forged)).await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn le_jeton_s_echange_contre_un_cookie_et_disparait_de_l_url() {
    let (_dir, server) = bind().await;
    let uri = format!("/?token={}", token(&server));
    let response = get(server.router(), &host(&server), &uri, None).await;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(response.headers()[header::LOCATION], "/");

    let set_cookie = response.headers()[header::SET_COOKIE].to_str().unwrap();
    assert!(set_cookie.starts_with(&format!("hodegos_{}=", server.port())));
    assert!(!set_cookie.contains(&token(&server)));
    for attribute in ["HttpOnly", "SameSite=Strict", "Path=/"] {
        assert!(set_cookie.contains(attribute), "{set_cookie}");
    }

    let cookie = session_cookie(&server).await;
    let response = get(server.router(), &host(&server), "/", Some(&cookie)).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        response.headers()[header::CONTENT_TYPE]
            .to_str()
            .unwrap()
            .starts_with("text/html")
    );
    let body = response.into_body().collect().await.unwrap().to_bytes();
    assert!(String::from_utf8_lossy(&body).contains("hodegos"));
}

#[tokio::test]
async fn le_jeton_reste_valable_tant_que_le_serveur_tourne() {
    let (_dir, server) = bind().await;
    assert_eq!(session_cookie(&server).await, session_cookie(&server).await);
}

#[tokio::test]
async fn toutes_les_reponses_portent_une_csp_stricte() {
    let (_dir, server) = bind().await;
    let cookie = session_cookie(&server).await;
    for (uri, cookie) in [("/", Some(cookie.as_str())), ("/", None)] {
        let response = get(server.router(), &host(&server), uri, cookie).await;
        let csp = response.headers()[header::CONTENT_SECURITY_POLICY]
            .to_str()
            .unwrap();
        assert!(csp.contains("default-src 'none'"), "{csp}");
        assert!(csp.contains("script-src 'self'"), "{csp}");
        assert!(
            csp.contains(&format!("connect-src ws://127.0.0.1:{}", server.port())),
            "{csp}"
        );
        assert!(!csp.contains("http"), "{csp}");
        assert_eq!(
            response.headers()[header::X_CONTENT_TYPE_OPTIONS],
            "nosniff"
        );
    }
}

#[tokio::test]
async fn refuse_un_host_etranger() {
    let (_dir, server) = bind().await;
    let cookie = session_cookie(&server).await;
    let other_port = format!("127.0.0.1:{}", server.port().wrapping_add(1));
    let foreign = format!("evil.test:{}", server.port());
    for host in [foreign.as_str(), other_port.as_str(), "127.0.0.1", ""] {
        let response = get(server.router(), host, "/", Some(&cookie)).await;
        assert_eq!(
            response.status(),
            StatusCode::MISDIRECTED_REQUEST,
            "Host {host:?}"
        );
    }
}

#[tokio::test]
async fn localhost_est_redirige_vers_127_0_0_1_avec_chemin_et_requete() {
    let (_dir, server) = bind().await;
    let host = format!("localhost:{}", server.port());
    let response = get(server.router(), &host, "/a/b?token=x", None).await;
    assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
    assert_eq!(
        response.headers()[header::LOCATION],
        format!("http://127.0.0.1:{}/a/b?token=x", server.port()).as_str()
    );
    assert!(response.headers().get(header::SET_COOKIE).is_none());
}

#[tokio::test]
async fn sert_les_fichiers_du_design_system() {
    let (_dir, server) = bind().await;
    let cookie = session_cookie(&server).await;
    for (uri, mime) in [
        ("/styles/tokens.css", "text/css"),
        ("/styles/fonts.css", "text/css"),
        ("/app.css", "text/css"),
        ("/js/hodegos.js", "text/javascript"),
        ("/js/app/main.js", "text/javascript"),
        ("/vendor/codemirror.js", "text/javascript"),
        ("/fonts/inter-latin-400-normal.woff2", "font/woff2"),
        ("/fonts/jetbrains-mono-latin-400-normal.woff2", "font/woff2"),
        ("/hodegos-mark.svg", "image/svg+xml"),
    ] {
        let response = get(server.router(), &host(&server), uri, Some(&cookie)).await;
        assert_eq!(response.status(), StatusCode::OK, "{uri}");
        assert_eq!(response.headers()[header::CONTENT_TYPE], mime, "{uri}");
    }
    let response = get(server.router(), &host(&server), "/absent.js", Some(&cookie)).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
