//! Datenmodell der Vereinsakte der Segelgruppe an der FH Kiel.
//!
//! Bezeichner sind englisch, Dokumentation und Fehlertexte deutsch
//! (Glossar: `openspec/changes/archive/2026-10-09-add-datenmodell/design.md`).

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

/// Erfundene, realistische Beispielakte für Tests. Nur in Tests oder mit dem
/// Feature `example` verfügbar – andere Crates aktivieren es ausschließlich
/// in ihren `[dev-dependencies]`, damit es im Produktiv-Build fehlt.
#[cfg(any(test, feature = "example"))]
pub mod example;

#[cfg(test)]
mod tests {
    use super::club_record::ClubRecord;
    use super::example::{self, at, *};
    use super::money::Money;

    const FIXTURE_PATH: &str = "testdata/vereinsakte_beispiel.json";

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

    /// Die Regeln auf Teildaten liefern mit nur den Daten eines Mitglieds
    /// (wie aus der Datenbank geladen) dasselbe wie auf der ganzen Akte.
    #[test]
    fn scoped_rules_match_whole_record() {
        use super::boat_permit::PermitHistory;
        use super::office::Offices;
        use super::sailing_permit::MemberYear;

        let club = example::club();
        for t in [
            at(2026, 6, 13, 10, 0),
            at(2026, 10, 9, 12, 0),
            at(2025, 7, 1, 12, 0),
        ] {
            let year = chrono::Datelike::year(&t);
            let active_offices: Vec<_> = club
                .offices
                .iter()
                .filter(|o| o.is_active_at(t))
                .cloned()
                .collect();
            let offices = Offices::new(&active_offices);
            for member in &club.members {
                let id = member.id;
                // Nur die eigenen Daten, als eigene Kopien.
                let entries: Vec<_> = club
                    .work_entries
                    .iter()
                    .filter(|e| e.member == id && e.year() == year)
                    .cloned()
                    .collect();
                let transfers: Vec<_> = club
                    .hour_transfers
                    .iter()
                    .filter(|h| h.year == year && (h.from == id || h.to == id))
                    .cloned()
                    .collect();
                let record = club
                    .annual_records
                    .iter()
                    .find(|r| r.member == id && r.year == year)
                    .cloned();
                let season = club.seasons.iter().find(|s| s.year == year).cloned();
                let scoped = MemberYear {
                    member,
                    year,
                    season: season.as_ref(),
                    annual_record: record.as_ref(),
                    work_entries: entries.iter().collect(),
                    hour_transfers: transfers.iter().collect(),
                };
                assert_eq!(
                    scoped.credited_hours(),
                    club.credited_hours(id, year),
                    "{id} {year}"
                );
                assert_eq!(
                    scoped.required_hours(),
                    club.required_hours(id, year),
                    "{id} {year}"
                );
                assert_eq!(
                    scoped.fee_arrears(),
                    club.fee_arrears(id, year),
                    "{id} {year}"
                );
                assert_eq!(
                    scoped.sailing_permit_at(t),
                    club.sailing_permit_at(id, t),
                    "{id} {t}"
                );

                let checkouts: Vec<_> = club
                    .boat_checkouts
                    .iter()
                    .filter(|c| c.member == id)
                    .cloned()
                    .collect();
                let revocations: Vec<_> = club
                    .revocations
                    .iter()
                    .filter(|r| r.member == id)
                    .cloned()
                    .collect();
                let history = PermitHistory {
                    member: id,
                    address_valid: member.address_valid,
                    checkouts: checkouts.iter().collect(),
                    revocations: revocations.iter().collect(),
                };
                for boat in &club.boats {
                    assert_eq!(
                        history.boat_permit(boat.id, t),
                        club.boat_permit(id, boat.id, t),
                        "{id} {}",
                        boat.name
                    );
                }
                assert_eq!(offices.is_board_member(id, t), club.is_board_member(id, t));
                assert_eq!(offices.holds(id, t, |_| true), club.holds(id, t, |_| true));
            }
        }
    }

    #[test]
    fn json_fixture_documents_the_format() {
        let fixture = include_str!("../testdata/vereinsakte_beispiel.json");
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
