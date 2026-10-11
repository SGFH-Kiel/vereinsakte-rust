//! Reservierungen: Kalender, Anlegen, Ändern, Stornieren.

use std::collections::HashMap;

use askama::Template;
use axum::Form;
use axum::extract::{Path, Query, State};
use axum::response::Response;
use chrono::{Duration, NaiveDate};
use serde::Deserialize;
use tower_sessions::Session;
use vereinsakte_app::reservations::{
    ReservationFilter, ReservationFormData, ReservationInput, ReservationList, ReservationPage,
};
use vereinsakte_app::{AppError, CurrentUser};
use vereinsakte_core::ids::{BoatId, ReservationId};

use crate::error::WebError;
use crate::forms::{FieldErrors, errors_from, redirect_with_flash, unprocessable};
use crate::pages::{Layout, render};
use crate::session::RequireUser;
use crate::state::AppState;

type Fields = HashMap<String, String>;

fn get<'a>(fields: &'a Fields, name: &str) -> &'a str {
    fields.get(name).map_or("", String::as_str)
}

fn parse_id(text: &str) -> Result<ReservationId, WebError> {
    text.parse()
        .map_err(|_| AppError::NotFound("Reservierung".into()).into())
}

#[derive(Deserialize)]
pub struct ListQuery {
    boot: Option<String>,
    von: Option<String>,
    bis: Option<String>,
    meine: Option<String>,
}

#[derive(Template)]
#[template(path = "reservations.html")]
struct ListPage {
    layout: Layout,
    v: ReservationList,
    from: String,
    to: String,
    from_iso: String,
    to_iso: String,
    boat: String,
    mine: bool,
    previous: String,
    next: String,
}

fn url(boat: &str, from: NaiveDate, to: NaiveDate, mine: bool) -> String {
    let mut text = format!("/reservierungen?von={from}&bis={to}");
    if !boat.is_empty() {
        text.push_str(&format!("&boot={boat}"));
    }
    if mine {
        text.push_str("&meine=1");
    }
    text
}

pub async fn list(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Query(query): Query<ListQuery>,
) -> Result<Response, WebError> {
    let date = |t: &Option<String>| {
        t.as_deref()
            .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
    };
    let boat_text = query.boot.clone().unwrap_or_default();
    let filter = ReservationFilter {
        boat: boat_text.parse::<BoatId>().ok(),
        from: date(&query.von),
        to: date(&query.bis),
        mine: query.meine.is_some(),
    };
    let v = state.club.reservation_list(&user, &filter).await?;
    let layout = Layout::build(
        &state,
        &session,
        Some(&user),
        "Reservierungen",
        "/reservierungen",
    )
    .await?;
    render(&ListPage {
        layout,
        from: v.from.format("%d.%m.%Y").to_string(),
        to: v.to.format("%d.%m.%Y").to_string(),
        from_iso: v.from.to_string(),
        to_iso: v.to.to_string(),
        previous: url(
            &boat_text,
            v.from - Duration::days(7),
            v.to - Duration::days(7),
            filter.mine,
        ),
        next: url(
            &boat_text,
            v.from + Duration::days(7),
            v.to + Duration::days(7),
            filter.mine,
        ),
        boat: boat_text,
        mine: filter.mine,
        v,
    })
}

#[derive(Template)]
#[template(path = "reservation.html")]
struct ShowPage {
    layout: Layout,
    p: ReservationPage,
}

pub async fn show(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Path(id): Path<String>,
) -> Result<Response, WebError> {
    let p = state.club.reservation_page(&user, parse_id(&id)?).await?;
    let layout = Layout::build(
        &state,
        &session,
        Some(&user),
        "Reservierung",
        &format!("/reservierungen/{id}"),
    )
    .await?;
    render(&ShowPage { layout, p })
}

#[derive(Template)]
#[template(path = "reservation_form.html")]
struct FormPage {
    layout: Layout,
    form: ReservationInput,
    errors: FieldErrors,
    data: ReservationFormData,
    is_new: bool,
    action: String,
    version: i64,
}

fn input_from(fields: &Fields) -> ReservationInput {
    let t = |n: &str| get(fields, n).trim().to_string();
    ReservationInput {
        boat: t("boat"),
        area: t("area"),
        from_date: t("from_date"),
        from_time: t("from_time"),
        to_date: t("to_date"),
        to_time: t("to_time"),
        tour: fields.contains_key("tour"),
        skipper: t("skipper"),
        co_skipper: t("co_skipper"),
        crew: get(fields, "crew").replace("\r\n", "\n"),
        guests: get(fields, "guests").replace("\r\n", "\n"),
        remark: t("remark"),
    }
}

async fn form_page(
    state: &AppState,
    session: &Session,
    user: &CurrentUser,
    form: ReservationInput,
    errors: FieldErrors,
    id: Option<(&str, i64)>,
) -> Result<Response, WebError> {
    let data = state.club.reservation_form_data(user).await?;
    let layout = Layout::build(
        state,
        session,
        Some(user),
        "Reservierung",
        "/reservierungen",
    )
    .await?;
    let invalid = !errors.is_empty();
    let (action, version, is_new) = match id {
        Some((id, version)) => (format!("/reservierungen/{id}/bearbeiten"), version, false),
        None => ("/reservierungen/neu".to_string(), 0, true),
    };
    let response = render(&FormPage {
        layout,
        form,
        errors,
        data,
        is_new,
        action,
        version,
    })?;
    Ok(if invalid {
        unprocessable(response)
    } else {
        response
    })
}

pub async fn new(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Query(query): Query<HashMap<String, String>>,
) -> Result<Response, WebError> {
    let form = ReservationInput {
        boat: query.get("boot").cloned().unwrap_or_default(),
        area: "home".into(),
        from_time: "10:00".into(),
        to_time: "18:00".into(),
        ..Default::default()
    };
    form_page(&state, &session, &user, form, FieldErrors::new(), None).await
}

pub async fn new_submit(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Form(fields): Form<Fields>,
) -> Result<Response, WebError> {
    let input = input_from(&fields);
    match state.club.create_reservation(&user, &input).await {
        Ok(id) => Ok(redirect_with_flash(
            &session,
            &format!("/reservierungen/{id}"),
            "Die Reservierung wurde angelegt.",
        )
        .await),
        Err(error) => {
            let errors = errors_from(error)?;
            form_page(&state, &session, &user, input, errors, None).await
        }
    }
}

pub async fn edit(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Path(id): Path<String>,
) -> Result<Response, WebError> {
    let (form, version) = state.club.reservation_input(&user, parse_id(&id)?).await?;
    form_page(
        &state,
        &session,
        &user,
        form,
        FieldErrors::new(),
        Some((&id, version)),
    )
    .await
}

pub async fn edit_submit(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Path(id): Path<String>,
    Form(fields): Form<Fields>,
) -> Result<Response, WebError> {
    let input = input_from(&fields);
    let version = get(&fields, "version").parse().unwrap_or(0);
    match state
        .club
        .update_reservation(&user, parse_id(&id)?, version, &input)
        .await
    {
        Ok(()) => Ok(redirect_with_flash(
            &session,
            &format!("/reservierungen/{id}"),
            "Die Reservierung wurde geändert.",
        )
        .await),
        Err(error) => {
            let errors = errors_from(error)?;
            form_page(&state, &session, &user, input, errors, Some((&id, version))).await
        }
    }
}

pub async fn cancel(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Path(id): Path<String>,
) -> Result<Response, WebError> {
    state.club.cancel_reservation(&user, parse_id(&id)?).await?;
    Ok(redirect_with_flash(
        &session,
        "/reservierungen",
        "Die Reservierung wurde storniert.",
    )
    .await)
}
