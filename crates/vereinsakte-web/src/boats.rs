//! Boote: Liste, Seite, Bearbeiten sowie Sperren, Freigabe, Abnahmen und Widerrufe.

use std::collections::HashMap;

use askama::Template;
use axum::Form;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Redirect, Response};
use tower_sessions::Session;
use vereinsakte_app::AppError;
use vereinsakte_app::boats::{BoatInput, BoatList, BoatPage};
use vereinsakte_core::ids::{BoatId, MemberId};

use crate::error::WebError;
use crate::forms::{FieldErrors, non_empty, parse_date, redirect_with_flash, unprocessable};
use crate::list::{Sort, SortCtx, SortKey, sort_rows, text_key};
use crate::pages::{Layout, render};
use crate::session::{FlashKind, RequireUser, flash};
use crate::state::AppState;

type Fields = HashMap<String, String>;

fn boat_id(text: &str) -> Result<BoatId, WebError> {
    text.parse()
        .map_err(|_| AppError::NotFound("Boot".into()).into())
}

fn get<'a>(fields: &'a Fields, name: &str) -> &'a str {
    fields.get(name).map_or("", String::as_str)
}

#[derive(Template)]
#[template(path = "boats.html")]
struct ListPage {
    layout: Layout,
    v: BoatList,
    sort: SortCtx,
}

#[derive(Template)]
#[template(path = "boat.html")]
struct ShowPage {
    layout: Layout,
    p: BoatPage,
    holder_sort: SortCtx,
    /// Geöffneter Bereich nach einem Fehler: „sperren“, „abnahme“ oder „widerruf“.
    open: String,
}

#[derive(Template)]
#[template(path = "boat_form.html")]
struct FormPage {
    layout: Layout,
    id: String,
    version: i64,
    values: BoatInput,
    errors: FieldErrors,
    is_new: bool,
    action: String,
}

pub async fn list(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Result<Response, WebError> {
    let mut v = state.club.boat_list(&user).await?;
    let sort = Sort::from_params(&params, "boote");
    sort_rows(
        &mut v.rows,
        &sort,
        &[
            ("name", |r| text_key(&r.name)),
            ("typ", |r| text_key(&r.type_tip)),
            ("zustand", |r| {
                SortKey::Num(match (&r.blocked, r.released) {
                    (Some(_), _) => 0,
                    (None, false) => 1,
                    (None, true) => 2,
                })
            }),
        ],
    );
    let ctx = SortCtx::new(
        "boote",
        "/boote",
        vec![],
        &sort,
        &[("name", "Boot"), ("typ", "Typ"), ("zustand", "Zustand")],
    );
    let layout = Layout::build(&state, &session, Some(&user), "Boote", "/boote").await?;
    render(&ListPage {
        layout,
        v,
        sort: ctx,
    })
}

pub async fn show(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Path(id): Path<String>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Response, WebError> {
    let open = params.get("offen").cloned().unwrap_or_default();
    let mut p = state.club.boat_page(&user, boat_id(&id)?).await?;
    let sort = Sort::from_params(&params, "bf");
    sort_rows(
        &mut p.holders,
        &sort,
        &[
            ("name", |e| text_key(&e.name)),
            ("art", |e| text_key(&e.membership_type)),
            ("email", |e| text_key(e.contact.email().unwrap_or(""))),
        ],
    );
    let holder_sort = SortCtx::new(
        "bf",
        &format!("/boote/{id}"),
        vec![],
        &sort,
        &[
            ("name", "Name"),
            ("art", "Mitgliedsart"),
            ("email", "E-Mail"),
        ],
    );
    let layout = Layout::build(
        &state,
        &session,
        Some(&user),
        &p.name.clone(),
        &format!("/boote/{id}"),
    )
    .await?;
    render(&ShowPage {
        layout,
        p,
        holder_sort,
        open,
    })
}

fn input_from(fields: &Fields) -> BoatInput {
    let t = |n: &str| get(fields, n).trim().to_string();
    BoatInput {
        name: t("name"),
        dinghy: get(fields, "boat_type") == "dinghy",
        class: t("class"),
        year_built: t("year_built"),
        length: t("length"),
        beam: t("beam"),
        draft: t("draft"),
        sail_area: t("sail_area"),
        berths: t("berths"),
        engine: t("engine"),
        mooring: t("mooring"),
        skipper_requirement: get(fields, "skipper_requirement").replace("\r\n", "\n"),
        home_min_members: t("home_min_members"),
        home_min_persons: t("home_min_persons"),
        home_co_skipper: fields.contains_key("home_co_skipper"),
        outside_min_members: t("outside_min_members"),
        outside_min_persons: t("outside_min_persons"),
        outside_co_skipper: fields.contains_key("outside_co_skipper"),
    }
}

fn errors_of(error: AppError) -> Result<FieldErrors, WebError> {
    match error {
        AppError::Rejected(list) => {
            let mut errors = FieldErrors::new();
            for e in list {
                if e.field.is_empty() {
                    errors.add_general(e.message);
                } else {
                    errors.add(&e.field, e.message);
                }
            }
            Ok(errors)
        }
        other => Err(other.into()),
    }
}

#[allow(clippy::too_many_arguments)] // Formularseite: Zustand, Sitzung, Benutzer und alle Anzeigewerte
async fn form_page(
    state: &AppState,
    session: &Session,
    user: &vereinsakte_app::CurrentUser,
    id: &str,
    version: i64,
    values: BoatInput,
    errors: FieldErrors,
    is_new: bool,
) -> Result<Response, WebError> {
    let layout = Layout::build(
        state,
        session,
        Some(user),
        if is_new {
            "Boot anlegen"
        } else {
            "Boot bearbeiten"
        },
        if is_new { "/boote/neu" } else { "/boote" },
    )
    .await?
    .form();
    let invalid = !errors.is_empty();
    let action = if is_new {
        "/boote/neu".to_string()
    } else {
        format!("/boote/{id}/bearbeiten")
    };
    let response = render(&FormPage {
        layout,
        id: id.into(),
        version,
        values,
        errors,
        is_new,
        action,
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
    state
        .club
        .principal(&user)
        .await?
        .require(&vereinsakte_core::rights::Capability::CreateBoat)?;
    let values = BoatInput {
        dinghy: true,
        ..Default::default()
    };
    form_page(
        &state,
        &session,
        &user,
        "",
        0,
        values,
        FieldErrors::new(),
        true,
    )
    .await
}

pub async fn new_submit(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Form(fields): Form<Fields>,
) -> Result<Response, WebError> {
    let values = input_from(&fields);
    match state.club.create_boat(&user, &values).await {
        Ok(id) => Ok(redirect_with_flash(
            &session,
            &format!("/boote/{id}"),
            "Das Boot wurde angelegt.",
        )
        .await),
        Err(error) => {
            let errors = errors_of(error)?;
            form_page(&state, &session, &user, "", 0, values, errors, true).await
        }
    }
}

pub async fn edit(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Path(id): Path<String>,
) -> Result<Response, WebError> {
    let form = state.club.boat_form(&user, boat_id(&id)?).await?;
    form_page(
        &state,
        &session,
        &user,
        &id,
        form.version,
        form.input,
        FieldErrors::new(),
        false,
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
    let boat = boat_id(&id)?;
    let version: i64 = get(&fields, "version").parse().unwrap_or(0);
    let values = input_from(&fields);
    match state.club.update_boat(&user, boat, version, &values).await {
        Ok(()) => Ok(redirect_with_flash(
            &session,
            &format!("/boote/{id}"),
            "Die Änderungen wurden gespeichert.",
        )
        .await),
        Err(error) => {
            let errors = errors_of(error)?;
            form_page(&state, &session, &user, &id, version, values, errors, false).await
        }
    }
}

/// Ergebnis einer Aktion auf der Bootsseite: Erfolg oder Fehlermeldung als Hinweis.
async fn done(
    session: &Session,
    id: &str,
    result: Result<(), AppError>,
    ok: &str,
) -> Result<Response, WebError> {
    done_in(session, id, result, ok, "").await
}

/// Wie [`done`]; bei Fehlern wird der betroffene Bereich (`panel`) geöffnet gezeigt.
async fn done_in(
    session: &Session,
    id: &str,
    result: Result<(), AppError>,
    ok: &str,
    panel: &str,
) -> Result<Response, WebError> {
    match result {
        Ok(()) => Ok(redirect_with_flash(session, &format!("/boote/{id}"), ok).await),
        Err(AppError::Rejected(list)) => {
            let message = list.first().map(|e| e.message.clone()).unwrap_or_default();
            flash(session, FlashKind::Error, message).await;
            let target = if panel.is_empty() {
                format!("/boote/{id}")
            } else {
                format!("/boote/{id}?offen={panel}")
            };
            Ok(Redirect::to(&target).into_response())
        }
        Err(other) => Err(other.into()),
    }
}

fn optional_date(fields: &Fields, name: &str) -> Result<Option<chrono::NaiveDate>, String> {
    non_empty(get(fields, name)).map(parse_date).transpose()
}

pub async fn block(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Path(id): Path<String>,
    Form(fields): Form<Fields>,
) -> Result<Response, WebError> {
    let boat = boat_id(&id)?;
    let (start, end) = match (
        optional_date(&fields, "start"),
        optional_date(&fields, "end"),
    ) {
        (Ok(s), Ok(e)) => (s, e),
        (Err(m), _) | (_, Err(m)) => {
            return done_in(&session, &id, Err(AppError::rejected("", m)), "", "sperren").await;
        }
    };
    let result = state
        .club
        .block_boat(&user, boat, get(&fields, "reason"), start, end)
        .await;
    done_in(&session, &id, result, "Das Boot wurde gesperrt.", "sperren").await
}

pub async fn lift(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Path(id): Path<String>,
) -> Result<Response, WebError> {
    let result = state.club.lift_block(&user, boat_id(&id)?).await;
    done(&session, &id, result, "Die Sperre wurde aufgehoben.").await
}

pub async fn release(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Path(id): Path<String>,
) -> Result<Response, WebError> {
    let result = state.club.release_boat(&user, boat_id(&id)?, None).await;
    done(
        &session,
        &id,
        result,
        "Das Boot wurde für die Saison freigegeben.",
    )
    .await
}

pub async fn checkout(
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
        return done_in(
            &session,
            &id,
            Err(AppError::rejected("member", "Bitte ein Mitglied wählen.")),
            "",
            "abnahme",
        )
        .await;
    };
    let passed = get(&fields, "result") != "failed";
    let result = state
        .club
        .record_checkout(&user, boat_id(&id)?, member, passed, get(&fields, "remark"))
        .await;
    done_in(
        &session,
        &id,
        result,
        "Die Abnahme wurde gespeichert.",
        "abnahme",
    )
    .await
}

pub async fn revoke(
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
        return done_in(
            &session,
            &id,
            Err(AppError::rejected("member", "Bitte ein Mitglied wählen.")),
            "",
            "widerruf",
        )
        .await;
    };
    let result = state
        .club
        .revoke_permit(&user, boat_id(&id)?, member, get(&fields, "reason"))
        .await;
    done_in(
        &session,
        &id,
        result,
        "Die Bootserlaubnis wurde widerrufen.",
        "widerruf",
    )
    .await
}

// ---- Bootsbild ------------------------------------------------------------------------------

/// Liefert das Bild mit festem Typ, `nosniff` und Sandbox-CSP (nur angemeldet).
pub async fn image(
    State(state): State<AppState>,
    RequireUser(user): RequireUser,
    Path(id): Path<String>,
) -> Result<Response, WebError> {
    use axum::http::{HeaderValue, header};
    let (meta, data) = state.club.boat_image_file(&user, boat_id(&id)?).await?;
    let mut response = data.into_response();
    let headers = response.headers_mut();
    if let Ok(value) = HeaderValue::from_str(&meta.content_type) {
        headers.insert(header::CONTENT_TYPE, value);
    }
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static("default-src 'none'; sandbox"),
    );
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    // Die Adresse trägt eine Versionszahl; Änderungen erzeugen eine neue Adresse.
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, max-age=86400"),
    );
    Ok(response)
}

/// Mehrteil-Upload: Felder `file` und `alt`; Fehler erscheinen als Hinweis auf der Bootsseite.
pub async fn image_upload(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Path(id): Path<String>,
    mut multipart: axum::extract::Multipart,
) -> Result<Response, WebError> {
    let boat = boat_id(&id)?;
    let (mut data, mut alt, mut too_large) = (Vec::new(), String::new(), false);
    loop {
        match multipart.next_field().await {
            Ok(Some(field)) => {
                let name = field.name().unwrap_or_default().to_string();
                match field.bytes().await {
                    Ok(bytes) if name == "file" => data = bytes.to_vec(),
                    Ok(bytes) if name == "alt" => {
                        alt = String::from_utf8_lossy(&bytes).to_string();
                    }
                    Ok(_) => {}
                    Err(_) => {
                        too_large = true;
                        break;
                    }
                }
            }
            Ok(None) => break,
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
                "Das Bild ist zu groß (höchstens {} MB).",
                vereinsakte_app::boat_images::MAX_BOAT_IMAGE_BYTES / (1024 * 1024)
            ),
        ))
    } else {
        state.club.set_boat_image(&user, boat, data, &alt).await
    };
    let to = format!("/boote/{id}");
    match result {
        Ok(()) => Ok(redirect_with_flash(&session, &to, "Das Bild wurde gespeichert.").await),
        Err(AppError::Rejected(list)) => {
            let message = list.first().map(|e| e.message.clone()).unwrap_or_default();
            flash(&session, FlashKind::Error, message).await;
            Ok(Redirect::to(&to).into_response())
        }
        Err(other) => Err(other.into()),
    }
}

pub async fn image_remove(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Path(id): Path<String>,
) -> Result<Response, WebError> {
    state.club.remove_boat_image(&user, boat_id(&id)?).await?;
    Ok(redirect_with_flash(
        &session,
        &format!("/boote/{id}"),
        "Das Bild wurde entfernt.",
    )
    .await)
}
