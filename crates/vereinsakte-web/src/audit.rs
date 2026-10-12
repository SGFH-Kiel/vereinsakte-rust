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
use crate::list::{ListQuery, Pager, Sort, SortCtx, date_key, paginate, sort_rows, text_key};
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
    #[serde(default)]
    sort_prot: String,
    #[serde(default)]
    dir_prot: String,
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
    sort: SortCtx,
    active_filters: usize,
    tab: &'static str,
    can_archive: bool,
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
    // Sortierung vor dem Seitenwechsel, damit die Seiten zusammenpassen.
    let sort_state = Sort {
        key: params.sort_prot.clone(),
        desc: params.dir_prot == "desc",
    };
    let mut all_rows = std::mem::take(&mut v.rows);
    sort_rows(
        &mut all_rows,
        &sort_state,
        &[
            ("zeit", |r| date_key(&r.when)),
            ("akteur", |r| text_key(&r.actor.name)),
            ("aktion", |r| text_key(&r.action)),
        ],
    );
    let filter_params = vec![
        ("von".to_string(), params.von.clone().unwrap_or_default()),
        ("bis".to_string(), params.bis.clone().unwrap_or_default()),
        (
            "akteur".to_string(),
            params.akteur.clone().unwrap_or_default(),
        ),
        ("art".to_string(), params.art.clone().unwrap_or_default()),
    ];
    let active_filters = filter_params.iter().filter(|(_, v)| !v.is_empty()).count();
    let mut list = list;
    if !sort_state.key.is_empty() {
        list.extra
            .push(("sort_prot".into(), sort_state.key.clone()));
        if sort_state.desc {
            list.extra.push(("dir_prot".into(), "desc".into()));
        }
    }
    let sort = SortCtx::new(
        "prot",
        "/protokoll",
        filter_params,
        &sort_state,
        &[
            ("zeit", "Zeitpunkt"),
            ("akteur", "Handelnd"),
            ("aktion", "Aktion"),
        ],
    );
    let can_archive = state
        .club
        .principal(&user)
        .await?
        .can(&vereinsakte_core::rights::Capability::ReadArchive);
    let (rows, pager) = paginate(all_rows, &list, "/protokoll");
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
        sort,
        active_filters,
        tab: "protokoll",
        can_archive,
    })
}
