//! Mitgliedskonto: Forderungen des Vereins und Einzahlungen eines Mitglieds.
//!
//! Einzahlungen werden keiner Forderung zugeordnet; was bezahlt ist, wird
//! berechnet, indem die Summe der Einzahlungen die Forderungen in der
//! Reihenfolge ihrer Fälligkeit tilgt.

use std::fmt;

use serde::{Deserialize, Serialize};

use super::club_record::ClubRecord;
use super::ids::{ClaimId, MemberId, PaymentId, ReservationId};
use super::money::Money;
use super::time::Timestamp;

/// Konto eines Mitglieds mit Forderungen und Einzahlungen.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MemberAccount {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub claims: Vec<Claim>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub payments: Vec<Payment>,
}

/// Eine Forderung des Vereins an ein Mitglied.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Claim {
    pub id: ClaimId,
    pub kind: ClaimKind,
    pub amount: Money,
    pub text: String,
    pub created_at: Timestamp,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due_at: Option<Timestamp>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cancelled: Option<Cancellation>,
}

/// Art einer Forderung.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimKind {
    AnnualFee {
        year: i32,
    },
    AdmissionFee,
    TourFee {
        reservation: ReservationId,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        guest: Option<String>,
    },
    WorkCompensation {
        year: i32,
    },
    Other,
}

/// Storno einer Forderung: Sie bleibt sichtbar, zählt aber nicht mehr.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cancellation {
    pub at: Timestamp,
    pub reason: String,
}

/// Eine Einzahlung eines Mitglieds.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Payment {
    pub id: PaymentId,
    pub amount: Money,
    pub at: Timestamp,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// Fehler einer Forderung oder Einzahlung.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AmountError {
    NotPositive,
}

impl fmt::Display for AmountError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Betrag muss größer als 0 sein")
    }
}

impl Claim {
    pub fn new(
        kind: ClaimKind,
        amount: Money,
        text: impl Into<String>,
        created_at: Timestamp,
        due_at: Option<Timestamp>,
    ) -> Self {
        Self {
            id: ClaimId::new(),
            kind,
            amount,
            text: text.into(),
            created_at,
            due_at,
            cancelled: None,
        }
    }

    pub fn validate(&self) -> Result<(), AmountError> {
        if self.amount.is_positive() {
            Ok(())
        } else {
            Err(AmountError::NotPositive)
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.is_some()
    }

    /// Storniert die Forderung; sie bleibt sichtbar, zählt aber nicht mehr.
    pub fn cancel(&mut self, at: Timestamp, reason: impl Into<String>) {
        self.cancelled = Some(Cancellation {
            at,
            reason: reason.into(),
        });
    }

    /// Zeitpunkt, nach dem getilgt wird: Fälligkeit, sonst Erstellung.
    fn settlement_order(&self) -> (Timestamp, Timestamp, ClaimId) {
        (
            self.due_at.unwrap_or(self.created_at),
            self.created_at,
            self.id,
        )
    }
}

impl Payment {
    pub fn new(amount: Money, at: Timestamp, text: Option<String>) -> Self {
        Self {
            id: PaymentId::new(),
            amount,
            at,
            text,
        }
    }

    pub fn validate(&self) -> Result<(), AmountError> {
        if self.amount.is_positive() {
            Ok(())
        } else {
            Err(AmountError::NotPositive)
        }
    }
}

/// Zahlstatus einer nicht stornierten Forderung.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClaimStatus {
    pub claim_id: ClaimId,
    pub amount: Money,
    /// Noch offener Betrag; 0 heißt bezahlt.
    pub open: Money,
}

impl ClaimStatus {
    pub fn is_paid(&self) -> bool {
        self.open == Money::ZERO
    }

    pub fn is_partially_paid(&self) -> bool {
        self.open > Money::ZERO && self.open < self.amount
    }
}

impl MemberAccount {
    pub fn is_empty(&self) -> bool {
        self.claims.is_empty() && self.payments.is_empty()
    }

    /// Gibt es bereits eine Forderung dieser Art – auch eine stornierte?
    pub fn has_claim(&self, kind: &ClaimKind) -> bool {
        self.claims.iter().any(|c| &c.kind == kind)
    }

    pub fn active_claims(&self) -> impl Iterator<Item = &Claim> {
        self.claims.iter().filter(|c| !c.is_cancelled())
    }

    /// Summe der nicht stornierten Forderungen.
    pub fn total_claimed(&self) -> Money {
        self.active_claims().map(|c| c.amount).sum()
    }

    pub fn total_paid(&self) -> Money {
        self.payments.iter().map(|p| p.amount).sum()
    }

    /// Kontostand: positiv = offener Betrag, negativ = Guthaben.
    pub fn balance(&self) -> Money {
        self.total_claimed() - self.total_paid()
    }

    /// Zahlstatus aller nicht stornierten Forderungen: Die Summe der
    /// Einzahlungen tilgt die Forderungen in der Reihenfolge ihrer Fälligkeit.
    pub fn claim_status(&self) -> Vec<ClaimStatus> {
        let mut claims: Vec<&Claim> = self.active_claims().collect();
        claims.sort_by_key(|c| c.settlement_order());
        let mut available = self.total_paid();
        claims
            .into_iter()
            .map(|claim| {
                let settled = available.min(claim.amount);
                available -= settled;
                ClaimStatus {
                    claim_id: claim.id,
                    amount: claim.amount,
                    open: claim.amount - settled,
                }
            })
            .collect()
    }

    /// Stand des Kontos zum Zeitpunkt `t`: nur bis dahin erstellte Forderungen
    /// (Stornos erst ab ihrem Zeitpunkt) und bis dahin eingegangene Einzahlungen.
    pub fn as_of(&self, t: Timestamp) -> MemberAccount {
        MemberAccount {
            claims: self
                .claims
                .iter()
                .filter(|c| c.created_at <= t)
                .cloned()
                .map(|mut c| {
                    c.cancelled = c.cancelled.filter(|cancel| cancel.at <= t);
                    c
                })
                .collect(),
            payments: self
                .payments
                .iter()
                .filter(|p| p.at <= t)
                .cloned()
                .collect(),
        }
    }

    /// Offener Betrag einer Forderung (`None` bei unbekannter oder stornierter Forderung).
    pub fn open_amount(&self, claim: ClaimId) -> Option<Money> {
        self.claim_status()
            .into_iter()
            .find(|s| s.claim_id == claim)
            .map(|s| s.open)
    }
}

/// Außenstände eines Mitglieds zu einem Zeitpunkt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Outstanding {
    pub member: MemberId,
    /// Gesamter offener Betrag.
    pub open: Money,
    /// Davon bereits fällig gewesen (Fälligkeit vor dem Zeitpunkt).
    pub overdue: Money,
}

impl ClubRecord {
    /// Trägt Forderungen in die Konten der Mitglieder ein und liefert die Anzahl
    /// eingetragener Forderungen; Forderungen an unbekannte Mitglieder werden
    /// übergangen.
    pub fn add_claims(&mut self, claims: Vec<(MemberId, Claim)>) -> usize {
        let mut added = 0;
        for (member, claim) in claims {
            if let Some(member) = self.member_mut(member) {
                member.account.claims.push(claim);
                added += 1;
            }
        }
        added
    }

    /// Alle Mitglieder mit offenem Betrag zum Zeitpunkt `t`.
    pub fn outstanding(&self, t: Timestamp) -> Vec<Outstanding> {
        self.members
            .iter()
            .filter_map(|member| {
                let account = &member.account.as_of(t);
                let statuses = account.claim_status();
                let open: Money = statuses.iter().map(|s| s.open).sum();
                if open == Money::ZERO {
                    return None;
                }
                let overdue = statuses
                    .iter()
                    .filter(|s| {
                        account
                            .claims
                            .iter()
                            .any(|c| c.id == s.claim_id && c.due_at.is_some_and(|due| due < t))
                    })
                    .map(|s| s.open)
                    .sum();
                Some(Outstanding {
                    member: member.id,
                    open,
                    overdue,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::model::example::{self, at, *};

    fn eur(s: &str) -> Money {
        s.parse().unwrap()
    }

    fn account(claims: &[(&str, Timestamp)], payments: &[&str]) -> MemberAccount {
        MemberAccount {
            claims: claims
                .iter()
                .map(|&(amount, due)| {
                    Claim::new(
                        ClaimKind::Other,
                        eur(amount),
                        "Forderung",
                        at(2026, 1, 1, 0, 0),
                        Some(due),
                    )
                })
                .collect(),
            payments: payments
                .iter()
                .map(|&a| Payment::new(eur(a), at(2026, 4, 3, 0, 0), None))
                .collect(),
        }
    }

    #[test]
    fn member_without_account_has_empty_account() {
        let value = serde_json::to_value(example::ben()).unwrap();
        assert!(value.get("account").is_none());
        let ben: crate::model::member::Member = serde_json::from_value(value).unwrap();
        assert!(ben.account.is_empty());
    }

    #[test]
    fn claim_kind_json_shape() {
        assert_eq!(
            serde_json::to_value(ClaimKind::AnnualFee { year: 2026 }).unwrap(),
            json!({"annual_fee": {"year": 2026}})
        );
        assert_eq!(
            serde_json::to_value(ClaimKind::AdmissionFee).unwrap(),
            json!("admission_fee")
        );
    }

    #[test]
    fn amounts_must_be_positive() {
        let zero = Claim::new(
            ClaimKind::Other,
            Money::ZERO,
            "x",
            at(2026, 1, 1, 0, 0),
            None,
        );
        assert_eq!(zero.validate(), Err(AmountError::NotPositive));
        assert_eq!(
            Payment::new(eur("-5"), at(2026, 1, 1, 0, 0), None).validate(),
            Err(AmountError::NotPositive)
        );
        assert_eq!(
            Payment::new(eur("5"), at(2026, 1, 1, 0, 0), None).validate(),
            Ok(())
        );
    }

    #[test]
    fn cancelled_claims_stay_visible_but_do_not_count() {
        let mut acc = account(
            &[("240", at(2026, 4, 1, 0, 0)), ("36", at(2026, 7, 21, 0, 0))],
            &[],
        );
        acc.claims[1].kind = ClaimKind::AdmissionFee;
        acc.claims[1].cancel(at(2026, 7, 22, 0, 0), "doppelt erfasst");
        assert_eq!(acc.claims.len(), 2);
        assert_eq!(acc.total_claimed(), eur("240"));
        assert_eq!(acc.claim_status().len(), 1);
        assert!(acc.has_claim(&ClaimKind::AdmissionFee));
    }

    #[test]
    fn balance() {
        let acc = account(
            &[("240", at(2026, 4, 1, 0, 0)), ("36", at(2026, 7, 21, 0, 0))],
            &["240"],
        );
        assert_eq!(acc.balance(), eur("36"));
    }

    #[test]
    fn payments_settle_oldest_claims_first() {
        let acc = account(
            &[("36", at(2026, 7, 21, 0, 0)), ("240", at(2026, 4, 1, 0, 0))],
            &["250"],
        );
        let status = acc.claim_status();
        assert_eq!(status[0].amount, eur("240"));
        assert!(status[0].is_paid());
        assert_eq!(status[1].open, eur("26"));
        assert!(status[1].is_partially_paid());
        let open: Money = status.iter().map(|s| s.open).sum();
        assert_eq!(open, acc.balance());
    }

    #[test]
    fn overpayment_is_credit() {
        let acc = account(&[("240", at(2026, 4, 1, 0, 0))], &["200", "100"]);
        assert!(acc.claim_status().iter().all(ClaimStatus::is_paid));
        assert_eq!(acc.balance(), eur("-60"));
    }

    #[test]
    fn outstanding_overview() {
        let mut club = example::club_without_accounts();
        club.member_mut(ANNA).unwrap().account = account(
            &[("36", at(2026, 7, 8, 0, 0)), ("50", at(2026, 8, 1, 0, 0))],
            &[],
        );
        club.member_mut(BEN).unwrap().account = account(&[("240", at(2026, 4, 1, 0, 0))], &["240"]);
        let overview = club.outstanding(at(2026, 7, 20, 0, 0));
        assert_eq!(
            overview,
            vec![Outstanding {
                member: ANNA,
                open: eur("86"),
                overdue: eur("36")
            }]
        );

        club.member_mut(ANNA)
            .unwrap()
            .account
            .payments
            .push(Payment::new(eur("86"), at(2026, 7, 21, 0, 0), None));
        assert!(club.outstanding(at(2026, 7, 22, 0, 0)).is_empty());
    }

    #[test]
    fn outstanding_in_example_club() {
        let club = example::club();
        let overview = club.outstanding(at(2026, 7, 1, 0, 0));
        assert_eq!(
            overview,
            vec![
                Outstanding {
                    member: CLARA,
                    open: eur("45"),
                    overdue: eur("45")
                },
                Outstanding {
                    member: LUKAS,
                    open: eur("240"),
                    overdue: eur("240")
                },
            ]
        );
        assert_eq!(
            club.member(ANNA).unwrap().account.balance(),
            eur("-10"),
            "Guthaben"
        );
        let ben = &club.member(BEN).unwrap().account;
        assert!(ben.claims[0].is_cancelled());
        assert_eq!(ben.balance(), Money::ZERO);
    }

    #[test]
    fn add_claims_to_member_accounts() {
        let mut club = example::club();
        let claim = Claim::new(
            ClaimKind::TourFee {
                reservation: crate::model::ids::ReservationId::new(),
                guest: None,
            },
            eur("36"),
            "Tourgebühr",
            at(2026, 7, 14, 0, 0),
            Some(at(2026, 7, 21, 0, 0)),
        );
        let unknown = crate::model::ids::MemberId::new(999).unwrap();
        assert_eq!(
            club.add_claims(vec![(ANNA, claim.clone()), (unknown, claim.clone())]),
            1
        );
        assert_eq!(
            club.member(ANNA).unwrap().account.claims.last(),
            Some(&claim)
        );
    }
}
