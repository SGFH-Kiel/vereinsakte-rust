//! Sitzungsinhalte, angemeldetes Mitglied und Schutz vor Cross-Site-Request-Forgery.

use axum::body::{Body, to_bytes};
use axum::extract::{FromRequestParts, Request};
use axum::http::request::Parts;
use axum::http::{Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Redirect, Response};
use serde::{Deserialize, Serialize};
use tower_sessions::Session;
use vereinsakte_app::CurrentUser;
use vereinsakte_core::ids::MemberId;

use crate::error::error_page;
use crate::state::AppState;

/// Mitgliedsnummer des angemeldeten Mitglieds.
pub const MEMBER: &str = "member";
/// CSRF-Token der Sitzung.
pub const CSRF: &str = "csrf";
/// Einmalige Rückmeldung für die nächste Seite.
pub const FLASH: &str = "flash";
/// Laufende Anmeldung beim Anbieter.
pub const LOGIN: &str = "login";
/// Name des Formularfelds bzw. Headers für das CSRF-Token.
pub const CSRF_FIELD: &str = "csrf";
pub const CSRF_HEADER: &str = "x-csrf-token";

/// Zwischen `/login` und `/auth/callback` in der Sitzung gemerkte Werte.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PendingLogin {
    pub state: String,
    pub nonce: String,
    pub pkce_verifier: String,
    pub next: String,
}

/// Liefert das CSRF-Token der Sitzung und legt es bei Bedarf an.
pub async fn csrf_token(session: &Session) -> String {
    if let Ok(Some(token)) = session.get::<String>(CSRF).await {
        return token;
    }
    let token = format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    );
    // Ein Fehler des Sitzungsspeichers fällt spätestens bei der CSRF-Prüfung auf.
    let _ = session.insert(CSRF, &token).await;
    token
}

/// Meldet ein Mitglied in der Sitzung an: neue Sitzungs-ID (Schutz vor
/// Session-Fixation), neues CSRF-Token.
pub async fn sign_in(
    session: &Session,
    member: MemberId,
) -> Result<(), tower_sessions::session::Error> {
    session.cycle_id().await?;
    session.remove::<String>(CSRF).await?;
    session.insert(MEMBER, member.get()).await?;
    Ok(())
}

/// Nur Weiterleitungen auf eigene Seiten zulassen.
pub fn safe_next(next: Option<&str>) -> String {
    match next {
        Some(path)
            if path.starts_with('/') && !path.starts_with("//") && !path.starts_with("/\\") =>
        {
            path.into()
        }
        _ => "/meine-daten".into(),
    }
}

pub fn encode(value: &str) -> String {
    url::form_urlencoded::byte_serialize(value.as_bytes()).collect()
}

/// Extraktor für geschützte Seiten: ohne Anmeldung Weiterleitung zu `/login?next=…`.
/// Ein nicht (mehr) aktives Mitglied verliert die Sitzung und muss sich neu anmelden.
pub struct RequireUser(pub CurrentUser);

impl FromRequestParts<AppState> for RequireUser {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let session = Session::from_request_parts(parts, state)
            .await
            .map_err(IntoResponse::into_response)?;
        let member = session
            .get::<u32>(MEMBER)
            .await
            .ok()
            .flatten()
            .and_then(MemberId::new);
        let login = |parts: &Parts| {
            let next = parts.uri.path_and_query().map_or("/", |pq| pq.as_str());
            Redirect::to(&format!("/login?next={}", encode(next))).into_response()
        };
        let Some(member) = member else {
            return Err(login(parts));
        };
        match state.club.is_active(member).await {
            Ok(true) => Ok(Self(CurrentUser { member })),
            Ok(false) => {
                let _ = session.flush().await;
                Err(login(parts))
            }
            Err(error) => Err(crate::error::WebError::from(error).into_response()),
        }
    }
}

/// Art einer einmaligen Rückmeldung nach einer Aktion.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlashKind {
    Ok,
    Warn,
    Error,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Flash {
    pub kind: FlashKind,
    pub text: String,
}

/// Merkt eine Meldung für die nächste Seite.
pub async fn flash(session: &Session, kind: FlashKind, text: impl Into<String>) {
    let _ = session
        .insert(
            FLASH,
            Flash {
                kind,
                text: text.into(),
            },
        )
        .await;
}

/// Holt die gemerkte Meldung und löscht sie.
pub async fn take_flash(session: &Session) -> Option<Flash> {
    session.remove::<Flash>(FLASH).await.ok().flatten()
}

/// Lehnt zustandsändernde Anfragen ohne gültiges CSRF-Token ab. Das Token kommt
/// aus dem Header `X-CSRF-Token` (htmx) oder dem Formularfeld `csrf`.
pub async fn csrf_guard(session: Session, request: Request, next: Next) -> Response {
    if matches!(*request.method(), Method::GET | Method::HEAD) {
        return next.run(request).await;
    }
    let expected = session.get::<String>(CSRF).await.ok().flatten();
    let header = request
        .headers()
        .get(CSRF_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let (parts, body) = request.into_parts();
    let Ok(bytes) = to_bytes(body, 64 * 1024).await else {
        return error_page(StatusCode::PAYLOAD_TOO_LARGE, "Anfrage zu groß");
    };
    let field = url::form_urlencoded::parse(&bytes)
        .find(|(key, _)| key == CSRF_FIELD)
        .map(|(_, value)| value.into_owned());
    let valid = matches!((&expected, header.or(field)), (Some(expected), Some(given)) if *expected == given);
    if !valid {
        return error_page(
            StatusCode::FORBIDDEN,
            "Die Anfrage ist abgelaufen oder ungültig. Bitte lade die Seite neu.",
        );
    }
    next.run(Request::from_parts(parts, Body::from(bytes)))
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_local_redirects() {
        assert_eq!(
            safe_next(Some("/mitglieder?q=peter")),
            "/mitglieder?q=peter"
        );
        assert_eq!(safe_next(Some("https://evil.example")), "/meine-daten");
        assert_eq!(safe_next(Some("//evil.example")), "/meine-daten");
        assert_eq!(safe_next(None), "/meine-daten");
    }
}
