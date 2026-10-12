//! Arbeit: Stunden, Einsätze, Bestätigungen, Termine, Aufgaben und Übersicht.

use std::collections::HashMap;

use askama::Template;
use axum::Form;
use axum::extract::{Path, Query, State};
use axum::response::{IntoResponse, Redirect, Response};
use serde::Deserialize;
use tower_sessions::Session;
use vereinsakte_app::work::{
    EntryInput, EntryLine, EventInput, HoursOverview, TaskInput, TasksView, WorkView,
};
use vereinsakte_app::{AppError, CurrentUser};
use vereinsakte_core::ids::{MemberId, TaskId, WorkEntryId, WorkEventId};
use vereinsakte_core::rights::Capability;
use vereinsakte_core::task::TaskStatus;

use crate::error::WebError;
use crate::forms::{
    FieldErrors, errors_from, non_empty, parse_money, redirect_with_flash, unprocessable,
};
use crate::list::{Sort, SortCtx, SortKey, date_key, num_key, sort_rows, text_key};
use crate::pages::{Layout, render};
use crate::session::{FlashKind, RequireUser, flash};
use crate::state::AppState;

type Fields = HashMap<String, String>;

fn get<'a>(fields: &'a Fields, name: &str) -> &'a str {
    fields.get(name).map_or("", String::as_str)
}

/// Unternavigation „Arbeit“.
pub struct WorkNav {
    pub current: &'static str,
    pub confirm: bool,
    pub all: bool,
}

async fn nav(
    state: &AppState,
    user: &CurrentUser,
    current: &'static str,
) -> Result<WorkNav, WebError> {
    let principal = state.club.principal(user).await?;
    Ok(WorkNav {
        current,
        confirm: principal.has_office(),
        all: principal.can(&Capability::ReadAllHours),
    })
}

fn entry_input(fields: &Fields) -> EntryInput {
    let t = |n: &str| get(fields, n).trim().to_string();
    // Der Umschalter „Stunden / Zeitraum“ bestimmt, welche Angaben zählen.
    let (hours, start, end) = match get(fields, "umfang") {
        "zeitraum" => (String::new(), t("start"), t("end")),
        "stunden" => (t("hours"), String::new(), String::new()),
        _ => (t("hours"), t("start"), t("end")),
    };
    EntryInput {
        date: t("date"),
        hours,
        start,
        end,
        subject: t("subject"),
        description: t("description"),
        reference: t("reference"),
    }
}

fn parse_id<T: std::str::FromStr>(text: &str, what: &str) -> Result<T, WebError> {
    text.parse()
        .map_err(|_| AppError::NotFound(what.into()).into())
}

// ---- Meine Stunden ---------------------------------------------------------------------

#[derive(Deserialize)]
pub struct YearQuery {
    jahr: Option<i32>,
    mitglied: Option<u32>,
    rueckstand: Option<String>,
    #[serde(default)]
    sort_std: String,
    #[serde(default)]
    dir_std: String,
    #[serde(default)]
    sort_alle: String,
    #[serde(default)]
    dir_alle: String,
}

#[derive(Template)]
#[template(path = "work.html")]
struct WorkPage {
    layout: Layout,
    nav: WorkNav,
    v: WorkView,
    sort: SortCtx,
    own: bool,
    form: EntryInput,
    errors: FieldErrors,
    action: String,
    version: i64,
    subjects: Vec<(String, String)>,
    references: Vec<(String, String)>,
}

#[allow(clippy::too_many_arguments)]
async fn work_page(
    state: &AppState,
    session: &Session,
    user: &CurrentUser,
    query: &YearQuery,
    form: EntryInput,
    errors: FieldErrors,
) -> Result<Response, WebError> {
    let target = query.mitglied.and_then(MemberId::new);
    let mut v = state.club.work_view(user, target, query.jahr).await?;
    let sort_state = Sort {
        key: query.sort_std.clone(),
        desc: query.dir_std == "desc",
    };
    sort_rows(
        &mut v.entries,
        &sort_state,
        &[
            ("datum", |e| date_key(&e.date)),
            ("stunden", |e| num_key(&e.hours)),
            ("gegenstand", |e| text_key(&e.subject)),
            ("status", |e| SortKey::Num(i64::from(e.confirmed))),
        ],
    );
    let sort = SortCtx::new(
        "std",
        "/arbeit",
        vec![
            (
                "jahr".to_string(),
                query.jahr.map(|y| y.to_string()).unwrap_or_default(),
            ),
            (
                "mitglied".to_string(),
                query.mitglied.map(|m| m.to_string()).unwrap_or_default(),
            ),
        ],
        &sort_state,
        &[
            ("datum", "Datum"),
            ("stunden", "Stunden"),
            ("gegenstand", "Gegenstand"),
            ("status", "Status"),
        ],
    );
    let own = target.is_none_or(|m| m == user.member);
    let layout = Layout::build(state, session, Some(user), "Arbeit", "/arbeit").await?;
    let invalid = !errors.is_empty();
    let response = render(&WorkPage {
        nav: nav(state, user, "stunden").await?,
        layout,
        sort,
        own,
        form,
        errors,
        action: "/arbeit/einsaetze".into(),
        version: 0,
        subjects: v.subjects.clone(),
        references: v.references.clone(),
        v,
    })?;
    Ok(if invalid {
        unprocessable(response)
    } else {
        response
    })
}

pub async fn hours(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Query(query): Query<YearQuery>,
) -> Result<Response, WebError> {
    let form = EntryInput {
        subject: "club".into(),
        ..Default::default()
    };
    work_page(&state, &session, &user, &query, form, FieldErrors::new()).await
}

pub async fn add_entry(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Form(fields): Form<Fields>,
) -> Result<Response, WebError> {
    let input = entry_input(&fields);
    match state.club.add_work_entry(&user, &input).await {
        Ok(_) => Ok(redirect_with_flash(
            &session,
            "/arbeit",
            "Der Einsatz wurde erfasst. Er zählt, sobald er bestätigt ist.",
        )
        .await),
        Err(error) => {
            let errors = errors_from(error)?;
            work_page(
                &state,
                &session,
                &user,
                &YearQuery {
                    jahr: None,
                    mitglied: None,
                    rueckstand: None,
                    sort_std: String::new(),
                    dir_std: String::new(),
                    sort_alle: String::new(),
                    dir_alle: String::new(),
                },
                input,
                errors,
            )
            .await
        }
    }
}

#[derive(Template)]
#[template(path = "work_entry_edit.html")]
struct EntryEditPage {
    layout: Layout,
    form: EntryInput,
    errors: FieldErrors,
    action: String,
    version: i64,
    subjects: Vec<(String, String)>,
    references: Vec<(String, String)>,
}

async fn entry_edit_page(
    state: &AppState,
    session: &Session,
    user: &CurrentUser,
    id: &str,
    form: EntryInput,
    version: i64,
    errors: FieldErrors,
) -> Result<Response, WebError> {
    let view = state.club.work_view(user, None, None).await?;
    let layout = Layout::build(state, session, Some(user), "Einsatz ändern", "/arbeit").await?;
    let invalid = !errors.is_empty();
    let response = render(&EntryEditPage {
        layout,
        form,
        errors,
        action: format!("/arbeit/einsaetze/{id}/bearbeiten"),
        version,
        subjects: view.subjects,
        references: view.references,
    })?;
    Ok(if invalid {
        unprocessable(response)
    } else {
        response
    })
}

pub async fn edit_entry(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Path(id): Path<String>,
) -> Result<Response, WebError> {
    let (form, version) = state
        .club
        .work_entry_input(&user, parse_id::<WorkEntryId>(&id, "Einsatz")?)
        .await?;
    entry_edit_page(
        &state,
        &session,
        &user,
        &id,
        form,
        version,
        FieldErrors::new(),
    )
    .await
}

pub async fn edit_entry_submit(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Path(id): Path<String>,
    Form(fields): Form<Fields>,
) -> Result<Response, WebError> {
    let entry = parse_id::<WorkEntryId>(&id, "Einsatz")?;
    let input = entry_input(&fields);
    let version = get(&fields, "version").parse().unwrap_or(0);
    match state
        .club
        .update_work_entry(&user, entry, version, &input)
        .await
    {
        Ok(()) => Ok(redirect_with_flash(&session, "/arbeit", "Der Einsatz wurde geändert.").await),
        Err(error) => {
            let errors = errors_from(error)?;
            entry_edit_page(&state, &session, &user, &id, input, version, errors).await
        }
    }
}

pub async fn delete_entry(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Path(id): Path<String>,
) -> Result<Response, WebError> {
    state
        .club
        .delete_work_entry(&user, parse_id::<WorkEntryId>(&id, "Einsatz")?)
        .await?;
    Ok(redirect_with_flash(&session, "/arbeit", "Der Einsatz wurde gelöscht.").await)
}

// ---- Bestätigungen ---------------------------------------------------------------------

#[derive(Template)]
#[template(path = "work_confirmations.html")]
struct ConfirmationsPage {
    layout: Layout,
    nav: WorkNav,
    lines: Vec<EntryLine>,
    sort: SortCtx,
}

pub async fn confirmations(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Response, WebError> {
    let mut lines = state.club.open_confirmations(&user).await?;
    let sort_state = Sort::from_params(&params, "best");
    sort_rows(
        &mut lines,
        &sort_state,
        &[
            ("mitglied", |e| text_key(&e.member)),
            ("datum", |e| date_key(&e.date)),
            ("stunden", |e| num_key(&e.hours)),
            ("gegenstand", |e| text_key(&e.subject)),
        ],
    );
    let sort = SortCtx::new(
        "best",
        "/arbeit/bestaetigungen",
        vec![],
        &sort_state,
        &[
            ("mitglied", "Mitglied"),
            ("datum", "Datum"),
            ("stunden", "Stunden"),
            ("gegenstand", "Gegenstand"),
        ],
    );
    let layout = Layout::build(
        &state,
        &session,
        Some(&user),
        "Bestätigungen",
        "/arbeit/bestaetigungen",
    )
    .await?;
    render(&ConfirmationsPage {
        nav: nav(&state, &user, "bestaetigungen").await?,
        layout,
        lines,
        sort,
    })
}

pub async fn confirm(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Path(id): Path<String>,
) -> Result<Response, WebError> {
    state
        .club
        .confirm_work(&user, parse_id::<WorkEntryId>(&id, "Einsatz")?)
        .await?;
    Ok(redirect_with_flash(
        &session,
        "/arbeit/bestaetigungen",
        "Der Einsatz wurde bestätigt.",
    )
    .await)
}

// ---- Termine -----------------------------------------------------------------------------

fn event_input(fields: &Fields) -> EventInput {
    let t = |n: &str| get(fields, n).trim().to_string();
    EventInput {
        title: t("title"),
        date: t("date"),
        start: t("start"),
        end: t("end"),
        location: t("location"),
        description: t("description"),
        boat: t("boat"),
        max: t("max"),
    }
}

/// Die Terminliste lebt im Kalender (`/kalender`).
pub async fn events() -> Redirect {
    Redirect::to("/kalender?art=termine")
}

#[derive(Template)]
#[template(path = "work_event.html")]
struct EventPage {
    layout: Layout,
    e: vereinsakte_app::work::EventLine,
    can_manage: bool,
}

pub async fn event_show(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Path(id): Path<String>,
) -> Result<Response, WebError> {
    let _: WorkEventId = parse_id(&id, "Arbeitstermin")?;
    let v = state.club.events_view(&user).await?;
    let can_manage = v.can_manage;
    let e = v
        .events
        .into_iter()
        .find(|e| e.id == id)
        .ok_or_else(|| AppError::NotFound("Arbeitstermin".into()))?;
    let layout = Layout::build(
        &state,
        &session,
        Some(&user),
        &e.title.clone(),
        &format!("/arbeit/termine/{id}"),
    )
    .await?;
    render(&EventPage {
        layout,
        e,
        can_manage,
    })
}

#[derive(Template)]
#[template(path = "work_event_edit.html")]
struct EventEditPage {
    layout: Layout,
    form: EventInput,
    errors: FieldErrors,
    action: String,
    version: i64,
    boats: Vec<(String, String)>,
    /// Kennung des Termins beim Ändern, leer beim Anlegen.
    id: String,
}

async fn event_edit_page(
    state: &AppState,
    session: &Session,
    user: &CurrentUser,
    id: Option<&str>,
    form: EventInput,
    version: i64,
    errors: FieldErrors,
) -> Result<Response, WebError> {
    let boats = state.club.events_view(user).await?.boats;
    let layout = Layout::build(
        state,
        session,
        Some(user),
        if id.is_some() {
            "Termin bearbeiten"
        } else {
            "Termin anlegen"
        },
        "/arbeit/termine",
    )
    .await?
    .form();
    let invalid = !errors.is_empty();
    let response = render(&EventEditPage {
        layout,
        form,
        errors,
        action: id.map_or_else(
            || "/arbeit/termine".to_string(),
            |id| format!("/arbeit/termine/{id}/bearbeiten"),
        ),
        version,
        boats,
        id: id.unwrap_or("").to_string(),
    })?;
    Ok(if invalid {
        unprocessable(response)
    } else {
        response
    })
}

pub async fn new_event(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
) -> Result<Response, WebError> {
    state
        .club
        .principal(&user)
        .await?
        .require(&Capability::ManageWorkEvents)?;
    event_edit_page(
        &state,
        &session,
        &user,
        None,
        EventInput::default(),
        0,
        FieldErrors::new(),
    )
    .await
}

pub async fn create_event(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Form(fields): Form<Fields>,
) -> Result<Response, WebError> {
    let input = event_input(&fields);
    match state.club.create_event(&user, &input).await {
        Ok(id) => Ok(redirect_with_flash(
            &session,
            &format!("/arbeit/termine/{id}"),
            "Der Termin wurde angelegt.",
        )
        .await),
        Err(error) => {
            let errors = errors_from(error)?;
            event_edit_page(&state, &session, &user, None, input, 0, errors).await
        }
    }
}

pub async fn edit_event(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Path(id): Path<String>,
) -> Result<Response, WebError> {
    let (form, version) = state
        .club
        .event_input(&user, parse_id::<WorkEventId>(&id, "Arbeitstermin")?)
        .await?;
    event_edit_page(
        &state,
        &session,
        &user,
        Some(&id),
        form,
        version,
        FieldErrors::new(),
    )
    .await
}

pub async fn edit_event_submit(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Path(id): Path<String>,
    Form(fields): Form<Fields>,
) -> Result<Response, WebError> {
    let event = parse_id::<WorkEventId>(&id, "Arbeitstermin")?;
    let input = event_input(&fields);
    let version = get(&fields, "version").parse().unwrap_or(0);
    match state.club.update_event(&user, event, version, &input).await {
        Ok(()) => Ok(redirect_with_flash(
            &session,
            &format!("/arbeit/termine/{id}"),
            "Der Termin wurde geändert.",
        )
        .await),
        Err(error) => {
            let errors = errors_from(error)?;
            event_edit_page(&state, &session, &user, Some(&id), input, version, errors).await
        }
    }
}

async fn event_result(
    session: &Session,
    id: &str,
    result: Result<(), AppError>,
    ok: &str,
) -> Result<Response, WebError> {
    let target = format!("/arbeit/termine/{id}");
    match result {
        Ok(()) => Ok(redirect_with_flash(session, &target, ok).await),
        Err(AppError::Rejected(list)) => {
            let message = list.first().map(|e| e.message.clone()).unwrap_or_default();
            flash(session, FlashKind::Error, message).await;
            Ok(Redirect::to(&target).into_response())
        }
        Err(other) => Err(other.into()),
    }
}

pub async fn register(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Path(id): Path<String>,
) -> Result<Response, WebError> {
    let result = state
        .club
        .register_event(&user, parse_id::<WorkEventId>(&id, "Arbeitstermin")?)
        .await;
    event_result(&session, &id, result, "Du bist angemeldet.").await
}

pub async fn unregister(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Path(id): Path<String>,
) -> Result<Response, WebError> {
    let result = state
        .club
        .unregister_event(&user, parse_id::<WorkEventId>(&id, "Arbeitstermin")?)
        .await;
    event_result(&session, &id, result, "Du bist abgemeldet.").await
}

// ---- Aufgaben --------------------------------------------------------------------------------

#[derive(Template)]
#[template(path = "work_tasks.html")]
struct TasksPage {
    layout: Layout,
    nav: WorkNav,
    v: TasksView,
    form: TaskInput,
    errors: FieldErrors,
}

async fn tasks_page(
    state: &AppState,
    session: &Session,
    user: &CurrentUser,
    form: TaskInput,
    errors: FieldErrors,
) -> Result<Response, WebError> {
    let v = state.club.tasks_view(user).await?;
    let layout = Layout::build(state, session, Some(user), "Aufgaben", "/arbeit/aufgaben").await?;
    let invalid = !errors.is_empty();
    let response = render(&TasksPage {
        nav: nav(state, user, "aufgaben").await?,
        layout,
        v,
        form,
        errors,
    })?;
    Ok(if invalid {
        unprocessable(response)
    } else {
        response
    })
}

pub async fn tasks(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
) -> Result<Response, WebError> {
    tasks_page(
        &state,
        &session,
        &user,
        TaskInput::default(),
        FieldErrors::new(),
    )
    .await
}

pub async fn create_task(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Form(fields): Form<Fields>,
) -> Result<Response, WebError> {
    let t = |n: &str| get(&fields, n).trim().to_string();
    let input = TaskInput {
        title: t("title"),
        description: t("description"),
        boat: t("boat"),
        due: t("due"),
        hours: t("hours"),
    };
    match state.club.create_task(&user, &input).await {
        Ok(_) => {
            Ok(
                redirect_with_flash(&session, "/arbeit/aufgaben", "Die Aufgabe wurde angelegt.")
                    .await,
            )
        }
        Err(error) => {
            let errors = errors_from(error)?;
            tasks_page(&state, &session, &user, input, errors).await
        }
    }
}

pub async fn assign_task(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Path(id): Path<String>,
    Form(fields): Form<Fields>,
) -> Result<Response, WebError> {
    let member = get(&fields, "member")
        .parse::<u32>()
        .ok()
        .and_then(MemberId::new);
    let Some(member) = member else {
        flash(&session, FlashKind::Error, "Bitte ein Mitglied wählen.").await;
        return Ok(Redirect::to("/arbeit/aufgaben").into_response());
    };
    match state
        .club
        .assign_task(&user, parse_id::<TaskId>(&id, "Aufgabe")?, member)
        .await
    {
        Ok(()) => Ok(redirect_with_flash(
            &session,
            "/arbeit/aufgaben",
            "Das Mitglied ist jetzt zuständig.",
        )
        .await),
        Err(AppError::Rejected(list)) => {
            flash(
                &session,
                FlashKind::Error,
                list.first().map(|e| e.message.clone()).unwrap_or_default(),
            )
            .await;
            Ok(Redirect::to("/arbeit/aufgaben").into_response())
        }
        Err(other) => Err(other.into()),
    }
}

pub async fn task_status(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Path(id): Path<String>,
    Form(fields): Form<Fields>,
) -> Result<Response, WebError> {
    let status = match get(&fields, "status") {
        "in_progress" => TaskStatus::InProgress,
        "done" => TaskStatus::Done,
        _ => TaskStatus::Open,
    };
    state
        .club
        .set_task_status(&user, parse_id::<TaskId>(&id, "Aufgabe")?, status)
        .await?;
    Ok(redirect_with_flash(&session, "/arbeit/aufgaben", "Der Status wurde gesetzt.").await)
}

// ---- Übersicht, Übertrag, Abgeltung (Vorstand, Revisoren) -------------------------------------

#[derive(Template)]
#[template(path = "work_hours.html")]
struct HoursPage {
    layout: Layout,
    nav: WorkNav,
    v: HoursOverview,
    deficit: bool,
    errors: FieldErrors,
    sort: SortCtx,
}

async fn hours_page(
    state: &AppState,
    session: &Session,
    user: &CurrentUser,
    year: Option<i32>,
    deficit: bool,
    errors: FieldErrors,
    sort_state: Sort,
) -> Result<Response, WebError> {
    let mut v = state.club.hours_overview(user, year, deficit).await?;
    sort_rows(
        &mut v.rows,
        &sort_state,
        &[
            ("name", |r| text_key(&r.name)),
            ("soll", |r| num_key(&r.required)),
            ("angerechnet", |r| num_key(&r.credited)),
            ("stand", |r| SortKey::Num(i64::from(r.missing.is_some()))),
        ],
    );
    let sort = SortCtx::new(
        "alle",
        "/arbeit/uebersicht",
        vec![
            (
                "jahr".to_string(),
                year.map(|y| y.to_string()).unwrap_or_default(),
            ),
            (
                "rueckstand".to_string(),
                if deficit { "1".into() } else { String::new() },
            ),
        ],
        &sort_state,
        &[
            ("name", "Mitglied"),
            ("soll", "Soll"),
            ("angerechnet", "Angerechnet"),
            ("stand", "Stand"),
        ],
    );
    let layout = Layout::build(
        state,
        session,
        Some(user),
        "Stunden aller Mitglieder",
        "/arbeit/uebersicht",
    )
    .await?;
    let invalid = !errors.is_empty();
    let response = render(&HoursPage {
        nav: nav(state, user, "uebersicht").await?,
        layout,
        v,
        deficit,
        errors,
        sort,
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
    Query(query): Query<YearQuery>,
) -> Result<Response, WebError> {
    hours_page(
        &state,
        &session,
        &user,
        query.jahr,
        query.rueckstand.is_some(),
        FieldErrors::new(),
        Sort {
            key: query.sort_alle.clone(),
            desc: query.dir_alle == "desc",
        },
    )
    .await
}

pub async fn transfer(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Form(fields): Form<Fields>,
) -> Result<Response, WebError> {
    let member = |n: &str| get(&fields, n).parse::<u32>().ok().and_then(MemberId::new);
    let year = get(&fields, "year").parse::<i32>().ok();
    let (Some(from), Some(to), Some(year)) = (member("from"), member("to"), year) else {
        return Err(AppError::NotFound("Mitglied".into()).into());
    };
    match state
        .club
        .transfer_hours(
            &user,
            from,
            to,
            year,
            get(&fields, "hours"),
            get(&fields, "description"),
        )
        .await
    {
        Ok(()) => Ok(redirect_with_flash(
            &session,
            &format!("/arbeit/uebersicht?jahr={year}"),
            "Der Übertrag wurde erfasst.",
        )
        .await),
        Err(error) => {
            let mut errors = errors_from(error)?;
            if errors.get("hours").is_empty() && errors.get("to").is_empty() {
                errors.add_general("Der Übertrag wurde nicht angenommen.");
            }
            hours_page(
                &state,
                &session,
                &user,
                Some(year),
                false,
                errors,
                Sort::default(),
            )
            .await
        }
    }
}

pub async fn compensate(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Form(fields): Form<Fields>,
) -> Result<Response, WebError> {
    let member = get(&fields, "member")
        .parse::<u32>()
        .ok()
        .and_then(MemberId::new);
    let year = get(&fields, "year").parse::<i32>().ok();
    let (Some(member), Some(year)) = (member, year) else {
        return Err(AppError::NotFound("Mitglied".into()).into());
    };
    let mut errors = FieldErrors::new();
    let amount =
        non_empty(get(&fields, "amount")).and_then(|a| errors.take("amount", parse_money(a)));
    if errors.is_empty() {
        match state
            .club
            .approve_compensation(
                &user,
                member,
                year,
                get(&fields, "comp_hours"),
                amount,
                get(&fields, "reason"),
            )
            .await
        {
            Ok(total) => {
                return Ok(redirect_with_flash(
                    &session,
                    &format!("/arbeit/uebersicht?jahr={year}"),
                    &format!(
                        "Die Abgeltung wurde genehmigt; die Forderung über {total} liegt im Konto."
                    ),
                )
                .await);
            }
            Err(error) => {
                let mut found = errors_from(error)?;
                if found.get("comp_hours").is_empty() && !found.get("hours").is_empty() {
                    found.add("comp_hours", found.get("hours"));
                }
                errors = found;
            }
        }
    }
    hours_page(
        &state,
        &session,
        &user,
        Some(year),
        false,
        errors,
        Sort::default(),
    )
    .await
}
