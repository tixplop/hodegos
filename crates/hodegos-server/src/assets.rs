use axum::http::{StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};

/// Le front, intégré au binaire.
#[derive(rust_embed::Embed)]
#[folder = "../../ui"]
#[exclude = "vendor/node_modules/*"]
struct Ui;

pub(crate) async fn serve(uri: Uri) -> Response {
    let path = match uri.path().trim_start_matches('/') {
        "" => "index.html",
        path => path,
    };
    match Ui::get(path) {
        Some(file) => (
            [(header::CONTENT_TYPE, file.metadata.mimetype().to_owned())],
            file.data,
        )
            .into_response(),
        None => (StatusCode::NOT_FOUND, "Introuvable.").into_response(),
    }
}
