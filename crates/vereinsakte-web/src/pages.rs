//! Layout und Seiten: „Meine Daten“, Mitgliederverzeichnis, Health-Check und
//! statische Dateien.

use askama::Template;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;
use tower_sessions::Session;
use vereinsakte_app::directory::{DirectoryFilter, DirectoryView};
use vereinsakte_app::members::{membership_types, parse_membership_type};
use vereinsakte_app::my_data::MyDataView;
use vereinsakte_app::service::LoginChoiceGroup;
use vereinsakte_app::{CurrentUser, Principal};

use crate::error::WebError;
use crate::session::{FlashKind, RequireUser, csrf_token, take_flash};
use crate::state::AppState;

/// Gemeinsame Angaben des Basislayouts.
pub struct Layout {
    pub title: String,
    pub csrf: String,
    pub user: Option<UserBar>,
    pub dev_bar: Option<DevBar>,
    pub flash: Option<FlashView>,
}

pub struct UserBar {
    pub name: String,
    pub offices: Vec<String>,
    pub nav: Vec<NavItem>,
}

/// Eintrag der Hauptnavigation.
pub struct NavItem {
    pub href: &'static str,
    pub label: &'static str,
    pub current: bool,
}

pub struct FlashView {
    pub class: &'static str,
    pub text: String,
}

/// Leiste für den schnellen Benutzerwechsel im Entwicklungsmodus.
pub struct DevBar {
    pub current: u32,
    pub current_name: String,
    pub next: String,
    pub groups: Vec<LoginChoiceGroup>,
}

/// Hauptnavigation nach Berechtigung; `path` bestimmt den aktuellen Bereich.
fn navigation(principal: &Principal, path: &str) -> Vec<NavItem> {
    let path = path.split('?').next().unwrap_or(path);
    let current = |href: &str| {
        if href == "/" {
            path == "/"
        } else {
            path == href || path.starts_with(&format!("{href}/"))
        }
    };
    let mut items = vec![
        ("/", "Start"),
        ("/meine-daten", "Meine Daten"),
        ("/mitglieder", "Mitglieder"),
        ("/boote", "Boote"),
        ("/reservierungen", "Reservierungen"),
        ("/arbeit", "Arbeit"),
        ("/aemter", "Ämter"),
        ("/konto", "Konto"),
        ("/protokoll", "Protokoll"),
    ];
    if principal.can(&vereinsakte_core::rights::Capability::ReadAllAccounts) {
        items.push(("/finanzen", "Finanzen"));
    }
    items
        .into_iter()
        .map(|(href, label)| NavItem {
            href,
            label,
            current: current(href),
        })
        .collect()
}

impl Layout {
    pub async fn build(
        state: &AppState,
        session: &Session,
        user: Option<&CurrentUser>,
        title: &str,
        next: &str,
    ) -> Result<Self, WebError> {
        let csrf = csrf_token(session).await;
        let (user_bar, dev_bar) = match user {
            Some(user) => {
                let principal = state.club.principal(user).await?;
                let dev_bar = if state.settings.development {
                    Some(DevBar {
                        current: user.member.get(),
                        current_name: principal.name.clone(),
                        next: next.to_string(),
                        groups: state.club.login_choices().await?,
                    })
                } else {
                    None
                };
                let nav = navigation(&principal, next);
                (
                    Some(UserBar {
                        name: principal.name,
                        offices: principal.offices,
                        nav,
                    }),
                    dev_bar,
                )
            }
            None => (None, None),
        };
        let flash = take_flash(session).await.map(|f| FlashView {
            class: match f.kind {
                FlashKind::Ok => "ok",
                FlashKind::Warn => "warn",
                FlashKind::Error => "error",
            },
            text: f.text,
        });
        Ok(Self {
            title: title.into(),
            csrf,
            user: user_bar,
            dev_bar,
            flash,
        })
    }
}

pub fn render(template: &impl Template) -> Result<Response, WebError> {
    Ok(Html(template.render()?).into_response())
}

#[derive(Template)]
#[template(path = "start.html")]
struct StartPage {
    layout: Layout,
    v: MyDataView,
    confirmations: usize,
    events: Vec<vereinsakte_app::work::EventLine>,
    tasks: Vec<vereinsakte_app::work::TaskLine>,
    reservations: Vec<vereinsakte_app::reservations::ReservationRow>,
}

/// Startseite: die nächsten Schritte des angemeldeten Mitglieds.
pub async fn home(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
) -> Result<Response, WebError> {
    let v = state.club.my_data(&user).await?;
    let principal = state.club.principal(&user).await?;
    let confirmations = if principal.has_office() {
        state.club.open_confirmations(&user).await?.len()
    } else {
        0
    };
    let events: Vec<_> = state
        .club
        .events_view(&user)
        .await?
        .events
        .into_iter()
        .filter(|e| !e.past)
        .take(3)
        .collect();
    let tasks: Vec<_> = state
        .club
        .tasks_view(&user)
        .await?
        .tasks
        .into_iter()
        .filter(|t| t.status_key != "done" && t.can_update && !t.assignees.is_empty())
        .filter(|t| t.assignees.contains(&principal.name))
        .collect();
    let today = principal.now.date_naive();
    let reservations = state
        .club
        .reservation_list(
            &user,
            &vereinsakte_app::reservations::ReservationFilter {
                from: Some(today),
                to: Some(today + chrono::Duration::days(60)),
                mine: true,
                ..Default::default()
            },
        )
        .await?
        .rows;
    let layout = Layout::build(&state, &session, Some(&user), "Start", "/").await?;
    render(&StartPage {
        layout,
        v,
        confirmations,
        events,
        tasks,
        reservations,
    })
}

#[derive(Template)]
#[template(path = "my_data.html")]
struct MyDataPage {
    layout: Layout,
    v: MyDataView,
}

pub async fn my_data(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
) -> Result<Response, WebError> {
    // Bewusst ohne Parameter: es gibt nur die Daten des angemeldeten Mitglieds.
    let v = state.club.my_data(&user).await?;
    let layout =
        Layout::build(&state, &session, Some(&user), "Meine Daten", "/meine-daten").await?;
    render(&MyDataPage { layout, v })
}

/// Hinweis zur Sichtbarkeit der Kontaktdaten bestätigen.
pub async fn dismiss_contact_notice(
    State(state): State<AppState>,
    RequireUser(user): RequireUser,
) -> Result<Response, WebError> {
    state.club.dismiss_contact_notice(&user).await?;
    Ok(axum::response::Redirect::to("/").into_response())
}

/// Sperrvermerk für die eigenen Kontaktdaten setzen oder entfernen.
pub async fn set_contact_block(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    axum::Form(fields): axum::Form<std::collections::HashMap<String, String>>,
) -> Result<Response, WebError> {
    let blocked = fields.contains_key("blocked");
    state
        .club
        .set_contact_blocked(&user, user.member, blocked)
        .await?;
    let text = if blocked {
        "Deine Kontaktdaten sind jetzt für andere Mitglieder gesperrt."
    } else {
        "Deine Kontaktdaten sind wieder für alle Mitglieder sichtbar."
    };
    Ok(crate::forms::redirect_with_flash(&session, "/meine-daten", text).await)
}

#[derive(Deserialize)]
pub struct DirectoryQuery {
    #[serde(default)]
    q: String,
    #[serde(default)]
    art: String,
    #[serde(default)]
    schein: String,
}

#[derive(Template)]
#[template(path = "directory.html")]
struct DirectoryPage {
    layout: Layout,
    v: DirectoryView,
    types: Vec<(&'static str, String)>,
}

#[derive(Template)]
#[template(path = "directory_table.html")]
struct DirectoryTable {
    v: DirectoryView,
}

pub async fn directory(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Query(query): Query<DirectoryQuery>,
    headers: HeaderMap,
) -> Result<Response, WebError> {
    let filter = DirectoryFilter {
        query: query.q,
        membership_type: parse_membership_type(&query.art),
        license: query.schein,
    };
    let v = state.club.directory_filtered(&user, &filter).await?;
    if headers.contains_key("hx-request") {
        return render(&DirectoryTable { v });
    }
    let mut params = vec![];
    if !v.query.is_empty() {
        params.push(format!("q={}", crate::session::encode(&v.query)));
    }
    if !v.membership_type.is_empty() {
        params.push(format!("art={}", v.membership_type));
    }
    if !v.license.is_empty() {
        params.push(format!("schein={}", crate::session::encode(&v.license)));
    }
    let next = if params.is_empty() {
        "/mitglieder".to_string()
    } else {
        format!("/mitglieder?{}", params.join("&"))
    };
    let layout = Layout::build(&state, &session, Some(&user), "Mitglieder", &next).await?;
    render(&DirectoryPage {
        layout,
        v,
        types: membership_types(),
    })
}

pub async fn health(State(state): State<AppState>) -> Response {
    match state.club.health().await {
        Ok(()) => (StatusCode::OK, "ok").into_response(),
        Err(error) => {
            tracing::warn!(%error, "Health-Check fehlgeschlagen");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                "Datenbank nicht erreichbar",
            )
                .into_response()
        }
    }
}

const HTMX: &[u8] = include_bytes!("../static/htmx.min.js");
const CSS: &str = include_str!("../static/app.css");

pub async fn htmx() -> Response {
    (
        [
            (header::CONTENT_TYPE, "text/javascript; charset=utf-8"),
            (header::CACHE_CONTROL, "public, max-age=86400"),
        ],
        HTMX,
    )
        .into_response()
}

pub async fn css() -> Response {
    (
        [
            (header::CONTENT_TYPE, "text/css; charset=utf-8"),
            (header::CACHE_CONTROL, "public, max-age=3600"),
        ],
        CSS,
    )
        .into_response()
}
