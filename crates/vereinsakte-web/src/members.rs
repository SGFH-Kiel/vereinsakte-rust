//! Seiten und Formulare rund um Mitglieder: Mitgliedsseite, Bearbeiten,
//! Aufnehmen, Austritt sowie Segelscheine und Funklizenzen.

use std::collections::HashMap;

use askama::Template;
use axum::Form;
use axum::extract::{Path, State};
use axum::response::{Redirect, Response};
use tower_sessions::Session;
use vereinsakte_app::members::{
    ContactInput, LicenseKind, MemberForm, MemberInput, MemberPage, membership_type_key,
    membership_types, parse_membership_type,
};
use vereinsakte_app::{AppError, CurrentUser};
use vereinsakte_core::ids::MemberId;

use crate::error::WebError;
use crate::forms::{FieldErrors, non_empty, parse_date, redirect_with_flash, unprocessable};
use crate::pages::{Layout, render};
use crate::session::RequireUser;
use crate::state::AppState;

type Fields = HashMap<String, String>;

fn text<'a>(fields: &'a Fields, name: &str) -> &'a str {
    fields.get(name).map_or("", String::as_str)
}

fn checked(fields: &Fields, name: &str) -> bool {
    fields.contains_key(name)
}

fn member_id(number: u32) -> Result<MemberId, WebError> {
    MemberId::new(number).ok_or_else(|| AppError::NotFound(format!("Mitglied {number}")).into())
}

/// Feldwerte eines Formulars (aus der Datenbank oder aus der abgeschickten Eingabe).
#[derive(Default)]
pub struct Values {
    pub number: u32,
    pub number_input: String,
    pub version: i64,
    pub name: String,
    pub first_name: String,
    pub last_name: String,
    pub street: String,
    pub postal_code: String,
    pub city: String,
    pub landline: String,
    pub business: String,
    pub mobile: String,
    pub email: String,
    pub login_email: String,
    pub membership_type: String,
    pub board_consent: bool,
    pub address_valid: bool,
    pub child_of: String,
    pub joined: String,
}

impl Values {
    fn from_form(f: &MemberForm) -> Self {
        Self {
            number: f.id,
            version: f.version,
            name: f.name.clone(),
            first_name: f.first_name.clone(),
            last_name: f.last_name.clone(),
            street: f.contact.street.clone(),
            postal_code: f.contact.postal_code.clone(),
            city: f.contact.city.clone(),
            landline: f.contact.landline.clone(),
            business: f.contact.business.clone(),
            mobile: f.contact.mobile.clone(),
            email: f.contact.email.clone(),
            login_email: f.login_email.clone(),
            membership_type: membership_type_key(f.membership_type).into(),
            board_consent: f.board_consent,
            address_valid: f.address_valid,
            child_of: f.child_of.map(|n| n.to_string()).unwrap_or_default(),
            ..Self::default()
        }
    }

    /// Eingaben aus dem abgeschickten Formular; fehlende Felder bleiben leer.
    fn from_fields(fields: &Fields, number: u32, name: &str) -> Self {
        let t = |n: &str| text(fields, n).trim().to_string();
        Self {
            number,
            number_input: t("number"),
            version: t("version").parse().unwrap_or(0),
            name: name.to_string(),
            first_name: t("first_name"),
            last_name: t("last_name"),
            street: t("street"),
            postal_code: t("postal_code"),
            city: t("city"),
            landline: t("landline"),
            business: t("business"),
            mobile: t("mobile"),
            email: t("email"),
            login_email: t("login_email"),
            membership_type: t("membership_type"),
            board_consent: checked(fields, "board_consent"),
            address_valid: checked(fields, "address_valid") || !fields.contains_key("first_name"),
            child_of: t("child_of"),
            joined: t("joined"),
        }
    }

    fn contact(&self) -> ContactInput {
        ContactInput {
            street: self.street.clone(),
            postal_code: self.postal_code.clone(),
            city: self.city.clone(),
            landline: self.landline.clone(),
            business: self.business.clone(),
            mobile: self.mobile.clone(),
            email: self.email.clone(),
        }
    }
}

pub struct LicenseView {
    pub kind_key: &'static str,
    pub index: usize,
    pub name: String,
    pub type_label: &'static str,
    pub acquired: Option<String>,
}

#[derive(Template)]
#[template(path = "member.html")]
struct MemberPageTemplate {
    layout: Layout,
    p: MemberPage,
}

#[derive(Template)]
#[template(path = "member_form.html")]
struct MemberFormTemplate {
    layout: Layout,
    values: Values,
    errors: FieldErrors,
    is_new: bool,
    full: bool,
    action: String,
    types: Vec<(&'static str, String)>,
    licenses: Vec<LicenseView>,
    license_name: String,
    license_error: String,
}

fn field_errors(error: AppError) -> Result<FieldErrors, WebError> {
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

fn license_views(form: &MemberForm) -> Vec<LicenseView> {
    form.licenses
        .iter()
        .map(|l| LicenseView {
            kind_key: match l.kind {
                LicenseKind::Sailing => "sailing",
                LicenseKind::Radio => "radio",
            },
            index: l.index,
            name: l.name.clone(),
            type_label: match l.kind {
                LicenseKind::Sailing => "Segelschein",
                LicenseKind::Radio => "Funklizenz",
            },
            acquired: l.acquired.clone(),
        })
        .collect()
}

pub async fn show(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Path(number): Path<u32>,
) -> Result<Response, WebError> {
    let p = state.club.member_page(&user, member_id(number)?).await?;
    let layout = Layout::build(
        &state,
        &session,
        Some(&user),
        &p.name.clone(),
        &format!("/mitglieder/{number}"),
    )
    .await?;
    render(&MemberPageTemplate { layout, p })
}

/// `/meine-daten/bearbeiten` führt zur eigenen Bearbeitungsseite.
pub async fn edit_own(RequireUser(user): RequireUser) -> Redirect {
    Redirect::to(&format!("/mitglieder/{}/bearbeiten", user.member.get()))
}

async fn edit_page(
    state: &AppState,
    session: &Session,
    user: &CurrentUser,
    number: u32,
    values: Option<Values>,
    errors: FieldErrors,
    license_error: (String, String),
) -> Result<Response, WebError> {
    let id = member_id(number)?;
    let form = state.club.member_form(user, id).await?;
    let principal = state.club.principal(user).await?;
    let full = principal.can(&vereinsakte_core::rights::Capability::ManageMembers);
    let values = values.unwrap_or_else(|| Values::from_form(&form));
    let layout = Layout::build(
        state,
        session,
        Some(user),
        &format!("{} bearbeiten", form.name),
        &format!("/mitglieder/{number}/bearbeiten"),
    )
    .await?;
    let status_invalid = !errors.is_empty() || !license_error.1.is_empty();
    let page = MemberFormTemplate {
        layout,
        values,
        errors,
        is_new: false,
        full,
        action: format!("/mitglieder/{number}/bearbeiten"),
        types: membership_types(),
        licenses: license_views(&form),
        license_name: license_error.0,
        license_error: license_error.1,
    };
    let response = render(&page)?;
    Ok(if status_invalid {
        unprocessable(response)
    } else {
        response
    })
}

pub async fn edit(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Path(number): Path<u32>,
) -> Result<Response, WebError> {
    edit_page(
        &state,
        &session,
        &user,
        number,
        None,
        FieldErrors::new(),
        Default::default(),
    )
    .await
}

pub async fn edit_submit(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Path(number): Path<u32>,
    Form(fields): Form<Fields>,
) -> Result<Response, WebError> {
    let id = member_id(number)?;
    let principal = state.club.principal(&user).await?;
    let full = principal.can(&vereinsakte_core::rights::Capability::ManageMembers);
    let version = text(&fields, "version").parse().unwrap_or(0);
    let current = state.club.member_form(&user, id).await?;
    let values = Values::from_fields(&fields, number, &current.name);
    let result = if full {
        state
            .club
            .update_member(&user, id, version, &member_input(&values, None, None))
            .await
    } else {
        state
            .club
            .update_contact(&user, id, version, &values.contact())
            .await
    };
    match result {
        Ok(()) => Ok(redirect_with_flash(
            &session,
            &format!("/mitglieder/{number}"),
            "Die Änderungen wurden gespeichert.",
        )
        .await),
        Err(error) => {
            let errors = field_errors(error)?;
            edit_page(
                &state,
                &session,
                &user,
                number,
                Some(values),
                errors,
                Default::default(),
            )
            .await
        }
    }
}

fn member_input(
    values: &Values,
    number: Option<u32>,
    joined: Option<chrono::NaiveDate>,
) -> MemberInput {
    MemberInput {
        number,
        first_name: values.first_name.clone(),
        last_name: values.last_name.clone(),
        contact: values.contact(),
        login_email: values.login_email.clone(),
        membership_type: parse_membership_type(&values.membership_type)
            .unwrap_or(vereinsakte_core::member::MembershipType::Regular),
        board_consent: values.board_consent,
        address_valid: values.address_valid,
        child_of: values.child_of.parse().ok(),
        joined,
    }
}

async fn new_page(
    state: &AppState,
    session: &Session,
    user: &CurrentUser,
    values: Values,
    errors: FieldErrors,
) -> Result<Response, WebError> {
    let layout = Layout::build(
        state,
        session,
        Some(user),
        "Mitglied aufnehmen",
        "/mitglieder/neu",
    )
    .await?;
    let invalid = !errors.is_empty();
    let page = MemberFormTemplate {
        layout,
        values,
        errors,
        is_new: true,
        full: true,
        action: "/mitglieder/neu".into(),
        types: membership_types(),
        licenses: vec![],
        license_name: String::new(),
        license_error: String::new(),
    };
    let response = render(&page)?;
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
        .require(&vereinsakte_core::rights::Capability::ManageMembers)?;
    let values = Values {
        membership_type: "regular".into(),
        address_valid: true,
        ..Values::default()
    };
    new_page(&state, &session, &user, values, FieldErrors::new()).await
}

pub async fn new_submit(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Form(fields): Form<Fields>,
) -> Result<Response, WebError> {
    let mut values = Values::from_fields(&fields, 0, "");
    let mut errors = FieldErrors::new();
    let number = match non_empty(&values.number_input) {
        None => None,
        Some(n) => errors.take(
            "number",
            n.parse::<u32>()
                .map_err(|_| "Bitte eine ganze Zahl angeben.".to_string()),
        ),
    };
    let joined = non_empty(&values.joined).and_then(|d| errors.take("joined", parse_date(d)));
    values.address_valid = true;
    if errors.is_empty() {
        match state
            .club
            .create_member(&user, &member_input(&values, number, joined))
            .await
        {
            Ok(id) => {
                return Ok(redirect_with_flash(
                    &session,
                    &format!("/mitglieder/{}", id.get()),
                    "Das Mitglied wurde aufgenommen.",
                )
                .await);
            }
            Err(error) => errors = field_errors(error)?,
        }
    }
    new_page(&state, &session, &user, values, errors).await
}

pub async fn end_membership(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Path(number): Path<u32>,
    Form(fields): Form<Fields>,
) -> Result<Response, WebError> {
    let id = member_id(number)?;
    let version = text(&fields, "version").parse().unwrap_or(0);
    let mut errors = FieldErrors::new();
    let date = errors.take("date", parse_date(text(&fields, "date")));
    if let Some(date) = date {
        match state
            .club
            .end_membership(&user, id, version, date, checked(&fields, "expelled"))
            .await
        {
            Ok(()) => {
                return Ok(redirect_with_flash(
                    &session,
                    &format!("/mitglieder/{number}"),
                    "Die Mitgliedschaft wurde beendet.",
                )
                .await);
            }
            Err(error) => errors = field_errors(error)?,
        }
    }
    let name = state.club.member_form(&user, id).await?.name;
    let values = Values::from_fields(&fields, number, &name);
    edit_page(
        &state,
        &session,
        &user,
        number,
        Some(values),
        errors,
        Default::default(),
    )
    .await
}

pub async fn add_license(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Path(number): Path<u32>,
    Form(fields): Form<Fields>,
) -> Result<Response, WebError> {
    let id = member_id(number)?;
    let kind = if text(&fields, "kind") == "radio" {
        LicenseKind::Radio
    } else {
        LicenseKind::Sailing
    };
    let acquired = match non_empty(text(&fields, "acquired")) {
        None => None,
        Some(d) => match parse_date(d) {
            Ok(date) => Some(date),
            Err(message) => {
                return edit_page(
                    &state,
                    &session,
                    &user,
                    number,
                    None,
                    FieldErrors::new(),
                    (text(&fields, "name").to_string(), message),
                )
                .await;
            }
        },
    };
    match state
        .club
        .add_license(&user, id, kind, text(&fields, "name"), acquired)
        .await
    {
        Ok(()) => Ok(redirect_with_flash(
            &session,
            &format!("/mitglieder/{number}/bearbeiten"),
            "Der Eintrag wurde hinzugefügt.",
        )
        .await),
        Err(AppError::Rejected(list)) => {
            let message = list.first().map(|e| e.message.clone()).unwrap_or_default();
            edit_page(
                &state,
                &session,
                &user,
                number,
                None,
                FieldErrors::new(),
                (text(&fields, "name").to_string(), message),
            )
            .await
        }
        Err(other) => Err(other.into()),
    }
}

pub async fn remove_license(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Path((number, kind, index)): Path<(u32, String, usize)>,
) -> Result<Response, WebError> {
    let kind = if kind == "radio" {
        LicenseKind::Radio
    } else {
        LicenseKind::Sailing
    };
    state
        .club
        .remove_license(&user, member_id(number)?, kind, index)
        .await?;
    Ok(redirect_with_flash(
        &session,
        &format!("/mitglieder/{number}/bearbeiten"),
        "Der Eintrag wurde entfernt.",
    )
    .await)
}
