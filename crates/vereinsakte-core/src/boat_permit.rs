//! Bootserlaubnis (Bootsführererlaubnis): erworben durch eine vereinsinterne
//! Abnahme, gültig bis auf Widerruf, ruhend ohne gültige Meldeadresse.

use std::fmt;

use serde::{Deserialize, Serialize};

use super::boat::SailingArea;
use super::club_record::ClubRecord;
use super::ids::{BoatId, CheckoutId, MemberId, RevocationId};
use super::member::Member;
use super::requirement::{AnyOf, Qualification, Requirement};
use super::sailing_permit::SailingPermitMissing;
use super::time::Timestamp;

/// Ergebnis einer Bootsabnahme.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckoutResult {
    Passed,
    Failed,
}

/// Bootsabnahme eines Mitglieds für ein Boot.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoatCheckout {
    pub id: CheckoutId,
    pub member: MemberId,
    pub boat: BoatId,
    pub at: Timestamp,
    pub result: CheckoutResult,
    /// Abnehmendes Mitglied.
    pub examiner: MemberId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remark: Option<String>,
}

/// Widerruf einer Bootserlaubnis durch den Vorstand.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Revocation {
    pub id: RevocationId,
    pub member: MemberId,
    pub boat: BoatId,
    pub at: Timestamp,
    pub reason: String,
    pub by: MemberId,
}

/// Grund, warum keine Bootserlaubnis besteht.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PermitMissing {
    /// Keine bestandene Abnahme für dieses Boot.
    NoCheckout,
    /// Die Erlaubnis wurde widerrufen.
    Revoked,
    /// Die Erlaubnis ruht, weil die Meldeadresse ungültig ist.
    AddressInvalid,
}

impl fmt::Display for PermitMissing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NoCheckout => "keine bestandene Bootsabnahme",
            Self::Revoked => "Bootserlaubnis widerrufen",
            Self::AddressInvalid => "Bootserlaubnis ruht: Meldeadresse ungültig",
        })
    }
}

/// Abnahmen und Widerrufe eines Mitglieds – genug, um seine Bootserlaubnisse
/// zu bestimmen, ohne die übrige Vereinsakte.
#[derive(Clone, Debug)]
pub struct PermitHistory<'a> {
    pub member: MemberId,
    /// Gültige Meldeadresse? Sonst ruhen alle Bootserlaubnisse.
    pub address_valid: bool,
    pub checkouts: Vec<&'a BoatCheckout>,
    pub revocations: Vec<&'a Revocation>,
}

impl PermitHistory<'_> {
    /// Hat das Mitglied zum Zeitpunkt die Bootserlaubnis für das Boot?
    /// Einträge anderer Mitglieder werden ignoriert.
    pub fn boat_permit(&self, boat: BoatId, t: Timestamp) -> Result<(), PermitMissing> {
        let last_passed = self
            .checkouts
            .iter()
            .filter(|c| {
                c.member == self.member
                    && c.boat == boat
                    && c.result == CheckoutResult::Passed
                    && c.at <= t
            })
            .map(|c| c.at)
            .max()
            .ok_or(PermitMissing::NoCheckout)?;
        let revoked = self
            .revocations
            .iter()
            .any(|r| r.member == self.member && r.boat == boat && r.at >= last_passed && r.at <= t);
        if revoked {
            return Err(PermitMissing::Revoked);
        }
        if !self.address_valid {
            return Err(PermitMissing::AddressInvalid);
        }
        Ok(())
    }
}

/// Grund, warum ein Mitglied ein Boot nicht als Skipper bzw. Co-Skipper führen darf.
#[derive(Clone, Debug, PartialEq)]
pub enum EligibilityIssue {
    UnknownMember(MemberId),
    UnknownBoat(BoatId),
    NoSailingPermit(Vec<SailingPermitMissing>),
    BoatPermit(PermitMissing),
    QualificationMissing(AnyOf),
    /// Für das Fahrtgebiet gibt es keine Besatzungsregel (nur mit Vorstandsentscheidung).
    AreaNotReleased(SailingArea),
}

impl fmt::Display for EligibilityIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownMember(id) => write!(f, "unbekanntes Mitglied {id}"),
            Self::UnknownBoat(id) => write!(f, "unbekanntes Boot {id}"),
            Self::NoSailingPermit(reasons) => {
                let reasons: Vec<String> = reasons.iter().map(ToString::to_string).collect();
                write!(f, "keine Segelerlaubnis ({})", reasons.join(", "))
            }
            Self::BoatPermit(missing) => write!(f, "{missing}"),
            Self::QualificationMissing(group) => write!(f, "Nachweis fehlt: {group}"),
            Self::AreaNotReleased(area) => write!(f, "Fahrtgebiet nicht freigegeben ({area})"),
        }
    }
}

fn into_result(issues: Vec<EligibilityIssue>) -> Result<(), Vec<EligibilityIssue>> {
    if issues.is_empty() {
        Ok(())
    } else {
        Err(issues)
    }
}

impl ClubRecord {
    /// Darf das Mitglied das Boot zum Zeitpunkt als Skipper führen?
    pub fn skipper_eligibility(
        &self,
        member: MemberId,
        boat: BoatId,
        t: Timestamp,
    ) -> Result<(), Vec<EligibilityIssue>> {
        let Some(m) = self.member(member) else {
            return Err(vec![EligibilityIssue::UnknownMember(member)]);
        };
        let Some(b) = self.boat(boat) else {
            return Err(vec![EligibilityIssue::UnknownBoat(boat)]);
        };
        let mut issues = Vec::new();
        if let Err(reasons) = self.sailing_permit_at(member, t) {
            issues.push(EligibilityIssue::NoSailingPermit(reasons));
        }
        if let Err(missing) = self.boat_permit(member, boat, t) {
            issues.push(EligibilityIssue::BoatPermit(missing));
        }
        if let Err(groups) = self.check_requirement(m, &b.skipper_requirement, t) {
            issues.extend(
                groups
                    .into_iter()
                    .map(EligibilityIssue::QualificationMissing),
            );
        }
        into_result(issues)
    }

    /// Ist das Mitglied im Fahrtgebiet als Co-Skipper des Bootes geeignet?
    pub fn co_skipper_eligibility(
        &self,
        member: MemberId,
        boat: BoatId,
        area: SailingArea,
        t: Timestamp,
    ) -> Result<(), Vec<EligibilityIssue>> {
        let Some(m) = self.member(member) else {
            return Err(vec![EligibilityIssue::UnknownMember(member)]);
        };
        let Some(b) = self.boat(boat) else {
            return Err(vec![EligibilityIssue::UnknownBoat(boat)]);
        };
        let mut issues = Vec::new();
        if let Err(reasons) = self.sailing_permit_at(member, t) {
            issues.push(EligibilityIssue::NoSailingPermit(reasons));
        }
        match b.crew_rule(area) {
            None => issues.push(EligibilityIssue::AreaNotReleased(area)),
            Some(rule) => {
                if let Err(groups) = self.check_requirement(m, &rule.co_skipper_requirement, t) {
                    issues.extend(
                        groups
                            .into_iter()
                            .map(EligibilityIssue::QualificationMissing),
                    );
                }
            }
        }
        into_result(issues)
    }

    /// Hat das Mitglied zum Zeitpunkt die Bootserlaubnis für das Boot?
    pub fn boat_permit(
        &self,
        member: MemberId,
        boat: BoatId,
        t: Timestamp,
    ) -> Result<(), PermitMissing> {
        self.permit_history(member).boat_permit(boat, t)
    }

    /// Abnahmen und Widerrufe eines Mitglieds.
    pub fn permit_history(&self, member: MemberId) -> PermitHistory<'_> {
        PermitHistory {
            member,
            address_valid: self.member(member).is_none_or(|m| m.address_valid),
            checkouts: self
                .boat_checkouts
                .iter()
                .filter(|c| c.member == member)
                .collect(),
            revocations: self
                .revocations
                .iter()
                .filter(|r| r.member == member)
                .collect(),
        }
    }

    fn has_qualification(
        &self,
        member: &Member,
        qualification: &Qualification,
        t: Timestamp,
    ) -> bool {
        match qualification {
            Qualification::SailingLicense(kind) => member.has_sailing_license(kind),
            Qualification::RadioLicense(kind) => member.has_radio_license(kind),
            Qualification::BoatPermit(boat) => self.boat_permit(member.id, *boat, t).is_ok(),
            Qualification::BoatPermitForType(boat_type) => self
                .boats
                .iter()
                .filter(|b| b.boat_type == *boat_type)
                .any(|b| self.boat_permit(member.id, b.id, t).is_ok()),
        }
    }

    /// Prüft eine Anforderung und liefert alle nicht erfüllten Gruppen.
    pub fn check_requirement(
        &self,
        member: &Member,
        requirement: &Requirement,
        t: Timestamp,
    ) -> Result<(), Vec<AnyOf>> {
        let missing: Vec<AnyOf> = requirement
            .0
            .iter()
            .filter(|group| !group.0.iter().any(|q| self.has_qualification(member, q, t)))
            .cloned()
            .collect();
        if missing.is_empty() {
            Ok(())
        } else {
            Err(missing)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::example::{self, at, *};
    use crate::member::RadioLicenseType;

    #[test]
    fn checkouts_and_revocations_survive_round_trip() {
        let club = example::club();
        let json = serde_json::to_string(&club.boat_checkouts).unwrap();
        assert_eq!(
            serde_json::from_str::<Vec<BoatCheckout>>(&json).unwrap(),
            club.boat_checkouts
        );
        let json = serde_json::to_string(&club.revocations).unwrap();
        assert_eq!(
            serde_json::from_str::<Vec<Revocation>>(&json).unwrap(),
            club.revocations
        );
    }

    #[test]
    fn examiner_is_required() {
        let mut value = serde_json::to_value(&example::club().boat_checkouts[0]).unwrap();
        value.as_object_mut().unwrap().remove("examiner");
        assert!(serde_json::from_value::<BoatCheckout>(value).is_err());
    }

    #[test]
    fn permit_lasts_until_revoked() {
        let club = example::club();
        let t = at(2026, 6, 1, 12, 0);
        // Karin hat die Folkeboot-Abnahme 2019 bestanden.
        assert_eq!(club.boat_permit(KARIN, HARALD_BLAUZAHN, t), Ok(()));
        assert_eq!(
            club.boat_permit(KARIN, TRIGON, t),
            Err(PermitMissing::NoCheckout)
        );
        // Bens Trigon-Erlaubnis wurde im September 2025 widerrufen.
        assert_eq!(club.boat_permit(BEN, TRIGON, at(2025, 8, 1, 12, 0)), Ok(()));
        assert_eq!(
            club.boat_permit(BEN, TRIGON, t),
            Err(PermitMissing::Revoked)
        );
    }

    #[test]
    fn new_checkout_after_revocation_restores_permit() {
        let mut club = example::club();
        club.boat_checkouts.push(BoatCheckout {
            id: CheckoutId::new(),
            member: BEN,
            boat: TRIGON,
            at: at(2026, 5, 16, 14, 0),
            result: CheckoutResult::Passed,
            examiner: FRANK,
            remark: None,
        });
        assert_eq!(
            club.boat_permit(BEN, TRIGON, at(2026, 5, 1, 12, 0)),
            Err(PermitMissing::Revoked)
        );
        assert_eq!(club.boat_permit(BEN, TRIGON, at(2026, 6, 1, 12, 0)), Ok(()));
    }

    #[test]
    fn failed_checkouts_grant_nothing() {
        let mut club = example::club();
        club.boat_checkouts.push(BoatCheckout {
            id: CheckoutId::new(),
            member: DAVID,
            boat: HARALD_BLAUZAHN,
            at: at(2026, 5, 9, 14, 0),
            result: CheckoutResult::Failed,
            examiner: FRANK,
            remark: Some("Mann-über-Bord-Manöver wiederholen".into()),
        });
        assert_eq!(
            club.boat_permit(DAVID, HARALD_BLAUZAHN, at(2026, 6, 1, 12, 0)),
            Err(PermitMissing::NoCheckout)
        );
    }

    #[test]
    fn permit_rests_without_valid_address() {
        let mut club = example::club();
        let t = at(2026, 6, 1, 12, 0);
        club.member_mut(CLARA).unwrap().address_valid = false;
        let err = club.boat_permit(CLARA, FILOU, t).unwrap_err();
        assert_eq!(err, PermitMissing::AddressInvalid);
        assert!(err.to_string().contains("Meldeadresse ungültig"));
        club.member_mut(CLARA).unwrap().address_valid = true;
        assert_eq!(club.boat_permit(CLARA, FILOU, t), Ok(()));
    }

    #[test]
    fn eligible_skipper() {
        let club = example::club();
        assert_eq!(
            club.skipper_eligibility(ANNA, SCHRUVTREKKER, at(2026, 6, 13, 10, 0)),
            Ok(())
        );
        // Jolle: die Bootserlaubnis genügt, kein Schein nötig.
        assert_eq!(
            club.skipper_eligibility(DAVID, DIPL_ING_LING, at(2026, 6, 13, 10, 0)),
            Ok(())
        );
    }

    #[test]
    fn skipper_issues_are_all_reported() {
        let club = example::club();
        let issues = club
            .skipper_eligibility(KARIN, SCHRUVTREKKER, at(2026, 6, 13, 10, 0))
            .unwrap_err();
        assert_eq!(
            issues,
            vec![
                EligibilityIssue::BoatPermit(PermitMissing::NoCheckout),
                EligibilityIssue::QualificationMissing(AnyOf::radio_licenses(&[
                    RadioLicenseType::Src,
                    RadioLicenseType::Lrc
                ])),
            ]
        );
        assert_eq!(issues[1].to_string(), "Nachweis fehlt: [SRC | LRC]");
        // Clara fehlen 2026 Arbeitsstunden – damit auch die Segelerlaubnis.
        let clara = club
            .skipper_eligibility(CLARA, FILOU, at(2026, 6, 13, 10, 0))
            .unwrap_err();
        assert!(matches!(&clara[..], [EligibilityIssue::NoSailingPermit(_)]));
    }

    #[test]
    fn co_skipper_needs_rule_requirement_not_boat_permit() {
        let club = example::club();
        let t = at(2026, 7, 1, 10, 0);
        // Karin: Folkeboot-Erlaubnis und SBF See, aber keine Dehler-Erlaubnis.
        assert_eq!(
            club.co_skipper_eligibility(KARIN, SCHRUVTREKKER, SailingArea::Outside, t),
            Ok(())
        );
        // Jonas hat SBF See, aber keine Folkeboot-Erlaubnis.
        assert_eq!(
            club.co_skipper_eligibility(JONAS, SCHRUVTREKKER, SailingArea::Outside, t),
            Err(vec![EligibilityIssue::QualificationMissing(AnyOf(vec![
                Qualification::BoatPermit(HARALD_BLAUZAHN)
            ]))])
        );
        // Im Heimrevier genügen Segelkenntnisse.
        assert_eq!(
            club.co_skipper_eligibility(JONAS, SCHRUVTREKKER, SailingArea::HomeWaters, t),
            Ok(())
        );
        assert_eq!(
            club.co_skipper_eligibility(ANNA, FILOU, SailingArea::Outside, t),
            Err(vec![EligibilityIssue::AreaNotReleased(
                SailingArea::Outside
            )])
        );
    }

    #[test]
    fn requirements_are_checked_group_by_group() {
        let club = example::club();
        let t = at(2026, 6, 1, 12, 0);
        let karin = club.member(KARIN).unwrap();
        let david = club.member(DAVID).unwrap();
        let clara = club.member(CLARA).unwrap();

        assert_eq!(
            club.check_requirement(david, &Requirement::none(), t),
            Ok(())
        );
        // BR zählt als „mindestens SKS“.
        let sks_plus = Requirement::none().and(sks_or_higher());
        assert_eq!(club.check_requirement(karin, &sks_plus, t), Ok(()));
        // David hat nur die Jollenerlaubnis der Filou – das sind Segelkenntnisse.
        let skills = Requirement::none().and(sailing_skills());
        assert_eq!(club.check_requirement(david, &skills, t), Ok(()));
        assert!(club.check_requirement(david, &sks_plus, t).is_err());
        // Clara fehlt für die Dehler nur die Funklizenz.
        let dehler = example::schruvtrekker().skipper_requirement;
        assert_eq!(
            club.check_requirement(clara, &dehler, t),
            Err(vec![AnyOf::radio_licenses(&[
                RadioLicenseType::Src,
                RadioLicenseType::Lrc
            ])])
        );
    }
}
