//! Formular-Infrastruktur: Eingabeformate, Feldfehler und Rückmeldungen.

use std::collections::BTreeMap;
use std::str::FromStr;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use chrono::NaiveDate;
use tower_sessions::Session;
use vereinsakte_core::money::Money;

use crate::error::WebError;
use crate::session::{FlashKind, flash};

/// Fehlermeldungen je Formularfeld (auf Deutsch) plus allgemeine Meldungen.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FieldErrors {
    fields: BTreeMap<String, Vec<String>>,
    general: Vec<String>,
}

impl FieldErrors {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, field: &str, message: impl Into<String>) {
        self.fields
            .entry(field.to_string())
            .or_default()
            .push(message.into());
    }

    pub fn add_general(&mut self, message: impl Into<String>) {
        self.general.push(message.into());
    }

    pub fn is_empty(&self) -> bool {
        self.fields.is_empty() && self.general.is_empty()
    }

    /// Erste Meldung zum Feld oder leerer Text (für Templates).
    pub fn get(&self, field: &str) -> String {
        self.fields
            .get(field)
            .and_then(|m| m.first())
            .cloned()
            .unwrap_or_default()
    }

    pub fn general(&self) -> &[String] {
        &self.general
    }

    /// Übernimmt das Ergebnis einer Feldeingabe; bei Fehler wird er zum Feld notiert.
    pub fn take<T>(&mut self, field: &str, result: Result<T, String>) -> Option<T> {
        match result {
            Ok(value) => Some(value),
            Err(message) => {
                self.add(field, message);
                None
            }
        }
    }
}

/// Feldfehler aus einer abgelehnten Eingabe; andere Fehler gehen weiter.
pub fn errors_from(error: vereinsakte_app::AppError) -> Result<FieldErrors, WebError> {
    match error {
        vereinsakte_app::AppError::Rejected(list) => {
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

/// Betrag in Euro; Komma oder Punkt als Dezimaltrenner, höchstens zwei Nachkommastellen.
pub fn parse_money(input: &str) -> Result<Money, String> {
    let text = input.trim().replace(',', ".").replace(['€', ' '], "");
    if text.is_empty() {
        return Err("Bitte einen Betrag eingeben.".into());
    }
    Money::from_str(&text).map_err(|_| "Bitte einen Betrag wie 12,50 eingeben.".to_string())
}

/// Datum als `TT.MM.JJJJ` oder `JJJJ-MM-TT` (Datumsfeld des Browsers).
pub fn parse_date(input: &str) -> Result<NaiveDate, String> {
    let text = input.trim();
    NaiveDate::parse_from_str(text, "%Y-%m-%d")
        .or_else(|_| NaiveDate::parse_from_str(text, "%d.%m.%Y"))
        .map_err(|_| "Bitte ein gültiges Datum eingeben (TT.MM.JJJJ).".to_string())
}

/// Dezimalzahl mit Komma oder Punkt, z. B. Stunden.
pub fn parse_decimal(input: &str) -> Result<f64, String> {
    input
        .trim()
        .replace(',', ".")
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
        .ok_or_else(|| "Bitte eine Zahl eingeben (z. B. 4,5).".to_string())
}

/// Leere Eingabe gilt als nicht angegeben.
pub fn non_empty(input: &str) -> Option<&str> {
    let text = input.trim();
    (!text.is_empty()).then_some(text)
}

/// Erfolg: Meldung merken und per 303 weiterleiten (Post/Redirect/Get).
pub async fn redirect_with_flash(session: &Session, to: &str, text: &str) -> Response {
    flash(session, FlashKind::Ok, text).await;
    Redirect::to(to).into_response()
}

/// Formular erneut zeigen, weil die Eingabe nicht angenommen wurde.
pub fn unprocessable(body: Response) -> Response {
    let (mut parts, body) = body.into_parts();
    parts.status = StatusCode::UNPROCESSABLE_ENTITY;
    Response::from_parts(parts, body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn money_accepts_comma_and_point() {
        assert_eq!(parse_money("12,50").unwrap(), parse_money("12.50").unwrap());
        assert_eq!(
            parse_money("12,50 €").unwrap().to_string(),
            parse_money("12.5").unwrap().to_string()
        );
        assert!(parse_money("zwölf").is_err());
        assert!(parse_money("").is_err());
    }

    #[test]
    fn dates_in_both_formats() {
        let expected = NaiveDate::from_ymd_opt(2026, 10, 9).unwrap();
        assert_eq!(parse_date("2026-10-09").unwrap(), expected);
        assert_eq!(parse_date("09.10.2026").unwrap(), expected);
        assert!(parse_date("31.02.2026").is_err());
    }

    #[test]
    fn decimals_with_comma() {
        assert_eq!(parse_decimal("4,5").unwrap(), 4.5);
        assert!(parse_decimal("x").is_err());
    }

    #[test]
    fn field_errors_collect_per_field() {
        let mut errors = FieldErrors::new();
        assert!(errors.take("betrag", parse_money("1,5")).is_some());
        assert!(errors.is_empty());
        assert!(errors.take("datum", parse_date("gestern")).is_none());
        assert!(!errors.is_empty());
        assert!(errors.get("datum").contains("gültiges Datum"));
        assert_eq!(errors.get("betrag"), "");
    }
}

#[cfg(test)]
mod render_tests {
    use askama::Template;
    use axum::body::to_bytes;
    use axum::response::Html;

    use super::*;

    #[derive(Template)]
    #[template(
        source = r#"{% import "_forms.html" as f %}{{ f::field("betrag", "Betrag", value, error, "text", "z. B. 12,50") }}"#,
        ext = "html"
    )]
    struct Probe<'a> {
        value: &'a str,
        error: &'a str,
    }

    #[test]
    fn field_shows_label_value_hint_and_error() {
        let html = Probe {
            value: "abc",
            error: "Bitte einen Betrag wie 12,50 eingeben.",
        }
        .render()
        .unwrap();
        assert!(
            html.contains("<label for=\"f-betrag\">Betrag</label>"),
            "{html}"
        );
        assert!(html.contains("value=\"abc\""), "Eingabe bleibt erhalten");
        assert!(html.contains("has-error") && html.contains("aria-invalid=\"true\""));
        assert!(html.contains("Bitte einen Betrag wie 12,50 eingeben."));
        let clean = Probe {
            value: "1",
            error: "",
        }
        .render()
        .unwrap();
        assert!(!clean.contains("has-error") && !clean.contains("error-text"));
    }

    #[tokio::test]
    async fn invalid_input_is_answered_with_422_and_keeps_the_page() {
        let response = unprocessable(Html("<form>…</form>").into_response());
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(&body[..], "<form>…</form>".as_bytes());
    }
}
