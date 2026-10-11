//! Fehler der Web-Schicht und Fehlerseiten.

use askama::Template;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use vereinsakte_app::AppError;

#[derive(Template)]
#[template(path = "error.html")]
struct ErrorPage<'a> {
    status: u16,
    title: &'a str,
    message: &'a str,
}

/// Deutsche Fehlerseite mit Status.
pub fn error_page(status: StatusCode, message: &str) -> Response {
    let title = match status {
        StatusCode::NOT_FOUND => "Seite nicht gefunden",
        StatusCode::FORBIDDEN => "Kein Zugriff",
        StatusCode::BAD_REQUEST => "Ungültige Anfrage",
        _ if status.is_server_error() => "Interner Fehler",
        _ => "Fehler",
    };
    let page = ErrorPage {
        status: status.as_u16(),
        title,
        message,
    };
    match page.render() {
        Ok(html) => (status, Html(html)).into_response(),
        Err(_) => (status, title.to_string()).into_response(),
    }
}

/// Fehler eines Handlers.
#[derive(Debug, thiserror::Error)]
pub enum WebError {
    #[error(transparent)]
    App(#[from] AppError),
    #[error("Darstellung fehlgeschlagen: {0}")]
    Template(#[from] askama::Error),
    #[error("Sitzung: {0}")]
    Session(#[from] tower_sessions::session::Error),
    #[error("{1}")]
    Status(StatusCode, String),
}

impl IntoResponse for WebError {
    fn into_response(self) -> Response {
        match self {
            Self::App(AppError::NotFound(what)) => {
                error_page(StatusCode::NOT_FOUND, &format!("{what} nicht gefunden"))
            }
            Self::App(AppError::Forbidden) => {
                error_page(StatusCode::FORBIDDEN, "Dafür fehlt dir die Berechtigung.")
            }
            Self::Status(status, message) => error_page(status, &message),
            other => {
                tracing::error!(error = %other, "Fehler bei der Bearbeitung");
                error_page(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Es ist ein Fehler aufgetreten. Bitte versuche es später erneut.",
                )
            }
        }
    }
}

pub async fn not_found() -> Response {
    error_page(StatusCode::NOT_FOUND, "Diese Seite gibt es nicht.")
}
