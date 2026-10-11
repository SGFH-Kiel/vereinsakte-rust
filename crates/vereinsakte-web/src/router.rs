//! Zusammenbau des Routers.

use axum::Router;
use axum::extract::Request;
use axum::http::{HeaderName, HeaderValue, header};
use axum::middleware;
use axum::routing::{get, post};
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::trace::TraceLayer;
use tower_sessions::cookie::SameSite;
use tower_sessions::{Expiry, SessionManagerLayer, SessionStore};

use crate::error::not_found;
use crate::session::csrf_guard;
use crate::state::AppState;
use crate::{login, pages};

const CSP: &str = "default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; \
                   form-action 'self'; frame-ancestors 'none'; base-uri 'self'";

/// Baut den Router. Der Sitzungsspeicher wird von außen übergeben, damit die
/// Web-Schicht die Speicherschicht nicht kennt.
pub fn router<S: SessionStore + Clone>(state: AppState, store: S) -> Router {
    let settings = state.settings.clone();
    let idle = time::Duration::try_from(settings.session_idle).unwrap_or(time::Duration::hours(8));
    let sessions = SessionManagerLayer::new(store)
        .with_secure(!settings.development)
        .with_http_only(true)
        .with_same_site(SameSite::Lax)
        .with_expiry(Expiry::OnInactivity(idle));

    let mut app = Router::new()
        .route("/", get(pages::home))
        .route("/login", get(login::login))
        .route("/auth/callback", get(login::callback))
        .route("/logout", post(login::logout))
        .route("/meine-daten", get(pages::my_data))
        .route("/mitglieder", get(pages::directory));
    if settings.development {
        // Nur im Entwicklungsmodus registriert – sonst existieren die Routen nicht.
        app = app
            .route("/dev/login", get(login::dev_login))
            .route("/dev/switch", post(login::dev_switch));
    }
    app
        // route_layer: gilt nur für vorhandene Routen, unbekannte bleiben 404.
        .route_layer(middleware::from_fn(csrf_guard))
        .fallback(not_found)
        .layer(sessions)
        .route("/health", get(pages::health))
        .route("/static/htmx.min.js", get(pages::htmx))
        .route("/static/app.css", get(pages::css))
        .layer(SetResponseHeaderLayer::if_not_present(header::CONTENT_SECURITY_POLICY, HeaderValue::from_static(CSP)))
        .layer(SetResponseHeaderLayer::if_not_present(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY")))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("referrer-policy"),
            HeaderValue::from_static("same-origin"),
        ))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &Request| {
                    // Nur Methode und Pfad – keine Query (könnte Codes enthalten), keine Cookies.
                    tracing::info_span!("http", method = %request.method(), path = %request.uri().path())
                })
                .on_response(|response: &axum::response::Response, latency: std::time::Duration, _span: &tracing::Span| {
                    tracing::info!(status = response.status().as_u16(), duration_ms = latency.as_millis() as u64, "Anfrage beantwortet");
                }),
        )
        .with_state(state)
}
