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
fn navigation(_principal: &Principal, path: &str) -> Vec<NavItem> {
    let path = path.split('?').next().unwrap_or(path);
    let current = |href: &str| {
        if href == "/" {
            path == "/"
        } else {
            path == href || path.starts_with(&format!("{href}/"))
        }
    };
    [
        ("/", "Start"),
        ("/meine-daten", "Meine Daten"),
        ("/mitglieder", "Mitglieder"),
        ("/boote", "Boote"),
        ("/aemter", "Ämter"),
    ]
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
}

/// Startseite: die nächsten Schritte des angemeldeten Mitglieds.
pub async fn home(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
) -> Result<Response, WebError> {
    let v = state.club.my_data(&user).await?;
    let layout = Layout::build(&state, &session, Some(&user), "Start", "/").await?;
    render(&StartPage { layout, v })
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
