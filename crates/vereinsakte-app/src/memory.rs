//! In-Memory-Implementierungen der Ports für Tests und Vorführungen. Sie
//! halten die Beispielakte als `ClubRecord`; das ist reine Testinfrastruktur.

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use vereinsakte_core::boat::Boat;
use vereinsakte_core::club_record::ClubRecord;
use vereinsakte_core::ids::MemberId;
use vereinsakte_core::member::Member;
use vereinsakte_core::office::Office;
use vereinsakte_core::time::Timestamp;

use crate::error::AppError;
use crate::ports::*;

/// Datenbank-Ersatz im Speicher mit Versionsstand je Mitglied.
#[derive(Debug, Default)]
pub struct MemoryStore {
    record: Mutex<ClubRecord>,
    versions: Mutex<HashMap<MemberId, Version>>,
    available: Mutex<bool>,
}

impl MemoryStore {
    pub fn new(record: ClubRecord) -> Self {
        Self {
            record: Mutex::new(record),
            versions: Mutex::default(),
            available: Mutex::new(true),
        }
    }

    /// Aktueller Inhalt (für Prüfungen in Tests).
    pub fn snapshot(&self) -> ClubRecord {
        self.record.lock().expect("Lock").clone()
    }

    /// Simuliert eine nicht erreichbare Datenbank.
    pub fn set_available(&self, available: bool) {
        *self.available.lock().expect("Lock") = available;
    }

    fn version(&self, id: MemberId) -> Version {
        self.versions
            .lock()
            .expect("Lock")
            .get(&id)
            .copied()
            .unwrap_or(Version(1))
    }

    fn with<T>(&self, f: impl FnOnce(&ClubRecord) -> T) -> Result<T, AppError> {
        if !*self.available.lock().expect("Lock") {
            return Err(AppError::Storage("Datenbank nicht erreichbar".into()));
        }
        Ok(f(&self.record.lock().expect("Lock")))
    }
}

#[async_trait]
impl MemberRepository for MemoryStore {
    async fn member(&self, id: MemberId) -> Result<Option<Versioned<Member>>, AppError> {
        let member = self.with(|r| r.member(id).cloned())?;
        Ok(member.map(|value| Versioned {
            value,
            version: self.version(id),
        }))
    }

    async fn member_by_login_email(&self, login: &str) -> Result<Option<Member>, AppError> {
        self.with(|r| {
            r.members
                .iter()
                .find(|m| {
                    m.login_email
                        .as_deref()
                        .is_some_and(|l| l.eq_ignore_ascii_case(login))
                })
                .cloned()
        })
    }

    async fn active_members(&self, t: Timestamp) -> Result<Vec<MemberSummary>, AppError> {
        self.with(|r| {
            let mut members: Vec<MemberSummary> = r
                .members
                .iter()
                .filter(|m| m.is_active_at(t))
                .map(MemberSummary::from)
                .collect();
            members
                .sort_by(|a, b| (&a.last_name, &a.first_name).cmp(&(&b.last_name, &b.first_name)));
            members
        })
    }

    async fn save_member(&self, member: &Member, expected: Version) -> Result<Version, AppError> {
        let mut record = self.record.lock().expect("Lock");
        let mut versions = self.versions.lock().expect("Lock");
        let current = versions.get(&member.id).copied().unwrap_or(Version(1));
        if current != expected {
            return Err(AppError::Conflict);
        }
        let slot = record
            .member_mut(member.id)
            .ok_or_else(|| AppError::NotFound(format!("Mitglied {}", member.id)))?;
        *slot = member.clone();
        let next = current.next();
        versions.insert(member.id, next);
        Ok(next)
    }
}

#[async_trait]
impl OfficeRepository for MemoryStore {
    async fn active_offices(&self, t: Timestamp) -> Result<Vec<Office>, AppError> {
        self.with(|r| r.active_offices(t).cloned().collect())
    }
}

#[async_trait]
impl BoatRepository for MemoryStore {
    async fn boats(&self) -> Result<Vec<Boat>, AppError> {
        self.with(|r| r.boats.clone())
    }
}

#[async_trait]
impl YearRepository for MemoryStore {
    async fn member_year(&self, member: MemberId, year: i32) -> Result<MemberYearData, AppError> {
        self.with(|r| match r.member_year(member, year) {
            Some(my) => MemberYearData {
                season: my.season.cloned(),
                annual_record: my.annual_record.cloned(),
                work_entries: my.work_entries.into_iter().cloned().collect(),
                hour_transfers: my.hour_transfers.into_iter().cloned().collect(),
            },
            None => MemberYearData::default(),
        })
    }
}

#[async_trait]
impl PermitRepository for MemoryStore {
    async fn permit_history(&self, member: MemberId) -> Result<PermitData, AppError> {
        self.with(|r| {
            let history = r.permit_history(member);
            PermitData {
                checkouts: history.checkouts.into_iter().cloned().collect(),
                revocations: history.revocations.into_iter().cloned().collect(),
            }
        })
    }
}

#[async_trait]
impl Health for MemoryStore {
    async fn ping(&self) -> Result<(), AppError> {
        self.with(|_| ())
    }
}

/// Identitätsverknüpfungen im Speicher.
#[derive(Debug, Default)]
pub struct MemoryIdentityRepository {
    links: Mutex<HashMap<(String, String), MemberId>>,
}

#[async_trait]
impl IdentityRepository for MemoryIdentityRepository {
    async fn find(&self, provider: &str, subject: &str) -> Result<Option<MemberId>, AppError> {
        let links = self.links.lock().expect("Lock");
        Ok(links
            .get(&(provider.to_string(), subject.to_string()))
            .copied())
    }

    async fn link(&self, provider: &str, subject: &str, member: MemberId) -> Result<(), AppError> {
        let mut links = self.links.lock().expect("Lock");
        let key = (provider.to_string(), subject.to_string());
        if links.contains_key(&key) || links.values().any(|&m| m == member) {
            return Err(AppError::Conflict);
        }
        links.insert(key, member);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use vereinsakte_core::example::{self, ANNA, BEN};

    use super::*;

    #[tokio::test]
    async fn member_versions_are_independent() {
        let store = MemoryStore::new(example::club());
        let anna = store.member(ANNA).await.unwrap().unwrap();
        let ben = store.member(BEN).await.unwrap().unwrap();

        let mut changed = anna.value.clone();
        changed.phone.mobile = Some("0151 11111111".into());
        let next = store.save_member(&changed, anna.version).await.unwrap();
        assert_eq!(next, anna.version.next());
        // Zweite Bearbeitung auf altem Stand von Anna: Konflikt, nichts geändert.
        let mut stale = anna.value.clone();
        stale.phone.mobile = Some("0151 22222222".into());
        assert!(matches!(
            store.save_member(&stale, anna.version).await,
            Err(AppError::Conflict)
        ));
        assert_eq!(
            store
                .snapshot()
                .member(ANNA)
                .unwrap()
                .phone
                .mobile
                .as_deref(),
            Some("0151 11111111")
        );
        // Ben ist davon unberührt.
        store.save_member(&ben.value, ben.version).await.unwrap();
    }

    #[tokio::test]
    async fn identities_are_unique_both_ways() {
        let repo = MemoryIdentityRepository::default();
        repo.link("entra", "oid-anna", ANNA).await.unwrap();
        assert_eq!(repo.find("entra", "oid-anna").await.unwrap(), Some(ANNA));
        assert_eq!(repo.find("entra", "unbekannt").await.unwrap(), None);
        assert!(matches!(
            repo.link("entra", "oid-anna", BEN).await,
            Err(AppError::Conflict)
        ));
        assert!(matches!(
            repo.link("entra", "oid-zweit", ANNA).await,
            Err(AppError::Conflict)
        ));
    }

    #[tokio::test]
    async fn unavailable_store() {
        let store = MemoryStore::new(example::club());
        assert!(store.ping().await.is_ok());
        store.set_available(false);
        assert!(store.ping().await.is_err());
    }
}
