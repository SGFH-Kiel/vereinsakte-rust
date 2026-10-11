//! Sichtmodell Mitgliederverzeichnis: Kontaktdaten nur für Mitglieder mit Amt.

use crate::error::AppError;
use vereinsakte_core::office::Offices;

use crate::service::{ClubService, CurrentUser, office_labels};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirectoryView {
    /// Sieht das angemeldete Mitglied Kontaktdaten?
    pub show_contact: bool,
    pub query: String,
    pub entries: Vec<DirectoryEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirectoryEntry {
    pub name: String,
    pub offices: Vec<String>,
    /// Nur gesetzt, wenn das angemeldete Mitglied ein Amt hat.
    pub contact: Option<Contact>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Contact {
    pub member_number: u32,
    pub membership_type: String,
    pub email: Option<String>,
    pub phones: Vec<String>,
}

impl ClubService {
    /// Aktive Mitglieder nach Nachnamen sortiert, optional gefiltert nach
    /// einem Teil des Vor- oder Nachnamens.
    pub async fn directory(
        &self,
        user: &CurrentUser,
        query: &str,
    ) -> Result<DirectoryView, AppError> {
        let now = self.now();
        let active = self.store.active_offices(now).await?;
        let offices = Offices::new(&active);
        let boats = self.store.boats().await?;
        let show_contact = offices.holds(user.member, now, |_| true);
        let needle = query.trim().to_lowercase();

        let entries = self
            .store
            .active_members(now)
            .await?
            .into_iter()
            .filter(|m| {
                needle.is_empty()
                    || m.first_name.to_lowercase().contains(&needle)
                    || m.last_name.to_lowercase().contains(&needle)
            })
            .map(|m| DirectoryEntry {
                name: format!("{}, {}", m.last_name, m.first_name),
                offices: office_labels(&offices, &boats, m.id, now),
                contact: show_contact.then(|| Contact {
                    member_number: m.id.get(),
                    membership_type: m.membership_type.to_string(),
                    email: m.email.clone(),
                    phones: [&m.phone.landline, &m.phone.business, &m.phone.mobile]
                        .into_iter()
                        .flatten()
                        .cloned()
                        .collect(),
                }),
            })
            .collect();
        Ok(DirectoryView {
            show_contact,
            query: query.trim().to_string(),
            entries,
        })
    }
}

#[cfg(test)]
mod tests {
    use vereinsakte_core::example::{BEN, GRETA};
    use vereinsakte_core::time::club_time;

    use super::*;
    use crate::service::tests::{service, service_at};

    #[tokio::test]
    async fn inactive_members_are_hidden() {
        let (service, _) = service_at(club_time(2026, 11, 15, 12, 0));
        let view = service
            .directory(&CurrentUser { member: BEN }, "")
            .await
            .unwrap();
        assert!(!view.entries.iter().any(|e| e.name == "Dupont, Inès"));
        let eva = view
            .entries
            .iter()
            .find(|e| e.name == "Möller, Eva")
            .unwrap();
        assert!(eva.offices.contains(&"1. Vorsitzende(r)".to_string()));
        let names: Vec<_> = view.entries.iter().map(|e| e.name.as_str()).collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted);
    }

    #[tokio::test]
    async fn contact_data_only_for_office_holders() {
        let (service, _) = service();
        let greta = service
            .directory(&CurrentUser { member: GRETA }, "")
            .await
            .unwrap();
        assert!(!greta.show_contact);
        assert!(greta.entries.iter().all(|e| e.contact.is_none()));
        assert!(!format!("{greta:?}").contains("@example.org"));

        let ben = service
            .directory(&CurrentUser { member: BEN }, "")
            .await
            .unwrap();
        assert!(ben.show_contact);
        let anna = ben
            .entries
            .iter()
            .find(|e| e.name == "Petersen, Anna")
            .unwrap();
        assert_eq!(
            anna.contact.as_ref().unwrap().email.as_deref(),
            Some("anna.petersen@example.org")
        );
    }

    #[tokio::test]
    async fn search_by_name_part() {
        let (service, _) = service();
        let view = service
            .directory(&CurrentUser { member: GRETA }, " peter ")
            .await
            .unwrap();
        let names: Vec<_> = view.entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["Petersen, Anna", "Petersen, Hannes"]);
        assert_eq!(view.query, "peter");
    }
}
