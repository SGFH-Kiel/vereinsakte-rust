//! Ämterübersicht, Ernennen, Abgeben und Historie.

use std::collections::HashMap;

use askama::Template;
use axum::Form;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Redirect, Response};
use tower_sessions::Session;
use vereinsakte_app::AppError;
use vereinsakte_app::offices::{OfficesView, parse_role};
use vereinsakte_core::ids::{MemberId, OfficeId};

use crate::error::WebError;
use crate::forms::{FieldErrors, non_empty, parse_date, redirect_with_flash, unprocessable};
use crate::list::{Sort, SortCtx, date_key, sort_rows, text_key};
use crate::pages::{Layout, render};
use crate::session::{FlashKind, RequireUser, flash};
use crate::state::AppState;

type Fields = HashMap<String, String>;

#[derive(Template)]
#[template(path = "offices.html")]
struct OfficesPage {
    layout: Layout,
    v: OfficesView,
    hints: Vec<String>,
    errors: FieldErrors,
    chosen_member: String,
    chosen_role: String,
    chosen_start: String,
    sorts: Vec<SortCtx>,
}

async fn page(
    state: &AppState,
    session: &Session,
    user: &vereinsakte_app::CurrentUser,
    errors: FieldErrors,
    chosen: (String, String, String),
    params: &HashMap<String, String>,
) -> Result<Response, WebError> {
    let mut v = state.club.offices_view(user).await?;
    let mut sorts = vec![];
    for (index, section) in v.sections.iter_mut().enumerate() {
        let id = format!("amt{index}");
        let sort_state = Sort::from_params(params, &id);
        sort_rows(
            &mut section.rows,
            &sort_state,
            &[
                ("amt", |r| text_key(&r.role)),
                ("inhaber", |r| text_key(&r.holder)),
                ("seit", |r| date_key(&r.since)),
            ],
        );
        let others: Vec<(String, String)> = params
            .iter()
            .filter(|(k, _)| k.starts_with("sort_amt") || k.starts_with("dir_amt"))
            .filter(|(k, _)| !k.ends_with(&id))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        sorts.push(SortCtx::new(
            &id,
            "/aemter",
            others,
            &sort_state,
            &[("amt", "Amt"), ("inhaber", "Inhaber"), ("seit", "Seit")],
        ));
    }
    let layout = Layout::build(state, session, Some(user), "Ämter", "/aemter").await?;
    let invalid = !errors.is_empty();
    let response = render(&OfficesPage {
        layout,
        v,
        hints: vec![],
        errors,
        chosen_member: chosen.0,
        chosen_role: chosen.1,
        chosen_start: chosen.2,
        sorts,
    })?;
    Ok(if invalid {
        unprocessable(response)
    } else {
        response
    })
}

pub async fn overview(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Result<Response, WebError> {
    page(
        &state,
        &session,
        &user,
        FieldErrors::new(),
        Default::default(),
        &params,
    )
    .await
}

pub async fn appoint(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Form(fields): Form<Fields>,
) -> Result<Response, WebError> {
    let get = |n: &str| fields.get(n).cloned().unwrap_or_default();
    let mut errors = FieldErrors::new();
    let member = get("member").parse::<u32>().ok().and_then(MemberId::new);
    if member.is_none() {
        errors.add("member", "Bitte ein Mitglied wählen.");
    }
    let role = parse_role(&get("role"));
    if role.is_none() {
        errors.add("role", "Bitte ein Amt wählen.");
    }
    let start = non_empty(&get("start")).and_then(|d| errors.take("start", parse_date(d)));
    let chosen = (get("member"), get("role"), get("start"));
    let (Some(member), Some(role)) = (member, role) else {
        return page(&state, &session, &user, errors, chosen, &HashMap::new()).await;
    };
    if !errors.is_empty() {
        return page(&state, &session, &user, errors, chosen, &HashMap::new()).await;
    }
    match state.club.appoint(&user, member, role, start).await {
        Ok(hints) => {
            let kind = if hints.is_empty() {
                FlashKind::Ok
            } else {
                FlashKind::Warn
            };
            let mut text = "Das Amt wurde vergeben.".to_string();
            for hint in &hints {
                text.push_str(&format!(" Hinweis: {hint}."));
            }
            flash(&session, kind, text).await;
            Ok(IntoResponse::into_response(Redirect::to("/aemter")))
        }
        Err(AppError::Rejected(list)) => {
            let mut errors = FieldErrors::new();
            for e in list {
                errors.add(&e.field, e.message);
            }
            page(&state, &session, &user, errors, chosen, &HashMap::new()).await
        }
        Err(other) => Err(other.into()),
    }
}

pub async fn resign(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Path(id): Path<String>,
    Form(fields): Form<Fields>,
) -> Result<Response, WebError> {
    let office: OfficeId = id.parse().map_err(|_| AppError::NotFound("Amt".into()))?;
    let date = match non_empty(fields.get("date").map_or("", String::as_str)) {
        None => None,
        Some(text) => match parse_date(text) {
            Ok(d) => Some(d),
            Err(message) => {
                flash(&session, FlashKind::Error, message).await;
                return Ok(IntoResponse::into_response(Redirect::to("/aemter")));
            }
        },
    };
    match state.club.resign(&user, office, date).await {
        Ok(()) => Ok(redirect_with_flash(&session, "/aemter", "Das Amt wurde abgegeben.").await),
        Err(AppError::Rejected(list)) => {
            let message = list.first().map(|e| e.message.clone()).unwrap_or_default();
            flash(&session, FlashKind::Error, message).await;
            Ok(IntoResponse::into_response(Redirect::to("/aemter")))
        }
        Err(other) => Err(other.into()),
    }
}
