//! In-Memory-Implementierungen der Ports für Tests und Vorführungen. Sie
//! halten die Beispielakte als `ClubRecord`; das ist reine Testinfrastruktur.

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use vereinsakte_core::boat::Boat;
use vereinsakte_core::club_record::ClubRecord;
use vereinsakte_core::ids::MemberId;
use vereinsakte_core::issues::RecordIssue;
use vereinsakte_core::member::Member;
use vereinsakte_core::office::Office;
use vereinsakte_core::time::Timestamp;

use crate::audit::{AuditEntry, AuditFilter, AuditRecord, AuditRepository};
use crate::changes::{Change, ChangeRepository, Record, RecordKind, Scoped};
use crate::error::AppError;
use crate::ports::*;
use crate::scope::{BoatScope, MemberScope, Scope, ScopeRepository};
use crate::task_extras::{TaskCounts, TaskExtrasRepository};
use vereinsakte_core::ids::TaskId;
use vereinsakte_core::task::{AttachmentKind, TaskAttachment, TaskComment};

/// Datenbank-Ersatz im Speicher mit Versionsstand je Mitglied.
#[derive(Debug, Default)]
pub struct MemoryStore {
    record: Mutex<ClubRecord>,
    versions: Mutex<HashMap<(RecordKind, String), Version>>,
    audit: Mutex<Vec<AuditRecord>>,
    comments: Mutex<Vec<TaskComment>>,
    attachments: Mutex<Vec<(TaskAttachment, Vec<u8>)>>,
    boat_images:
        Mutex<HashMap<vereinsakte_core::ids::BoatId, (crate::boat_images::BoatImageMeta, Vec<u8>)>>,
    available: Mutex<bool>,
}

impl MemoryStore {
    pub fn new(record: ClubRecord) -> Self {
        Self {
            record: Mutex::new(record),
            versions: Mutex::default(),
            audit: Mutex::default(),
            comments: Mutex::default(),
            attachments: Mutex::default(),
            boat_images: Mutex::default(),
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
            .get(&(RecordKind::Member, id.to_string()))
            .copied()
            .unwrap_or(Version(1))
    }

    /// Alle Protokolleinträge (für Prüfungen in Tests).
    pub fn audit_entries(&self) -> Vec<AuditEntry> {
        self.audit
            .lock()
            .expect("Lock")
            .iter()
            .map(|r| r.entry.clone())
            .collect()
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

    async fn next_member_number(&self) -> Result<u32, AppError> {
        self.with(|r| r.members.iter().map(|m| m.id.get()).max().unwrap_or(0) + 1)
    }

    async fn save_member(&self, member: &Member, expected: Version) -> Result<Version, AppError> {
        {
            let record = self.record.lock().expect("Lock");
            if record.member(member.id).is_none() {
                return Err(AppError::NotFound(format!("Mitglied {}", member.id)));
            }
        }
        self.commit(
            vec![Change::update(Record::Member(member.clone()), expected)],
            vec![],
        )
        .await?;
        Ok(self.version(member.id))
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

/// Ersetzt oder hängt `item` an, je nachdem ob ein Element mit gleichem Schlüssel existiert.
fn put<T: Clone>(items: &mut Vec<T>, item: &T, same: impl Fn(&T) -> bool) {
    match items.iter_mut().find(|x| same(x)) {
        Some(slot) => *slot = item.clone(),
        None => items.push(item.clone()),
    }
}

fn exists(record: &ClubRecord, r: &Record) -> bool {
    let key = r.key();
    macro_rules! has {
        ($items:expr, $k:expr) => {
            $items.iter().any(|x| $k(x) == key)
        };
    }
    match r.kind() {
        RecordKind::Member => has!(record.members, |x: &Member| x.id.to_string()),
        RecordKind::Boat => has!(record.boats, |x: &Boat| x.id.to_string()),
        RecordKind::Office => has!(record.offices, |x: &Office| x.id.to_string()),
        RecordKind::Checkout => has!(
            record.boat_checkouts,
            |x: &vereinsakte_core::boat_permit::BoatCheckout| x.id.to_string()
        ),
        RecordKind::Revocation => has!(
            record.revocations,
            |x: &vereinsakte_core::boat_permit::Revocation| x.id.to_string()
        ),
        RecordKind::WorkEvent => has!(
            record.work_events,
            |x: &vereinsakte_core::work_event::WorkEvent| x.id.to_string()
        ),
        RecordKind::Task => has!(record.tasks, |x: &vereinsakte_core::task::Task| x
            .id
            .to_string()),
        RecordKind::WorkEntry => has!(
            record.work_entries,
            |x: &vereinsakte_core::work_hours::WorkEntry| x.id.to_string()
        ),
        RecordKind::HourTransfer => has!(
            record.hour_transfers,
            |x: &vereinsakte_core::work_hours::HourTransfer| x.id.to_string()
        ),
        RecordKind::Season => has!(record.seasons, |x: &vereinsakte_core::fees::Season| x
            .year
            .to_string()),
        RecordKind::AnnualRecord => has!(
            record.annual_records,
            |x: &vereinsakte_core::sailing_permit::AnnualRecord| format!("{}:{}", x.member, x.year)
        ),
        RecordKind::Reservation => has!(
            record.reservations,
            |x: &vereinsakte_core::reservation::Reservation| x.id.to_string()
        ),
    }
}

fn save(record: &mut ClubRecord, r: &Record) {
    let key = r.key();
    match r {
        Record::Member(x) => put(&mut record.members, x, |o| o.id.to_string() == key),
        Record::Boat(x) => put(&mut record.boats, x, |o| o.id.to_string() == key),
        Record::Office(x) => put(&mut record.offices, x, |o| o.id.to_string() == key),
        Record::Checkout(x) => put(&mut record.boat_checkouts, x, |o| o.id.to_string() == key),
        Record::Revocation(x) => put(&mut record.revocations, x, |o| o.id.to_string() == key),
        Record::WorkEvent(x) => put(&mut record.work_events, x, |o| o.id.to_string() == key),
        Record::Task(x) => put(&mut record.tasks, x, |o| o.id.to_string() == key),
        Record::WorkEntry(x) => put(&mut record.work_entries, x, |o| o.id.to_string() == key),
        Record::HourTransfer(x) => put(&mut record.hour_transfers, x, |o| o.id.to_string() == key),
        Record::Season(x) => put(&mut record.seasons, x, |o| o.year.to_string() == key),
        Record::AnnualRecord(x) => put(&mut record.annual_records, x, |o| {
            format!("{}:{}", o.member, o.year) == key
        }),
        Record::Reservation(x) => put(&mut record.reservations, x, |o| o.id.to_string() == key),
    }
}

fn remove(record: &mut ClubRecord, kind: RecordKind, key: &str) {
    match kind {
        RecordKind::Member => record.members.retain(|x| x.id.to_string() != key),
        RecordKind::Boat => record.boats.retain(|x| x.id.to_string() != key),
        RecordKind::Office => record.offices.retain(|x| x.id.to_string() != key),
        RecordKind::Checkout => record.boat_checkouts.retain(|x| x.id.to_string() != key),
        RecordKind::Revocation => record.revocations.retain(|x| x.id.to_string() != key),
        RecordKind::WorkEvent => record.work_events.retain(|x| x.id.to_string() != key),
        RecordKind::Task => record.tasks.retain(|x| x.id.to_string() != key),
        RecordKind::WorkEntry => record.work_entries.retain(|x| x.id.to_string() != key),
        RecordKind::HourTransfer => record.hour_transfers.retain(|x| x.id.to_string() != key),
        RecordKind::Season => record.seasons.retain(|x| x.year.to_string() != key),
        RecordKind::AnnualRecord => record
            .annual_records
            .retain(|x| format!("{}:{}", x.member, x.year) != key),
        RecordKind::Reservation => record.reservations.retain(|x| x.id.to_string() != key),
    }
}

#[async_trait]
impl ChangeRepository for MemoryStore {
    async fn commit(&self, changes: Vec<Change>, audit: Vec<AuditEntry>) -> Result<(), AppError> {
        if !*self.available.lock().expect("Lock") {
            return Err(AppError::Storage("Datenbank nicht erreichbar".into()));
        }
        let mut record = self.record.lock().expect("Lock");
        let mut versions = self.versions.lock().expect("Lock");
        let mut next = record.clone();
        let mut next_versions = versions.clone();
        for change in &changes {
            match change {
                Change::Save {
                    record: r,
                    expected,
                } => {
                    let key = (r.kind(), r.key());
                    let current = next_versions.get(&key).copied().unwrap_or(Version(1));
                    let version = match (exists(&next, r), expected) {
                        (true, Some(e)) if *e == current => current.next(),
                        (false, None) => Version(1),
                        _ => return Err(AppError::Conflict),
                    };
                    save(&mut next, r);
                    next_versions.insert(key, version);
                }
                Change::Delete {
                    kind,
                    key,
                    expected,
                } => {
                    let k = (*kind, key.clone());
                    let current = next_versions.get(&k).copied().unwrap_or(Version(1));
                    if current != *expected {
                        return Err(AppError::Conflict);
                    }
                    remove(&mut next, *kind, key);
                    next_versions.remove(&k);
                }
            }
        }
        // Anmeldekonten bleiben ohne Beachtung der Schreibweise eindeutig.
        let mut seen = std::collections::HashSet::new();
        for login in next.members.iter().filter_map(|m| m.login_email.as_deref()) {
            if !seen.insert(login.to_lowercase()) {
                return Err(AppError::Invalid(vec![RecordIssue::DuplicateLoginEmail(
                    login.to_lowercase(),
                )]));
            }
        }
        *record = next;
        *versions = next_versions;
        let mut log = self.audit.lock().expect("Lock");
        for entry in audit {
            let id = log.len() as i64 + 1;
            log.push(AuditRecord { id, entry });
        }
        Ok(())
    }
}

#[async_trait]
impl AuditRepository for MemoryStore {
    async fn audit(&self, filter: &AuditFilter) -> Result<Vec<AuditRecord>, AppError> {
        let log = self.audit.lock().expect("Lock");
        let mut found: Vec<AuditRecord> = log
            .iter()
            .filter(|r| {
                let e = &r.entry;
                filter.from.is_none_or(|from| e.at >= from)
                    && filter.to.is_none_or(|to| e.at < to)
                    && filter.actor.is_none_or(|a| e.actor == a)
                    && filter.action.as_ref().is_none_or(|a| &e.action == a)
                    && filter
                        .involving
                        .is_none_or(|m| e.actor == m || e.subject == Some(m))
                    && filter.object_id.as_ref().is_none_or(|o| &e.object_id == o)
            })
            .cloned()
            .collect();
        found.sort_by_key(|r| std::cmp::Reverse((r.entry.at, r.id)));
        let iter = found.into_iter().skip(filter.offset as usize);
        Ok(match filter.limit {
            Some(limit) => iter.take(limit as usize).collect(),
            None => iter.collect(),
        })
    }
}

#[cfg(test)]
mod commit_tests {
    use vereinsakte_core::example::{self, ANNA, BEN, EVA, FILOU};
    use vereinsakte_core::time::club_time;

    use super::*;

    fn entry(actor: MemberId) -> AuditEntry {
        AuditEntry {
            at: club_time(2026, 10, 9, 12, 0),
            actor,
            action: "boat.edit".into(),
            object_kind: "boat".into(),
            object_id: FILOU.to_string(),
            subject: None,
            summary: "Boot geändert".into(),
        }
    }

    #[tokio::test]
    async fn commit_saves_change_and_audit_together() {
        let store = MemoryStore::new(example::club());
        let mut boat = example::club().boat(FILOU).unwrap().clone();
        boat.name = "Filou II".into();
        store
            .commit(
                vec![Change::update(Record::Boat(boat), Version(1))],
                vec![entry(EVA)],
            )
            .await
            .unwrap();
        assert_eq!(store.snapshot().boat(FILOU).unwrap().name, "Filou II");
        assert_eq!(store.audit_entries().len(), 1);
    }

    #[tokio::test]
    async fn conflict_changes_nothing() {
        let store = MemoryStore::new(example::club());
        let club = example::club();
        let mut boat = club.boat(FILOU).unwrap().clone();
        boat.name = "X".into();
        let mut anna = club.member(ANNA).unwrap().clone();
        anna.address.city = "Y".into();
        let result = store
            .commit(
                vec![
                    Change::update(Record::Boat(boat), Version(1)),
                    Change::update(Record::Member(anna), Version(9)),
                ],
                vec![entry(EVA)],
            )
            .await;
        assert_eq!(result, Err(AppError::Conflict));
        assert_eq!(store.snapshot(), club);
        assert!(store.audit_entries().is_empty());
    }

    #[tokio::test]
    async fn two_members_with_their_own_versions_are_both_saved() {
        let store = MemoryStore::new(example::club());
        let club = example::club();
        for (id, city) in [(ANNA, "A"), (BEN, "B")] {
            let mut m = club.member(id).unwrap().clone();
            m.address.city = city.into();
            store
                .commit(vec![Change::update(Record::Member(m), Version(1))], vec![])
                .await
                .unwrap();
        }
        let snap = store.snapshot();
        assert_eq!(snap.member(ANNA).unwrap().address.city, "A");
        assert_eq!(snap.member(BEN).unwrap().address.city, "B");
    }

    #[tokio::test]
    async fn duplicate_login_email_is_invalid() {
        let store = MemoryStore::new(example::club());
        let club = example::club();
        let mut ben = club.member(BEN).unwrap().clone();
        ben.login_email = club
            .member(ANNA)
            .unwrap()
            .login_email
            .clone()
            .map(|l| l.to_uppercase());
        let result = store
            .commit(
                vec![Change::update(Record::Member(ben), Version(1))],
                vec![],
            )
            .await;
        assert!(matches!(result, Err(AppError::Invalid(_))));
    }
}

#[async_trait]
impl ScopeRepository for MemoryStore {
    async fn scope(&self, scope: &Scope) -> Result<Scoped, AppError> {
        let versions = self.versions.lock().expect("Lock").clone();
        let record = self.with(|full| {
            let wanted = |m: MemberId| match &scope.members {
                MemberScope::None => false,
                MemberScope::Some(ids) => ids.contains(&m),
                MemberScope::All => true,
            };
            let boat_wanted = |b: &Boat| match &scope.boats {
                BoatScope::None => false,
                BoatScope::Some(ids) => ids.contains(&b.id),
                BoatScope::All => true,
            };
            ClubRecord {
                members: full
                    .members
                    .iter()
                    .filter(|m| wanted(m.id))
                    .cloned()
                    .collect(),
                annual_records: full
                    .annual_records
                    .iter()
                    .filter(|a| wanted(a.member))
                    .cloned()
                    .collect(),
                boat_checkouts: full
                    .boat_checkouts
                    .iter()
                    .filter(|c| wanted(c.member))
                    .cloned()
                    .collect(),
                revocations: full
                    .revocations
                    .iter()
                    .filter(|r| wanted(r.member))
                    .cloned()
                    .collect(),
                work_entries: full
                    .work_entries
                    .iter()
                    .filter(|e| wanted(e.member))
                    .cloned()
                    .collect(),
                hour_transfers: full
                    .hour_transfers
                    .iter()
                    .filter(|t| wanted(t.from) || wanted(t.to))
                    .cloned()
                    .collect(),
                boats: full
                    .boats
                    .iter()
                    .filter(|b| boat_wanted(b))
                    .cloned()
                    .collect(),
                seasons: if scope.seasons {
                    full.seasons.clone()
                } else {
                    vec![]
                },
                offices: if scope.offices {
                    full.offices.clone()
                } else {
                    vec![]
                },
                work_events: if scope.work_events {
                    full.work_events.clone()
                } else {
                    vec![]
                },
                tasks: if scope.tasks {
                    full.tasks.clone()
                } else {
                    vec![]
                },
                reservations: full
                    .reservations
                    .iter()
                    .filter(|r| {
                        scope.reservations.as_ref().is_some_and(|w| {
                            w.boats.contains(&r.boat) && r.start < w.to && r.end > w.from
                        }) || scope
                            .reservations_between
                            .is_some_and(|(from, to)| r.start < to && r.end > from)
                    })
                    .cloned()
                    .collect(),
            }
        })?;
        Ok(Scoped { record, versions })
    }
}

impl MemoryStore {
    fn log(&self, entry: AuditEntry) {
        let mut audit = self.audit.lock().expect("Lock");
        let id = i64::try_from(audit.len()).expect("Anzahl") + 1;
        audit.push(AuditRecord { id, entry });
    }

    fn check_available(&self) -> Result<(), AppError> {
        self.with(|_| ())
    }
}

#[async_trait]
impl TaskExtrasRepository for MemoryStore {
    async fn counts(&self) -> Result<HashMap<TaskId, TaskCounts>, AppError> {
        self.check_available()?;
        let mut map: HashMap<TaskId, TaskCounts> = HashMap::new();
        for c in self.comments.lock().expect("Lock").iter() {
            map.entry(c.task).or_default().comments += 1;
        }
        for (a, _) in self.attachments.lock().expect("Lock").iter() {
            let e = map.entry(a.task).or_default();
            e.attachments += 1;
            if a.kind == AttachmentKind::Image {
                e.images += 1;
            }
        }
        Ok(map)
    }

    async fn comments(&self, task: TaskId) -> Result<Vec<TaskComment>, AppError> {
        self.check_available()?;
        let mut list: Vec<TaskComment> = self
            .comments
            .lock()
            .expect("Lock")
            .iter()
            .filter(|c| c.task == task)
            .cloned()
            .collect();
        list.sort_by_key(|c| c.at);
        Ok(list)
    }

    async fn comment(&self, id: uuid::Uuid) -> Result<Option<TaskComment>, AppError> {
        self.check_available()?;
        Ok(self
            .comments
            .lock()
            .expect("Lock")
            .iter()
            .find(|c| c.id == id)
            .cloned())
    }

    async fn add_comment(&self, comment: &TaskComment, audit: AuditEntry) -> Result<(), AppError> {
        self.check_available()?;
        self.comments.lock().expect("Lock").push(comment.clone());
        self.log(audit);
        Ok(())
    }

    async fn edit_comment(
        &self,
        id: uuid::Uuid,
        text: &str,
        audit: AuditEntry,
    ) -> Result<(), AppError> {
        self.check_available()?;
        let mut list = self.comments.lock().expect("Lock");
        let c = list
            .iter_mut()
            .find(|c| c.id == id)
            .ok_or_else(|| AppError::NotFound("Kommentar".into()))?;
        c.text = text.to_string();
        drop(list);
        self.log(audit);
        Ok(())
    }

    async fn delete_comment(&self, id: uuid::Uuid, audit: AuditEntry) -> Result<(), AppError> {
        self.check_available()?;
        let mut list = self.comments.lock().expect("Lock");
        let before = list.len();
        list.retain(|c| c.id != id);
        if list.len() == before {
            return Err(AppError::NotFound("Kommentar".into()));
        }
        drop(list);
        self.log(audit);
        Ok(())
    }

    async fn attachments(&self, task: TaskId) -> Result<Vec<TaskAttachment>, AppError> {
        self.check_available()?;
        let mut list: Vec<TaskAttachment> = self
            .attachments
            .lock()
            .expect("Lock")
            .iter()
            .filter(|(a, _)| a.task == task)
            .map(|(a, _)| a.clone())
            .collect();
        list.sort_by_key(|a| a.at);
        Ok(list)
    }

    async fn attachment(
        &self,
        id: uuid::Uuid,
    ) -> Result<Option<(TaskAttachment, Vec<u8>)>, AppError> {
        self.check_available()?;
        Ok(self
            .attachments
            .lock()
            .expect("Lock")
            .iter()
            .find(|(a, _)| a.id == id)
            .cloned())
    }

    async fn attachment_meta(&self, id: uuid::Uuid) -> Result<Option<TaskAttachment>, AppError> {
        Ok(self.attachment(id).await?.map(|(a, _)| a))
    }

    async fn add_attachment(
        &self,
        attachment: &TaskAttachment,
        data: Vec<u8>,
        audit: AuditEntry,
    ) -> Result<(), AppError> {
        self.check_available()?;
        self.attachments
            .lock()
            .expect("Lock")
            .push((attachment.clone(), data));
        self.log(audit);
        Ok(())
    }

    async fn delete_attachment(&self, id: uuid::Uuid, audit: AuditEntry) -> Result<(), AppError> {
        self.check_available()?;
        let mut list = self.attachments.lock().expect("Lock");
        let before = list.len();
        list.retain(|(a, _)| a.id != id);
        if list.len() == before {
            return Err(AppError::NotFound("Anhang".into()));
        }
        drop(list);
        self.log(audit);
        Ok(())
    }
}

#[async_trait]
impl crate::boat_images::BoatImageRepository for MemoryStore {
    async fn boat_images(
        &self,
    ) -> Result<HashMap<vereinsakte_core::ids::BoatId, crate::boat_images::BoatImageMeta>, AppError>
    {
        self.check_available()?;
        Ok(self
            .boat_images
            .lock()
            .expect("Lock")
            .iter()
            .map(|(k, (m, _))| (*k, m.clone()))
            .collect())
    }

    async fn boat_image(
        &self,
        boat: vereinsakte_core::ids::BoatId,
    ) -> Result<Option<(crate::boat_images::BoatImageMeta, Vec<u8>)>, AppError> {
        self.check_available()?;
        Ok(self.boat_images.lock().expect("Lock").get(&boat).cloned())
    }

    async fn set_boat_image(
        &self,
        boat: vereinsakte_core::ids::BoatId,
        meta: &crate::boat_images::BoatImageMeta,
        data: Vec<u8>,
        audit: AuditEntry,
    ) -> Result<(), AppError> {
        self.check_available()?;
        self.boat_images
            .lock()
            .expect("Lock")
            .insert(boat, (meta.clone(), data));
        self.log(audit);
        Ok(())
    }

    async fn remove_boat_image(
        &self,
        boat: vereinsakte_core::ids::BoatId,
        audit: AuditEntry,
    ) -> Result<(), AppError> {
        self.check_available()?;
        if self
            .boat_images
            .lock()
            .expect("Lock")
            .remove(&boat)
            .is_none()
        {
            return Err(AppError::NotFound("Bild".into()));
        }
        self.log(audit);
        Ok(())
    }
}
