//! Die Vereinsakte als Wurzelobjekt aller Daten.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::boat::Boat;
use super::boat_permit::{BoatCheckout, Revocation};
use super::fees::Season;
use super::ids::{BoatId, MemberId};
use super::issues::RecordIssue;
use super::member::Member;
use super::office::Office;
use super::reservation::Reservation;
use super::sailing_permit::AnnualRecord;
use super::task::Task;
use super::work_event::WorkEvent;
use super::work_hours::{HourTransfer, WorkEntry};

/// Vereinsakte: alle Daten des Vereins in einem JSON-Dokument.
///
/// Fehlende Listen gelten beim Einlesen als leer, leere Listen werden nicht
/// geschrieben.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ClubRecord {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub members: Vec<Member>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub boats: Vec<Boat>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub offices: Vec<Office>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub boat_checkouts: Vec<BoatCheckout>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub revocations: Vec<Revocation>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub work_entries: Vec<WorkEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hour_transfers: Vec<HourTransfer>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub work_events: Vec<WorkEvent>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tasks: Vec<Task>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub seasons: Vec<Season>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub annual_records: Vec<AnnualRecord>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reservations: Vec<Reservation>,
}

impl ClubRecord {
    pub fn member(&self, id: MemberId) -> Option<&Member> {
        self.members.iter().find(|m| m.id == id)
    }

    pub fn boat(&self, id: BoatId) -> Option<&Boat> {
        self.boats.iter().find(|b| b.id == id)
    }

    pub fn boat_mut(&mut self, id: BoatId) -> Option<&mut Boat> {
        self.boats.iter_mut().find(|b| b.id == id)
    }

    pub fn member_mut(&mut self, id: MemberId) -> Option<&mut Member> {
        self.members.iter_mut().find(|m| m.id == id)
    }

    /// Prüft Akte-weite Invarianten und meldet alle Verstöße.
    pub fn validate(&self) -> Result<(), Vec<RecordIssue>> {
        let mut issues = Vec::new();
        self.check_member_ids(&mut issues);
        self.check_board_offices(&mut issues);
        self.check_work_hours(&mut issues);
        self.check_recorded_actions(&mut issues);
        if issues.is_empty() {
            Ok(())
        } else {
            Err(issues)
        }
    }

    fn check_work_hours(&self, issues: &mut Vec<RecordIssue>) {
        for entry in &self.work_entries {
            if let Err(found) = self.validate_work_entry(entry) {
                issues.extend(
                    found
                        .into_iter()
                        .map(|issue| RecordIssue::InvalidWorkEntry {
                            entry: entry.id,
                            issue,
                        }),
                );
            }
        }
        for transfer in &self.hour_transfers {
            if let Err(errors) = transfer.validate() {
                issues.extend(
                    errors
                        .into_iter()
                        .map(|error| RecordIssue::InvalidHourTransfer {
                            transfer: transfer.id,
                            error,
                        }),
                );
            }
        }
    }

    fn check_member_ids(&self, issues: &mut Vec<RecordIssue>) {
        let mut counts = BTreeMap::new();
        for member in &self.members {
            *counts.entry(member.id).or_insert(0) += 1;
        }
        issues.extend(
            counts
                .into_iter()
                .filter(|&(_, n)| n > 1)
                .map(|(id, _)| RecordIssue::DuplicateMemberId(id)),
        );
        let mut logins = BTreeMap::new();
        for login in self.members.iter().filter_map(|m| m.login_email.as_deref()) {
            *logins.entry(login.to_lowercase()).or_insert(0) += 1;
        }
        issues.extend(
            logins
                .into_iter()
                .filter(|&(_, n)| n > 1)
                .map(|(login, _)| RecordIssue::DuplicateLoginEmail(login)),
        );
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn duplicate_login_email_is_reported_case_insensitively() {
        let mut club = crate::example::club();
        club.member_mut(crate::example::BEN).unwrap().login_email =
            Some("Ben.Hansen@sgfh.example".into());
        club.member_mut(crate::example::GRETA).unwrap().login_email =
            Some("ben.hansen@sgfh.example".into());
        assert_eq!(
            club.validate(),
            Err(vec![RecordIssue::DuplicateLoginEmail(
                "ben.hansen@sgfh.example".into()
            )])
        );
    }

    use super::*;
    use crate::example;

    #[test]
    fn empty_record_is_empty_object() {
        let record: ClubRecord = serde_json::from_str("{}").unwrap();
        assert_eq!(record, ClubRecord::default());
        assert_eq!(serde_json::to_string(&record).unwrap(), "{}");
    }

    #[test]
    fn duplicate_member_number_is_reported() {
        let mut club = example::club();
        assert_eq!(club.validate(), Ok(()));
        let mut twin = example::ben();
        twin.id = example::ANNA;
        twin.login_email = None;
        club.members.push(twin);
        let issues = club.validate().unwrap_err();
        assert_eq!(issues, vec![RecordIssue::DuplicateMemberId(example::ANNA)]);
        assert!(issues[0].to_string().contains("101"));
    }
}
