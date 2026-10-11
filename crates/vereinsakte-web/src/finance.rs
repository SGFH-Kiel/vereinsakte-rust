//! Finanzen: Konten, Buchungen, Außenstände, Saisons, Jahresbeiträge, Touren, Nachweis.

use std::collections::HashMap;

use askama::Template;
use axum::Form;
use axum::extract::{Path, Query, State};
use axum::response::{IntoResponse, Redirect, Response};
use chrono::NaiveDate;
use serde::Deserialize;
use tower_sessions::Session;
use vereinsakte_app::finance::{
    AccountView, AnnualFeePreview, AnnualProof, OutstandingView, SeasonsView, TourBilling,
};
use vereinsakte_app::{AppError, CurrentUser};
use vereinsakte_core::ids::{ClaimId, MemberId, ReservationId};

use crate::error::WebError;
use crate::forms::{
    FieldErrors, errors_from, non_empty, parse_date, parse_money, redirect_with_flash,
    unprocessable,
};
use crate::pages::{Layout, render};
use crate::session::RequireUser;
use crate::state::AppState;

type Fields = HashMap<String, String>;

fn get<'a>(fields: &'a Fields, name: &str) -> &'a str {
    fields.get(name).map_or("", String::as_str)
}

fn member_id(number: u32) -> Result<MemberId, WebError> {
    MemberId::new(number).ok_or_else(|| AppError::NotFound(format!("Mitglied {number}")).into())
}

// ---- Konten ----------------------------------------------------------------------------------

#[derive(Template)]
#[template(path = "account.html")]
struct AccountPage {
    layout: Layout,
    v: AccountView,
    errors: FieldErrors,
}

async fn account_page(
    state: &AppState,
    session: &Session,
    user: &CurrentUser,
    member: MemberId,
    errors: FieldErrors,
) -> Result<Response, WebError> {
    let v = state.club.account_view(user, member).await?;
    let path = if v.is_self {
        "/konto".to_string()
    } else {
        format!("/finanzen/konten/{}", member.get())
    };
    let layout = Layout::build(state, session, Some(user), "Konto", &path).await?;
    let invalid = !errors.is_empty();
    let response = render(&AccountPage { layout, v, errors })?;
    Ok(if invalid {
        unprocessable(response)
    } else {
        response
    })
}

pub async fn own_account(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
) -> Result<Response, WebError> {
    account_page(&state, &session, &user, user.member, FieldErrors::new()).await
}

pub async fn account(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Path(number): Path<u32>,
) -> Result<Response, WebError> {
    account_page(
        &state,
        &session,
        &user,
        member_id(number)?,
        FieldErrors::new(),
    )
    .await
}

fn back(number: u32) -> String {
    format!("/finanzen/konten/{number}")
}

pub async fn payment(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Path(number): Path<u32>,
    Form(fields): Form<Fields>,
) -> Result<Response, WebError> {
    let member = member_id(number)?;
    let mut errors = FieldErrors::new();
    let amount = errors.take("amount", parse_money(get(&fields, "amount")));
    let date = non_empty(get(&fields, "date")).and_then(|d| errors.take("date", parse_date(d)));
    if let (true, Some(amount)) = (errors.is_empty(), amount) {
        match state
            .club
            .post_payment(&user, member, amount, date, get(&fields, "text"))
            .await
        {
            Ok(()) => {
                return Ok(redirect_with_flash(
                    &session,
                    &back(number),
                    "Die Einzahlung wurde gebucht.",
                )
                .await);
            }
            Err(error) => errors = errors_from(error)?,
        }
    }
    account_page(&state, &session, &user, member, errors).await
}

pub async fn claim(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Path(number): Path<u32>,
    Form(fields): Form<Fields>,
) -> Result<Response, WebError> {
    let member = member_id(number)?;
    let mut errors = FieldErrors::new();
    let amount = errors.take("claim_amount", parse_money(get(&fields, "claim_amount")));
    let due = non_empty(get(&fields, "due")).and_then(|d| errors.take("due", parse_date(d)));
    if let (true, Some(amount)) = (errors.is_empty(), amount) {
        match state
            .club
            .post_claim(&user, member, amount, get(&fields, "claim_text"), due)
            .await
        {
            Ok(()) => {
                return Ok(redirect_with_flash(
                    &session,
                    &back(number),
                    "Die Forderung wurde angelegt.",
                )
                .await);
            }
            Err(error) => {
                let mapped = errors_from(error)?;
                errors = FieldErrors::new();
                for (field, key) in [("amount", "claim_amount"), ("text", "claim_text")] {
                    if !mapped.get(field).is_empty() {
                        errors.add(key, mapped.get(field));
                    }
                }
                for message in mapped.general() {
                    errors.add_general(message.clone());
                }
            }
        }
    }
    account_page(&state, &session, &user, member, errors).await
}

pub async fn cancel_claim(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Path((number, id)): Path<(u32, String)>,
    Form(fields): Form<Fields>,
) -> Result<Response, WebError> {
    let member = member_id(number)?;
    let claim: ClaimId = id
        .parse()
        .map_err(|_| AppError::NotFound("Forderung".into()))?;
    match state
        .club
        .cancel_claim(&user, member, claim, get(&fields, "reason"))
        .await
    {
        Ok(()) => {
            Ok(
                redirect_with_flash(&session, &back(number), "Die Forderung wurde storniert.")
                    .await,
            )
        }
        Err(error) => {
            let mut errors = errors_from(error)?;
            if errors.general().is_empty() {
                let message = errors.get("reason");
                errors.add_general(message);
            }
            account_page(&state, &session, &user, member, errors).await
        }
    }
}

// ---- Außenstände -----------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct OutstandingQuery {
    stichtag: Option<String>,
    ueberfaellig: Option<String>,
}

#[derive(Template)]
#[template(path = "finance.html")]
struct OutstandingPage {
    layout: Layout,
    v: OutstandingView,
    at: String,
    at_iso: String,
    nav: &'static str,
}

pub async fn outstanding(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Query(query): Query<OutstandingQuery>,
) -> Result<Response, WebError> {
    let date = query
        .stichtag
        .as_deref()
        .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok());
    let v = state
        .club
        .outstanding_view(&user, date, query.ueberfaellig.is_some())
        .await?;
    let layout = Layout::build(&state, &session, Some(&user), "Finanzen", "/finanzen").await?;
    render(&OutstandingPage {
        layout,
        at: v.at.format("%d.%m.%Y").to_string(),
        at_iso: v.at.to_string(),
        nav: "aussenstaende",
        v,
    })
}

// ---- Saisons ---------------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct YearQuery {
    jahr: Option<i32>,
}

#[derive(Template)]
#[template(path = "finance_seasons.html")]
struct SeasonsPage {
    layout: Layout,
    v: SeasonsView,
    errors: FieldErrors,
    nav: &'static str,
}

async fn seasons_page(
    state: &AppState,
    session: &Session,
    user: &CurrentUser,
    year: Option<i32>,
    errors: FieldErrors,
) -> Result<Response, WebError> {
    let v = state.club.seasons_view(user, year).await?;
    let layout = Layout::build(
        state,
        session,
        Some(user),
        "Saisons und Gebühren",
        "/finanzen/saisons",
    )
    .await?;
    let invalid = !errors.is_empty();
    let response = render(&SeasonsPage {
        layout,
        v,
        errors,
        nav: "saisons",
    })?;
    Ok(if invalid {
        unprocessable(response)
    } else {
        response
    })
}

pub async fn seasons(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Query(query): Query<YearQuery>,
) -> Result<Response, WebError> {
    seasons_page(&state, &session, &user, query.jahr, FieldErrors::new()).await
}

pub async fn save_season(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Form(fields): Form<Fields>,
) -> Result<Response, WebError> {
    let year: i32 = get(&fields, "year")
        .parse()
        .map_err(|_| AppError::NotFound("Saison".into()))?;
    match state.club.save_season(&user, year, &fields).await {
        Ok(()) => Ok(redirect_with_flash(
            &session,
            &format!("/finanzen/saisons?jahr={year}"),
            "Die Saison wurde gespeichert.",
        )
        .await),
        Err(error) => {
            let errors = errors_from(error)?;
            seasons_page(&state, &session, &user, Some(year), errors).await
        }
    }
}

// ---- Jahresbeiträge --------------------------------------------------------------------------

#[derive(Template)]
#[template(path = "finance_fees.html")]
struct FeesPage {
    layout: Layout,
    v: AnnualFeePreview,
    nav: &'static str,
}

pub async fn fees(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Query(query): Query<YearQuery>,
) -> Result<Response, WebError> {
    let v = state.club.annual_fee_preview(&user, query.jahr).await?;
    let layout = Layout::build(
        &state,
        &session,
        Some(&user),
        "Jahresbeiträge",
        "/finanzen/beitraege",
    )
    .await?;
    render(&FeesPage {
        layout,
        v,
        nav: "beitraege",
    })
}

pub async fn create_fees(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Form(fields): Form<Fields>,
) -> Result<Response, WebError> {
    let year: i32 = get(&fields, "year")
        .parse()
        .map_err(|_| AppError::NotFound("Saison".into()))?;
    let count = state.club.create_annual_fees(&user, year).await?;
    let text = if count == 0 {
        "Es gab nichts anzulegen: alle Beiträge sind bereits vorhanden.".to_string()
    } else {
        format!("{count} Jahresbeiträge {year} wurden angelegt.")
    };
    Ok(redirect_with_flash(&session, &format!("/finanzen/beitraege?jahr={year}"), &text).await)
}

// ---- Touren ------------------------------------------------------------------------------------

#[derive(Template)]
#[template(path = "finance_tours.html")]
struct ToursPage {
    layout: Layout,
    v: TourBilling,
    nav: &'static str,
}

pub async fn tours(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
) -> Result<Response, WebError> {
    let v = state.club.tour_billing(&user).await?;
    let layout = Layout::build(
        &state,
        &session,
        Some(&user),
        "Tourgebühren",
        "/finanzen/touren",
    )
    .await?;
    render(&ToursPage {
        layout,
        v,
        nav: "touren",
    })
}

pub async fn bill_tour(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Path(id): Path<String>,
) -> Result<Response, WebError> {
    let reservation: ReservationId = id
        .parse()
        .map_err(|_| AppError::NotFound("Reservierung".into()))?;
    match state.club.bill_tour(&user, reservation).await {
        Ok(0) => Ok(redirect_with_flash(
            &session,
            "/finanzen/touren",
            "Es gab nichts anzulegen: die Tour ist bereits abgerechnet.",
        )
        .await),
        Ok(count) => Ok(redirect_with_flash(
            &session,
            "/finanzen/touren",
            &format!("{count} Tourgebühren wurden angelegt."),
        )
        .await),
        Err(AppError::Rejected(list)) => {
            crate::session::flash(
                &session,
                crate::session::FlashKind::Error,
                list.first().map(|e| e.message.clone()).unwrap_or_default(),
            )
            .await;
            Ok(Redirect::to("/finanzen/touren").into_response())
        }
        Err(other) => Err(other.into()),
    }
}

// ---- Jahresnachweis -------------------------------------------------------------------------------

#[derive(Template)]
#[template(path = "annual_proof.html")]
struct ProofPage {
    layout: Layout,
    p: AnnualProof,
}

pub async fn proof(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Path(number): Path<u32>,
    Query(query): Query<YearQuery>,
) -> Result<Response, WebError> {
    let p = state
        .club
        .annual_proof(&user, member_id(number)?, query.jahr)
        .await?;
    let layout = Layout::build(
        &state,
        &session,
        Some(&user),
        "Jahresnachweis",
        &format!("/finanzen/nachweis/{number}"),
    )
    .await?;
    render(&ProofPage { layout, p })
}
