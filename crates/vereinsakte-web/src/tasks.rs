//! Aufgabenboard: Board, Aufgabenseite, Beschreibung, Kommentare, Anhänge.

use std::collections::HashMap;

use askama::Template;
use axum::Form;
use axum::extract::{Multipart, Path, Query, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Redirect, Response};
use tower_sessions::Session;
use uuid::Uuid;
use vereinsakte_app::tasks::{
    BoardFilter, BoardView, COMMENT_WINDOW_MINUTES, FORMAT_ERROR, MAX_ATTACHMENT_BYTES, TaskPage,
    parse_status,
};
use vereinsakte_app::work::TaskInput;
use vereinsakte_app::{AppError, CurrentUser};
use vereinsakte_core::ids::{BoatId, MemberId, TaskId};
use vereinsakte_core::task::AttachmentKind;

use crate::error::WebError;
use crate::forms::{FieldErrors, errors_from, redirect_with_flash, unprocessable};
use crate::markdown;
use crate::pages::{Layout, render};
use crate::session::{FlashKind, RequireUser, flash};
use crate::state::AppState;

type Fields = HashMap<String, String>;

fn get<'a>(fields: &'a Fields, name: &str) -> &'a str {
    fields.get(name).map_or("", String::as_str)
}

fn task_id(text: &str) -> Result<TaskId, WebError> {
    text.parse()
        .map_err(|_| AppError::NotFound("Aufgabe".into()).into())
}

fn uuid(text: &str, what: &str) -> Result<Uuid, WebError> {
    text.parse()
        .map_err(|_| AppError::NotFound(what.into()).into())
}

// ---- Board ----------------------------------------------------------------------------------

#[derive(Template)]
#[template(path = "tasks_board.html")]
struct BoardPage {
    layout: Layout,
    v: BoardView,
    can_archive: bool,
}

/// Die frühere Aufgabenliste ist das Board.
pub async fn old_list() -> Redirect {
    Redirect::to("/aufgaben")
}

pub async fn board(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Query(params): Query<Fields>,
) -> Result<Response, WebError> {
    let filter = BoardFilter {
        mine: params.get("meine").is_some_and(|v| v == "1"),
        boat: params.get("boot").and_then(|b| b.parse::<BoatId>().ok()),
    };
    let v = state.club.board(&user, &filter).await?;
    let principal = state.club.principal(&user).await?;
    let layout = Layout::build(&state, &session, Some(&user), "Aufgaben", "/aufgaben").await?;
    render(&BoardPage {
        layout,
        v,
        can_archive: principal.can(&vereinsakte_core::rights::Capability::ReadArchive),
    })
}

// ---- Neue Aufgabe ---------------------------------------------------------------------------

#[derive(Template)]
#[template(path = "task_new.html")]
struct NewPage {
    layout: Layout,
    form: TaskInput,
    errors: FieldErrors,
    boats: Vec<(String, String)>,
}

fn task_input(fields: &Fields) -> TaskInput {
    let t = |n: &str| get(fields, n).trim().to_string();
    TaskInput {
        title: t("title"),
        description: get(fields, "description").trim().to_string(),
        boat: t("boat"),
        due: t("due"),
        hours: t("hours"),
    }
}

async fn new_page(
    state: &AppState,
    session: &Session,
    user: &CurrentUser,
    form: TaskInput,
    errors: FieldErrors,
) -> Result<Response, WebError> {
    let board = state.club.board(user, &BoardFilter::default()).await?;
    if !board.can_manage {
        return Err(AppError::Forbidden.into());
    }
    let layout = Layout::build(state, session, Some(user), "Neue Aufgabe", "/aufgaben/neu")
        .await?
        .form();
    let invalid = !errors.is_empty();
    let response = render(&NewPage {
        layout,
        form,
        errors,
        boats: board.boats,
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
) -> Result<Response, WebError> {
    new_page(
        &state,
        &session,
        &user,
        TaskInput::default(),
        FieldErrors::new(),
    )
    .await
}

pub async fn new_submit(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Form(fields): Form<Fields>,
) -> Result<Response, WebError> {
    let input = task_input(&fields);
    match state.club.create_task(&user, &input).await {
        Ok(id) => Ok(redirect_with_flash(
            &session,
            &format!("/aufgaben/{id}"),
            "Die Aufgabe wurde angelegt.",
        )
        .await),
        Err(AppError::Forbidden) => Err(AppError::Forbidden.into()),
        Err(error) => {
            let errors = errors_from(error)?;
            new_page(&state, &session, &user, input, errors).await
        }
    }
}

// ---- Aufgabenseite --------------------------------------------------------------------------

/// Zustand der Seite, den ein fehlgeschlagener oder Vorschau-Aufruf mitbringt.
#[derive(Default)]
struct PageState {
    errors: FieldErrors,
    /// Entwurf der Beschreibung (Vorschau oder Fehler) – öffnet das Bearbeitungsfeld.
    description_draft: Option<String>,
    comment_draft: String,
    form: Option<TaskInput>,
}

#[derive(Template)]
#[template(path = "task.html")]
struct TaskTemplate {
    layout: Layout,
    p: TaskPage,
    description_html: String,
    comments_html: Vec<String>,
    preview_html: String,
    draft: String,
    editing: bool,
    comment_draft: String,
    form: TaskInput,
    errors: FieldErrors,
    window: i64,
    max_mb: usize,
    format_hint: &'static str,
}

async fn task_page(
    state: &AppState,
    session: &Session,
    user: &CurrentUser,
    id: &str,
    page_state: PageState,
) -> Result<Response, WebError> {
    let p = state.club.task_page(user, task_id(id)?).await?;
    let layout = Layout::build(
        state,
        session,
        Some(user),
        &p.title,
        &format!("/aufgaben/{id}"),
    )
    .await?
    .form();
    let editing = page_state.description_draft.is_some();
    let draft = page_state
        .description_draft
        .clone()
        .unwrap_or_else(|| p.description.clone());
    let preview_html = if editing {
        markdown::render(&draft)
    } else {
        String::new()
    };
    let form = page_state.form.unwrap_or_else(|| TaskInput {
        title: p.title.clone(),
        description: String::new(),
        boat: p.boat_id.clone(),
        due: p.due_input.clone(),
        hours: p.estimated_input.clone(),
    });
    let invalid = !page_state.errors.is_empty();
    let response = render(&TaskTemplate {
        description_html: markdown::render(&p.description),
        comments_html: p
            .comments
            .iter()
            .map(|c| markdown::render(&c.text))
            .collect(),
        layout,
        p,
        preview_html,
        draft,
        editing,
        comment_draft: page_state.comment_draft,
        form,
        errors: page_state.errors,
        window: COMMENT_WINDOW_MINUTES,
        max_mb: MAX_ATTACHMENT_BYTES / 1_000_000,
        format_hint: FORMAT_ERROR,
    })?;
    Ok(if invalid {
        unprocessable(response)
    } else {
        response
    })
}

pub async fn show(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Path(id): Path<String>,
) -> Result<Response, WebError> {
    task_page(&state, &session, &user, &id, PageState::default()).await
}

fn version_of(fields: &Fields) -> Option<i64> {
    get(fields, "version").parse().ok()
}

/// Verschieben in eine andere Spalte (Formular an der Karte, Aufgabenseite, Ziehen).
pub async fn move_status(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Path(id): Path<String>,
    Form(fields): Form<Fields>,
) -> Result<Response, WebError> {
    let status = parse_status(get(&fields, "status"))
        .ok_or_else(|| WebError::Status(StatusCode::BAD_REQUEST, "Unbekannter Status.".into()))?;
    state
        .club
        .move_task(&user, task_id(&id)?, status, version_of(&fields))
        .await?;
    if get(&fields, "ajax") == "1" {
        return Ok(StatusCode::NO_CONTENT.into_response());
    }
    let to = if get(&fields, "next") == "board" {
        "/aufgaben".to_string()
    } else {
        format!("/aufgaben/{id}")
    };
    Ok(redirect_with_flash(&session, &to, "Die Aufgabe wurde verschoben.").await)
}

pub async fn edit(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Path(id): Path<String>,
    Form(fields): Form<Fields>,
) -> Result<Response, WebError> {
    let input = task_input(&fields);
    match state
        .club
        .edit_task(&user, task_id(&id)?, &input, version_of(&fields))
        .await
    {
        Ok(()) => Ok(redirect_with_flash(
            &session,
            &format!("/aufgaben/{id}"),
            "Die Aufgabe wurde geändert.",
        )
        .await),
        Err(AppError::Rejected(list)) => {
            let mut errors = FieldErrors::new();
            for e in list {
                if e.field.is_empty() {
                    errors.add_general(e.message);
                } else {
                    errors.add(&e.field, e.message);
                }
            }
            task_page(
                &state,
                &session,
                &user,
                &id,
                PageState {
                    errors,
                    form: Some(input),
                    ..PageState::default()
                },
            )
            .await
        }
        Err(other) => Err(other.into()),
    }
}

pub async fn assign(
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
    let to = format!("/aufgaben/{id}");
    let Some(member) = member else {
        flash(&session, FlashKind::Error, "Bitte ein Mitglied wählen.").await;
        return Ok(Redirect::to(&to).into_response());
    };
    let result = if get(&fields, "action") == "entziehen" {
        state
            .club
            .unassign_task(&user, task_id(&id)?, member)
            .await
            .map(|()| "Die Zuständigkeit wurde entzogen.")
    } else {
        state
            .club
            .assign_task(&user, task_id(&id)?, member)
            .await
            .map(|()| "Das Mitglied ist jetzt zuständig.")
    };
    match result {
        Ok(text) => Ok(redirect_with_flash(&session, &to, text).await),
        Err(AppError::Rejected(list)) => {
            flash(
                &session,
                FlashKind::Error,
                list.first().map(|e| e.message.clone()).unwrap_or_default(),
            )
            .await;
            Ok(Redirect::to(&to).into_response())
        }
        Err(other) => Err(other.into()),
    }
}

/// Beschreibung: „Vorschau“ rendert ohne zu speichern, „Speichern“ speichert.
pub async fn description(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Path(id): Path<String>,
    Form(fields): Form<Fields>,
) -> Result<Response, WebError> {
    let text = get(&fields, "text").to_string();
    // Auch die Vorschau ist Zuständigen und Amtsträgern vorbehalten.
    let page = state.club.task_page(&user, task_id(&id)?).await?;
    if !page.can_edit {
        return Err(AppError::Forbidden.into());
    }
    if get(&fields, "action") == "preview" {
        return task_page(
            &state,
            &session,
            &user,
            &id,
            PageState {
                description_draft: Some(text),
                ..PageState::default()
            },
        )
        .await;
    }
    match state
        .club
        .set_task_description(&user, task_id(&id)?, &text, version_of(&fields))
        .await
    {
        Ok(()) => Ok(redirect_with_flash(
            &session,
            &format!("/aufgaben/{id}"),
            "Die Beschreibung wurde gespeichert.",
        )
        .await),
        Err(error) => {
            let errors = errors_from(error)?;
            task_page(
                &state,
                &session,
                &user,
                &id,
                PageState {
                    errors,
                    description_draft: Some(text),
                    ..PageState::default()
                },
            )
            .await
        }
    }
}

pub async fn comment(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Path(id): Path<String>,
    Form(fields): Form<Fields>,
) -> Result<Response, WebError> {
    let text = get(&fields, "text").to_string();
    match state.club.add_comment(&user, task_id(&id)?, &text).await {
        Ok(()) => Ok(redirect_with_flash(
            &session,
            &format!("/aufgaben/{id}#kommentare"),
            "Der Kommentar wurde gespeichert.",
        )
        .await),
        Err(error) => {
            let errors = errors_from(error)?;
            task_page(
                &state,
                &session,
                &user,
                &id,
                PageState {
                    errors,
                    comment_draft: text,
                    ..PageState::default()
                },
            )
            .await
        }
    }
}

pub async fn comment_edit(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Path(cid): Path<String>,
    Form(fields): Form<Fields>,
) -> Result<Response, WebError> {
    let cid = uuid(&cid, "Kommentar")?;
    match state
        .club
        .edit_comment(&user, cid, get(&fields, "text"))
        .await
    {
        Ok(task) => Ok(redirect_with_flash(
            &session,
            &format!("/aufgaben/{task}#kommentare"),
            "Der Kommentar wurde korrigiert.",
        )
        .await),
        Err(AppError::Rejected(list)) => {
            flash(
                &session,
                FlashKind::Error,
                list.first().map(|e| e.message.clone()).unwrap_or_default(),
            )
            .await;
            Ok(Redirect::to("/aufgaben").into_response())
        }
        Err(other) => Err(other.into()),
    }
}

pub async fn comment_delete(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Path(cid): Path<String>,
) -> Result<Response, WebError> {
    let task = state
        .club
        .delete_comment(&user, uuid(&cid, "Kommentar")?)
        .await?;
    Ok(redirect_with_flash(
        &session,
        &format!("/aufgaben/{task}#kommentare"),
        "Der Kommentar wurde gelöscht.",
    )
    .await)
}

// ---- Anhänge --------------------------------------------------------------------------------

/// Mehrteil-Upload: Feld `file` (Datei) und `alt` (Beschreibung).
pub async fn attach(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Path(id): Path<String>,
    mut multipart: Multipart,
) -> Result<Response, WebError> {
    let mut file: Option<(String, Vec<u8>)> = None;
    let mut alt = String::new();
    let mut too_large = false;
    loop {
        let field = match multipart.next_field().await {
            Ok(Some(field)) => field,
            Ok(None) => break,
            Err(_) => {
                too_large = true;
                break;
            }
        };
        let name = field.name().unwrap_or_default().to_string();
        let filename = field.file_name().unwrap_or_default().to_string();
        match field.bytes().await {
            Ok(bytes) if name == "file" => file = Some((filename, bytes.to_vec())),
            Ok(bytes) if name == "alt" => alt = String::from_utf8_lossy(&bytes).to_string(),
            Ok(_) => {}
            Err(_) => {
                too_large = true;
                break;
            }
        }
    }
    let result = if too_large {
        Err(AppError::rejected(
            "file",
            format!(
                "Die Datei ist zu groß (höchstens {} MB).",
                MAX_ATTACHMENT_BYTES / 1_000_000
            ),
        ))
    } else {
        let (filename, data) = file.unwrap_or_default();
        state
            .club
            .add_attachment(&user, task_id(&id)?, &filename, data, &alt)
            .await
    };
    match result {
        Ok(()) => Ok(redirect_with_flash(
            &session,
            &format!("/aufgaben/{id}#anhaenge"),
            "Der Anhang wurde hochgeladen.",
        )
        .await),
        Err(error) => {
            let errors = errors_from(error)?;
            task_page(
                &state,
                &session,
                &user,
                &id,
                PageState {
                    errors,
                    ..PageState::default()
                },
            )
            .await
        }
    }
}

/// Dateiname für `Content-Disposition` nach RFC 6266/5987.
fn disposition(kind: &str, filename: &str) -> String {
    let ascii: String = filename
        .chars()
        .map(|c| {
            if c.is_ascii_graphic() || c == ' ' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let mut encoded = String::new();
    for byte in filename.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    format!("{kind}; filename=\"{ascii}\"; filename*=UTF-8''{encoded}")
}

/// Liefert einen Anhang aus: fester Typ, kein Erraten durch den Browser,
/// Sandbox; PDFs nur als Download.
pub async fn attachment(
    State(state): State<AppState>,
    RequireUser(user): RequireUser,
    Path(aid): Path<String>,
) -> Result<Response, WebError> {
    let (meta, data) = state
        .club
        .attachment_file(&user, uuid(&aid, "Anhang")?)
        .await?;
    let kind = if meta.kind == AttachmentKind::Pdf {
        "attachment"
    } else {
        "inline"
    };
    let mut response = data.into_response();
    let headers = response.headers_mut();
    let value = |text: &str| {
        HeaderValue::from_str(text).unwrap_or_else(|_| HeaderValue::from_static("download"))
    };
    headers.insert(header::CONTENT_TYPE, value(&meta.content_type));
    headers.insert(
        header::CONTENT_DISPOSITION,
        value(&disposition(kind, &meta.filename)),
    );
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static("default-src 'none'; sandbox"),
    );
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, max-age=3600"),
    );
    Ok(response)
}

pub async fn attachment_delete(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Path(aid): Path<String>,
) -> Result<Response, WebError> {
    let task = state
        .club
        .delete_attachment(&user, uuid(&aid, "Anhang")?)
        .await?;
    Ok(redirect_with_flash(
        &session,
        &format!("/aufgaben/{task}#anhaenge"),
        "Der Anhang wurde entfernt.",
    )
    .await)
}

#[cfg(test)]
mod tests {
    use super::disposition;

    #[test]
    fn download_names_are_encoded() {
        let value = disposition("attachment", "Angebot Segeltuch äöü.pdf");
        assert!(value.starts_with("attachment; filename=\"Angebot Segeltuch ___"));
        assert!(value.contains("filename*=UTF-8''Angebot%20Segeltuch%20%C3%A4%C3%B6%C3%BC.pdf"));
    }
}
