//! Reservierungen: Kalender, Anlegen, Ändern, Stornieren.

use std::collections::HashMap;

use askama::Template;
use axum::Form;
use axum::extract::{Path, Query, State};
use axum::response::Response;
use tower_sessions::Session;
use vereinsakte_app::reservations::{ReservationFormData, ReservationInput, ReservationPage};
use vereinsakte_app::{AppError, CurrentUser};
use vereinsakte_core::ids::ReservationId;

use crate::error::WebError;
use crate::forms::{FieldErrors, errors_from, redirect_with_flash, unprocessable};
use crate::pages::{Layout, render};
use crate::session::RequireUser;
use crate::state::AppState;

fn parse_id(text: &str) -> Result<ReservationId, WebError> {
    text.parse()
        .map_err(|_| AppError::NotFound("Reservierung".into()).into())
}

/// Die frühere Reservierungsliste lebt im Kalender (`/kalender`).
pub async fn list(Query(query): Query<HashMap<String, String>>) -> axum::response::Redirect {
    let mut params: Vec<String> = vec!["art=reservierungen".into()];
    for (key, target) in [
        ("boot", "boot"),
        ("von", "von"),
        ("bis", "bis"),
        ("meine", "meine"),
    ] {
        if let Some(value) = query.get(key).filter(|v| !v.is_empty()) {
            params.push(format!("{target}={}", crate::session::encode(value)));
        }
    }
    axum::response::Redirect::to(&format!("/kalender?{}", params.join("&")))
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

/// Eine Zeile der Crew-Liste im Formular.
pub struct CrewRow {
    pub number: String,
    pub name: String,
    pub initials: String,
    /// Zeiten im Format `JJJJ-MM-TTTHH:MM` (leer ohne eigenen Zeitraum).
    pub from: String,
    pub to: String,
    /// Anzeige, z. B. „ab 07.07. 10:00 bis 14.07. 18:00“.
    pub period: String,
}

#[derive(Template)]
#[template(path = "reservation_form.html")]
struct FormPage {
    layout: Layout,
    form: ReservationInput,
    errors: FieldErrors,
    data: ReservationFormData,
    crew: Vec<CrewRow>,
    add_member: String,
    add_from: String,
    add_to: String,
    is_new: bool,
    action: String,
    version: i64,
}

/// Normalisiert Zeitangaben („2026-07-07 10:00“, „07.07.2026 10:00“, …) auf `JJJJ-MM-TTTHH:MM`.
fn normalize_stamp(text: &str) -> String {
    let text = text.trim();
    ["%Y-%m-%d %H:%M", "%d.%m.%Y %H:%M", "%Y-%m-%dT%H:%M"]
        .iter()
        .find_map(|f| chrono::NaiveDateTime::parse_from_str(text, f).ok())
        .map(|n| n.format("%Y-%m-%dT%H:%M").to_string())
        .unwrap_or_default()
}

fn stamp_display(text: &str) -> String {
    chrono::NaiveDateTime::parse_from_str(text, "%Y-%m-%dT%H:%M")
        .map(|n| n.format("%d.%m.%Y %H:%M").to_string())
        .unwrap_or_default()
}

fn period_text(from: &str, to: &str) -> String {
    let mut text = String::new();
    if !from.is_empty() {
        text.push_str(&format!(" ab {}", stamp_display(from)));
    }
    if !to.is_empty() {
        text.push_str(&format!(" bis {}", stamp_display(to)));
    }
    text
}

/// Crew-Zeilen (`Nummer; von; bis`) in Listenzeilen umwandeln.
fn rows_from_text(text: &str, members: &[(u32, String)]) -> Vec<CrewRow> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(|line| {
            let parts: Vec<&str> = line.split(';').map(str::trim).collect();
            let number = parts[0].to_string();
            let name = number
                .parse::<u32>()
                .ok()
                .and_then(|n| members.iter().find(|(id, _)| *id == n))
                .map_or_else(|| number.clone(), |(_, name)| name.clone());
            let from = parts.get(1).map(|t| normalize_stamp(t)).unwrap_or_default();
            let to = parts.get(2).map(|t| normalize_stamp(t)).unwrap_or_default();
            CrewRow {
                period: period_text(&from, &to),
                initials: crate::pages::initials(&name),
                number,
                name,
                from,
                to,
            }
        })
        .collect()
}

fn rows_to_text(rows: &[CrewRow]) -> String {
    rows.iter()
        .map(|r| format!("{}; {}; {}", r.number, r.from, r.to))
        .collect::<Vec<_>>()
        .join("\n")
}

fn first<'a>(pairs: &'a [(String, String)], name: &str) -> &'a str {
    pairs
        .iter()
        .find(|(k, _)| k == name)
        .map_or("", |(_, v)| v.as_str())
}

fn all<'a>(pairs: &'a [(String, String)], name: &str) -> Vec<&'a str> {
    pairs
        .iter()
        .filter(|(k, _)| k == name)
        .map(|(_, v)| v.as_str())
        .collect()
}

/// Eingaben aus den abgeschickten Feldern; die Crew-Liste kommt aus den wiederholten Feldern.
fn input_from(
    pairs: &[(String, String)],
    members: &[(u32, String)],
) -> (ReservationInput, Vec<CrewRow>) {
    let t = |n: &str| first(pairs, n).trim().to_string();
    let numbers = all(pairs, "crew_member");
    let froms = all(pairs, "crew_from");
    let tos = all(pairs, "crew_to");
    let crew: Vec<CrewRow> = numbers
        .iter()
        .enumerate()
        .map(|(i, number)| {
            let name = number
                .parse::<u32>()
                .ok()
                .and_then(|n| members.iter().find(|(id, _)| *id == n))
                .map_or_else(|| (*number).to_string(), |(_, name)| name.clone());
            let from = froms.get(i).map(|t| normalize_stamp(t)).unwrap_or_default();
            let to = tos.get(i).map(|t| normalize_stamp(t)).unwrap_or_default();
            CrewRow {
                period: period_text(&from, &to),
                initials: crate::pages::initials(&name),
                number: (*number).to_string(),
                name,
                from,
                to,
            }
        })
        .collect();
    let input = ReservationInput {
        boat: t("boat"),
        area: t("area"),
        from_date: t("from_date"),
        from_time: t("from_time"),
        to_date: t("to_date"),
        to_time: t("to_time"),
        skipper: t("skipper"),
        co_skipper: t("co_skipper"),
        crew: rows_to_text(&crew),
        remark: first(pairs, "remark").replace("\r\n", "\n"),
    };
    (input, crew)
}

#[allow(clippy::too_many_arguments)]
async fn form_page(
    state: &AppState,
    session: &Session,
    user: &CurrentUser,
    form: ReservationInput,
    crew: Option<Vec<CrewRow>>,
    errors: FieldErrors,
    id: Option<(&str, i64)>,
    add: (String, String, String),
    ok_status: bool,
) -> Result<Response, WebError> {
    let data = state.club.reservation_form_data(user).await?;
    let crew = crew.unwrap_or_else(|| rows_from_text(&form.crew, &data.members));
    let layout = Layout::build(
        state,
        session,
        Some(user),
        "Reservierung",
        "/reservierungen",
    )
    .await?
    .form();
    let invalid = !errors.is_empty() && !ok_status;
    let (action, version, is_new) = match id {
        Some((id, version)) => (format!("/reservierungen/{id}/bearbeiten"), version, false),
        None => ("/reservierungen/neu".to_string(), 0, true),
    };
    let response = render(&FormPage {
        layout,
        form,
        errors,
        data,
        crew,
        add_member: add.0,
        add_from: add.1,
        add_to: add.2,
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

/// Reaktion auf „Hinzufügen“ und „Entfernen“ der Crew-Liste (ohne Speichern).
/// Liefert `None`, wenn die Reservierung gespeichert werden soll.
fn crew_action(
    pairs: &[(String, String)],
    crew: &mut Vec<CrewRow>,
    members: &[(u32, String)],
    form: &ReservationInput,
    me: u32,
    errors: &mut FieldErrors,
) -> Option<(String, String, String)> {
    let action = first(pairs, "action");
    if let Some(index) = action.strip_prefix("crew_remove:") {
        if let Ok(index) = index.parse::<usize>()
            && index < crew.len()
        {
            crew.remove(index);
        }
        return Some(Default::default());
    }
    if action != "crew_add" {
        return None;
    }
    let number = first(pairs, "add_member").trim().to_string();
    let (from, to) = (
        normalize_stamp(first(pairs, "add_from")),
        normalize_stamp(first(pairs, "add_to")),
    );
    let keep = (
        number.clone(),
        first(pairs, "add_from").to_string(),
        first(pairs, "add_to").to_string(),
    );
    let Some((id, name)) = number
        .parse::<u32>()
        .ok()
        .and_then(|n| members.iter().find(|(id, _)| *id == n))
    else {
        errors.add("crew", "Bitte ein Mitglied wählen.");
        return Some(keep);
    };
    let skipper = form.skipper.trim();
    let skipper = if skipper.is_empty() {
        me.to_string()
    } else {
        skipper.to_string()
    };
    let on_board = crew.iter().any(|r| r.number == number)
        || skipper == number
        || form.co_skipper.trim() == number;
    if on_board {
        errors.add("crew", format!("{name} ist bereits an Bord."));
        return Some(keep);
    }
    let _ = id;
    crew.push(CrewRow {
        period: period_text(&from, &to),
        initials: crate::pages::initials(name),
        number,
        name: name.clone(),
        from,
        to,
    });
    Some(Default::default())
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
    form_page(
        &state,
        &session,
        &user,
        form,
        None,
        FieldErrors::new(),
        None,
        Default::default(),
        false,
    )
    .await
}

pub async fn new_submit(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Result<Response, WebError> {
    let data = state.club.reservation_form_data(&user).await?;
    let (input, mut crew) = input_from(&pairs, &data.members);
    let mut errors = FieldErrors::new();
    if let Some(add) = crew_action(
        &pairs,
        &mut crew,
        &data.members,
        &input,
        user.member.get(),
        &mut errors,
    ) {
        let input = ReservationInput {
            crew: rows_to_text(&crew),
            ..input
        };
        return form_page(
            &state,
            &session,
            &user,
            input,
            Some(crew),
            errors,
            None,
            add,
            true,
        )
        .await;
    }
    match state.club.create_reservation(&user, &input).await {
        Ok(id) => Ok(redirect_with_flash(
            &session,
            &format!("/reservierungen/{id}"),
            "Die Reservierung wurde angelegt.",
        )
        .await),
        Err(error) => {
            let errors = errors_from(error)?;
            form_page(
                &state,
                &session,
                &user,
                input,
                Some(crew),
                errors,
                None,
                Default::default(),
                false,
            )
            .await
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
        None,
        FieldErrors::new(),
        Some((&id, version)),
        Default::default(),
        false,
    )
    .await
}

pub async fn edit_submit(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Path(id): Path<String>,
    Form(pairs): Form<Vec<(String, String)>>,
) -> Result<Response, WebError> {
    let data = state.club.reservation_form_data(&user).await?;
    let (input, mut crew) = input_from(&pairs, &data.members);
    let version = first(&pairs, "version").parse().unwrap_or(0);
    let mut errors = FieldErrors::new();
    if let Some(add) = crew_action(
        &pairs,
        &mut crew,
        &data.members,
        &input,
        user.member.get(),
        &mut errors,
    ) {
        let input = ReservationInput {
            crew: rows_to_text(&crew),
            ..input
        };
        return form_page(
            &state,
            &session,
            &user,
            input,
            Some(crew),
            errors,
            Some((&id, version)),
            add,
            true,
        )
        .await;
    }
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
            form_page(
                &state,
                &session,
                &user,
                input,
                Some(crew),
                errors,
                Some((&id, version)),
                Default::default(),
                false,
            )
            .await
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
    Ok(redirect_with_flash(&session, "/kalender", "Die Reservierung wurde storniert.").await)
}
