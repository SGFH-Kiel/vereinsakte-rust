//! Eingebettete Dateien des Auftritts: Vereinslogo und Schrift. Alles wird von
//! der eigenen Adresse ausgeliefert (kein Laden von der Vereinswebseite).

use axum::extract::Path;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};

struct Asset {
    path: &'static str,
    content_type: &'static str,
    body: &'static [u8],
}

const ASSETS: &[Asset] = &[
    Asset {
        path: "img/sgfh-logo.svg",
        content_type: "image/svg+xml",
        body: include_bytes!("../static/img/sgfh-logo.svg"),
    },
    Asset {
        path: "fonts/Amaranth-Regular.woff2",
        content_type: "font/woff2",
        body: include_bytes!("../static/fonts/Amaranth-Regular.woff2"),
    },
    Asset {
        path: "fonts/Amaranth-Bold.woff2",
        content_type: "font/woff2",
        body: include_bytes!("../static/fonts/Amaranth-Bold.woff2"),
    },
    Asset {
        path: "fonts/OFL.txt",
        content_type: "text/plain; charset=utf-8",
        body: include_bytes!("../static/fonts/OFL.txt"),
    },
];

/// Liefert eine eingebettete Datei unter `/static/…` aus.
pub async fn asset(Path(path): Path<String>) -> Response {
    match ASSETS.iter().find(|a| a.path == path) {
        Some(a) => (
            [
                (header::CONTENT_TYPE, a.content_type),
                (header::CACHE_CONTROL, "public, max-age=604800"),
            ],
            a.body,
        )
            .into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}
