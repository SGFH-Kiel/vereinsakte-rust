//! Jährliche Segelerlaubnis aus Mitgliedschaft, Beitrag und Arbeitsstunden
//! (Segelordnung 1.1).

use std::fmt;

use chrono::Datelike;
use serde::{Deserialize, Serialize};

use super::account::{Claim, ClaimKind};
use super::club_record::ClubRecord;
use super::ids::MemberId;
use super::member::MembershipType;
use super::money::Money;
use super::time::Timestamp;
use super::work_hours::WorkEntry;

/// Toleranz beim Vergleich von Stunden.
const HOURS_EPSILON: f64 = 1e-9;

/// Jahresnachweis eines Mitglieds.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AnnualRecord {
    pub member: MemberId,
    pub year: i32,
    /// Vom Vorstand individuell reduziertes Soll.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reduced_quota: Option<Reduction>,
    /// Vom Vorstand genehmigte finanzielle Abgeltung von Arbeitsstunden.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compensation: Option<Compensation>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Reduction {
    pub hours: f64,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Compensation {
    pub hours: f64,
    /// Ohne Angabe: Stunden × Stundensatz der Saison.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount: Option<Money>,
    pub approved_at: Timestamp,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Grund, warum die Segelerlaubnis fehlt.
#[derive(Clone, Debug, PartialEq)]
pub enum SailingPermitMissing {
    /// Keine aktive Mitgliedschaft.
    Inactive,
    /// Offene Jahresbeiträge.
    FeeArrears(Money),
    HoursMissing {
        missing: f64,
    },
    /// Außerordentliches Mitglied ohne Nutzungszustimmung des Vorstands.
    NoBoardConsent,
}

impl fmt::Display for SailingPermitMissing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Inactive => f.write_str("keine aktive Mitgliedschaft"),
            Self::FeeArrears(amount) => write!(f, "Beitragsrückstand von {amount}"),
            Self::HoursMissing { missing } => {
                write!(f, "es fehlen {} Arbeitsstunden", format_hours(*missing))
            }
            Self::NoBoardConsent => f.write_str("keine Nutzungszustimmung des Vorstands"),
        }
    }
}

/// Stunden in deutscher Schreibweise ohne überflüssige Nachkommastellen, z. B. „4,5“.
fn format_hours(hours: f64) -> String {
    let text = format!("{hours:.2}");
    text.trim_end_matches('0')
        .trim_end_matches('.')
        .replace('.', ",")
}

impl ClubRecord {
    pub fn annual_record(&self, member: MemberId, year: i32) -> Option<&AnnualRecord> {
        self.annual_records
            .iter()
            .find(|r| r.member == member && r.year == year)
    }

    /// Betrag einer Abgeltung: angegebener Betrag oder Stunden × Stundensatz.
    pub fn compensation_amount(&self, year: i32, compensation: &Compensation) -> Money {
        compensation
            .amount
            .unwrap_or_else(|| self.compensation_rate(year).times_f64(compensation.hours))
    }

    /// Fehlende Forderungen zu genehmigten Abgeltungen.
    pub fn compensation_claims(&self, now: Timestamp) -> Vec<(MemberId, Claim)> {
        self.annual_records
            .iter()
            .filter_map(|record| {
                let compensation = record.compensation.as_ref()?;
                let kind = ClaimKind::WorkCompensation { year: record.year };
                let member = self.member(record.member)?;
                if member.account.has_claim(&kind) {
                    return None;
                }
                let amount = self.compensation_amount(record.year, compensation);
                let text = format!(
                    "Arbeitsstunden-Abgeltung {} ({} h)",
                    record.year, compensation.hours
                );
                Some((
                    record.member,
                    Claim::new(kind, amount, text, now, Some(compensation.approved_at)),
                ))
            })
            .collect()
    }

    /// Alle Arbeitseinsätze eines Mitglieds im Jahr, bestätigt oder nicht.
    pub fn work_entries_of(&self, member: MemberId, year: i32) -> Vec<&WorkEntry> {
        self.work_entries
            .iter()
            .filter(|e| e.member == member && e.year() == year)
            .collect()
    }

    /// Anrechenbare Stunden: bestätigte Einsätze + erhaltene − abgegebene
    /// Überträge + abgegoltene Stunden.
    pub fn credited_hours(&self, member: MemberId, year: i32) -> f64 {
        let worked: f64 = self
            .work_entries_of(member, year)
            .iter()
            .filter(|e| e.is_confirmed())
            .map(|e| e.hours())
            .sum();
        let transfers: f64 = self
            .hour_transfers
            .iter()
            .filter(|t| t.year == year)
            .map(|t| {
                if t.to == member {
                    t.hours
                } else if t.from == member {
                    -t.hours
                } else {
                    0.0
                }
            })
            .sum();
        let compensated = self
            .annual_record(member, year)
            .and_then(|r| r.compensation.as_ref())
            .map_or(0.0, |c| c.hours);
        worked + transfers + compensated
    }

    /// Soll-Stunden: 0 ohne Stundenpflicht, sonst reduziertes Soll oder Richtsatz.
    pub fn required_hours(&self, member: MemberId, year: i32) -> f64 {
        let Some(m) = self.member(member) else {
            return 0.0;
        };
        if !m.membership_type.owes_hours() {
            return 0.0;
        }
        self.annual_record(member, year)
            .and_then(|r| r.reduced_quota.as_ref())
            .map_or_else(|| self.hours_quota(year), |r| r.hours)
    }

    /// Offene Jahresbeiträge bis einschließlich `year`; fehlt bei
    /// Beitragspflicht die Forderung für `year`, zählt der volle Beitrag.
    pub fn fee_arrears(&self, member: MemberId, year: i32) -> Option<Money> {
        let m = self.member(member)?;
        let account = &m.account;
        let statuses = account.claim_status();
        let mut arrears: Money = account
            .active_claims()
            .filter(|c| matches!(c.kind, ClaimKind::AnnualFee { year: y } if y <= year))
            .filter_map(|c| statuses.iter().find(|s| s.claim_id == c.id).map(|s| s.open))
            .sum();
        if m.membership_type.pays_fee()
            && m.is_active_in(year)
            && !account.has_claim(&ClaimKind::AnnualFee { year })
        {
            arrears += self.annual_fee_for(m, year);
        }
        arrears.is_positive().then_some(arrears)
    }

    /// Hat das Mitglied im Jahr die Segelerlaubnis? Sonst alle Gründe.
    pub fn sailing_permit(
        &self,
        member: MemberId,
        year: i32,
    ) -> Result<(), Vec<SailingPermitMissing>> {
        let Some(m) = self.member(member) else {
            return Err(vec![SailingPermitMissing::Inactive]);
        };
        let mut missing = Vec::new();
        if !m.is_active_in(year) {
            missing.push(SailingPermitMissing::Inactive);
        }
        if let Some(arrears) = self.fee_arrears(member, year) {
            missing.push(SailingPermitMissing::FeeArrears(arrears));
        }
        let gap = self.required_hours(member, year) - self.credited_hours(member, year);
        if gap > HOURS_EPSILON {
            missing.push(SailingPermitMissing::HoursMissing { missing: gap });
        }
        if m.membership_type == MembershipType::Extraordinary && !m.board_consent {
            missing.push(SailingPermitMissing::NoBoardConsent);
        }
        if missing.is_empty() {
            Ok(())
        } else {
            Err(missing)
        }
    }

    /// Segelerlaubnis zu einem Zeitpunkt: zusätzlich muss das Mitglied dann aktiv sein.
    pub fn sailing_permit_at(
        &self,
        member: MemberId,
        t: Timestamp,
    ) -> Result<(), Vec<SailingPermitMissing>> {
        let mut missing = self
            .sailing_permit(member, t.year())
            .err()
            .unwrap_or_default();
        let active = self.member(member).is_some_and(|m| m.is_active_at(t));
        if !active && !missing.contains(&SailingPermitMissing::Inactive) {
            missing.insert(0, SailingPermitMissing::Inactive);
        }
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
    use crate::model::account::{MemberAccount, Payment};
    use crate::model::example::{self, at, *};
    use crate::model::work_hours::{WorkExtent, WorkSubject};

    fn eur(s: &str) -> Money {
        s.parse().unwrap()
    }

    #[test]
    fn compensation_amount_from_rate() {
        let club = example::club();
        let comp = Compensation {
            hours: 5.0,
            amount: None,
            approved_at: at(2026, 6, 1, 0, 0),
            reason: None,
        };
        assert_eq!(club.compensation_amount(2026, &comp), eur("75"));
        assert_eq!(
            serde_json::to_value(club.compensation_amount(2026, &comp)).unwrap(),
            "75.00"
        );
    }

    #[test]
    fn compensation_claims_without_duplicates() {
        let club = example::club_without_accounts();
        let claims = club.compensation_claims(at(2026, 6, 2, 10, 0));
        let (_, clara) = claims.iter().find(|(m, _)| *m == CLARA).unwrap();
        assert_eq!(clara.kind, ClaimKind::WorkCompensation { year: 2026 });
        assert_eq!(clara.amount, eur("45"));
        assert!(clara.text.starts_with("Arbeitsstunden-Abgeltung 2026"));
        let (_, jonas) = claims.iter().find(|(m, _)| *m == JONAS).unwrap();
        assert_eq!(jonas.amount, eur("375"));
        assert!(
            example::club()
                .compensation_claims(at(2026, 6, 2, 10, 0))
                .is_empty()
        );
    }

    #[test]
    fn credited_hours_count_confirmed_entries_transfers_and_compensation() {
        let club = example::club();
        // bestätigt 4 + 2,5 + 8, unbestätigt 6, +5 von Ben, −2 an David, +3 abgegolten
        assert_eq!(club.work_entries_of(CLARA, 2026).len(), 4);
        assert!(
            club.work_entries_of(CLARA, 2026)
                .iter()
                .any(|e| !e.is_confirmed())
        );
        assert_eq!(club.credited_hours(CLARA, 2026), 20.5);
        assert_eq!(club.credited_hours(BEN, 2026), 25.0);
        assert_eq!(club.credited_hours(JONAS, 2026), 25.0);
        // Silvesterwache zählt zu 2025.
        assert_eq!(club.credited_hours(FRANK, 2025), 3.0);
        assert_eq!(club.credited_hours(FRANK, 2026), 30.0);
    }

    #[test]
    fn other_years_do_not_count() {
        let mut club = example::club();
        let mut entry = crate::model::work_hours::WorkEntry::new(
            GRETA,
            WorkExtent::Amount {
                date: at(2025, 9, 1, 0, 0),
                hours: 8.0,
            },
            WorkSubject::Club,
        );
        entry.confirmation = Some(crate::model::work_hours::Confirmation {
            by: FRANK,
            at: at(2025, 9, 2, 0, 0),
        });
        club.work_entries.push(entry);
        assert_eq!(club.credited_hours(GRETA, 2025), 8.0);
        assert_eq!(club.credited_hours(GRETA, 2026), 0.0);
    }

    #[test]
    fn registration_alone_credits_nothing() {
        let mut club = example::club();
        let before = club.credited_hours(MALTE, 2026);
        club.work_events
            .iter_mut()
            .find(|e| e.id == AUSKRANEN)
            .unwrap()
            .register(LUKAS)
            .unwrap();
        assert_eq!(club.credited_hours(MALTE, 2026), before);
        assert_eq!(club.credited_hours(LUKAS, 2026), 12.0);
    }

    #[test]
    fn required_hours() {
        let club = example::club();
        assert_eq!(club.required_hours(ANNA, 2026), 25.0);
        assert_eq!(club.required_hours(EVA, 2026), 10.0, "reduziertes Soll");
        assert_eq!(
            club.required_hours(HANNES, 2026),
            0.0,
            "Kind eines Mitglieds"
        );
        assert_eq!(club.required_hours(INES, 2026), 0.0, "Erasmus");
        assert_eq!(
            club.annual_record(EVA, 2026)
                .unwrap()
                .reduced_quota
                .as_ref()
                .unwrap()
                .reason,
            "Alter, Absprache mit dem Vorstand"
        );
    }

    #[test]
    fn permit_when_everything_is_fine() {
        let club = example::club();
        for member in [
            ANNA, BEN, DAVID, EVA, FRANK, HANNES, INES, JONAS, KARIN, MALTE,
        ] {
            assert_eq!(club.sailing_permit(member, 2026), Ok(()), "{member}");
        }
    }

    #[test]
    fn missing_hours() {
        let mut club = example::club();
        club.work_entries
            .retain(|e| e.description.as_deref() != Some("Polster der Trigon gereinigt"));
        let missing = club.sailing_permit(ANNA, 2026).unwrap_err();
        assert_eq!(
            missing,
            vec![SailingPermitMissing::HoursMissing { missing: 5.0 }]
        );
        assert_eq!(missing[0].to_string(), "es fehlen 5 Arbeitsstunden");
        assert_eq!(
            club.sailing_permit(CLARA, 2026).unwrap_err()[0].to_string(),
            "es fehlen 4,5 Arbeitsstunden"
        );
    }

    #[test]
    fn fee_not_claimed_counts_as_arrears() {
        let mut club = example::club();
        club.member_mut(ANNA).unwrap().account = MemberAccount::default();
        assert_eq!(
            club.sailing_permit(ANNA, 2026),
            Err(vec![SailingPermitMissing::FeeArrears(eur("240"))])
        );
        // Ehrenmitglieder haben keine Beitragsforderung und keinen Rückstand.
        assert_eq!(club.fee_arrears(EVA, 2026), None);
    }

    #[test]
    fn payment_settles_older_tour_fee_first() {
        let mut club = example::club();
        let tour = Claim::new(
            ClaimKind::Other,
            eur("36"),
            "Tourgebühr",
            at(2026, 3, 1, 0, 0),
            Some(at(2026, 3, 15, 0, 0)),
        );
        let fee = Claim::new(
            ClaimKind::AnnualFee { year: 2026 },
            eur("240"),
            "Jahresbeitrag 2026",
            at(2026, 3, 15, 0, 0),
            Some(at(2026, 4, 1, 0, 0)),
        );
        club.member_mut(ANNA).unwrap().account = MemberAccount {
            claims: vec![fee, tour],
            payments: vec![Payment::new(eur("240"), at(2026, 4, 1, 0, 0), None)],
        };
        assert_eq!(club.fee_arrears(ANNA, 2026), Some(eur("36")));
        assert_eq!(
            club.sailing_permit(ANNA, 2026),
            Err(vec![SailingPermitMissing::FeeArrears(eur("36"))])
        );
    }

    #[test]
    fn several_reasons_at_once() {
        let club = example::club();
        let missing = club.sailing_permit(LUKAS, 2026).unwrap_err();
        assert_eq!(
            missing,
            vec![
                SailingPermitMissing::FeeArrears(eur("240")),
                SailingPermitMissing::HoursMissing { missing: 13.0 }
            ]
        );
        assert_eq!(missing[0].to_string(), "Beitragsrückstand von 240,00 €");
    }

    #[test]
    fn extraordinary_member_needs_board_consent() {
        let mut club = example::club();
        assert_eq!(
            club.sailing_permit(GRETA, 2026),
            Err(vec![SailingPermitMissing::NoBoardConsent])
        );
        club.member_mut(GRETA).unwrap().board_consent = true;
        assert_eq!(club.sailing_permit(GRETA, 2026), Ok(()));
    }

    #[test]
    fn inactive_members() {
        let mut club = example::club();
        assert_eq!(
            club.sailing_permit(INES, 2027).unwrap_err()[0],
            SailingPermitMissing::Inactive
        );
        // Ben scheidet zum 31. März 2026 aus: 2026 war er aktiv, im Juni aber nicht mehr.
        club.member_mut(BEN).unwrap().membership.left_at = Some(at(2026, 3, 31, 0, 0));
        assert_eq!(club.sailing_permit(BEN, 2026), Ok(()));
        assert_eq!(
            club.sailing_permit_at(BEN, at(2026, 6, 1, 12, 0)),
            Err(vec![SailingPermitMissing::Inactive])
        );
        assert_eq!(club.sailing_permit_at(ANNA, at(2026, 6, 1, 12, 0)), Ok(()));
    }
}
