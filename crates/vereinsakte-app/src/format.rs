//! Deutsche Darstellung von Werten für Sichtmodelle.

use vereinsakte_core::time::Timestamp;

/// Stunden ohne überflüssige Nachkommastellen, z. B. „20,5“ oder „25“.
pub fn hours(value: f64) -> String {
    // `-0.0` (Summe über nichts) soll als „0“ erscheinen.
    let text = format!("{:.2}", value + 0.0);
    text.trim_end_matches('0')
        .trim_end_matches('.')
        .replace('.', ",")
}

/// Initialen aus Vor- und Nachname für den Namenskreis, z. B. „EM“.
pub fn initials(first: &str, last: &str) -> String {
    first
        .chars()
        .next()
        .into_iter()
        .chain(last.chars().next())
        .flat_map(char::to_uppercase)
        .collect()
}

/// Farbklasse des Namenskreises, stabil je Mitgliedsnummer.
pub fn avatar_class(member: u32) -> String {
    format!("av-{}", member % 6)
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
