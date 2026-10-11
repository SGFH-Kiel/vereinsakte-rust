//! Saison, Gebührensätze und Beitragsforderungen nach der Beitragsordnung.

use chrono::{Datelike, Duration, NaiveDate, Weekday};
use serde::{Deserialize, Serialize};

use super::account::{Claim, ClaimKind};
use super::club_record::ClubRecord;
use super::ids::MemberId;
use super::member::{Member, MembershipType, TariffGroup};
use super::money::Money;
use super::time::{Timestamp, club_time, easter_sunday};

/// Richtsatz der Arbeitsstunden, wenn die Saison nichts anderes festlegt.
pub const DEFAULT_HOURS_QUOTA: f64 = 25.0;
/// Stundensatz der finanziellen Abgeltung in Cent, wenn die Saison nichts anderes festlegt.
pub const DEFAULT_COMPENSATION_RATE_CENTS: i64 = 1500;

/// Festlegungen des Vorstands für ein Kalenderjahr.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Season {
    pub year: i32,
    /// Richtsatz der Arbeitsstunden.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hours_quota: Option<f64>,
    /// Stundensatz für die finanzielle Abgeltung.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compensation_rate: Option<Money>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fees: Option<FeeSchedule>,
}

/// Gebührentabelle; ohne Angabe gelten die Sätze der Beitragsordnung.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeeSchedule {
    pub annual_fee: Vec<TypeRate>,
    pub admission_fee: Vec<TypeRate>,
    pub tour: Vec<TourRate>,
}

/// Betrag je Mitgliedsart.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeRate {
    pub membership_type: MembershipType,
    pub amount: Money,
}

/// Tourgebühren einer Tarifgruppe.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TourRate {
    pub tariff_group: TariffGroup,
    pub day: Money,
    pub week: Money,
    /// Abweichender Satz für die erste volle Woche.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_week: Option<Money>,
}

impl Default for FeeSchedule {
    /// Sätze der Beitragsordnung der Segelgruppe.
    fn default() -> Self {
        use MembershipType::*;
        let eur = |euros: i64| Money::from_cents(euros * 100);
        let rate = |membership_type, euros| TypeRate {
            membership_type,
            amount: eur(euros),
        };
        let tour = |tariff_group, day, week, first_week: Option<i64>| TourRate {
            tariff_group,
            day: eur(day),
            week: eur(week),
            first_week: first_week.map(eur),
        };
        Self {
            annual_fee: vec![
                rate(Regular, 240),
                rate(Student, 170),
                rate(Extraordinary, 60),
                rate(Erasmus, 100),
                rate(Honorary, 0),
                rate(ChildOfMember, 0),
            ],
            admission_fee: vec![
                rate(Regular, 60),
                rate(Student, 30),
                rate(Extraordinary, 0),
                rate(Erasmus, 0),
                rate(Honorary, 0),
                rate(ChildOfMember, 0),
            ],
            tour: vec![
                tour(TariffGroup::Regular, 12, 70, None),
                tour(TariffGroup::Student, 8, 50, None),
                tour(TariffGroup::Extraordinary, 30, 70, Some(180)),
                tour(TariffGroup::Guest, 30, 240, None),
            ],
        }
    }
}

fn rate_for(rates: &[TypeRate], membership_type: MembershipType) -> Money {
    rates
        .iter()
        .find(|r| r.membership_type == membership_type)
        .map_or(Money::ZERO, |r| r.amount)
}

impl FeeSchedule {
    pub fn annual_fee(&self, membership_type: MembershipType) -> Money {
        rate_for(&self.annual_fee, membership_type)
    }

    pub fn admission_fee(&self, membership_type: MembershipType) -> Money {
        rate_for(&self.admission_fee, membership_type)
    }

    pub fn tour_rate(&self, tariff_group: TariffGroup) -> Option<&TourRate> {
        self.tour.iter().find(|r| r.tariff_group == tariff_group)
    }
}

/// Fälligkeit des Jahresbeitrags: 1. April, an Wochenenden, Karfreitag und
/// Ostermontag der nächste Geschäftstag (Beitragsordnung).
pub fn annual_fee_due_date(year: i32) -> Timestamp {
    let easter = easter_sunday(year);
    let holidays = [easter - Duration::days(2), easter + Duration::days(1)];
    let mut day = NaiveDate::from_ymd_opt(year, 4, 1).expect("gültiges Datum");
    while matches!(day.weekday(), Weekday::Sat | Weekday::Sun) || holidays.contains(&day) {
        day += Duration::days(1);
    }
    club_time(year, day.month(), day.day(), 0, 0)
}

/// Richtsatz der Arbeitsstunden einer Saison (Standard ohne Saison).
pub fn hours_quota(season: Option<&Season>) -> f64 {
    season
        .and_then(|s| s.hours_quota)
        .unwrap_or(DEFAULT_HOURS_QUOTA)
}

/// Stundensatz der Abgeltung einer Saison (Standard ohne Saison).
pub fn compensation_rate(season: Option<&Season>) -> Money {
    season
        .and_then(|s| s.compensation_rate)
        .unwrap_or(Money::from_cents(DEFAULT_COMPENSATION_RATE_CENTS))
}

/// Gebührentabelle einer Saison (Beitragsordnung ohne Saison bzw. Tabelle).
pub fn fees(season: Option<&Season>) -> FeeSchedule {
    season.and_then(|s| s.fees.clone()).unwrap_or_default()
}

/// Jahresbeitrag; im Eintrittsjahr monatsweise anteilig ab dem Eintrittsmonat
/// (außer der Erasmus-Pauschale).
pub fn annual_fee_for(fees: &FeeSchedule, member: &Member, year: i32) -> Money {
    let rate = fees.annual_fee(member.membership_type);
    let joined = member.membership.joined_at;
    if joined.year() == year && member.membership_type != MembershipType::Erasmus {
        rate.prorate(13 - joined.month(), 12)
    } else {
        rate
    }
}

const MONTHS: [&str; 12] = [
    "Januar",
    "Februar",
    "März",
    "April",
    "Mai",
    "Juni",
    "Juli",
    "August",
    "September",
    "Oktober",
    "November",
    "Dezember",
];

impl ClubRecord {
    pub fn season(&self, year: i32) -> Option<&Season> {
        self.seasons.iter().find(|s| s.year == year)
    }

    pub fn hours_quota(&self, year: i32) -> f64 {
        hours_quota(self.season(year))
    }

    pub fn compensation_rate(&self, year: i32) -> Money {
        compensation_rate(self.season(year))
    }

    pub fn fees(&self, year: i32) -> FeeSchedule {
        fees(self.season(year))
    }

    /// Jahresbeitrag eines Mitglieds; im Eintrittsjahr monatsweise anteilig
    /// ab dem Eintrittsmonat (außer der Erasmus-Pauschale).
    pub fn annual_fee_for(&self, member: &Member, year: i32) -> Money {
        annual_fee_for(&self.fees(year), member, year)
    }

    /// Fehlende Jahresbeitragsforderungen eines Jahres (Vorschau; eintragen
    /// mit [`ClubRecord::add_claims`]).
    pub fn annual_fee_claims(&self, year: i32, now: Timestamp) -> Vec<(MemberId, Claim)> {
        let kind = ClaimKind::AnnualFee { year };
        self.members
            .iter()
            .filter(|m| {
                m.is_active_in(year) && m.membership_type.pays_fee() && !m.account.has_claim(&kind)
            })
            .filter_map(|m| {
                let amount = self.annual_fee_for(m, year);
                if !amount.is_positive() {
                    return None;
                }
                let joined = m.membership.joined_at;
                let mut text = format!("Jahresbeitrag {year} ({})", m.membership_type);
                if joined.year() == year && m.membership_type != MembershipType::Erasmus {
                    let month = joined.month();
                    text.push_str(&format!(
                        ", anteilig ab {} ({}/12)",
                        MONTHS[month as usize - 1],
                        13 - month
                    ));
                }
                let due = annual_fee_due_date(year).max(joined);
                Some((m.id, Claim::new(kind.clone(), amount, text, now, Some(due))))
            })
            .collect()
    }

    /// Fehlende Aufnahmegebühren der im Jahr eingetretenen Mitglieder.
    pub fn admission_fee_claims(&self, year: i32, now: Timestamp) -> Vec<(MemberId, Claim)> {
        self.members
            .iter()
            .filter(|m| {
                m.membership.joined_at.year() == year
                    && !m.account.has_claim(&ClaimKind::AdmissionFee)
            })
            .filter_map(|m| {
                let amount = self.fees(year).admission_fee(m.membership_type);
                amount.is_positive().then(|| {
                    let text = format!("Aufnahmegebühr ({})", m.membership_type);
                    (
                        m.id,
                        Claim::new(
                            ClaimKind::AdmissionFee,
                            amount,
                            text,
                            now,
                            Some(m.membership.joined_at),
                        ),
                    )
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::example::{self, at, *};

    fn eur(s: &str) -> Money {
        s.parse().unwrap()
    }

    fn claim_for(claims: &[(MemberId, Claim)], member: MemberId) -> Option<&Claim> {
        claims.iter().find(|(m, _)| *m == member).map(|(_, c)| c)
    }

    #[test]
    fn defaults_without_season() {
        let club = example::club();
        assert_eq!(club.hours_quota(2027), 25.0);
        assert_eq!(club.compensation_rate(2027), eur("15"));
        assert_eq!(club.fees(2027), FeeSchedule::default());
        assert_eq!(
            club.fees(2026).annual_fee(MembershipType::Student),
            eur("170")
        );
        assert_eq!(
            club.fees(2026).admission_fee(MembershipType::Student),
            eur("30")
        );
    }

    #[test]
    fn season_overrides() {
        let mut club = example::club();
        club.seasons.push(Season {
            year: 2027,
            hours_quota: Some(30.0),
            compensation_rate: Some(eur("20")),
            fees: None,
        });
        assert_eq!(club.hours_quota(2027), 30.0);
        assert_eq!(club.compensation_rate(2027), eur("20"));

        let mut fees = FeeSchedule::default();
        fees.annual_fee[0].amount = eur("260");
        club.seasons.last_mut().unwrap().fees = Some(fees);
        assert_eq!(
            club.fees(2027).annual_fee(MembershipType::Regular),
            eur("260")
        );
        let mut fresh = example::club_without_accounts();
        fresh.seasons = club.seasons.clone();
        let claims = fresh.annual_fee_claims(2027, at(2027, 3, 15, 10, 0));
        assert_eq!(claim_for(&claims, ANNA).unwrap().amount, eur("260"));
    }

    #[test]
    fn due_date_moves_to_next_business_day() {
        assert_eq!(annual_fee_due_date(2026), at(2026, 4, 1, 0, 0)); // Mittwoch
        assert_eq!(annual_fee_due_date(2023), at(2023, 4, 3, 0, 0)); // Samstag
        assert_eq!(annual_fee_due_date(2024), at(2024, 4, 2, 0, 0)); // Ostermontag
        assert_eq!(annual_fee_due_date(2018), at(2018, 4, 3, 0, 0)); // Ostersonntag + Ostermontag
    }

    #[test]
    fn annual_fees_by_membership_type() {
        let club = example::club_without_accounts();
        let claims = club.annual_fee_claims(2026, at(2026, 3, 15, 10, 0));
        assert_eq!(claim_for(&claims, ANNA).unwrap().amount, eur("240"));
        assert_eq!(
            claim_for(&claims, ANNA).unwrap().due_at,
            Some(at(2026, 4, 1, 0, 0))
        );
        assert_eq!(claim_for(&claims, CLARA).unwrap().amount, eur("170"));
        assert_eq!(claim_for(&claims, GRETA).unwrap().amount, eur("60"));
        assert!(claim_for(&claims, EVA).is_none(), "Ehrenmitglied");
        assert!(claim_for(&claims, HANNES).is_none(), "Kind eines Mitglieds");
        // Erasmus: Pauschale, nicht anteilig, obwohl im März eingetreten
        assert_eq!(claim_for(&claims, INES).unwrap().amount, eur("100"));
        // Malte ist im Mai eingetreten: 8/12 von 170 € = 113,33 €, fällig mit Eintritt
        let malte = claim_for(&claims, MALTE).unwrap();
        assert_eq!(malte.amount, eur("113.33"));
        assert_eq!(malte.due_at, Some(at(2026, 5, 15, 0, 0)));
        assert!(
            malte.text.contains("anteilig ab Mai (8/12)"),
            "{}",
            malte.text
        );
    }

    #[test]
    fn joining_in_july_pays_half() {
        let mut club = example::club_without_accounts();
        let mut ole = example::anna();
        ole.id = MemberId::new(150).unwrap();
        ole.membership.joined_at = at(2026, 7, 1, 0, 0);
        club.members.push(ole);
        let claims = club.annual_fee_claims(2026, at(2026, 7, 2, 10, 0));
        assert_eq!(
            claim_for(&claims, MemberId::new(150).unwrap())
                .unwrap()
                .amount,
            eur("120")
        );
    }

    #[test]
    fn admission_fees() {
        let club = example::club_without_accounts();
        let claims = club.admission_fee_claims(2026, at(2026, 5, 16, 10, 0));
        assert_eq!(
            claims.len(),
            1,
            "nur Malte; Inès (Erasmus) zahlt keine Aufnahmegebühr"
        );
        let malte = claim_for(&claims, MALTE).unwrap();
        assert_eq!(malte.amount, eur("30"));
        assert_eq!(malte.due_at, Some(at(2026, 5, 15, 0, 0)));
    }

    #[test]
    fn second_run_creates_no_duplicates() {
        let mut club = example::club_without_accounts();
        let now = at(2026, 3, 15, 10, 0);
        let first = club.annual_fee_claims(2026, now);
        assert!(!first.is_empty());
        club.add_claims(first);
        assert!(club.annual_fee_claims(2026, now).is_empty());
        // Die Beispielakte enthält die Forderungen 2026 bereits.
        assert!(example::club().annual_fee_claims(2026, now).is_empty());
        assert!(example::club().admission_fee_claims(2026, now).is_empty());
    }
}
