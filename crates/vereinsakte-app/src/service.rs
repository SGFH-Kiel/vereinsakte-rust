//! Anwendungsfälle rund um die Vereinsakte. Jeder Anwendungsfall lädt über die
//! Ports nur die Daten, die er braucht.

use std::sync::Arc;

use vereinsakte_core::boat::Boat;
use vereinsakte_core::ids::MemberId;
use vereinsakte_core::office::{Offices, Role};
use vereinsakte_core::time::Timestamp;

use crate::error::AppError;
use crate::ports::{Clock, ClubStore};

/// Das angemeldete Mitglied; Pflichtparameter aller Anwendungsfälle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CurrentUser {
    pub member: MemberId,
}

/// Angaben zum angemeldeten Mitglied für Layout und Navigation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserContext {
    pub member: MemberId,
    pub name: String,
    pub offices: Vec<String>,
    pub has_office: bool,
    pub is_board: bool,
}

/// Eintrag der Auswahlliste im Entwicklungs-Login.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoginChoice {
    pub member: MemberId,
    pub name: String,
    pub offices: Vec<String>,
}

/// Gruppe der Auswahlliste („Vorstand“, „Ämter“, „ohne Amt“).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoginChoiceGroup {
    pub label: &'static str,
    pub choices: Vec<LoginChoice>,
}

/// Anwendungsfälle auf der Vereinsakte.
#[derive(Clone)]
pub struct ClubService {
    pub(crate) store: Arc<dyn ClubStore>,
    clock: Arc<dyn Clock>,
}

impl ClubService {
    pub fn new(store: Arc<dyn ClubStore>, clock: Arc<dyn Clock>) -> Self {
        Self { store, clock }
    }

    pub fn now(&self) -> Timestamp {
        self.clock.now()
    }

    /// Ist die Datenbank erreichbar?
    pub async fn health(&self) -> Result<(), AppError> {
        self.store.ping().await
    }

    /// Name und aktuelle Ämter des angemeldeten Mitglieds.
    pub async fn user_context(&self, user: &CurrentUser) -> Result<UserContext, AppError> {
        let now = self.now();
        let member = self
            .store
            .member(user.member)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Mitglied {}", user.member)))?
            .value;
        let active = self.store.active_offices(now).await?;
        let offices = Offices::new(&active);
        let labels = office_labels(&offices, &self.store.boats().await?, user.member, now);
        Ok(UserContext {
            member: user.member,
            name: member.full_name(),
            has_office: !labels.is_empty(),
            is_board: offices.is_board_member(user.member, now),
            offices: labels,
        })
    }

    /// Ist das Mitglied zum aktuellen Zeitpunkt aktiv?
    pub async fn is_active(&self, member: MemberId) -> Result<bool, AppError> {
        let now = self.now();
        Ok(self
            .store
            .member(member)
            .await?
            .is_some_and(|m| m.value.is_active_at(now)))
    }

    /// Aktive Mitglieder, gruppiert nach Vorstand, sonstigen Ämtern und ohne Amt.
    pub async fn login_choices(&self) -> Result<Vec<LoginChoiceGroup>, AppError> {
        let now = self.now();
        let members = self.store.active_members(now).await?;
        let active = self.store.active_offices(now).await?;
        let offices = Offices::new(&active);
        let boats = self.store.boats().await?;
        let mut groups = [
            LoginChoiceGroup {
                label: "Vorstand",
                choices: Vec::new(),
            },
            LoginChoiceGroup {
                label: "Ämter",
                choices: Vec::new(),
            },
            LoginChoiceGroup {
                label: "ohne Amt",
                choices: Vec::new(),
            },
        ];
        for member in members {
            let labels = office_labels(&offices, &boats, member.id, now);
            let group = if offices.is_board_member(member.id, now) {
                0
            } else if labels.is_empty() {
                2
            } else {
                1
            };
            groups[group].choices.push(LoginChoice {
                member: member.id,
                name: member.full_name(),
                offices: labels,
            });
        }
        Ok(groups
            .into_iter()
            .filter(|g| !g.choices.is_empty())
            .collect())
    }
}

/// Aktuelle Ämter eines Mitglieds als Text, Boote mit Namen.
pub(crate) fn office_labels(
    offices: &Offices<'_>,
    boats: &[Boat],
    member: MemberId,
    now: Timestamp,
) -> Vec<String> {
    offices
        .roles_of(member, now)
        .into_iter()
        .map(|role| role_label(boats, role))
        .collect()
}

/// Bezeichnung eines Amtes, bei Booten mit dem Bootsnamen.
pub fn role_label(boats: &[Boat], role: Role) -> String {
    let boat_name = |id| {
        boats
            .iter()
            .find(|b| b.id == id)
            .map_or_else(|| id.to_string(), |b| b.name.clone())
    };
    match role {
        Role::Boatwarden(boat) => format!("Bootswart ({})", boat_name(boat)),
        Role::Examiner(boat) => format!("Abnehmer ({})", boat_name(boat)),
        other => other.to_string(),
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use vereinsakte_core::example::{self, BEN, EVA, GRETA};
    use vereinsakte_core::time::club_time;

    use super::*;
    use crate::format;
    use crate::memory::MemoryStore;
    use crate::ports::FixedClock;

    pub fn service_at(now: Timestamp) -> (ClubService, Arc<MemoryStore>) {
        let store = Arc::new(MemoryStore::new(example::club()));
        (
            ClubService::new(store.clone(), Arc::new(FixedClock(now))),
            store,
        )
    }

    pub fn service() -> (ClubService, Arc<MemoryStore>) {
        service_at(club_time(2026, 10, 9, 12, 0))
    }

    #[tokio::test]
    async fn user_context_and_login_choices() {
        let (service, _) = service();
        let ben = service
            .user_context(&CurrentUser { member: BEN })
            .await
            .unwrap();
        assert_eq!(ben.offices, vec!["Bootswart (Harald Blauzahn)".to_string()]);
        assert!(ben.has_office && !ben.is_board);

        let groups = service.login_choices().await.unwrap();
        let find = |m| {
            groups
                .iter()
                .flat_map(|g| g.choices.iter().map(move |c| (g.label, c)))
                .find(|(_, c)| c.member == m)
        };
        let (label, eva) = find(EVA).unwrap();
        assert_eq!(label, "Vorstand");
        assert!(eva.offices.contains(&"1. Vorsitzende(r)".to_string()));
        let (label, greta) = find(GRETA).unwrap();
        assert_eq!(label, "ohne Amt");
        assert!(greta.offices.is_empty());
        assert_eq!(find(BEN).unwrap().0, "Ämter");
    }

    #[tokio::test]
    async fn health_reflects_store() {
        let (service, store) = service();
        assert!(service.health().await.is_ok());
        store.set_available(false);
        assert!(service.health().await.is_err());
    }

    #[test]
    fn hours_are_formatted_german() {
        assert_eq!(format::hours(20.5), "20,5");
        assert_eq!(format::hours(25.0), "25");
    }
}
