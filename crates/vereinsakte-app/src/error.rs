//! Fehler der Anwendungsfälle.

use vereinsakte_core::issues::RecordIssue;

#[derive(Debug, thiserror::Error)]
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
    #[error("Speicherfehler: {0}")]
    Storage(String),
}

fn join(issues: &[RecordIssue]) -> String {
    issues
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ")
}
