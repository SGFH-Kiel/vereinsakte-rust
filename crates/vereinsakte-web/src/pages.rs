//! Layout und Seiten: „Meine Daten“, Mitgliederverzeichnis, Health-Check und
//! statische Dateien.

use askama::Template;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{Html, IntoResponse, Redirect, Response};
use serde::Deserialize;
use tower_sessions::Session;
use vereinsakte_app::CurrentUser;
use vereinsakte_app::directory::DirectoryView;
use vereinsakte_app::my_data::MyDataView;
use vereinsakte_app::service::LoginChoiceGroup;

use crate::error::WebError;
use crate::session::{RequireUser, csrf_token};
use crate::state::AppState;

/// Gemeinsame Angaben des Basislayouts.
pub struct Layout {
    pub title: String,
    pub csrf: String,
    pub user: Option<UserBar>,
    pub dev_bar: Option<DevBar>,
}

pub struct UserBar {
    pub name: String,
    pub offices: Vec<String>,
}

/// Leiste für den schnellen Benutzerwechsel im Entwicklungsmodus.
pub struct DevBar {
    pub current: u32,
    pub current_name: String,
    pub next: String,
    pub groups: Vec<LoginChoiceGroup>,
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
                let context = state.club.user_context(user).await?;
                let dev_bar = if state.settings.development {
                    Some(DevBar {
                        current: user.member.get(),
                        current_name: context.name.clone(),
                        next: next.to_string(),
                        groups: state.club.login_choices().await?,
                    })
                } else {
                    None
                };
                (
                    Some(UserBar {
                        name: context.name,
                        offices: context.offices,
                    }),
                    dev_bar,
                )
            }
            None => (None, None),
        };
        Ok(Self {
            title: title.into(),
            csrf,
            user: user_bar,
            dev_bar,
        })
    }
}

pub fn render(template: &impl Template) -> Result<Response, WebError> {
    Ok(Html(template.render()?).into_response())
}

pub async fn home() -> Redirect {
    Redirect::to("/meine-daten")
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
}

#[derive(Template)]
#[template(path = "directory.html")]
struct DirectoryPage {
    layout: Layout,
    v: DirectoryView,
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
    let v = state.club.directory(&user, &query.q).await?;
    if headers.contains_key("hx-request") {
        return render(&DirectoryTable { v });
    }
    let next = if v.query.is_empty() {
        "/mitglieder".to_string()
    } else {
        format!("/mitglieder?q={}", crate::session::encode(&v.query))
    };
    let layout = Layout::build(&state, &session, Some(&user), "Mitglieder", &next).await?;
    render(&DirectoryPage { layout, v })
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
