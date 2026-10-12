//! Archiv: alles, was nicht mehr aktuell ist (beendete Ämter, ausgeschiedene
//! Mitglieder, aufgehobene und abgelaufene Sperren, vergangene Reservierungen
//! und Arbeitstermine). Aus den Tagesansichten verschwindet es; hier ist es
//! für Vorstand und Revisoren einsehbar.

use chrono::{Datelike, NaiveDate};
use vereinsakte_core::rights::Capability;

use crate::error::AppError;
use crate::format;
use crate::offices::OfficeRow;
use crate::people::PersonRef;
use crate::reservations::{ReservationFilter, ReservationRow};
use crate::scope::{BoatScope, MemberScope, Scope};
use crate::service::{ClubService, CurrentUser};
use crate::work::EventLine;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArchivedMember {
    pub person: PersonRef,
    pub membership_type: String,
    pub joined: String,
    pub ended: String,
    /// „ausgetreten“ oder „ausgeschlossen“.
    pub reason: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArchivedBlock {
    pub boat: String,
    pub boat_id: String,
    pub period: String,
    pub reason: String,
    pub by: PersonRef,
}

/// Auswahl des Zeitraums: ein Jahr oder alle.
pub fn year_bounds(year: Option<i32>, today: NaiveDate) -> (NaiveDate, NaiveDate) {
    let yesterday = today - chrono::Duration::days(1);
    match year {
        Some(y) => (
            NaiveDate::from_ymd_opt(y, 1, 1).expect("Jahr"),
            NaiveDate::from_ymd_opt(y, 12, 31)
                .expect("Jahr")
                .min(yesterday),
        ),
        None => (
            NaiveDate::from_ymd_opt(today.year() - 20, 1, 1).expect("Jahr"),
            yesterday,
        ),
    }
}

impl ClubService {
    async fn require_archive(
        &self,
        user: &CurrentUser,
    ) -> Result<crate::principal::Principal, AppError> {
        let principal = self.principal(user).await?;
        principal.require(&Capability::ReadArchive)?;
        Ok(principal)
    }

    /// Alle Ämter mit Beginn und Ende (beendete und laufende).
    pub async fn archive_offices(&self, user: &CurrentUser) -> Result<Vec<OfficeRow>, AppError> {
        self.require_archive(user).await?;
        self.office_history(user).await
    }

    /// Ausgeschiedene und ausgeschlossene Mitglieder.
    pub async fn archive_members(
        &self,
        user: &CurrentUser,
    ) -> Result<Vec<ArchivedMember>, AppError> {
        let principal = self.require_archive(user).await?;
        let scoped = self
            .load(&Scope {
                members: MemberScope::All,
                ..Scope::default()
            })
            .await?;
        let mut rows: Vec<ArchivedMember> = scoped
            .record
            .members
            .iter()
            .filter(|m| !m.is_active_at(principal.now))
            .map(|m| {
                let (ended, reason) = match (m.membership.expelled_at, m.membership.left_at) {
                    (Some(t), _) => (Some(t), "ausgeschlossen"),
                    (None, Some(t)) => (Some(t), "ausgetreten"),
                    _ => (m.membership.limited_until, "befristet beendet"),
                };
                (
                    ended,
                    ArchivedMember {
                        person: PersonRef::text(m.full_name()),
                        membership_type: m.membership_type.to_string(),
                        joined: format::date(m.membership.joined_at),
                        ended: ended.map(format::date).unwrap_or_default(),
                        reason,
                    },
                )
            })
            .map(|(_, row)| row)
            .collect();
        rows.sort_by(|a, b| a.person.name.cmp(&b.person.name));
        Ok(rows)
    }

    /// Aufgehobene und abgelaufene Sperren der Boote.
    pub async fn archive_blocks(&self, user: &CurrentUser) -> Result<Vec<ArchivedBlock>, AppError> {
        let principal = self.require_archive(user).await?;
        let scoped = self
            .load(&Scope {
                boats: BoatScope::All,
                members: MemberScope::None,
                ..Scope::default()
            })
            .await?;
        let names = self.store.active_members(principal.now).await?;
        let name = |id: vereinsakte_core::ids::MemberId| {
            names.iter().find(|m| m.id == id).map_or_else(
                || PersonRef::text(format!("Mitglied {id}")),
                |m| PersonRef::member(id, m.full_name()),
            )
        };
        let mut rows = vec![];
        for boat in &scoped.record.boats {
            for block in boat
                .blocks
                .iter()
                .filter(|b| !b.is_active_at(principal.now) && b.start <= principal.now)
            {
                rows.push((
                    block.end,
                    ArchivedBlock {
                        boat: boat.name.clone(),
                        boat_id: boat.id.to_string(),
                        period: match block.end {
                            Some(end) => {
                                format!("{} bis {}", format::date(block.start), format::date(end))
                            }
                            None => format!("ab {}", format::date(block.start)),
                        },
                        reason: block.reason.clone(),
                        by: name(block.by),
                    },
                ));
            }
        }
        rows.sort_by_key(|r| std::cmp::Reverse(r.0));
        Ok(rows.into_iter().map(|(_, row)| row).collect())
    }

    /// Vergangene Reservierungen, optional eines Jahres.
    pub async fn archive_reservations(
        &self,
        user: &CurrentUser,
        year: Option<i32>,
    ) -> Result<Vec<ReservationRow>, AppError> {
        let principal = self.require_archive(user).await?;
        let (from, to) = year_bounds(year, principal.now.date_naive());
        let list = self
            .reservation_list(
                user,
                &ReservationFilter {
                    from: Some(from),
                    to: Some(to),
                    ..Default::default()
                },
            )
            .await?;
        let mut rows: Vec<ReservationRow> = list.rows.into_iter().filter(|r| r.past).collect();
        rows.reverse();
        Ok(rows)
    }

    /// Vergangene Arbeitstermine, optional eines Jahres.
    pub async fn archive_events(
        &self,
        user: &CurrentUser,
        year: Option<i32>,
    ) -> Result<Vec<EventLine>, AppError> {
        self.require_archive(user).await?;
        let mut events: Vec<EventLine> = self
            .events_view(user)
            .await?
            .events
            .into_iter()
            .filter(|e| e.past && year.is_none_or(|y| e.start_date.year() == y))
            .collect();
        events.sort_by_key(|e| std::cmp::Reverse(e.start_date));
        Ok(events)
    }
}
