//! Fehler der Anwendungsfälle.

use vereinsakte_core::issues::RecordIssue;

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum AppError {
    #[error("nicht gefunden: {0}")]
    NotFound(String),
    #[error("keine Berechtigung")]
    Forbidden,
    /// Die Akte wurde zwischenzeitlich geändert bzw. ein Eintrag existiert bereits.
    #[error("die Daten wurden zwischenzeitlich geändert – bitte erneut versuchen")]
    Conflict,
    /// Die Änderung verstößt gegen Regeln der Vereinsakte.
    #[error("ungültige Änderung: {}", join(.0))]
    Invalid(Vec<RecordIssue>),
    /// Eingaben wurden nicht angenommen; je Feld eine deutsche Meldung.
    #[error("Eingabe nicht angenommen: {}", join_fields(.0))]
    Rejected(Vec<FieldError>),
    #[error("Speicherfehler: {0}")]
    Storage(String),
}

/// Meldung zu einem Eingabefeld; `field` ist leer für allgemeine Meldungen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldError {
    pub field: String,
    pub message: String,
}

impl FieldError {
    pub fn new(field: &str, message: impl Into<String>) -> Self {
        Self {
            field: field.to_string(),
            message: message.into(),
        }
    }
}

impl AppError {
    pub fn rejected(field: &str, message: impl Into<String>) -> Self {
        Self::Rejected(vec![FieldError::new(field, message)])
    }
}

fn join_fields(errors: &[FieldError]) -> String {
    errors
        .iter()
        .map(|e| e.message.clone())
        .collect::<Vec<_>>()
        .join("; ")
}

fn join(issues: &[RecordIssue]) -> String {
    issues
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ")
}
