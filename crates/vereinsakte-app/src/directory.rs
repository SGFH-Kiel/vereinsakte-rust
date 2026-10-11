//! Sichtmodell Mitgliederverzeichnis: Kontaktdaten vereinsintern sichtbar,
//! außer bei Sperrvermerk.

use vereinsakte_core::member::MembershipType;
use vereinsakte_core::office::Offices;

use crate::contact::ContactInfo;
use crate::error::AppError;

use crate::service::{ClubService, CurrentUser, office_labels};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirectoryView {
    /// Darf das Mitglied neue Mitglieder aufnehmen?
    pub can_manage: bool,
    pub query: String,
    pub membership_type: String,
    pub license: String,
    pub entries: Vec<DirectoryEntry>,
}

/// Filter der Mitgliederliste.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DirectoryFilter {
    pub query: String,
    pub membership_type: Option<MembershipType>,
    /// Segelschein, z. B. „SKS“ (ohne Beachtung der Schreibweise).
    pub license: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirectoryEntry {
    pub id: u32,
    pub licenses: Vec<String>,
    pub radio_licenses: Vec<String>,
    /// Schlüssel der Mitgliedsart für das Symbol.
    pub type_key: &'static str,
    pub type_icon: &'static str,
    pub name: String,
    pub initials: String,
    pub avatar: String,
    pub offices: Vec<String>,
    pub member_number: u32,
    pub membership_type: String,
    /// E-Mail und Telefon oder „gesperrt“.
    pub contact: ContactInfo,
}

impl ClubService {
    /// Aktive Mitglieder nach Nachnamen sortiert, optional gefiltert nach
    /// einem Teil des Vor- oder Nachnamens.
    pub async fn directory(
        &self,
        user: &CurrentUser,
        query: &str,
    ) -> Result<DirectoryView, AppError> {
        self.directory_filtered(
            user,
            &DirectoryFilter {
                query: query.to_string(),
                ..Default::default()
            },
        )
        .await
    }

    pub async fn directory_filtered(
        &self,
        user: &CurrentUser,
        filter: &DirectoryFilter,
    ) -> Result<DirectoryView, AppError> {
        let query = filter.query.as_str();
        let license = filter.license.trim().to_lowercase();
        let now = self.now();
        let active = self.store.active_offices(now).await?;
        let offices = Offices::new(&active);
        let boats = self.store.boats().await?;
        let principal = self.principal(user).await?;
        let needle = query.trim().to_lowercase();

        let entries = self
            .store
            .active_members(now)
            .await?
            .into_iter()
            .filter(|m| {
                (needle.is_empty()
                    || m.first_name.to_lowercase().contains(&needle)
                    || m.last_name.to_lowercase().contains(&needle))
                    && filter
                        .membership_type
                        .is_none_or(|t| m.membership_type == t)
                    && (license.is_empty()
                        || m.sailing_licenses
                            .iter()
                            .any(|l| l.to_lowercase() == license))
            })
            .map(|m| DirectoryEntry {
                id: m.id.get(),
                licenses: m.sailing_licenses.clone(),
                radio_licenses: m.radio_licenses.clone(),
                type_key: crate::members::membership_type_key(m.membership_type),
                type_icon: crate::members::membership_type_icon(m.membership_type),
                name: format!("{}, {}", m.last_name, m.first_name),
                initials: crate::format::initials(&m.first_name, &m.last_name),
                avatar: crate::format::avatar_class(m.id.get()),
                offices: office_labels(&offices, &boats, m.id, now),
                member_number: m.id.get(),
                membership_type: m.membership_type.to_string(),
                contact: ContactInfo::of(
                    &principal,
                    m.id,
                    m.contact_blocked,
                    m.email.as_ref(),
                    &m.phone,
                ),
            })
            .collect();
        let can_manage = principal.can(&vereinsakte_core::rights::Capability::ManageMembers);
        Ok(DirectoryView {
            can_manage,
            membership_type: filter
                .membership_type
                .map(|t| crate::members::membership_type_key(t).to_string())
                .unwrap_or_default(),
            license: filter.license.trim().to_string(),
            query: query.trim().to_string(),
            entries,
        })
    }
}

#[cfg(test)]
mod tests {
    use vereinsakte_core::example::{BEN, EVA, GRETA, HANNES};
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

    fn entry<'a>(view: &'a DirectoryView, name: &str) -> &'a DirectoryEntry {
        view.entries.iter().find(|e| e.name == name).unwrap()
    }

    #[tokio::test]
    async fn contact_data_is_visible_to_every_member() {
        let (service, _) = service();
        let greta = service
            .directory(&CurrentUser { member: GRETA }, "")
            .await
            .unwrap();
        // Mitglied ohne Amt sieht E-Mail und Telefon, Nummer und Art.
        let anna = entry(&greta, "Petersen, Anna");
        assert!(matches!(
            &anna.contact,
            ContactInfo::Visible { email: Some(email), .. } if email == "anna.petersen@example.org"
        ));
        assert_eq!(anna.member_number, 101);
        assert!(!anna.membership_type.is_empty());
    }

    #[tokio::test]
    async fn blocked_contact_is_hidden_except_for_self_and_board() {
        let (service, _) = service();
        // Hannes setzt den Sperrvermerk.
        service
            .set_contact_blocked(&CurrentUser { member: HANNES }, HANNES, true)
            .await
            .unwrap();
        let name = "Petersen, Hannes";
        for viewer in [GRETA, BEN] {
            let view = service
                .directory(&CurrentUser { member: viewer }, "")
                .await
                .unwrap();
            assert_eq!(entry(&view, name).contact, ContactInfo::Blocked);
            // Weder E-Mail noch Nummern tauchen im Sichtmodell auf.
            let dump = format!("{:?}", entry(&view, name));
            assert!(!dump.contains('@'), "{dump}");
        }
        for viewer in [HANNES, EVA] {
            let view = service
                .directory(&CurrentUser { member: viewer }, "")
                .await
                .unwrap();
            assert!(matches!(
                entry(&view, name).contact,
                ContactInfo::Visible { .. }
            ));
        }
        // Zurücknehmen macht sie wieder sichtbar.
        service
            .set_contact_blocked(&CurrentUser { member: HANNES }, HANNES, false)
            .await
            .unwrap();
        let view = service
            .directory(&CurrentUser { member: GRETA }, "")
            .await
            .unwrap();
        assert!(matches!(
            entry(&view, name).contact,
            ContactInfo::Visible { .. }
        ));
    }

    #[tokio::test]
    async fn only_the_member_sets_the_flag() {
        let (service, _) = service();
        for actor in [EVA, GRETA] {
            let err = service
                .set_contact_blocked(&CurrentUser { member: actor }, HANNES, true)
                .await
                .unwrap_err();
            assert_eq!(err, AppError::Forbidden);
        }
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
