//! Reservierungen von Vereinsbooten und ihre Prüfung gegen die Segelordnung.

use std::fmt;

use serde::{Deserialize, Serialize};

use super::boat::SailingArea;
use super::boat_permit::EligibilityIssue;
use super::club_record::ClubRecord;
use super::ids::{BoatId, MemberId, ReservationId};
use super::issues::ReservationIssue;
use super::time::Timestamp;

/// Reservierung eines Bootes im Vormerkkalender.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Reservation {
    pub id: ReservationId,
    pub boat: BoatId,
    pub start: Timestamp,
    pub end: Timestamp,
    pub area: SailingArea,
    /// Tour = Fahrt mit mindestens einer Übernachtung in einem fremden Hafen.
    #[serde(default, skip_serializing_if = "is_false")]
    pub tour: bool,
    pub skipper: MemberId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub co_skipper: Option<MemberId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub crew: Vec<CrewSeat>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub guests: Vec<Guest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remark: Option<String>,
}

fn is_false(value: &bool) -> bool {
    !*value
}

/// Crew-Mitglied, optional mit eigenem Zeitraum an Bord (z. B. Crewwechsel).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrewSeat {
    pub member: MemberId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<Timestamp>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub until: Option<Timestamp>,
}

impl CrewSeat {
    pub fn new(member: MemberId) -> Self {
        Self {
            member,
            from: None,
            until: None,
        }
    }
}

/// Gast an Bord; kein Vereinsmitglied.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Guest {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    /// Einladendes Mitglied; ohne Angabe gilt der Skipper.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invited_by: Option<MemberId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<Timestamp>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub until: Option<Timestamp>,
}

impl Guest {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            phone: None,
            invited_by: None,
            from: None,
            until: None,
        }
    }
}

/// Plausibilitätsfehler einer Reservierung.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReservationError {
    EndNotAfterStart,
    SkipperIsCoSkipper,
    SkipperInCrew,
    CoSkipperInCrew,
    DuplicateCrewMember(MemberId),
    InvalidCrewPeriod(MemberId),
    InvalidGuestPeriod(String),
}

impl fmt::Display for ReservationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EndNotAfterStart => f.write_str("Ende nicht nach Beginn"),
            Self::SkipperIsCoSkipper => f.write_str("Skipper und Co-Skipper sind dieselbe Person"),
            Self::SkipperInCrew => f.write_str("Skipper steht zugleich in der Crew"),
            Self::CoSkipperInCrew => f.write_str("Co-Skipper steht zugleich in der Crew"),
            Self::DuplicateCrewMember(id) => write!(f, "Mitglied {id} steht mehrfach in der Crew"),
            Self::InvalidCrewPeriod(id) => {
                write!(f, "Zeitraum an Bord von Mitglied {id} ist unzulässig")
            }
            Self::InvalidGuestPeriod(name) => {
                write!(f, "Zeitraum an Bord von Gast {name} ist unzulässig")
            }
        }
    }
}

impl Reservation {
    pub fn new(
        boat: BoatId,
        start: Timestamp,
        end: Timestamp,
        area: SailingArea,
        skipper: MemberId,
    ) -> Self {
        Self {
            id: ReservationId::new(),
            boat,
            start,
            end,
            area,
            tour: false,
            skipper,
            co_skipper: None,
            crew: Vec::new(),
            guests: Vec::new(),
            remark: None,
        }
    }

    /// Zeitraum an Bord: eigener Zeitraum oder der der Reservierung.
    pub fn period(
        &self,
        from: Option<Timestamp>,
        until: Option<Timestamp>,
    ) -> (Timestamp, Timestamp) {
        (from.unwrap_or(self.start), until.unwrap_or(self.end))
    }

    fn period_is_valid(&self, from: Option<Timestamp>, until: Option<Timestamp>) -> bool {
        let (from, until) = self.period(from, until);
        self.start <= from && until <= self.end && from < until
    }

    /// Alle Mitglieder an Bord: Skipper, Co-Skipper und Crew.
    pub fn members_on_board(&self) -> Vec<MemberId> {
        let mut members = vec![self.skipper];
        members.extend(self.co_skipper);
        members.extend(self.crew.iter().map(|seat| seat.member));
        members
    }

    pub fn validate(&self) -> Result<(), Vec<ReservationError>> {
        let mut errors = Vec::new();
        if self.end <= self.start {
            errors.push(ReservationError::EndNotAfterStart);
        }
        if self.co_skipper == Some(self.skipper) {
            errors.push(ReservationError::SkipperIsCoSkipper);
        }
        if self.crew.iter().any(|seat| seat.member == self.skipper) {
            errors.push(ReservationError::SkipperInCrew);
        }
        if self
            .co_skipper
            .is_some_and(|co| self.crew.iter().any(|seat| seat.member == co))
        {
            errors.push(ReservationError::CoSkipperInCrew);
        }
        for (i, seat) in self.crew.iter().enumerate() {
            let first = self.crew.iter().position(|s| s.member == seat.member) == Some(i);
            let repeated = self.crew[i + 1..].iter().any(|s| s.member == seat.member);
            if first && repeated {
                errors.push(ReservationError::DuplicateCrewMember(seat.member));
            }
        }
        if self.end > self.start {
            for seat in &self.crew {
                if !self.period_is_valid(seat.from, seat.until) {
                    errors.push(ReservationError::InvalidCrewPeriod(seat.member));
                }
            }
            for guest in &self.guests {
                if !self.period_is_valid(guest.from, guest.until) {
                    errors.push(ReservationError::InvalidGuestPeriod(guest.name.clone()));
                }
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Betreffen beide Reservierungen dasselbe Boot in sich überschneidenden
    /// Zeiträumen? Aneinandergrenzende Zeiträume überschneiden sich nicht.
    pub fn overlaps(&self, other: &Reservation) -> bool {
        self.boat == other.boat && self.start < other.end && other.start < self.end
    }
}

impl ClubRecord {
    pub fn reservation(&self, id: ReservationId) -> Option<&Reservation> {
        self.reservations.iter().find(|r| r.id == id)
    }

    /// Prüft eine Reservierung zum Beginn gegen die Segelordnung und meldet
    /// alle Beanstandungen.
    pub fn check_reservation(&self, r: &Reservation) -> Result<(), Vec<ReservationIssue>> {
        let mut issues: Vec<ReservationIssue> = r
            .validate()
            .err()
            .unwrap_or_default()
            .into_iter()
            .map(ReservationIssue::Invalid)
            .collect();
        let t = r.start;

        let unknown: Vec<MemberId> = r
            .members_on_board()
            .into_iter()
            .filter(|&m| self.member(m).is_none())
            .collect();
        issues.extend(unknown.iter().map(|&m| ReservationIssue::UnknownMember(m)));
        let known = |m: &MemberId| !unknown.contains(m);

        let Some(boat) = self.boat(r.boat) else {
            issues.push(ReservationIssue::UnknownBoat(r.boat));
            return Err(issues);
        };
        if let Err(reasons) = boat.availability_at(t) {
            issues.extend(reasons.into_iter().map(ReservationIssue::BoatUnavailable));
        }

        if known(&r.skipper)
            && let Err(found) = self.skipper_eligibility(r.skipper, r.boat, t)
        {
            issues.push(ReservationIssue::Skipper(found));
        }
        if let Some(co) = r.co_skipper.filter(known)
            && let Err(found) = self.co_skipper_eligibility(co, r.boat, r.area, t)
        {
            // Ein fehlendes Fahrtgebiet wird unten einmal für die ganze Reservierung gemeldet.
            let found: Vec<EligibilityIssue> = found
                .into_iter()
                .filter(|i| !matches!(i, EligibilityIssue::AreaNotReleased(_)))
                .collect();
            if !found.is_empty() {
                issues.push(ReservationIssue::CoSkipper(found));
            }
        }
        for seat in r.crew.iter().filter(|seat| known(&seat.member)) {
            if let Err(reasons) = self.sailing_permit_at(seat.member, t) {
                issues.push(ReservationIssue::NoSailingPermit {
                    member: seat.member,
                    reasons,
                });
            }
        }

        match boat.crew_rule(r.area) {
            None => issues.push(ReservationIssue::AreaNotReleased(r.area)),
            Some(rule) => {
                // Jede eingetragene Person zählt einmal, unabhängig von eigenen
                // Zeiträumen an Bord; die zeitliche Prüfung macht der Vorstand.
                let members = r.members_on_board().len() as u32;
                let persons = members + r.guests.len() as u32;
                if rule.co_skipper_required && r.co_skipper.is_none() {
                    issues.push(ReservationIssue::CoSkipperMissing);
                }
                if members < u32::from(rule.min_members) {
                    issues.push(ReservationIssue::TooFewMembers {
                        actual: members,
                        min: rule.min_members.into(),
                    });
                }
                if persons < u32::from(rule.min_persons) {
                    issues.push(ReservationIssue::TooFewPersons {
                        actual: persons,
                        min: rule.min_persons.into(),
                    });
                }
            }
        }

        if issues.is_empty() {
            Ok(())
        } else {
            Err(issues)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boat::BoatUnavailable;
    use crate::boat_permit::PermitMissing;
    use crate::example::{self, at, *};
    use crate::sailing_permit::SailingPermitMissing;

    fn find(club: &ClubRecord, id: ReservationId) -> Reservation {
        club.reservation(id).unwrap().clone()
    }

    #[test]
    fn reservation_survives_round_trip() {
        let club = example::club();
        let tour = find(&club, DEHLER_TOUR);
        assert_eq!(tour.co_skipper, Some(KARIN));
        assert_eq!(tour.guests[0].name, "Max Gast");
        let json = serde_json::to_string(&tour).unwrap();
        assert_eq!(serde_json::from_str::<Reservation>(&json).unwrap(), tour);

        let mut value = serde_json::to_value(find(&club, JOLLE_SAMSTAG)).unwrap();
        assert!(value.get("tour").is_none());
        value.as_object_mut().unwrap().remove("tour");
        assert!(!serde_json::from_value::<Reservation>(value).unwrap().tour);
    }

    #[test]
    fn plausibility() {
        let mut r = Reservation::new(
            SCHRUVTREKKER,
            at(2026, 6, 13, 18, 0),
            at(2026, 6, 13, 10, 0),
            SailingArea::HomeWaters,
            ANNA,
        );
        assert_eq!(r.validate(), Err(vec![ReservationError::EndNotAfterStart]));
        assert_eq!(
            ReservationError::EndNotAfterStart.to_string(),
            "Ende nicht nach Beginn"
        );
        r.end = r.start;
        assert_eq!(r.validate(), Err(vec![ReservationError::EndNotAfterStart]));

        let mut r = Reservation::new(
            SCHRUVTREKKER,
            at(2026, 6, 13, 10, 0),
            at(2026, 6, 13, 18, 0),
            SailingArea::HomeWaters,
            ANNA,
        );
        r.co_skipper = Some(ANNA);
        assert_eq!(
            r.validate(),
            Err(vec![ReservationError::SkipperIsCoSkipper])
        );
        r.co_skipper = Some(KARIN);
        r.crew = vec![
            CrewSeat::new(ANNA),
            CrewSeat::new(KARIN),
            CrewSeat::new(BEN),
            CrewSeat::new(BEN),
        ];
        assert_eq!(
            r.validate(),
            Err(vec![
                ReservationError::SkipperInCrew,
                ReservationError::CoSkipperInCrew,
                ReservationError::DuplicateCrewMember(BEN),
            ])
        );
    }

    #[test]
    fn periods_on_board_must_lie_within_reservation() {
        let club = example::club();
        let mut tour = find(&club, DEHLER_TOUR);
        assert_eq!(tour.validate(), Ok(()));
        let ben = tour.crew.iter().find(|s| s.member == BEN).unwrap();
        assert_eq!(
            tour.period(ben.from, ben.until),
            (tour.start, at(2026, 7, 7, 18, 0))
        );
        tour.crew[0].from = Some(at(2026, 6, 30, 10, 0));
        tour.guests[0].until = Some(at(2026, 7, 20, 10, 0));
        assert_eq!(
            tour.validate(),
            Err(vec![
                ReservationError::InvalidCrewPeriod(tour.crew[0].member),
                ReservationError::InvalidGuestPeriod("Max Gast".into())
            ])
        );
    }

    #[test]
    fn overlaps() {
        let a = Reservation::new(
            FILOU,
            at(2026, 6, 13, 10, 0),
            at(2026, 6, 13, 14, 0),
            SailingArea::HomeWaters,
            DAVID,
        );
        let b = Reservation::new(
            FILOU,
            at(2026, 6, 13, 14, 0),
            at(2026, 6, 13, 18, 0),
            SailingArea::HomeWaters,
            JONAS,
        );
        let c = Reservation::new(
            FILOU,
            at(2026, 6, 13, 13, 0),
            at(2026, 6, 13, 18, 0),
            SailingArea::HomeWaters,
            JONAS,
        );
        let d = Reservation {
            boat: DIPL_ING_LING,
            ..c.clone()
        };
        assert!(!a.overlaps(&b));
        assert!(a.overlaps(&c) && c.overlaps(&a));
        assert!(!a.overlaps(&d));
    }

    #[test]
    fn valid_dinghy_reservation() {
        let club = example::club();
        assert_eq!(club.check_reservation(&find(&club, JOLLE_SAMSTAG)), Ok(()));
    }

    #[test]
    fn crew_without_sailing_permit() {
        let club = example::club();
        let issues = club
            .check_reservation(&find(&club, TRIGON_WOCHENENDE))
            .unwrap_err();
        assert_eq!(
            issues,
            vec![ReservationIssue::NoSailingPermit {
                member: LUKAS,
                reasons: vec![
                    SailingPermitMissing::FeeArrears("240".parse().unwrap()),
                    SailingPermitMissing::HoursMissing { missing: 13.0 },
                ],
            }]
        );
        assert!(
            issues[0]
                .to_string()
                .starts_with("Mitglied 112 hat keine Segelerlaubnis")
        );
    }

    #[test]
    fn skipper_without_boat_permit() {
        let club = example::club();
        let mut r = find(&club, FOLKEBOOT_NACHMITTAG);
        r.skipper = JONAS;
        r.crew = vec![CrewSeat::new(KARIN)];
        let issues = club.check_reservation(&r).unwrap_err();
        assert!(matches!(&issues[..], [ReservationIssue::Skipper(found)]
            if found.contains(&EligibilityIssue::BoatPermit(PermitMissing::NoCheckout))));
    }

    #[test]
    fn blocked_and_unreleased_boats() {
        let mut club = example::club();
        club.block_boat(
            BEN,
            HARALD_BLAUZAHN,
            at(2026, 6, 18, 9, 0),
            None,
            "Ruderschaden",
        )
        .unwrap();
        let issues = club
            .check_reservation(&find(&club, FOLKEBOOT_NACHMITTAG))
            .unwrap_err();
        assert_eq!(
            issues,
            vec![ReservationIssue::BoatUnavailable(
                BoatUnavailable::Blocked {
                    reason: "Ruderschaden".into()
                }
            )]
        );
        assert!(issues[0].to_string().contains("Ruderschaden"));

        let mut next_year = find(&club, JOLLE_SAMSTAG);
        next_year.start = at(2027, 6, 12, 14, 0);
        next_year.end = at(2027, 6, 12, 18, 0);
        let issues = club.check_reservation(&next_year).unwrap_err();
        assert!(issues.contains(&ReservationIssue::BoatUnavailable(
            BoatUnavailable::NotReleased { season: 2027 }
        )));
    }

    #[test]
    fn unknown_member() {
        let club = example::club();
        let mut r = find(&club, FOLKEBOOT_NACHMITTAG);
        let stranger = MemberId::new(999).unwrap();
        r.crew = vec![CrewSeat::new(stranger)];
        assert_eq!(
            club.check_reservation(&r),
            Err(vec![ReservationIssue::UnknownMember(stranger)])
        );
    }

    #[test]
    fn minimum_crew() {
        let club = example::club();
        // Folkeboot im Heimrevier zu zweit
        assert_eq!(
            club.check_reservation(&find(&club, FOLKEBOOT_NACHMITTAG)),
            Ok(())
        );

        // Matcher außerhalb nur mit Skipper und Co-Skipper
        let mut trigon = find(&club, TRIGON_WOCHENENDE);
        trigon.crew.clear();
        trigon.guests.clear();
        assert_eq!(
            club.check_reservation(&trigon),
            Err(vec![ReservationIssue::TooFewPersons { actual: 2, min: 3 }])
        );

        // Co-Skipper verlangt, aber nicht eingetragen
        trigon.co_skipper = None;
        trigon.crew = vec![CrewSeat::new(FRANK), CrewSeat::new(KARIN)];
        assert_eq!(
            club.check_reservation(&trigon),
            Err(vec![ReservationIssue::CoSkipperMissing])
        );

        // Gäste zählen nicht als Mitglieder
        let mut folkeboot = find(&club, FOLKEBOOT_NACHMITTAG);
        folkeboot.crew.clear();
        folkeboot.guests = vec![Guest::new("Paula Gast")];
        assert_eq!(
            club.check_reservation(&folkeboot),
            Err(vec![ReservationIssue::TooFewMembers { actual: 1, min: 2 }])
        );

        // Jolle außerhalb des Heimreviers
        let mut filou = find(&club, JOLLE_SAMSTAG);
        filou.area = SailingArea::Outside;
        let issues = club.check_reservation(&filou).unwrap_err();
        assert_eq!(
            issues,
            vec![ReservationIssue::AreaNotReleased(SailingArea::Outside)]
        );
        assert_eq!(
            issues[0].to_string(),
            "Fahrtgebiet nicht freigegeben (außerhalb des Heimreviers)"
        );
    }

    #[test]
    fn crew_change_is_not_checked_over_time() {
        let club = example::club();
        let mut tour = find(&club, DEHLER_TOUR);
        // Nur Skipper, Co-Skipper und zwei nacheinander an Bord befindliche Crew-Mitglieder
        tour.crew
            .retain(|seat| seat.member == BEN || seat.member == DAVID);
        tour.guests.clear();
        assert!(
            tour.crew
                .iter()
                .all(|s| s.from.is_some() || s.until.is_some())
        );
        assert_eq!(club.check_reservation(&tour), Ok(()));
    }
}
