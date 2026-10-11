//! Deutsche Darstellung von Werten für Sichtmodelle.

use vereinsakte_core::time::Timestamp;

/// Stunden ohne überflüssige Nachkommastellen, z. B. „20,5“ oder „25“.
pub fn hours(value: f64) -> String {
    let text = format!("{value:.2}");
    text.trim_end_matches('0')
        .trim_end_matches('.')
        .replace('.', ",")
}

/// Datum, z. B. „09.10.2026“.
pub fn date(t: Timestamp) -> String {
    t.format("%d.%m.%Y").to_string()
}
