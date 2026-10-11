//! Ämter ansehen, ernennen und abgeben.

use chrono::{Datelike, NaiveDate};
use vereinsakte_core::boat::Boat;
use vereinsakte_core::ids::{BoatId, MemberId, OfficeId};
use vereinsakte_core::issues::RecordIssue;
use vereinsakte_core::office::{Office, Role};
use vereinsakte_core::rights::Capability;
use vereinsakte_core::time::{Timestamp, club_time};

use crate::commands::audit;
use crate::error::AppError;
use crate::format;
use crate::scope::{BoatScope, MemberScope, Scope};
use crate::service::{ClubService, CurrentUser, role_label};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OfficeRow {
    pub id: String,
    pub role: String,
    pub holder: String,
    pub holder_id: u32,
    pub since: String,
    pub until: Option<String>,
    pub can_resign: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OfficeSection {
    pub title: &'static str,
    pub rows: Vec<OfficeRow>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OfficesView {
    pub sections: Vec<OfficeSection>,
    pub can_appoint: bool,
    pub can_see_history: bool,
    /// Auswahl für das Ernennen: Schlüssel und Bezeichnung.
    pub role_options: Vec<(String, String)>,
    pub member_options: Vec<(u32, String)>,
}

/// Schlüssel eines Amtes für Formulare.
pub fn role_key(role: Role) -> String {
    match role {
        Role::Chair => "chair".into(),
        Role::ViceChair => "vice_chair".into(),
        Role::Treasurer => "treasurer".into(),
        Role::Secretary => "secretary".into(),
        Role::HeadBoatwarden => "head_boatwarden".into(),
        Role::Auditor => "auditor".into(),
        Role::Boatwarden(boat) => format!("boatwarden:{boat}"),
        Role::Examiner(boat) => format!("examiner:{boat}"),
    }
}

pub fn parse_role(key: &str) -> Option<Role> {
    Some(match key {
        "chair" => Role::Chair,
        "vice_chair" => Role::ViceChair,
        "treasurer" => Role::Treasurer,
        "secretary" => Role::Secretary,
        "head_boatwarden" => Role::HeadBoatwarden,
        "auditor" => Role::Auditor,
        other => {
            let (kind, boat) = other.split_once(':')?;
            let boat: BoatId = boat.parse().ok()?;
            match kind {
                "boatwarden" => Role::Boatwarden(boat),
                "examiner" => Role::Examiner(boat),
                _ => return None,
            }
        }
    })
}

fn section_of(role: Role) -> usize {
    match role {
        r if r.is_board() => 0,
        Role::Auditor => 1,
        _ => 2,
    }
}

fn midnight(date: NaiveDate) -> Timestamp {
    club_time(date.year(), date.month(), date.day(), 0, 0)
}

fn rank(role: Role) -> u8 {
    match role {
        Role::Chair => 0,
        Role::ViceChair => 1,
        Role::Treasurer => 2,
        Role::Secretary => 3,
        Role::HeadBoatwarden => 4,
        Role::Auditor => 5,
        Role::Boatwarden(_) => 6,
        Role::Examiner(_) => 7,
    }
}

impl ClubService {
    fn office_rows(
        &self,
        offices: &[&Office],
        boats: &[Boat],
        names: &dyn Fn(MemberId) -> String,
        can_resign: &dyn Fn(MemberId) -> bool,
    ) -> Vec<OfficeRow> {
        let mut sorted: Vec<&Office> = offices.to_vec();
        sorted.sort_by_key(|o| (rank(o.role), role_label(boats, o.role), o.start));
        sorted
            .into_iter()
            .map(|o| OfficeRow {
                id: o.id.to_string(),
                role: role_label(boats, o.role),
                holder: names(o.member),
                holder_id: o.member.get(),
                since: format::date(o.start),
                until: o.end.map(format::date),
                can_resign: o.end.is_none() && can_resign(o.member),
            })
            .collect()
    }

    /// Aktuelle Besetzung für alle; Ernennen und Abgeben nach Berechtigung.
    pub async fn offices_view(&self, user: &CurrentUser) -> Result<OfficesView, AppError> {
        let principal = self.principal(user).await?;
        let scoped = self
            .load(&Scope {
                offices: true,
                boats: BoatScope::All,
                ..Scope::default()
            })
            .await?;
        let members = self.store.active_members(principal.now).await?;
        let name_of = |id: MemberId| {
            members
                .iter()
                .find(|m| m.id == id)
                .map_or_else(|| format!("Mitglied {id}"), |m| m.full_name())
        };
        let record = &scoped.record;
        let active: Vec<&Office> = record
            .offices
            .iter()
            .filter(|o| o.is_active_at(principal.now))
            .collect();
        let can_resign = |member: MemberId| principal.can(&Capability::Resign(member));
        let mut sections = vec![
            OfficeSection {
                title: "Vorstand",
                rows: vec![],
            },
            OfficeSection {
                title: "Revisoren",
                rows: vec![],
            },
            OfficeSection {
                title: "Bootswarte und Abnehmer",
                rows: vec![],
            },
        ];
        for (index, section) in sections.iter_mut().enumerate() {
            let of_section: Vec<&Office> = active
                .iter()
                .copied()
                .filter(|o| section_of(o.role) == index)
                .collect();
            section.rows = self.office_rows(&of_section, &record.boats, &name_of, &can_resign);
        }
        let can_appoint = principal.can(&Capability::Appoint(Role::Chair));
        let mut role_options: Vec<(String, String)> = [
            Role::Chair,
            Role::ViceChair,
            Role::Treasurer,
            Role::Secretary,
            Role::HeadBoatwarden,
            Role::Auditor,
        ]
        .into_iter()
        .map(|r| (role_key(r), r.to_string()))
        .collect();
        let mut boats: Vec<&Boat> = record.boats.iter().collect();
        boats.sort_by(|a, b| a.name.cmp(&b.name));
        for boat in boats {
            for role in [Role::Boatwarden(boat.id), Role::Examiner(boat.id)] {
                role_options.push((role_key(role), role_label(&record.boats, role)));
            }
        }
        Ok(OfficesView {
            sections,
            can_appoint,
            can_see_history: principal.can(&Capability::ReadOfficeHistory),
            role_options: if can_appoint { role_options } else { vec![] },
            member_options: if can_appoint {
                members
                    .iter()
                    .map(|m| (m.id.get(), m.full_name()))
                    .collect()
            } else {
                vec![]
            },
        })
    }

    /// Alle Ämter samt beendeter, nach Amt und Beginn sortiert (nur Vorstand).
    pub async fn office_history(&self, user: &CurrentUser) -> Result<Vec<OfficeRow>, AppError> {
        let principal = self.principal(user).await?;
        principal.require(&Capability::ReadOfficeHistory)?;
        let scoped = self
            .load(&Scope {
                offices: true,
                boats: BoatScope::All,
                ..Scope::default()
            })
            .await?;
        let members = self.store.active_members(principal.now).await?;
        let record = &scoped.record;
        let all: Vec<&Office> = record.offices.iter().collect();
        let name_of = |id: MemberId| {
            members
                .iter()
                .find(|m| m.id == id)
                .map_or_else(|| format!("Mitglied {id}"), |m| m.full_name())
        };
        Ok(self.office_rows(&all, &record.boats, &name_of, &|_| false))
    }

    /// Ernennt ein aktives Mitglied; liefert Hinweise der Prüfung (Doppelbesetzung usw.).
    pub async fn appoint(
        &self,
        user: &CurrentUser,
        member: MemberId,
        role: Role,
        start: Option<NaiveDate>,
    ) -> Result<Vec<String>, AppError> {
        let principal = self.principal(user).await?;
        principal.require(&Capability::Appoint(role))?;
        let scoped = self
            .load(&Scope {
                offices: true,
                members: MemberScope::Some(vec![member]),
                boats: BoatScope::All,
                ..Scope::default()
            })
            .await?;
        let Some(holder) = scoped.record.member(member) else {
            return Err(AppError::rejected(
                "member",
                "Dieses Mitglied gibt es nicht.",
            ));
        };
        if !holder.is_active_at(principal.now) {
            return Err(AppError::rejected(
                "member",
                "Das Mitglied ist nicht aktiv.",
            ));
        }
        if let Role::Boatwarden(boat) | Role::Examiner(boat) = role
            && scoped.record.boat(boat).is_none()
        {
            return Err(AppError::rejected("role", "Dieses Boot gibt es nicht."));
        }
        let start = start.map_or(principal.now, midnight);
        if scoped
            .record
            .offices
            .iter()
            .any(|o| o.member == member && o.role == role && o.is_active_at(start))
        {
            return Err(AppError::rejected(
                "role",
                "Das Mitglied hat dieses Amt bereits inne.",
            ));
        }
        let hints = board_hints(&scoped.record, holder, role, start);
        let mut new = scoped.record.clone();
        let id = new.appoint(member, role, start);
        let label = role_label(&new.boats, role);
        let entry = audit(
            &principal,
            "office.appoint",
            "office",
            id,
            Some(member),
            format!("Ernennung: {label}"),
        );
        self.save(&scoped, &new, vec![entry]).await?;
        Ok(hints)
    }

    /// Beendet ein Amt zum Datum (Standard: jetzt).
    pub async fn resign(
        &self,
        user: &CurrentUser,
        office: OfficeId,
        on: Option<NaiveDate>,
    ) -> Result<(), AppError> {
        let principal = self.principal(user).await?;
        let scoped = self
            .load(&Scope {
                offices: true,
                boats: BoatScope::All,
                ..Scope::default()
            })
            .await?;
        let Some(found) = scoped
            .record
            .offices
            .iter()
            .find(|o| o.id == office)
            .cloned()
        else {
            return Err(AppError::NotFound("Amt".into()));
        };
        principal.require(&Capability::Resign(found.member))?;
        let at = on.map_or(principal.now, midnight);
        let mut new = scoped.record.clone();
        new.resign(found.member, found.role, at)
            .map_err(|e| AppError::rejected("date", e.to_string()))?;
        let entry = audit(
            &principal,
            "office.resign",
            "office",
            office,
            Some(found.member),
            format!(
                "Amt abgegeben: {}",
                role_label(&scoped.record.boats, found.role)
            ),
        );
        self.save(&scoped, &new, vec![entry]).await
    }
}

/// Hinweise zur neuen Ernennung (Spec `aemter`): nicht stimmberechtigtes Mitglied,
/// doppelt besetztes Vorstandsamt, Häufung mehrerer Vorstandsämter.
fn board_hints(
    old: &vereinsakte_core::club_record::ClubRecord,
    member: &vereinsakte_core::member::Member,
    role: Role,
    start: Timestamp,
) -> Vec<String> {
    let mut hints = vec![];
    if !role.is_board() {
        return hints;
    }
    if !member.has_voting_right_at(start) {
        hints.push(
            RecordIssue::BoardMemberWithoutVote {
                member: member.id,
                role,
            }
            .to_string(),
        );
    }
    let overlapping = || {
        old.offices
            .iter()
            .filter(move |o| o.role.is_board() && o.end.is_none_or(|end| start < end))
    };
    if overlapping().any(|o| o.role == role) {
        hints.push(RecordIssue::BoardOfficeTwice { role }.to_string());
    }
    if let Some(other) = overlapping().find(|o| o.member == member.id && o.role != role) {
        hints.push(
            RecordIssue::BoardOfficesAccumulated {
                member: member.id,
                roles: (other.role, role),
            }
            .to_string(),
        );
    }
    hints
}

#[cfg(test)]
mod tests {
    use vereinsakte_core::example::{BEN, EVA, FILOU, FRANK, GRETA, HARALD_BLAUZAHN, JONAS};

    use super::*;
    use crate::service::tests::service;

    fn user(member: MemberId) -> CurrentUser {
        CurrentUser { member }
    }

    fn office_of(store: &crate::memory::MemoryStore, member: MemberId, role: Role) -> OfficeId {
        store
            .snapshot()
            .offices
            .iter()
            .find(|o| o.member == member && o.role == role && o.end.is_none())
            .unwrap()
            .id
    }

    #[tokio::test]
    async fn overview_lists_current_holders_without_admin_rights_for_members() {
        let (service, _) = service();
        let view = service.offices_view(&user(GRETA)).await.unwrap();
        let board = &view.sections[0];
        assert!(
            board
                .rows
                .iter()
                .any(|r| r.role == "1. Vorsitzende(r)" && r.holder == "Eva Möller")
        );
        let boats = &view.sections[2];
        assert!(
            boats
                .rows
                .iter()
                .any(|r| r.role == "Bootswart (Harald Blauzahn)" && r.holder == "Ben Hansen")
        );
        assert!(!view.can_appoint && view.role_options.is_empty() && !view.can_see_history);
        assert!(
            view.sections
                .iter()
                .flat_map(|s| &s.rows)
                .all(|r| !r.can_resign)
        );
        let frank = service.offices_view(&user(FRANK)).await.unwrap();
        assert!(
            frank.can_appoint && !frank.role_options.is_empty() && !frank.member_options.is_empty()
        );
    }

    #[tokio::test]
    async fn board_appoints_a_boatwarden_who_may_then_block_the_boat() {
        let (service, store) = service();
        let role = Role::Boatwarden(FILOU);
        assert!(
            !service
                .principal(&user(GRETA))
                .await
                .unwrap()
                .can(&Capability::BlockBoat(FILOU))
        );
        let hints = service
            .appoint(&user(FRANK), GRETA, role, None)
            .await
            .unwrap();
        assert!(hints.is_empty());
        assert!(
            service
                .principal(&user(GRETA))
                .await
                .unwrap()
                .can(&Capability::BlockBoat(FILOU))
        );
        let log = store.audit_entries();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].summary, "Ernennung: Bootswart (Filou)");
        let again = service.appoint(&user(FRANK), GRETA, role, None).await;
        assert!(matches!(again, Err(AppError::Rejected(_))), "schon inne");
    }

    #[tokio::test]
    async fn appointing_needs_the_board() {
        let (service, store) = service();
        let result = service.appoint(&user(BEN), GRETA, Role::Chair, None).await;
        assert_eq!(result, Err(AppError::Forbidden));
        assert!(store.audit_entries().is_empty());
    }

    #[tokio::test]
    async fn board_checks_give_hints_but_still_save() {
        let (service, store) = service();
        // Greta ist außerordentliches Mitglied ohne Stimmrecht; Karin ist Schriftwart.
        let hints = service
            .appoint(&user(EVA), GRETA, Role::Secretary, None)
            .await
            .unwrap();
        assert!(
            hints
                .iter()
                .any(|h| h.contains("Stimmrecht") || h.contains("stimm")),
            "{hints:?}"
        );
        assert!(
            hints.iter().any(|h| h.to_lowercase().contains("mehrfach")
                || h.contains("zweimal")
                || h.contains("doppelt")
                || h.contains("besetzt")),
            "{hints:?}"
        );
        let jonas = service
            .appoint(&user(EVA), JONAS, Role::Secretary, None)
            .await
            .unwrap();
        assert!(!jonas.is_empty(), "Kassenwart wird zusätzlich Schriftwart");
        assert_eq!(
            store.audit_entries().len(),
            2,
            "gespeichert trotz Hinweisen"
        );
    }

    #[tokio::test]
    async fn resigning_ends_the_office_and_rights_vanish_immediately() {
        let (service, store) = service();
        let office = office_of(&store, BEN, Role::Boatwarden(HARALD_BLAUZAHN));
        assert!(
            service
                .principal(&user(BEN))
                .await
                .unwrap()
                .can(&Capability::BlockBoat(HARALD_BLAUZAHN))
        );
        // Fremde dürfen nicht abgeben lassen.
        assert_eq!(
            service.resign(&user(GRETA), office, None).await,
            Err(AppError::Forbidden)
        );
        let today = NaiveDate::from_ymd_opt(2026, 10, 9);
        service.resign(&user(BEN), office, today).await.unwrap();
        assert!(
            !service
                .principal(&user(BEN))
                .await
                .unwrap()
                .can(&Capability::BlockBoat(HARALD_BLAUZAHN))
        );
        let history = service.office_history(&user(EVA)).await.unwrap();
        let row = history.iter().find(|r| r.id == office.to_string()).unwrap();
        assert_eq!(row.until.as_deref(), Some("09.10.2026"));
        let overview = service.offices_view(&user(EVA)).await.unwrap();
        assert!(
            overview.sections[2]
                .rows
                .iter()
                .all(|r| r.id != office.to_string())
        );
    }

    #[tokio::test]
    async fn end_before_start_is_rejected_and_history_is_for_the_board() {
        let (service, store) = service();
        let office = office_of(&store, BEN, Role::Boatwarden(HARALD_BLAUZAHN));
        let early = NaiveDate::from_ymd_opt(2026, 1, 1);
        let result = service.resign(&user(BEN), office, early).await;
        let Err(AppError::Rejected(errors)) = result else {
            panic!("{result:?}")
        };
        assert!(errors[0].message.contains("nicht vor seinem Beginn"));
        assert_eq!(
            service.office_history(&user(GRETA)).await,
            Err(AppError::Forbidden)
        );
        assert!(store.audit_entries().is_empty());
    }
}
