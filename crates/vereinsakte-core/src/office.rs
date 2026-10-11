//! Ämter nach Satzung (§10 Vorstand, §11 Revisoren, §12 Bootswarte) und
//! Segelordnung (Abnehmer) mit Amtszeiten.

use std::fmt;

use serde::{Deserialize, Serialize};

use super::boat::{Block, Release};
use super::boat_permit::{BoatCheckout, CheckoutResult, Revocation};
use super::club_record::ClubRecord;
use super::ids::{BoatId, CheckoutId, MemberId, OfficeId, RevocationId, WorkEntryId};
use super::issues::RecordIssue;
use super::rights::{Authorizer, Capability};
use super::time::Timestamp;
use super::work_hours::Confirmation;

/// Rolle eines Amtes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// 1. Vorsitzende(r)
    Chair,
    /// 2. Vorsitzende(r)
    ViceChair,
    /// Kassenwart
    Treasurer,
    /// Schriftwart
    Secretary,
    /// Oberbootswart
    HeadBoatwarden,
    /// Revisor
    Auditor,
    /// Bootswart eines Bootes
    Boatwarden(BoatId),
    /// vom Vorstand bestellter Abnehmer für Bootserlaubnisse eines Bootes
    Examiner(BoatId),
}

impl Role {
    /// Gehört die Rolle zum Vorstand (Satzung §10 Abs. 2)?
    pub fn is_board(self) -> bool {
        matches!(
            self,
            Self::Chair
                | Self::ViceChair
                | Self::Treasurer
                | Self::Secretary
                | Self::HeadBoatwarden
        )
    }

    /// Vorstand im Sinne des § 26 BGB (Satzung §10 Abs. 3).
    pub fn is_legal_representative(self) -> bool {
        matches!(self, Self::Chair | Self::Treasurer | Self::Secretary)
    }
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Chair => f.write_str("1. Vorsitzende(r)"),
            Self::ViceChair => f.write_str("2. Vorsitzende(r)"),
            Self::Treasurer => f.write_str("Kassenwart"),
            Self::Secretary => f.write_str("Schriftwart"),
            Self::HeadBoatwarden => f.write_str("Oberbootswart"),
            Self::Auditor => f.write_str("Revisor"),
            Self::Boatwarden(boat) => write!(f, "Bootswart (Boot {boat})"),
            Self::Examiner(boat) => write!(f, "Abnehmer (Boot {boat})"),
        }
    }
}

/// Ein Amt eines Mitglieds; `end` ist exklusiv. Ämter werden nie gelöscht,
/// die Liste der Ämter ist die Historie.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Office {
    pub id: OfficeId,
    pub role: Role,
    pub member: MemberId,
    pub start: Timestamp,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<Timestamp>,
}

impl Office {
    pub fn is_active_at(&self, t: Timestamp) -> bool {
        self.start <= t && self.end.is_none_or(|end| t < end)
    }

    fn overlaps(&self, other: &Office) -> bool {
        self.end.is_none_or(|end| other.start < end) && other.end.is_none_or(|end| self.start < end)
    }
}

/// Ein besetztes Vorstandsamt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoardSeat {
    pub role: Role,
    pub member: MemberId,
    /// Vertretung nach § 26 BGB
    pub legal_representative: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OfficeError {
    /// Das Mitglied hat das Amt zu diesem Zeitpunkt nicht inne.
    NotHolding,
    /// Das Ende läge vor dem Beginn des Amtes.
    EndBeforeStart,
}

impl fmt::Display for OfficeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NotHolding => "Das Mitglied hat dieses Amt zu diesem Zeitpunkt nicht inne",
            Self::EndBeforeStart => "Das Amt kann nicht vor seinem Beginn abgegeben werden",
        })
    }
}

impl ClubRecord {
    /// Ernennt ein Mitglied ab `start` in ein Amt.
    pub fn appoint(&mut self, member: MemberId, role: Role, start: Timestamp) -> OfficeId {
        let id = OfficeId::new();
        self.offices.push(Office {
            id,
            role,
            member,
            start,
            end: None,
        });
        id
    }

    /// Das Mitglied gibt das Amt zum Zeitpunkt `at` ab.
    pub fn resign(
        &mut self,
        member: MemberId,
        role: Role,
        at: Timestamp,
    ) -> Result<(), OfficeError> {
        let office = self
            .offices
            .iter_mut()
            .filter(|o| o.member == member && o.role == role && o.end.is_none_or(|end| at < end))
            .min_by_key(|o| o.start)
            .ok_or(OfficeError::NotHolding)?;
        if at < office.start {
            return Err(OfficeError::EndBeforeStart);
        }
        office.end = Some(at);
        Ok(())
    }

    /// Alle zum Zeitpunkt aktiven Ämter.
    pub fn active_offices(&self, t: Timestamp) -> impl Iterator<Item = &Office> {
        self.offices.iter().filter(move |o| o.is_active_at(t))
    }

    pub fn office_queries(&self) -> Offices<'_> {
        Offices::new(&self.offices)
    }

    /// Inhaber einer Rolle zum Zeitpunkt.
    pub fn holders(&self, role: Role, t: Timestamp) -> Vec<MemberId> {
        self.office_queries().holders(role, t)
    }

    pub fn boatwardens(&self, boat: BoatId, t: Timestamp) -> Vec<MemberId> {
        self.office_queries().boatwardens(boat, t)
    }

    /// Besetzte Vorstandsämter zum Zeitpunkt.
    pub fn board(&self, t: Timestamp) -> Vec<BoardSeat> {
        self.office_queries().board(t)
    }

    /// Hat das Mitglied zum Zeitpunkt ein Amt, das `predicate` erfüllt?
    pub fn holds(&self, member: MemberId, t: Timestamp, predicate: impl Fn(Role) -> bool) -> bool {
        self.office_queries().holds(member, t, predicate)
    }

    pub fn is_board_member(&self, member: MemberId, t: Timestamp) -> bool {
        self.office_queries().is_board_member(member, t)
    }

    pub(crate) fn check_board_offices(&self, issues: &mut Vec<RecordIssue>) {
        let board: Vec<&Office> = self.offices.iter().filter(|o| o.role.is_board()).collect();
        for office in &board {
            let has_vote = self
                .member(office.member)
                .is_some_and(|m| m.has_voting_right_at(office.start));
            if !has_vote {
                issues.push(RecordIssue::BoardMemberWithoutVote {
                    member: office.member,
                    role: office.role,
                });
            }
        }
        for (i, a) in board.iter().enumerate() {
            for b in &board[i + 1..] {
                if !a.overlaps(b) {
                    continue;
                }
                if a.role == b.role {
                    issues.push(RecordIssue::BoardOfficeTwice { role: a.role });
                } else if a.member == b.member {
                    issues.push(RecordIssue::BoardOfficesAccumulated {
                        member: a.member,
                        roles: (a.role, b.role),
                    });
                }
            }
        }
    }
}

/// Abfragen auf einer Liste von Ämtern – etwa den aus der Datenbank geladenen
/// aktiven Ämtern, ohne die übrige Vereinsakte.
#[derive(Clone, Copy, Debug)]
pub struct Offices<'a> {
    offices: &'a [Office],
}

impl<'a> Offices<'a> {
    pub fn new(offices: &'a [Office]) -> Self {
        Self { offices }
    }

    pub fn active_at(&self, t: Timestamp) -> impl Iterator<Item = &'a Office> {
        self.offices.iter().filter(move |o| o.is_active_at(t))
    }

    pub fn holders(&self, role: Role, t: Timestamp) -> Vec<MemberId> {
        self.active_at(t)
            .filter(|o| o.role == role)
            .map(|o| o.member)
            .collect()
    }

    pub fn boatwardens(&self, boat: BoatId, t: Timestamp) -> Vec<MemberId> {
        self.holders(Role::Boatwarden(boat), t)
    }

    pub fn board(&self, t: Timestamp) -> Vec<BoardSeat> {
        self.active_at(t)
            .filter(|o| o.role.is_board())
            .map(|o| BoardSeat {
                role: o.role,
                member: o.member,
                legal_representative: o.role.is_legal_representative(),
            })
            .collect()
    }

    pub fn holds(&self, member: MemberId, t: Timestamp, predicate: impl Fn(Role) -> bool) -> bool {
        self.active_at(t)
            .any(|o| o.member == member && predicate(o.role))
    }

    pub fn is_board_member(&self, member: MemberId, t: Timestamp) -> bool {
        self.holds(member, t, Role::is_board)
    }

    /// Aktuelle Ämter eines Mitglieds.
    pub fn roles_of(&self, member: MemberId, t: Timestamp) -> Vec<Role> {
        self.active_at(t)
            .filter(|o| o.member == member)
            .map(|o| o.role)
            .collect()
    }
}

// --- Rechte ------------------------------------------------------------------

/// Handlung, für die ein Amt nötig ist.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    BlockBoat(BoatId),
    LiftBlock(BoatId),
    ReleaseBoat(BoatId),
    ConfirmWork(WorkEntryId),
    RecordCheckout(BoatId),
    Revoke(BoatId),
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BlockBoat(boat) => write!(f, "Sperre von Boot {boat}"),
            Self::LiftBlock(boat) => write!(f, "Aufhebung der Sperre von Boot {boat}"),
            Self::ReleaseBoat(boat) => write!(f, "Saisonfreigabe von Boot {boat}"),
            Self::ConfirmWork(entry) => write!(f, "Bestätigung von Arbeitseinsatz {entry}"),
            Self::RecordCheckout(boat) => write!(f, "Bootsabnahme für Boot {boat}"),
            Self::Revoke(boat) => write!(f, "Widerruf einer Bootserlaubnis für Boot {boat}"),
        }
    }
}

/// Grund, warum eine Handlung nicht erlaubt ist.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotAuthorized {
    /// Das Mitglied hat zu diesem Zeitpunkt kein berechtigendes Amt.
    MissingOffice,
    /// Eigene Arbeitseinsätze dürfen nicht selbst bestätigt werden.
    OwnWorkEntry,
    UnknownWorkEntry,
}

impl fmt::Display for NotAuthorized {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::MissingOffice => "kein berechtigendes Amt",
            Self::OwnWorkEntry => "eigene Arbeitseinsätze können nicht selbst bestätigt werden",
            Self::UnknownWorkEntry => "unbekannter Arbeitseinsatz",
        })
    }
}

/// Fehler einer Handlung, die die Vereinsakte ändert.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionError {
    NotAuthorized(NotAuthorized),
    UnknownBoat(BoatId),
    UnknownWorkEntry(WorkEntryId),
    /// Es gibt keine unbefristete Sperre, die aufgehoben werden könnte.
    NoOpenBlock,
}

impl From<NotAuthorized> for ActionError {
    fn from(reason: NotAuthorized) -> Self {
        Self::NotAuthorized(reason)
    }
}

impl fmt::Display for ActionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAuthorized(reason) => write!(f, "nicht berechtigt: {reason}"),
            Self::UnknownBoat(boat) => write!(f, "unbekanntes Boot {boat}"),
            Self::UnknownWorkEntry(entry) => write!(f, "unbekannter Arbeitseinsatz {entry}"),
            Self::NoOpenBlock => f.write_str("keine offene Sperre vorhanden"),
        }
    }
}

impl ClubRecord {
    /// Darf `actor` die Handlung zum Zeitpunkt `t` vornehmen?
    pub fn authorize(
        &self,
        actor: MemberId,
        action: &Action,
        t: Timestamp,
    ) -> Result<(), NotAuthorized> {
        let capability = match *action {
            Action::BlockBoat(boat) => Capability::BlockBoat(boat),
            Action::LiftBlock(boat) => Capability::LiftBlock(boat),
            Action::ReleaseBoat(boat) => Capability::ReleaseBoat(boat),
            Action::RecordCheckout(boat) => Capability::RecordCheckout(boat),
            Action::Revoke(boat) => Capability::Revoke(boat),
            Action::ConfirmWork(id) => {
                let entry = self.work_entry(id).ok_or(NotAuthorized::UnknownWorkEntry)?;
                Capability::ConfirmWork {
                    owner: entry.member,
                    subject: entry.subject,
                }
            }
        };
        Authorizer::new(self.office_queries(), actor, t).allows(&capability)
    }

    fn require_boat(&self, boat: BoatId) -> Result<(), ActionError> {
        self.boat(boat)
            .map(|_| ())
            .ok_or(ActionError::UnknownBoat(boat))
    }

    /// Sperrt ein Boot ab `start` (ohne `end` unbefristet).
    pub fn block_boat(
        &mut self,
        actor: MemberId,
        boat: BoatId,
        start: Timestamp,
        end: Option<Timestamp>,
        reason: impl Into<String>,
    ) -> Result<(), ActionError> {
        self.require_boat(boat)?;
        self.authorize(actor, &Action::BlockBoat(boat), start)?;
        let block = Block {
            start,
            end,
            reason: reason.into(),
            by: actor,
            lifted_by: None,
        };
        self.boat_mut(boat).expect("geprüft").blocks.push(block);
        Ok(())
    }

    /// Hebt die zum Zeitpunkt `at` bestehende unbefristete Sperre auf.
    pub fn lift_block(
        &mut self,
        actor: MemberId,
        boat: BoatId,
        at: Timestamp,
    ) -> Result<(), ActionError> {
        self.require_boat(boat)?;
        self.authorize(actor, &Action::LiftBlock(boat), at)?;
        let block = self
            .boat_mut(boat)
            .expect("geprüft")
            .blocks
            .iter_mut()
            .find(|b| b.end.is_none() && b.start <= at)
            .ok_or(ActionError::NoOpenBlock)?;
        block.end = Some(at);
        block.lifted_by = Some(actor);
        Ok(())
    }

    pub fn release_boat(
        &mut self,
        actor: MemberId,
        boat: BoatId,
        season: i32,
        at: Timestamp,
    ) -> Result<(), ActionError> {
        self.require_boat(boat)?;
        self.authorize(actor, &Action::ReleaseBoat(boat), at)?;
        self.boat_mut(boat)
            .expect("geprüft")
            .releases
            .push(Release {
                season,
                by: actor,
                at,
            });
        Ok(())
    }

    pub fn confirm_work(
        &mut self,
        actor: MemberId,
        entry: WorkEntryId,
        at: Timestamp,
    ) -> Result<(), ActionError> {
        if self.work_entry(entry).is_none() {
            return Err(ActionError::UnknownWorkEntry(entry));
        }
        self.authorize(actor, &Action::ConfirmWork(entry), at)?;
        let entry = self
            .work_entries
            .iter_mut()
            .find(|e| e.id == entry)
            .expect("geprüft");
        entry.confirmation = Some(Confirmation { by: actor, at });
        Ok(())
    }

    /// Erfasst eine Bootsabnahme; `examiner` ist das abnehmende Mitglied.
    pub fn record_checkout(
        &mut self,
        examiner: MemberId,
        member: MemberId,
        boat: BoatId,
        at: Timestamp,
        result: CheckoutResult,
        remark: Option<String>,
    ) -> Result<CheckoutId, ActionError> {
        self.require_boat(boat)?;
        self.authorize(examiner, &Action::RecordCheckout(boat), at)?;
        let id = CheckoutId::new();
        self.boat_checkouts.push(BoatCheckout {
            id,
            member,
            boat,
            at,
            result,
            examiner,
            remark,
        });
        Ok(id)
    }

    pub fn revoke(
        &mut self,
        by: MemberId,
        member: MemberId,
        boat: BoatId,
        at: Timestamp,
        reason: impl Into<String>,
    ) -> Result<RevocationId, ActionError> {
        self.require_boat(boat)?;
        self.authorize(by, &Action::Revoke(boat), at)?;
        let id = RevocationId::new();
        self.revocations.push(Revocation {
            id,
            member,
            boat,
            at,
            reason: reason.into(),
            by,
        });
        Ok(id)
    }

    /// Alle gespeicherten Handlungen mit Urheber und Zeitpunkt.
    fn recorded_actions(&self) -> Vec<(Action, MemberId, Timestamp)> {
        let mut actions = Vec::new();
        for boat in &self.boats {
            actions.extend(
                boat.releases
                    .iter()
                    .map(|r| (Action::ReleaseBoat(boat.id), r.by, r.at)),
            );
            for block in &boat.blocks {
                actions.push((Action::BlockBoat(boat.id), block.by, block.start));
                if let (Some(by), Some(end)) = (block.lifted_by, block.end) {
                    actions.push((Action::LiftBlock(boat.id), by, end));
                }
            }
        }
        actions.extend(self.work_entries.iter().filter_map(|e| {
            e.confirmation
                .map(|c| (Action::ConfirmWork(e.id), c.by, c.at))
        }));
        actions.extend(
            self.boat_checkouts
                .iter()
                .map(|c| (Action::RecordCheckout(c.boat), c.examiner, c.at)),
        );
        actions.extend(
            self.revocations
                .iter()
                .map(|r| (Action::Revoke(r.boat), r.by, r.at)),
        );
        actions
    }

    pub(crate) fn check_recorded_actions(&self, issues: &mut Vec<RecordIssue>) {
        for (action, actor, at) in self.recorded_actions() {
            if self.authorize(actor, &action, at).is_err() {
                issues.push(RecordIssue::Unauthorized { action, actor, at });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::example::{self, at, *};
    use crate::work_hours::WorkSubject;

    #[test]
    fn role_json_shape() {
        assert_eq!(serde_json::to_value(Role::Chair).unwrap(), json!("chair"));
        assert_eq!(
            serde_json::to_value(Role::Boatwarden(TRIGON)).unwrap(),
            json!({"boatwarden": TRIGON.to_string()})
        );
    }

    #[test]
    fn boatwarden_of_one_boat_only() {
        let club = example::club();
        let may = at(2026, 5, 1, 12, 0);
        assert!(club.boatwardens(HARALD_BLAUZAHN, may).contains(&BEN));
        assert!(!club.boatwardens(TRIGON, may).contains(&BEN));
        assert!(
            !club
                .boatwardens(HARALD_BLAUZAHN, at(2026, 3, 1, 12, 0))
                .contains(&BEN)
        );
    }

    #[test]
    fn office_ends() {
        let mut club = example::club();
        club.appoint(ANNA, Role::Treasurer, at(2026, 1, 1, 0, 0));
        club.offices.last_mut().unwrap().end = Some(at(2026, 4, 30, 0, 0));
        assert!(
            club.holders(Role::Treasurer, at(2026, 4, 29, 12, 0))
                .contains(&ANNA)
        );
        assert!(
            !club
                .holders(Role::Treasurer, at(2026, 5, 1, 12, 0))
                .contains(&ANNA)
        );
    }

    #[test]
    fn several_boatwardens_per_boat() {
        let club = example::club();
        let wardens = club.boatwardens(TRIGON, at(2026, 5, 1, 12, 0));
        assert_eq!(wardens, vec![CLARA, LUKAS]);
    }

    #[test]
    fn board_with_legal_representatives() {
        let club = example::club();
        let board = club.board(at(2026, 5, 1, 12, 0));
        assert_eq!(board.len(), 5);
        let legal: Vec<Role> = board
            .iter()
            .filter(|s| s.legal_representative)
            .map(|s| s.role)
            .collect();
        assert_eq!(legal.len(), 3);
        assert!(
            legal.contains(&Role::Chair)
                && legal.contains(&Role::Treasurer)
                && legal.contains(&Role::Secretary)
        );
        assert!(
            board
                .iter()
                .any(|s| s.role == Role::HeadBoatwarden && s.member == FRANK)
        );
    }

    #[test]
    fn appoint_and_resign_keep_history() {
        let mut club = example::club();
        club.appoint(BEN, Role::Boatwarden(TRIGON), at(2026, 4, 1, 0, 0));
        assert_eq!(
            club.resign(BEN, Role::Boatwarden(TRIGON), at(2026, 9, 15, 0, 0)),
            Ok(())
        );
        assert!(
            club.boatwardens(TRIGON, at(2026, 7, 1, 12, 0))
                .contains(&BEN)
        );
        assert!(
            !club
                .boatwardens(TRIGON, at(2026, 10, 1, 12, 0))
                .contains(&BEN)
        );
        let ben_trigon = |club: &ClubRecord| {
            club.offices
                .iter()
                .filter(|o| o.member == BEN && o.role == Role::Boatwarden(TRIGON))
                .count()
        };
        assert_eq!(ben_trigon(&club), 1);

        club.appoint(BEN, Role::Boatwarden(TRIGON), at(2028, 4, 1, 0, 0));
        assert_eq!(ben_trigon(&club), 2);
        assert!(
            !club
                .boatwardens(TRIGON, at(2026, 10, 1, 12, 0))
                .contains(&BEN)
        );
        assert!(
            club.boatwardens(TRIGON, at(2028, 5, 1, 12, 0))
                .contains(&BEN)
        );
    }

    #[test]
    fn resign_errors() {
        let mut club = example::club();
        assert_eq!(
            club.resign(DAVID, Role::Chair, at(2026, 5, 1, 0, 0)),
            Err(OfficeError::NotHolding)
        );
        club.appoint(DAVID, Role::Examiner(TRIGON), at(2026, 6, 1, 0, 0));
        assert_eq!(
            club.resign(DAVID, Role::Examiner(TRIGON), at(2026, 5, 1, 0, 0)),
            Err(OfficeError::EndBeforeStart)
        );
    }

    #[test]
    fn example_board_is_valid() {
        assert_eq!(example::club().validate(), Ok(()));
    }

    #[test]
    fn board_member_needs_voting_right() {
        let mut club = example::club();
        club.appoint(GRETA, Role::Secretary, at(2030, 1, 1, 0, 0));
        club.appoint(MALTE, Role::ViceChair, at(2026, 6, 1, 0, 0));
        let issues = club.validate().unwrap_err();
        assert!(issues.contains(&RecordIssue::BoardMemberWithoutVote {
            member: GRETA,
            role: Role::Secretary
        }));
        assert!(issues.contains(&RecordIssue::BoardMemberWithoutVote {
            member: MALTE,
            role: Role::ViceChair
        }));
    }

    #[test]
    fn board_offices_cannot_be_combined_or_doubled() {
        let mut club = example::club();
        club.appoint(JONAS, Role::Secretary, at(2026, 6, 1, 0, 0));
        club.appoint(ANNA, Role::Treasurer, at(2026, 6, 1, 0, 0));
        let issues = club.validate().unwrap_err();
        assert!(issues.iter().any(
            |i| matches!(i, RecordIssue::BoardOfficesAccumulated { member, .. } if *member == JONAS)
        ));
        assert!(issues.contains(&RecordIssue::BoardOfficeTwice {
            role: Role::Secretary
        }));
        assert!(issues.contains(&RecordIssue::BoardOfficeTwice {
            role: Role::Treasurer
        }));
    }

    #[test]
    fn head_boatwarden_may_also_be_boatwarden() {
        let mut club = example::club();
        club.appoint(FRANK, Role::Boatwarden(TRIGON), at(2026, 6, 1, 0, 0));
        assert_eq!(club.validate(), Ok(()));
    }

    // --- Rechte ---------------------------------------------------------------

    use crate::work_hours::{WorkEntry, WorkExtent};

    fn entry(club: &mut ClubRecord, member: MemberId, subject: WorkSubject) -> WorkEntryId {
        let entry = WorkEntry::new(
            member,
            WorkExtent::Amount {
                date: at(2026, 6, 6, 0, 0),
                hours: 3.0,
            },
            subject,
        );
        let id = entry.id;
        club.work_entries.push(entry);
        id
    }

    #[test]
    fn boatwarden_may_block_own_boat_only() {
        let club = example::club();
        let t = at(2026, 6, 3, 9, 0);
        assert_eq!(
            club.authorize(BEN, &Action::BlockBoat(HARALD_BLAUZAHN), t),
            Ok(())
        );
        assert_eq!(
            club.authorize(BEN, &Action::BlockBoat(TRIGON), t),
            Err(NotAuthorized::MissingOffice)
        );
        for actor in [FRANK, JONAS] {
            for boat in [FILOU, TRIGON] {
                assert_eq!(club.authorize(actor, &Action::BlockBoat(boat), t), Ok(()));
                assert_eq!(club.authorize(actor, &Action::ReleaseBoat(boat), t), Ok(()));
            }
        }
        assert_eq!(
            club.authorize(GRETA, &Action::ReleaseBoat(FILOU), t),
            Err(NotAuthorized::MissingOffice)
        );
    }

    #[test]
    fn confirming_boat_work() {
        let mut club = example::club();
        let t = at(2026, 6, 8, 12, 0);
        let trigon = entry(&mut club, ANNA, WorkSubject::Boat(TRIGON));
        let filou = entry(&mut club, ANNA, WorkSubject::Boat(FILOU));
        assert_eq!(
            club.authorize(CLARA, &Action::ConfirmWork(trigon), t),
            Ok(())
        );
        assert_eq!(
            club.authorize(CLARA, &Action::ConfirmWork(filou), t),
            Err(NotAuthorized::MissingOffice)
        );
        assert_eq!(
            club.authorize(FRANK, &Action::ConfirmWork(filou), t),
            Ok(())
        );
        let own = entry(&mut club, FRANK, WorkSubject::Boat(TRIGON));
        assert_eq!(
            club.authorize(FRANK, &Action::ConfirmWork(own), t),
            Err(NotAuthorized::OwnWorkEntry)
        );
    }

    #[test]
    fn club_work_may_be_confirmed_by_any_office() {
        let mut club = example::club();
        let t = at(2026, 6, 8, 12, 0);
        let website = entry(&mut club, DAVID, WorkSubject::Club);
        for actor in [BEN, LUKAS, FRANK, EVA] {
            assert_eq!(
                club.authorize(actor, &Action::ConfirmWork(website), t),
                Ok(()),
                "{actor}"
            );
        }
        assert_eq!(
            club.authorize(GRETA, &Action::ConfirmWork(website), t),
            Err(NotAuthorized::MissingOffice)
        );
        assert_eq!(
            club.authorize(DAVID, &Action::ConfirmWork(website), t),
            Err(NotAuthorized::OwnWorkEntry)
        );
        // David ist Abnehmer der Filou – auch das ist ein Amt.
        let karin = entry(&mut club, KARIN, WorkSubject::Club);
        assert_eq!(
            club.authorize(DAVID, &Action::ConfirmWork(karin), t),
            Ok(())
        );
    }

    #[test]
    fn checkouts_and_revocations() {
        let club = example::club();
        let t = at(2026, 6, 6, 14, 0);
        assert_eq!(
            club.authorize(DAVID, &Action::RecordCheckout(FILOU), t),
            Ok(())
        );
        assert_eq!(
            club.authorize(BEN, &Action::RecordCheckout(HARALD_BLAUZAHN), t),
            Ok(())
        );
        assert_eq!(
            club.authorize(BEN, &Action::RecordCheckout(TRIGON), t),
            Err(NotAuthorized::MissingOffice)
        );
        assert_eq!(
            club.authorize(BEN, &Action::Revoke(HARALD_BLAUZAHN), t),
            Err(NotAuthorized::MissingOffice)
        );
        assert_eq!(club.authorize(EVA, &Action::Revoke(TRIGON), t), Ok(()));
    }

    #[test]
    fn rights_end_with_office() {
        let mut club = example::club();
        club.resign(BEN, Role::Boatwarden(HARALD_BLAUZAHN), at(2026, 9, 1, 0, 0))
            .unwrap();
        assert_eq!(
            club.authorize(
                BEN,
                &Action::BlockBoat(HARALD_BLAUZAHN),
                at(2026, 8, 1, 0, 0)
            ),
            Ok(())
        );
        assert_eq!(
            club.authorize(
                BEN,
                &Action::BlockBoat(HARALD_BLAUZAHN),
                at(2026, 9, 2, 0, 0)
            ),
            Err(NotAuthorized::MissingOffice)
        );
    }

    #[test]
    fn successful_block_records_actor() {
        let mut club = example::club();
        club.block_boat(
            BEN,
            HARALD_BLAUZAHN,
            at(2026, 6, 3, 9, 0),
            None,
            "Ruderschaden",
        )
        .unwrap();
        let block = club
            .boat(HARALD_BLAUZAHN)
            .unwrap()
            .blocks
            .last()
            .unwrap()
            .clone();
        assert_eq!(block.by, BEN);
        assert_eq!(block.reason, "Ruderschaden");
        club.lift_block(FRANK, HARALD_BLAUZAHN, at(2026, 6, 15, 0, 0))
            .unwrap();
        let block = club
            .boat(HARALD_BLAUZAHN)
            .unwrap()
            .blocks
            .last()
            .unwrap()
            .clone();
        assert_eq!(
            (block.end, block.lifted_by),
            (Some(at(2026, 6, 15, 0, 0)), Some(FRANK))
        );
        assert_eq!(
            club.lift_block(FRANK, HARALD_BLAUZAHN, at(2026, 6, 16, 0, 0)),
            Err(ActionError::NoOpenBlock)
        );
        assert_eq!(club.validate(), Ok(()));
    }

    #[test]
    fn rejected_actions_leave_record_unchanged() {
        let mut club = example::club();
        let before = club.clone();
        let t = at(2026, 6, 3, 9, 0);
        let denied = ActionError::NotAuthorized(NotAuthorized::MissingOffice);
        assert_eq!(
            club.block_boat(BEN, TRIGON, t, None, "Riss im Großsegel"),
            Err(denied)
        );
        assert_eq!(club.release_boat(GRETA, FILOU, 2027, t), Err(denied));
        assert_eq!(
            club.revoke(BEN, ANNA, HARALD_BLAUZAHN, t, "Test"),
            Err(denied)
        );
        assert_eq!(
            club.record_checkout(BEN, MALTE, TRIGON, t, CheckoutResult::Passed, None),
            Err(denied)
        );
        let unconfirmed = club
            .work_entries
            .iter()
            .find(|e| !e.is_confirmed())
            .unwrap()
            .id;
        assert_eq!(club.confirm_work(BEN, unconfirmed, t), Err(denied));
        assert_eq!(club, before);
    }

    #[test]
    fn confirm_work_sets_confirmation() {
        let mut club = example::club();
        let unconfirmed = club
            .work_entries
            .iter()
            .find(|e| !e.is_confirmed())
            .unwrap()
            .id;
        let t = at(2026, 5, 4, 19, 0);
        club.confirm_work(DAVID, unconfirmed, t).unwrap();
        assert_eq!(
            club.work_entry(unconfirmed).unwrap().confirmation,
            Some(Confirmation { by: DAVID, at: t })
        );
    }

    #[test]
    fn stored_unauthorized_actions_are_reported() {
        let mut club = example::club();
        let unconfirmed = club
            .work_entries
            .iter()
            .find(|e| !e.is_confirmed())
            .unwrap()
            .id;
        let mut value = serde_json::to_value(&club).unwrap();
        let entries = value["work_entries"].as_array_mut().unwrap();
        let raw = entries
            .iter_mut()
            .find(|e| e["id"] == json!(unconfirmed.to_string()))
            .unwrap();
        raw["confirmation"] = json!({"by": GRETA.get(), "at": "2026-05-04T19:00:00+02:00"});
        let imported: ClubRecord = serde_json::from_value(value).unwrap();
        let issues = imported.validate().unwrap_err();
        assert_eq!(
            issues,
            vec![RecordIssue::Unauthorized {
                action: Action::ConfirmWork(unconfirmed),
                actor: GRETA,
                at: at(2026, 5, 4, 19, 0),
            }]
        );

        club.confirm_work(DAVID, unconfirmed, at(2026, 5, 4, 19, 0))
            .unwrap();
        assert_eq!(club.validate(), Ok(()));
    }
}
