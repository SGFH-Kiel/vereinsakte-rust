//! Zusammenbau des Routers.

use axum::Router;
use axum::extract::{DefaultBodyLimit, Request};
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
use crate::{audit, boats, finance, login, members, offices, pages, reservations, tasks, work};

/// Obergrenze für einen Upload (Datei bis 10 MB plus Formularfelder).
const UPLOAD_BODY_LIMIT: usize = 11 * 1024 * 1024;

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
        .route("/meine-daten/sperrvermerk", post(pages::set_contact_block))
        .route("/hinweis/kontakt", post(pages::dismiss_contact_notice))
        .route("/mitglieder", get(pages::directory))
        .route(
            "/mitglieder/neu",
            get(members::new).post(members::new_submit),
        )
        .route("/mitglieder/{nr}", get(members::show))
        .route(
            "/mitglieder/{nr}/bearbeiten",
            get(members::edit).post(members::edit_submit),
        )
        .route("/mitglieder/{nr}/austritt", post(members::end_membership))
        .route("/mitglieder/{nr}/scheine", post(members::add_license))
        .route(
            "/mitglieder/{nr}/scheine/{kind}/{idx}/entfernen",
            post(members::remove_license),
        )
        .route("/meine-daten/bearbeiten", get(members::edit_own))
        .route("/boote", get(boats::list))
        .route("/boote/neu", get(boats::new).post(boats::new_submit))
        .route("/boote/{id}", get(boats::show))
        .route(
            "/boote/{id}/bearbeiten",
            get(boats::edit).post(boats::edit_submit),
        )
        .route("/boote/{id}/sperren", post(boats::block))
        .route("/boote/{id}/entsperren", post(boats::lift))
        .route("/boote/{id}/freigabe", post(boats::release))
        .route("/boote/{id}/abnahme", post(boats::checkout))
        .route("/boote/{id}/widerruf", post(boats::revoke))
        .route("/protokoll", get(audit::show))
        .route("/protokoll/archiv", get(crate::archive::show))
        .route("/konto", get(finance::own_account))
        .route("/finanzen", get(finance::outstanding))
        .route("/finanzen/konten/{nr}", get(finance::account))
        .route("/finanzen/konten/{nr}/einzahlung", post(finance::payment))
        .route("/finanzen/konten/{nr}/forderung", post(finance::claim))
        .route(
            "/finanzen/konten/{nr}/forderungen/{id}/storno",
            post(finance::cancel_claim),
        )
        .route(
            "/finanzen/saisons",
            get(finance::seasons).post(finance::save_season),
        )
        .route(
            "/finanzen/beitraege",
            get(finance::fees).post(finance::create_fees),
        )
        .route("/finanzen/touren", get(finance::tours))
        .route("/finanzen/touren/{id}/abrechnen", post(finance::bill_tour))
        .route("/finanzen/nachweis/{nr}", get(finance::proof))
        .route("/kalender", get(crate::calendar::show))
        .route("/reservierungen", get(reservations::list))
        .route(
            "/reservierungen/neu",
            get(reservations::new).post(reservations::new_submit),
        )
        .route("/reservierungen/{id}", get(reservations::show))
        .route(
            "/reservierungen/{id}/bearbeiten",
            get(reservations::edit).post(reservations::edit_submit),
        )
        .route(
            "/reservierungen/{id}/stornieren",
            post(reservations::cancel),
        )
        .route("/arbeit", get(work::hours))
        .route("/arbeit/einsaetze", post(work::add_entry))
        .route(
            "/arbeit/einsaetze/{id}/bearbeiten",
            get(work::edit_entry).post(work::edit_entry_submit),
        )
        .route("/arbeit/einsaetze/{id}/loeschen", post(work::delete_entry))
        .route("/arbeit/einsaetze/{id}/bestaetigen", post(work::confirm))
        .route("/arbeit/bestaetigungen", get(work::confirmations))
        .route(
            "/arbeit/termine",
            get(work::events).post(work::create_event),
        )
        .route("/arbeit/termine/neu", get(work::new_event))
        .route("/arbeit/termine/{id}", get(work::event_show))
        .route(
            "/arbeit/termine/{id}/bearbeiten",
            get(work::edit_event).post(work::edit_event_submit),
        )
        .route("/arbeit/termine/{id}/anmelden", post(work::register))
        .route("/arbeit/termine/{id}/abmelden", post(work::unregister))
        .route("/arbeit/aufgaben", get(tasks::old_list))
        .route("/aufgaben", get(tasks::board))
        .route("/aufgaben/neu", get(tasks::new).post(tasks::new_submit))
        .route("/aufgaben/{id}", get(tasks::show))
        .route("/aufgaben/{id}/status", post(tasks::move_status))
        .route("/aufgaben/{id}/bearbeiten", post(tasks::edit))
        .route("/aufgaben/{id}/zuweisen", post(tasks::assign))
        .route("/aufgaben/{id}/beschreibung", post(tasks::description))
        .route("/aufgaben/{id}/kommentar", post(tasks::comment))
        .route(
            "/aufgaben/kommentar/{cid}/aendern",
            post(tasks::comment_edit),
        )
        .route(
            "/aufgaben/kommentar/{cid}/loeschen",
            post(tasks::comment_delete),
        )
        .route(
            "/aufgaben/{id}/anhang",
            post(tasks::attach).layer(DefaultBodyLimit::max(UPLOAD_BODY_LIMIT)),
        )
        .route("/aufgaben/anhang/{aid}", get(tasks::attachment))
        .route(
            "/aufgaben/anhang/{aid}/loeschen",
            post(tasks::attachment_delete),
        )
        .route("/arbeit/uebersicht", get(work::overview))
        .route("/arbeit/uebertrag", post(work::transfer))
        .route("/arbeit/abgeltung", post(work::compensate))
        .route("/aemter", get(offices::overview).post(offices::appoint))
        .route("/aemter/historie", get(crate::archive::offices_history))
        .route("/aemter/{id}/abgeben", post(offices::resign));
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
        .route("/static/{*path}", get(crate::assets::asset))
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
