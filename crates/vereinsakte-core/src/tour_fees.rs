//! Tourgebühren nach der Beitragsordnung: je Person an Bord, nach Tourtagen
//! und Tarifgruppe.

use chrono::{Datelike, Duration};

use super::account::{Claim, ClaimKind};
use super::club_record::ClubRecord;
use super::fees::TourRate;
use super::ids::MemberId;
use super::member::TariffGroup;
use super::money::Money;
use super::reservation::Reservation;
use super::time::{Timestamp, club_time};

impl TourRate {
    /// Gebühr für eine Zahl von Tourtagen: volle Wochen × Wochensatz (die
    /// erste ggf. zum abweichenden Satz) + Resttage × Tagessatz.
    pub fn fee(&self, days: u32) -> Money {
        let weeks = days / 7;
        let rest = days % 7;
        let weekly = match (self.first_week, weeks) {
            (_, 0) => Money::ZERO,
            (Some(first), _) => first + self.week * (weeks - 1),
            (None, _) => self.week * weeks,
        };
        weekly + self.day * rest
    }
}

/// Tourtage: Kalendertage an Bord, angefangene und endende Tage zählen voll.
pub fn tour_days(from: Timestamp, until: Timestamp) -> u32 {
    let days = (until.date_naive() - from.date_naive()).num_days() + 1;
    u32::try_from(days.max(0)).unwrap_or(0)
}

/// Zahlstatus einer Tourgebühr.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaymentStatus {
    Paid,
    /// Teilweise bezahlt; enthält den offenen Restbetrag.
    Partial(Money),
    Open,
}

/// Eine Zeile der Übersicht der Tourgebühren einer Reservierung.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TourFeeLine {
    /// Konto, auf dem die Gebühr steht.
    pub account: MemberId,
    /// Gast, für den die Gebühr erhoben wird (sonst das Mitglied selbst).
    pub guest: Option<String>,
    pub amount: Money,
    pub status: PaymentStatus,
}

struct Person {
    account: MemberId,
    guest: Option<String>,
    tariff: TariffGroup,
    from: Timestamp,
    until: Timestamp,
}

impl ClubRecord {
    fn persons_on_tour(&self, r: &Reservation) -> Vec<Person> {
        let member = |id: MemberId, from, until| {
            self.member(id).map(|m| {
                let (from, until) = r.period(from, until);
                Person {
                    account: id,
                    guest: None,
                    tariff: m.membership_type.tariff_group(),
                    from,
                    until,
                }
            })
        };
        let mut persons: Vec<Person> = Vec::new();
        persons.extend(member(r.skipper, None, None));
        persons.extend(r.co_skipper.and_then(|co| member(co, None, None)));
        persons.extend(
            r.crew
                .iter()
                .filter_map(|seat| member(seat.member, seat.from, seat.until)),
        );
        persons
    }

    /// Fehlende Tourgebühren einer Reservierung (Vorschau; eintragen mit
    /// [`ClubRecord::add_claims`]). Nur Touren sind gebührenpflichtig.
    pub fn tour_fee_claims(&self, r: &Reservation, now: Timestamp) -> Vec<(MemberId, Claim)> {
        if !r.is_tour() {
            return Vec::new();
        }
        let fees = self.fees(r.start.year());
        let boat = self
            .boat(r.boat)
            .map_or_else(|| r.boat.to_string(), |b| b.name.clone());
        self.persons_on_tour(r)
            .into_iter()
            .filter_map(|p| {
                let kind = ClaimKind::TourFee {
                    reservation: r.id,
                    guest: p.guest.clone(),
                };
                let account = &self.member(p.account)?.account;
                if account.has_claim(&kind) {
                    return None;
                }
                let days = tour_days(p.from, p.until);
                let amount = fees.tour_rate(p.tariff)?.fee(days);
                if !amount.is_positive() {
                    return None;
                }
                let who = p
                    .guest
                    .as_ref()
                    .map(|g| format!(" für Gast {g}"))
                    .unwrap_or_default();
                let text = format!(
                    "Tourgebühr {boat} {}–{}{who} ({days} Tage)",
                    p.from.format("%d.%m."),
                    p.until.format("%d.%m.%Y")
                );
                let due_day = p.until.date_naive() + Duration::days(7);
                let due = club_time(due_day.year(), due_day.month(), due_day.day(), 0, 0);
                Some((p.account, Claim::new(kind, amount, text, now, Some(due))))
            })
            .collect()
    }

    /// Geforderte Tourgebühren einer Reservierung mit Zahlstatus.
    pub fn tour_fee_overview(&self, r: &Reservation) -> Vec<TourFeeLine> {
        self.members
            .iter()
            .flat_map(|member| {
                let statuses = member.account.claim_status();
                member
                    .account
                    .active_claims()
                    .filter_map(move |claim| match &claim.kind {
                        ClaimKind::TourFee { reservation, guest } if *reservation == r.id => {
                            let open = statuses.iter().find(|s| s.claim_id == claim.id)?.open;
                            let status = if open == Money::ZERO {
                                PaymentStatus::Paid
                            } else if open < claim.amount {
                                PaymentStatus::Partial(open)
                            } else {
                                PaymentStatus::Open
                            };
                            Some(TourFeeLine {
                                account: member.id,
                                guest: guest.clone(),
                                amount: claim.amount,
                                status,
                            })
                        }
                        _ => None,
                    })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::Payment;
    use crate::example::{self, at, *};
    use crate::fees::FeeSchedule;

    fn eur(s: &str) -> Money {
        s.parse().unwrap()
    }

    fn rate(group: TariffGroup) -> TourRate {
        *FeeSchedule::default().tour_rate(group).unwrap()
    }

    #[test]
    fn tour_days_count_started_days() {
        // Freitag 16:00 bis Sonntag 18:00
        assert_eq!(tour_days(at(2026, 6, 12, 16, 0), at(2026, 6, 14, 18, 0)), 3);
        // Crewwechsel am 7. Juli: beide zahlen den Tag voll
        assert_eq!(tour_days(at(2026, 7, 1, 10, 0), at(2026, 7, 7, 18, 0)), 7);
        assert_eq!(tour_days(at(2026, 7, 7, 10, 0), at(2026, 7, 14, 16, 0)), 8);
    }

    #[test]
    fn fees_per_tariff_group() {
        assert_eq!(rate(TariffGroup::Regular).fee(10), eur("106"));
        assert_eq!(rate(TariffGroup::Extraordinary).fee(15), eur("280"));
        assert_eq!(rate(TariffGroup::Extraordinary).fee(3), eur("90"));
        assert_eq!(rate(TariffGroup::Student).fee(3), eur("24"));
        assert_eq!(rate(TariffGroup::Guest).fee(7), eur("240"));
        assert_eq!(rate(TariffGroup::Guest).fee(3), eur("90"));
    }

    #[test]
    fn no_fees_without_tour() {
        let club = example::club_without_accounts();
        let r = club.reservation(FOLKEBOOT_NACHMITTAG).unwrap();
        assert!(club.tour_fee_claims(r, at(2026, 6, 21, 10, 0)).is_empty());
    }

    #[test]
    fn claims_for_everyone_on_board() {
        let club = example::club_without_accounts();
        let r = club.reservation(DEHLER_TOUR).unwrap();
        let claims = club.tour_fee_claims(r, at(2026, 7, 15, 10, 0));
        let amount = |member: MemberId, guest: Option<&str>| {
            claims
                .iter()
                .find(|(m, c)| {
                    *m == member && matches!(&c.kind, ClaimKind::TourFee { guest: g, .. } if g.as_deref() == guest)
                })
                .map(|(_, c)| c.amount)
        };
        assert_eq!(
            claims.len(),
            5,
            "Skipperin, Co-Skipperin, drei Crew-Mitglieder; keine Gäste"
        );
        assert_eq!(
            amount(ANNA, None),
            Some(eur("140")),
            "Skipperin, 14 Tage ordentlich"
        );
        assert_eq!(amount(KARIN, None), Some(eur("140")), "Co-Skipperin");
        assert_eq!(
            amount(BEN, None),
            Some(eur("70")),
            "Ben bis zum Crewwechsel, 7 Tage"
        );
        assert_eq!(
            amount(DAVID, None),
            Some(eur("58")),
            "David ab dem Crewwechsel, 8 Tage studentisch"
        );
        assert_eq!(
            amount(HANNES, None),
            Some(eur("100")),
            "Kind: studentischer Satz"
        );
        assert_eq!(
            amount(ANNA, Some("Max Gast")),
            None,
            "Gäste werden nicht abgerechnet"
        );
        assert!(claims.iter().all(|(_, c)| !c.text.contains("Gast")));
        let (_, anna) = claims.iter().find(|(m, _)| *m == ANNA).unwrap();
        assert_eq!(anna.due_at, Some(at(2026, 7, 21, 0, 0)));
    }

    #[test]
    fn honorary_member_pays_regular_rate_and_guests_are_not_billed() {
        let club = example::club_without_accounts();
        let r = club.reservation(TRIGON_WOCHENENDE).unwrap();
        let claims = club.tour_fee_claims(r, at(2026, 8, 17, 10, 0));
        assert!(
            claims
                .iter()
                .any(|(m, c)| *m == EVA && c.amount == eur("36"))
        );
        assert!(claims.iter().all(|(_, c)| !c.text.contains("Ole Svensson")));
        assert_eq!(claims.len(), 3, "Eva, Frank und Lukas");
    }

    #[test]
    fn second_run_creates_no_duplicates() {
        let mut club = example::club_without_accounts();
        let r = club.reservation(DEHLER_TOUR).unwrap().clone();
        let claims = club.tour_fee_claims(&r, at(2026, 7, 15, 10, 0));
        club.add_claims(claims);
        assert!(club.tour_fee_claims(&r, at(2026, 7, 16, 10, 0)).is_empty());
        assert!(
            example::club()
                .tour_fee_claims(&r, at(2026, 7, 16, 10, 0))
                .is_empty()
        );
    }

    #[test]
    fn overview_shows_payment_status() {
        let mut club = example::club();
        let r = club.reservation(DEHLER_TOUR).unwrap().clone();
        let status = |club: &ClubRecord, member, guest: Option<&str>| {
            club.tour_fee_overview(&r)
                .into_iter()
                .find(|l| l.account == member && l.guest.as_deref() == guest)
                .unwrap()
                .status
        };
        assert_eq!(status(&club, ANNA, None), PaymentStatus::Paid);
        assert_eq!(status(&club, BEN, None), PaymentStatus::Paid);
        assert_eq!(status(&club, DAVID, None), PaymentStatus::Open);
        club.member_mut(DAVID)
            .unwrap()
            .account
            .payments
            .push(Payment::new(eur("20"), at(2026, 8, 1, 0, 0), None));
        assert_eq!(
            status(&club, DAVID, None),
            PaymentStatus::Partial(eur("38"))
        );
        assert_eq!(club.tour_fee_overview(&r).len(), 5);
    }
}
