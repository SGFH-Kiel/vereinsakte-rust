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

/// Dezimalzahl mit Komma oder Punkt.
pub fn parse_decimal(input: &str) -> Option<f64> {
    input
        .trim()
        .replace(',', ".")
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
}

/// Meter oder ähnliches mit höchstens zwei Nachkommastellen, deutsch.
pub fn number(value: f64) -> String {
    hours(value)
}
