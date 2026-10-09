//! Zeitpunkte und Vereinszeit.
//!
//! Alle Zeitpunkte tragen einen Zeitzonen-Offset und werden als RFC3339
//! serialisiert. Jahres- und Tageszuordnung erfolgen im gespeicherten Offset.

use chrono::{DateTime, Datelike, Duration, FixedOffset, NaiveDate, TimeZone, Weekday};

/// Zeitpunkt mit Zeitzonen-Offset.
pub type Timestamp = DateTime<FixedOffset>;

/// Offset der Vereinszeit (Kiel) an einem Kalendertag: MESZ (+02:00) vom
/// letzten Sonntag im März bis vor dem letzten Sonntag im Oktober, sonst MEZ
/// (+01:00). Die Umstellungsstunde selbst wird nicht genauer behandelt.
pub fn club_offset(date: NaiveDate) -> FixedOffset {
    let summer_start = last_sunday(date.year(), 3);
    let summer_end = last_sunday(date.year(), 10);
    let hours = if date >= summer_start && date < summer_end {
        2
    } else {
        1
    };
    FixedOffset::east_opt(hours * 3600).expect("gültiger Offset")
}

/// Zeitpunkt in Vereinszeit.
///
/// # Panics
/// Bei einem ungültigen Datum oder einer ungültigen Uhrzeit.
pub fn club_time(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> Timestamp {
    let date = NaiveDate::from_ymd_opt(year, month, day).expect("gültiges Datum");
    let naive = date.and_hms_opt(hour, minute, 0).expect("gültige Uhrzeit");
    club_offset(date)
        .from_local_datetime(&naive)
        .single()
        .expect("eindeutiger Zeitpunkt")
}

/// Beginn eines Kalenderjahres (1. Januar, 00:00 Vereinszeit).
pub fn year_start(year: i32) -> Timestamp {
    club_time(year, 1, 1, 0, 0)
}

/// Ostersonntag nach der Gaußschen Osterformel (gregorianischer Kalender).
pub fn easter_sunday(year: i32) -> NaiveDate {
    let a = year % 19;
    let b = year / 100;
    let c = year % 100;
    let d = b / 4;
    let e = b % 4;
    let f = (b + 8) / 25;
    let g = (b - f + 1) / 3;
    let h = (19 * a + b - d - g + 15) % 30;
    let i = c / 4;
    let k = c % 4;
    let l = (32 + 2 * e + 2 * i - h - k) % 7;
    let m = (a + 11 * h + 22 * l) / 451;
    let month = (h + l - 7 * m + 114) / 31;
    let day = (h + l - 7 * m + 114) % 31 + 1;
    NaiveDate::from_ymd_opt(year, month as u32, day as u32).expect("gültiges Osterdatum")
}

fn last_sunday(year: i32, month: u32) -> NaiveDate {
    let first_of_next = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    }
    .expect("gültiges Datum");
    let mut day = first_of_next - Duration::days(1);
    while day.weekday() != Weekday::Sun {
        day -= Duration::days(1);
    }
    day
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamp_keeps_offset_in_json() {
        let t = club_time(2026, 5, 1, 10, 0);
        let json = serde_json::to_string(&t).unwrap();
        assert_eq!(json, "\"2026-05-01T10:00:00+02:00\"");
        assert_eq!(serde_json::from_str::<Timestamp>(&json).unwrap(), t);
    }

    #[test]
    fn timestamp_without_offset_or_time_is_rejected() {
        assert!(serde_json::from_str::<Timestamp>("\"2026-05-01T10:00:00\"").is_err());
        assert!(serde_json::from_str::<Timestamp>("\"2026-05-01\"").is_err());
    }

    #[test]
    fn club_offset_follows_summer_time() {
        assert_eq!(
            club_time(2026, 1, 15, 12, 0).offset().local_minus_utc(),
            3600
        );
        assert_eq!(
            club_time(2026, 7, 15, 12, 0).offset().local_minus_utc(),
            7200
        );
        // 2026: Umstellung am 29. März und 25. Oktober
        assert_eq!(
            club_time(2026, 3, 28, 12, 0).offset().local_minus_utc(),
            3600
        );
        assert_eq!(
            club_time(2026, 3, 29, 12, 0).offset().local_minus_utc(),
            7200
        );
        assert_eq!(
            club_time(2026, 10, 25, 12, 0).offset().local_minus_utc(),
            3600
        );
    }

    #[test]
    fn easter_dates() {
        assert_eq!(
            easter_sunday(2024),
            NaiveDate::from_ymd_opt(2024, 3, 31).unwrap()
        );
        assert_eq!(
            easter_sunday(2026),
            NaiveDate::from_ymd_opt(2026, 4, 5).unwrap()
        );
        assert_eq!(
            easter_sunday(2027),
            NaiveDate::from_ymd_opt(2027, 3, 28).unwrap()
        );
    }
}
