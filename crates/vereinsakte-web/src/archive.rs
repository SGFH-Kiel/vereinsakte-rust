//! Archiv beim Protokoll: nicht mehr Aktuelles (Vorstand und Revisoren).

use std::collections::HashMap;

use askama::Template;
use axum::extract::{Query, State};
use axum::response::{Redirect, Response};
use tower_sessions::Session;
use vereinsakte_app::archive::{ArchivedBlock, ArchivedMember};
use vereinsakte_app::offices::OfficeRow;
use vereinsakte_app::reservations::ReservationRow;
use vereinsakte_app::work::EventLine;

use crate::error::WebError;
use crate::list::{Sort, SortCtx, SortKey, date_key, sort_rows, text_key};
use crate::pages::{Layout, render};
use crate::session::RequireUser;
use crate::state::AppState;

const TABS: [(&str, &str, &str); 5] = [
    ("aemter", "Ämter", "award"),
    ("mitglieder", "Mitglieder", "users"),
    ("sperren", "Sperren", "lock"),
    ("reservierungen", "Reservierungen", "boat"),
    ("termine", "Arbeitstermine", "wrench"),
];

#[derive(Template)]
#[template(path = "archive.html")]
struct ArchivePage {
    layout: Layout,
    tab: String,
    tabs: Vec<(&'static str, &'static str, &'static str, String)>,
    year: String,
    sort: SortCtx,
    offices: Vec<OfficeRow>,
    members: Vec<ArchivedMember>,
    blocks: Vec<ArchivedBlock>,
    reservations: Vec<ReservationRow>,
    events: Vec<EventLine>,
    can_archive: bool,
}

/// Die frühere Amtshistorie ist ein Reiter im Archiv.
pub async fn offices_history() -> Redirect {
    Redirect::to("/protokoll/archiv?tab=aemter")
}

pub async fn show(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Response, WebError> {
    let tab = params
        .get("tab")
        .filter(|t| TABS.iter().any(|(key, _, _)| key == t))
        .cloned()
        .unwrap_or_else(|| "aemter".to_string());
    let year: Option<i32> = params.get("jahr").and_then(|y| y.trim().parse().ok());
    let sort_state = Sort::from_params(&params, "arch");
    let (mut offices, mut members, mut blocks, mut reservations, mut events) =
        (vec![], vec![], vec![], vec![], vec![]);
    let options: &[(&str, &str)];
    match tab.as_str() {
        "aemter" => {
            offices = state.club.archive_offices(&user).await?;
            sort_rows(
                &mut offices,
                &sort_state,
                &[
                    ("amt", |r| text_key(&r.role)),
                    ("inhaber", |r| text_key(&r.holder)),
                    ("von", |r| date_key(&r.since)),
                    ("bis", |r| {
                        date_key(r.until.as_deref().unwrap_or("31.12.9999"))
                    }),
                ],
            );
            options = &[
                ("amt", "Amt"),
                ("inhaber", "Inhaber"),
                ("von", "Von"),
                ("bis", "Bis"),
            ];
        }
        "mitglieder" => {
            members = state.club.archive_members(&user).await?;
            sort_rows(
                &mut members,
                &sort_state,
                &[
                    ("name", |m| text_key(&m.person.name)),
                    ("art", |m| text_key(&m.membership_type)),
                    ("eintritt", |m| date_key(&m.joined)),
                    ("ende", |m| date_key(&m.ended)),
                ],
            );
            options = &[
                ("name", "Name"),
                ("art", "Mitgliedsart"),
                ("eintritt", "Eintritt"),
                ("ende", "Ende"),
            ];
        }
        "sperren" => {
            blocks = state.club.archive_blocks(&user).await?;
            sort_rows(
                &mut blocks,
                &sort_state,
                &[
                    ("boot", |b| text_key(&b.boat)),
                    ("grund", |b| text_key(&b.reason)),
                    ("zeitraum", |b| date_key(&b.period)),
                ],
            );
            options = &[
                ("boot", "Boot"),
                ("zeitraum", "Zeitraum"),
                ("grund", "Grund"),
            ];
        }
        "reservierungen" => {
            reservations = state.club.archive_reservations(&user, year).await?;
            sort_rows(
                &mut reservations,
                &sort_state,
                &[
                    ("wann", |r| date_key(&r.day)),
                    ("boot", |r| text_key(&r.boat)),
                    ("skipper", |r| text_key(&r.skipper)),
                ],
            );
            options = &[("wann", "Wann"), ("boot", "Boot"), ("skipper", "Skipper")];
        }
        _ => {
            events = state.club.archive_events(&user, year).await?;
            sort_rows(
                &mut events,
                &sort_state,
                &[
                    ("wann", |e| {
                        SortKey::Num(i64::from(chrono::Datelike::num_days_from_ce(&e.start_date)))
                    }),
                    ("titel", |e| text_key(&e.title)),
                    ("ort", |e| text_key(&e.location)),
                ],
            );
            options = &[("wann", "Wann"), ("titel", "Titel"), ("ort", "Ort")];
        }
    }
    let sort = SortCtx::new(
        "arch",
        "/protokoll/archiv",
        vec![
            ("tab".to_string(), tab.clone()),
            (
                "jahr".to_string(),
                year.map(|y| y.to_string()).unwrap_or_default(),
            ),
        ],
        &sort_state,
        options,
    );
    let tabs = TABS
        .iter()
        .map(|(key, label, icon)| (*key, *label, *icon, format!("/protokoll/archiv?tab={key}")))
        .collect();
    let layout =
        Layout::build(&state, &session, Some(&user), "Archiv", "/protokoll/archiv").await?;
    render(&ArchivePage {
        layout,
        tab,
        tabs,
        year: year.map(|y| y.to_string()).unwrap_or_default(),
        sort,
        offices,
        members,
        blocks,
        reservations,
        events,
        can_archive: true,
    })
}
