//! Änderungsprotokoll: Einsicht mit Filtern, neueste zuerst.

use askama::Template;
use axum::extract::{Query, State};
use axum::response::Response;
use chrono::NaiveDate;
use serde::Deserialize;
use tower_sessions::Session;
use vereinsakte_app::audit::{AuditQuery, AuditRow, AuditView};
use vereinsakte_core::ids::MemberId;

use crate::error::WebError;
use crate::list::{ListQuery, Pager, paginate};
use crate::pages::{Layout, render};
use crate::session::RequireUser;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct AuditParams {
    von: Option<String>,
    bis: Option<String>,
    akteur: Option<String>,
    art: Option<String>,
    seite: Option<usize>,
}

#[derive(Template)]
#[template(path = "audit.html")]
struct AuditPage {
    layout: Layout,
    v: AuditView,
    rows: Vec<AuditRow>,
    pager: Pager,
    from: String,
    to: String,
    actor: String,
    action: String,
}

pub async fn show(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Query(params): Query<AuditParams>,
) -> Result<Response, WebError> {
    let date = |t: &Option<String>| {
        t.as_deref()
            .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
    };
    let query = AuditQuery {
        from: date(&params.von),
        to: date(&params.bis),
        actor: params
            .akteur
            .as_deref()
            .and_then(|a| a.parse::<u32>().ok())
            .and_then(MemberId::new),
        action: params.art.clone().unwrap_or_default(),
    };
    let mut v = state.club.audit_view(&user, &query).await?;
    let list = ListQuery {
        seite: params.seite,
        extra: vec![
            ("von".into(), params.von.clone().unwrap_or_default()),
            ("bis".into(), params.bis.clone().unwrap_or_default()),
            ("akteur".into(), params.akteur.clone().unwrap_or_default()),
            ("art".into(), params.art.clone().unwrap_or_default()),
        ],
        ..ListQuery::default()
    };
    let (rows, pager) = paginate(std::mem::take(&mut v.rows), &list, "/protokoll");
    let layout = Layout::build(&state, &session, Some(&user), "Protokoll", "/protokoll").await?;
    render(&AuditPage {
        layout,
        v,
        rows,
        pager,
        from: params.von.unwrap_or_default(),
        to: params.bis.unwrap_or_default(),
        actor: params.akteur.unwrap_or_default(),
        action: params.art.unwrap_or_default(),
    })
}
