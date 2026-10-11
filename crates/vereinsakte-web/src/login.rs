//! Anmeldung, Rückruf des Anbieters, Abmeldung sowie Entwicklungs-Login und
//! Benutzerwechsel.

use askama::Template;
use axum::Form;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use serde::Deserialize;
use tower_sessions::Session;
use vereinsakte_app::auth::LoginClaims;
use vereinsakte_app::service::LoginChoiceGroup;
use vereinsakte_core::ids::MemberId;

use crate::error::{WebError, error_page};
use crate::session::{LOGIN, PendingLogin, csrf_token, encode, safe_next, sign_in};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct NextQuery {
    next: Option<String>,
}

/// Startet die Anmeldung: im Entwicklungsmodus Auswahlliste, sonst Microsoft.
pub async fn login(
    State(state): State<AppState>,
    session: Session,
    Query(query): Query<NextQuery>,
) -> Result<Response, WebError> {
    let next = safe_next(query.next.as_deref());
    if state.settings.development {
        return Ok(Redirect::to(&format!("/dev/login?next={}", encode(&next))).into_response());
    }
    let provider = state.login.as_ref().ok_or_else(|| {
        WebError::Status(
            StatusCode::SERVICE_UNAVAILABLE,
            "Anmeldung ist nicht eingerichtet.".into(),
        )
    })?;
    let request = provider.authorize();
    let pending = PendingLogin {
        state: request.state,
        nonce: request.nonce,
        pkce_verifier: request.pkce_verifier,
        next,
    };
    session.insert(LOGIN, &pending).await?;
    Ok(Redirect::to(request.url.as_str()).into_response())
}

#[derive(Deserialize)]
pub struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

fn rejected(message: &str) -> Response {
    error_page(StatusCode::FORBIDDEN, message)
}

/// Rückruf von Microsoft Entra ID.
pub async fn callback(
    State(state): State<AppState>,
    session: Session,
    Query(query): Query<CallbackQuery>,
) -> Result<Response, WebError> {
    let pending: Option<PendingLogin> = session.remove(LOGIN).await?;
    let (Some(pending), Some(code), Some(returned_state)) = (pending, query.code, query.state)
    else {
        let reason = query
            .error
            .unwrap_or_else(|| "unvollständige Antwort".into());
        tracing::info!(%reason, "Anmeldung abgebrochen");
        return Ok(rejected(
            "Die Anmeldung wurde abgebrochen oder ist abgelaufen. Bitte versuche es erneut.",
        ));
    };
    if returned_state != pending.state {
        tracing::warn!("Anmeldung mit ungültigem state abgelehnt");
        return Ok(rejected(
            "Die Anmeldung ist ungültig. Bitte versuche es erneut.",
        ));
    }
    let Some(provider) = state.login.as_ref() else {
        return Ok(rejected("Anmeldung ist nicht eingerichtet."));
    };
    let verified = match provider
        .finish(&code, &pending.pkce_verifier, &pending.nonce)
        .await
    {
        Ok(verified) => verified,
        Err(error) => {
            tracing::warn!(%error, "ID-Token abgelehnt");
            return Ok(rejected("Die Anmeldung konnte nicht bestätigt werden."));
        }
    };
    let expected_tenant = state.settings.tenant_id.as_deref();
    if expected_tenant.is_none() || verified.tenant_id.as_deref() != expected_tenant {
        tracing::warn!(tenant = ?verified.tenant_id, "Konto aus fremdem Mandanten abgelehnt");
        return Ok(rejected(
            "Mit diesem Konto ist keine Anmeldung möglich. Bitte nutze dein Vereinskonto.",
        ));
    }
    let claims = LoginClaims {
        provider: verified.issuer,
        subject: verified.object_id.unwrap_or(verified.subject),
        email: verified.email.or(verified.preferred_username),
    };
    match state.auth.resolve_login(&claims).await {
        Ok(member) => {
            sign_in(&session, member).await?;
            tracing::info!(%member, "angemeldet");
            Ok(Redirect::to(&pending.next).into_response())
        }
        Err(error) => {
            tracing::info!(%error, "Anmeldung abgelehnt");
            Ok(rejected(&error.to_string()))
        }
    }
}

/// Abmelden (POST mit CSRF-Token).
pub async fn logout(session: Session) -> Result<Redirect, WebError> {
    session.flush().await?;
    Ok(Redirect::to("/"))
}

#[derive(Template)]
#[template(path = "dev_login.html")]
struct DevLoginPage {
    csrf: String,
    next: String,
    groups: Vec<LoginChoiceGroup>,
}

/// Entwicklungs-Login: Auswahlliste der aktiven Mitglieder mit ihren Ämtern.
pub async fn dev_login(
    State(state): State<AppState>,
    session: Session,
    Query(query): Query<NextQuery>,
) -> Result<Response, WebError> {
    let page = DevLoginPage {
        csrf: csrf_token(&session).await,
        next: safe_next(query.next.as_deref()),
        groups: state.club.login_choices().await?,
    };
    crate::pages::render(&page)
}

#[derive(Deserialize)]
pub struct SwitchForm {
    member: u32,
    next: Option<String>,
}

/// Entwicklungsmodus: als anderes Mitglied anmelden (auch ohne vorherige Abmeldung).
pub async fn dev_switch(
    State(state): State<AppState>,
    session: Session,
    Form(form): Form<SwitchForm>,
) -> Result<Response, WebError> {
    let member = MemberId::new(form.member).ok_or_else(|| {
        WebError::Status(StatusCode::BAD_REQUEST, "Ungültige Mitgliedsnummer".into())
    })?;
    if !state.club.is_active(member).await? {
        return Err(WebError::Status(
            StatusCode::BAD_REQUEST,
            format!("Mitglied {member} ist nicht aktiv"),
        ));
    }
    sign_in(&session, member).await?;
    Ok(Redirect::to(&safe_next(form.next.as_deref())).into_response())
}
