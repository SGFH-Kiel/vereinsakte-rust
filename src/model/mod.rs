//! Datenmodell der Vereinsakte der Segelgruppe an der FH Kiel.
//!
//! Bezeichner sind englisch, Dokumentation und Fehlertexte deutsch
//! (Glossar: `openspec/changes/add-datenmodell/design.md`).

pub mod account;
pub mod boat;
pub mod boat_permit;
pub mod club_record;
pub mod fees;
pub mod ids;
pub mod issues;
pub mod member;
pub mod money;
pub mod office;
pub mod requirement;
pub mod reservation;
pub mod sailing_permit;
pub mod task;
pub mod time;
pub mod tour_fees;
pub mod work_event;
pub mod work_hours;

#[cfg(test)]
mod example;

#[cfg(test)]
mod tests {
    use super::club_record::ClubRecord;
    use super::example::{self, at, *};
    use super::money::Money;

    const FIXTURE_PATH: &str = "src/model/testdata/vereinsakte_beispiel.json";

    fn eur(s: &str) -> Money {
        s.parse().unwrap()
    }

    #[test]
    fn full_club_record_round_trip_and_rules() {
        let club = example::club();
        assert!(!club.members.is_empty() && !club.boats.is_empty() && !club.offices.is_empty());
        assert!(club.work_entries.iter().any(|e| !e.is_confirmed()));
        assert!(
            club.members
                .iter()
                .flat_map(|m| &m.account.claims)
                .any(|c| c.is_cancelled())
        );

        let json = serde_json::to_string(&club).unwrap();
        let back: ClubRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(back, club);
        assert_eq!(back.validate(), Ok(()));

        let outstanding = club.outstanding(at(2026, 10, 9, 12, 0));
        let summary: Vec<_> = outstanding
            .iter()
            .map(|o| (o.member, o.open, o.overdue))
            .collect();
        assert_eq!(
            summary,
            vec![
                (CLARA, eur("45"), eur("45")),
                (DAVID, eur("58"), eur("58")),
                (LUKAS, eur("276"), eur("276")),
            ]
        );

        let tour = club.reservation(DEHLER_TOUR).unwrap();
        assert_eq!(club.check_reservation(tour), Ok(()));
    }

    #[test]
    fn json_fixture_documents_the_format() {
        let fixture = include_str!("testdata/vereinsakte_beispiel.json");
        let club: ClubRecord = serde_json::from_str(fixture).unwrap();
        assert_eq!(club.validate(), Ok(()));
        assert_eq!(club, example::club());
        assert_eq!(
            serde_json::to_string_pretty(&club).unwrap(),
            fixture.trim_end()
        );
    }

    /// Schreibt die Beispieldatei neu: `cargo test write_json_fixture -- --ignored`
    #[test]
    #[ignore = "erzeugt die JSON-Beispieldatei"]
    fn write_json_fixture() {
        let json = serde_json::to_string_pretty(&example::club()).unwrap();
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURE_PATH);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, json + "\n").unwrap();
    }
}
