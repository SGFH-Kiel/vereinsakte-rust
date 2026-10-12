//! Schreibender Port: Änderungen und Protokolleinträge in einer Transaktion
//! mit Versionsprüfung je Datensatz; Protokoll lesen.

use async_trait::async_trait;
use sqlx::{Row, SqliteConnection};
use vereinsakte_app::AppError;
use vereinsakte_app::audit::{AuditEntry, AuditFilter, AuditRecord, AuditRepository};
use vereinsakte_app::changes::{Change, ChangeRepository, Record, RecordKind};
use vereinsakte_app::ports::Version;
use vereinsakte_core::ids::MemberId;
use vereinsakte_core::issues::RecordIssue;

use crate::db::StorageError;
use crate::repo::{SqliteClubStore, check_foreign_keys};
use crate::values::{
    Val, bind_all, delete_where, get, get_member, get_ts, int, rows, text, ts, where_clause,
};
use crate::write::Aggregate;

fn kind_table(kind: RecordKind) -> &'static str {
    match kind {
        RecordKind::Member => "members",
        RecordKind::Boat => "boats",
        RecordKind::Office => "offices",
        RecordKind::Checkout => "boat_checkouts",
        RecordKind::Revocation => "revocations",
        RecordKind::WorkEvent => "work_events",
        RecordKind::Task => "tasks",
        RecordKind::WorkEntry => "work_entries",
        RecordKind::HourTransfer => "hour_transfers",
        RecordKind::Season => "seasons",
        RecordKind::AnnualRecord => "annual_records",
        RecordKind::Reservation => "reservations",
    }
}

/// Schlüsselspalten eines Datensatzes anhand seiner Art und seines Schlüsseltextes.
fn key_columns(kind: RecordKind, key: &str) -> Result<Vec<(&'static str, Val)>, AppError> {
    let invalid = || AppError::NotFound(format!("ungültiger Schlüssel {key}"));
    Ok(match kind {
        RecordKind::Member => vec![("id", int(key.parse::<i64>().map_err(|_| invalid())?))],
        RecordKind::Season => vec![("year", int(key.parse::<i64>().map_err(|_| invalid())?))],
        RecordKind::AnnualRecord => {
            let (member, year) = key.split_once(':').ok_or_else(invalid)?;
            vec![
                (
                    "member_id",
                    int(member.parse::<i64>().map_err(|_| invalid())?),
                ),
                ("year", int(year.parse::<i64>().map_err(|_| invalid())?)),
            ]
        }
        _ => vec![("id", text(key))],
    })
}

async fn current(
    conn: &mut SqliteConnection,
    table: &str,
    key: Vec<(&str, Val)>,
) -> Result<Option<(Version, i64)>, StorageError> {
    let sql = format!(
        "SELECT version, position FROM {table} WHERE {}",
        where_clause(&key)
    );
    let row = bind_all(sqlx::query(&sql), key.into_iter().map(|(_, v)| v))
        .fetch_optional(conn)
        .await?;
    Ok(row.map(|r| (Version(r.get("version")), r.get("position"))))
}

async fn next_position(conn: &mut SqliteConnection, table: &str) -> Result<i64, StorageError> {
    let sql = format!("SELECT COALESCE(MAX(position), -1) + 1 AS next FROM {table}");
    Ok(sqlx::query(&sql).fetch_one(conn).await?.get("next"))
}

async fn save<T: Aggregate>(
    conn: &mut SqliteConnection,
    item: &T,
    expected: Option<Version>,
) -> Result<(), AppError> {
    let existing = current(conn, T::TABLE, item.key()).await?;
    let position = match (existing, expected) {
        (Some((version, position)), Some(expected)) if version == expected => position,
        (None, None) => next_position(conn, T::TABLE).await?,
        _ => return Err(AppError::Conflict),
    };
    item.write(conn, position).await.map_err(AppError::from)
}

async fn apply(conn: &mut SqliteConnection, change: &Change) -> Result<(), AppError> {
    match change {
        Change::Save { record, expected } => {
            let expected = *expected;
            match &**record {
                Record::Member(x) => {
                    let existing = current(conn, "members", x.key()).await?;
                    let position = match (existing, expected) {
                        (Some((version, position)), Some(e)) if version == e => position,
                        (None, None) => next_position(conn, "members").await?,
                        _ => return Err(AppError::Conflict),
                    };
                    x.write(conn, position).await.map_err(|error| match error {
                        StorageError::Database(sqlx::Error::Database(e))
                            if e.is_unique_violation() && e.message().contains("login_email") =>
                        {
                            AppError::Invalid(vec![RecordIssue::DuplicateLoginEmail(
                                x.login_email.clone().unwrap_or_default().to_lowercase(),
                            )])
                        }
                        other => other.into(),
                    })
                }
                Record::Boat(x) => save(conn, x, expected).await,
                Record::Office(x) => save(conn, x, expected).await,
                Record::Checkout(x) => save(conn, x, expected).await,
                Record::Revocation(x) => save(conn, x, expected).await,
                Record::WorkEvent(x) => save(conn, x, expected).await,
                Record::Task(x) => save(conn, x, expected).await,
                Record::WorkEntry(x) => save(conn, x, expected).await,
                Record::HourTransfer(x) => save(conn, x, expected).await,
                Record::Season(x) => save(conn, x, expected).await,
                Record::AnnualRecord(x) => save(conn, x, expected).await,
                Record::Reservation(x) => save(conn, x, expected).await,
            }
        }
        Change::Delete {
            kind,
            key,
            expected,
        } => {
            let table = kind_table(*kind);
            let columns = key_columns(*kind, key)?;
            match current(conn, table, columns.clone()).await? {
                Some((version, _)) if version == *expected => {}
                _ => return Err(AppError::Conflict),
            }
            delete_where(conn, table, columns).await?;
            Ok(())
        }
    }
}

pub(crate) async fn insert_audit(
    conn: &mut SqliteConnection,
    entry: &AuditEntry,
) -> Result<(), StorageError> {
    sqlx::query(
        "INSERT INTO audit_log (at, actor_id, action, object_kind, object_id, subject_member_id, summary)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(entry.at.to_rfc3339())
    .bind(i64::from(entry.actor.get()))
    .bind(&entry.action)
    .bind(&entry.object_kind)
    .bind(&entry.object_id)
    .bind(entry.subject.map(|m| i64::from(m.get())))
    .bind(&entry.summary)
    .execute(conn)
    .await?;
    Ok(())
}

#[async_trait]
impl ChangeRepository for SqliteClubStore {
    async fn commit(&self, changes: Vec<Change>, audit: Vec<AuditEntry>) -> Result<(), AppError> {
        let mut tx = self.db().pool().begin().await.map_err(StorageError::from)?;
        sqlx::query("PRAGMA defer_foreign_keys = ON")
            .execute(&mut *tx)
            .await
            .map_err(StorageError::from)?;
        for change in &changes {
            apply(&mut tx, change).await?;
        }
        for entry in &audit {
            insert_audit(&mut tx, entry).await?;
        }
        check_foreign_keys(&mut tx).await?;
        tx.commit().await.map_err(StorageError::from)?;
        Ok(())
    }
}

#[async_trait]
impl AuditRepository for SqliteClubStore {
    async fn audit(&self, filter: &AuditFilter) -> Result<Vec<AuditRecord>, AppError> {
        let mut conn = self
            .db()
            .pool()
            .acquire()
            .await
            .map_err(StorageError::from)?;
        let mut conditions = vec!["1 = 1".to_string()];
        let mut binds: Vec<Val> = vec![];
        if let Some(from) = filter.from {
            conditions.push("julianday(at) >= julianday(?)".into());
            binds.push(ts(from));
        }
        if let Some(to) = filter.to {
            conditions.push("julianday(at) < julianday(?)".into());
            binds.push(ts(to));
        }
        if let Some(actor) = filter.actor {
            conditions.push("actor_id = ?".into());
            binds.push(int(actor.get()));
        }
        if let Some(action) = &filter.action {
            conditions.push("action = ?".into());
            binds.push(text(action));
        }
        if let Some(member) = filter.involving {
            conditions.push("(actor_id = ? OR subject_member_id = ?)".into());
            binds.push(int(member.get()));
            binds.push(int(member.get()));
        }
        if let Some(object) = &filter.object_id {
            conditions.push("object_id = ?".into());
            binds.push(text(object));
        }
        let limit = filter.limit.map_or(-1, i64::from);
        let sql = format!(
            "SELECT id, at, actor_id, action, object_kind, object_id, subject_member_id, summary
             FROM audit_log WHERE {} ORDER BY julianday(at) DESC, id DESC LIMIT {limit} OFFSET {}",
            conditions.join(" AND "),
            filter.offset
        );
        let result: Result<Vec<AuditRecord>, StorageError> = rows(&mut conn, &sql, &binds)
            .await?
            .iter()
            .map(|r| {
                let subject: Option<i64> = get(r, "subject_member_id")?;
                Ok(AuditRecord {
                    id: get(r, "id")?,
                    entry: AuditEntry {
                        at: get_ts(r, "at")?,
                        actor: get_member(r, "actor_id")?,
                        action: get(r, "action")?,
                        object_kind: get(r, "object_kind")?,
                        object_id: get(r, "object_id")?,
                        subject: subject
                            .and_then(|n| u32::try_from(n).ok())
                            .and_then(MemberId::new),
                        summary: get(r, "summary")?,
                    },
                })
            })
            .collect();
        Ok(result?)
    }
}

#[cfg(test)]
mod tests {
    use chrono::Timelike;
    use vereinsakte_core::club_record::ClubRecord;
    use vereinsakte_core::example::{self, ANNA, BEN, EVA, FILOU, TRIGON, at};
    use vereinsakte_core::work_hours::WorkEntry;

    use super::*;
    use crate::Database;

    async fn filled() -> SqliteClubStore {
        let db = Database::in_memory().await.unwrap();
        db.migrate().await.unwrap();
        let store = SqliteClubStore::new(db);
        store.import(&example::club(), false).await.unwrap();
        store
    }

    fn entry(actor: MemberId, action: &str, minute: u32) -> AuditEntry {
        AuditEntry {
            at: at(2026, 10, 9, 12, minute),
            actor,
            action: action.into(),
            object_kind: "boat".into(),
            object_id: FILOU.to_string(),
            subject: None,
            summary: "Boot gesperrt".into(),
        }
    }

    fn all_records(club: &ClubRecord) -> Vec<Record> {
        let mut all = vec![];
        all.extend(club.members.iter().cloned().map(Record::Member));
        all.extend(club.boats.iter().cloned().map(Record::Boat));
        all.extend(club.offices.iter().cloned().map(Record::Office));
        all.extend(club.boat_checkouts.iter().cloned().map(Record::Checkout));
        all.extend(club.revocations.iter().cloned().map(Record::Revocation));
        all.extend(club.work_events.iter().cloned().map(Record::WorkEvent));
        all.extend(club.tasks.iter().cloned().map(Record::Task));
        all.extend(club.work_entries.iter().cloned().map(Record::WorkEntry));
        all.extend(
            club.hour_transfers
                .iter()
                .cloned()
                .map(Record::HourTransfer),
        );
        all.extend(club.seasons.iter().cloned().map(Record::Season));
        all.extend(
            club.annual_records
                .iter()
                .cloned()
                .map(Record::AnnualRecord),
        );
        all.extend(club.reservations.iter().cloned().map(Record::Reservation));
        all
    }

    async fn audit_count(store: &SqliteClubStore) -> usize {
        store.audit(&AuditFilter::default()).await.unwrap().len()
    }

    #[tokio::test]
    async fn audit_entry_is_stored_with_the_change() {
        let store = filled().await;
        let mut boat = example::club().boat(FILOU).unwrap().clone();
        boat.name = "Filou II".into();
        store
            .commit(
                vec![Change::update(Record::Boat(boat), Version(1))],
                vec![entry(EVA, "boat.edit", 0)],
            )
            .await
            .unwrap();
        let log = store.audit(&AuditFilter::default()).await.unwrap();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].entry.actor, EVA);
        assert_eq!(
            store.export().await.unwrap().boat(FILOU).unwrap().name,
            "Filou II"
        );
    }

    #[tokio::test]
    async fn failed_change_leaves_no_entry_and_no_partial_change() {
        let store = filled().await;
        let club = example::club();
        let mut boat = club.boat(FILOU).unwrap().clone();
        boat.name = "Neu".into();
        let mut member = club.member(ANNA).unwrap().clone();
        member.address.city = "Anderswo".into();
        // Zweite Änderung mit falscher Version → alles wird zurückgerollt.
        let result = store
            .commit(
                vec![
                    Change::update(Record::Boat(boat), Version(1)),
                    Change::update(Record::Member(member), Version(7)),
                ],
                vec![entry(EVA, "boat.edit", 0)],
            )
            .await;
        assert_eq!(result, Err(AppError::Conflict));
        assert_eq!(audit_count(&store).await, 0);
        assert_eq!(store.export().await.unwrap(), club);
    }

    #[tokio::test]
    async fn audit_entries_cannot_be_changed_or_deleted() {
        let store = filled().await;
        store
            .commit(vec![], vec![entry(EVA, "boat.block", 0)])
            .await
            .unwrap();
        let pool = store.db().pool();
        let update = sqlx::query("UPDATE audit_log SET summary = 'x'")
            .execute(pool)
            .await;
        assert!(update.is_err());
        let delete = sqlx::query("DELETE FROM audit_log").execute(pool).await;
        assert!(delete.is_err());
        assert_eq!(audit_count(&store).await, 1);
    }

    #[tokio::test]
    async fn audit_filters_by_period_actor_and_action() {
        let store = filled().await;
        store
            .commit(
                vec![],
                vec![
                    entry(EVA, "boat.block", 0),
                    entry(BEN, "boat.block", 10),
                    entry(EVA, "payment.post", 20),
                ],
            )
            .await
            .unwrap();
        let list = |filter: AuditFilter| {
            let store = &store;
            async move { store.audit(&filter).await.unwrap() }
        };
        let all = list(AuditFilter::default()).await;
        assert_eq!(
            all.iter().map(|r| r.entry.at.minute()).collect::<Vec<_>>(),
            [20, 10, 0]
        );
        let by_actor = list(AuditFilter {
            actor: Some(EVA),
            ..Default::default()
        })
        .await;
        assert_eq!(by_actor.len(), 2);
        let by_action = list(AuditFilter {
            action: Some("payment.post".into()),
            ..Default::default()
        })
        .await;
        assert_eq!(by_action.len(), 1);
        let period = list(AuditFilter {
            from: Some(at(2026, 10, 9, 12, 5)),
            to: Some(at(2026, 10, 9, 12, 15)),
            ..Default::default()
        })
        .await;
        assert_eq!(period.len(), 1);
        assert_eq!(period[0].entry.actor, BEN);
        let page = list(AuditFilter {
            limit: Some(1),
            offset: 1,
            ..Default::default()
        })
        .await;
        assert_eq!(page[0].entry.at.minute(), 10);
        let involving = list(AuditFilter {
            involving: Some(BEN),
            ..Default::default()
        })
        .await;
        assert_eq!(involving.len(), 1);
    }

    #[tokio::test]
    async fn every_kind_of_record_can_be_saved_with_its_version() {
        let store = filled().await;
        let club = example::club();
        let changes = all_records(&club)
            .into_iter()
            .map(|record| Change::update(record, Version(1)))
            .collect();
        store.commit(changes, vec![]).await.unwrap();
        assert_eq!(store.export().await.unwrap(), club);
        // Jede Speicherung hat die Version erhöht: dieselbe Änderung mit Version 1 scheitert jetzt.
        let again = all_records(&club)
            .into_iter()
            .take(1)
            .map(|record| Change::update(record, Version(1)))
            .collect();
        assert_eq!(store.commit(again, vec![]).await, Err(AppError::Conflict));
    }

    #[tokio::test]
    async fn stale_version_conflicts_and_leaves_data_unchanged() {
        let store = filled().await;
        let club = example::club();
        let mut anna = club.member(ANNA).unwrap().clone();
        anna.address.city = "Neu".into();
        store
            .commit(
                vec![Change::update(Record::Member(anna.clone()), Version(1))],
                vec![],
            )
            .await
            .unwrap();
        anna.address.city = "Noch neuer".into();
        let result = store
            .commit(
                vec![Change::update(Record::Member(anna), Version(1))],
                vec![],
            )
            .await;
        assert_eq!(result, Err(AppError::Conflict));
        assert_eq!(
            store
                .export()
                .await
                .unwrap()
                .member(ANNA)
                .unwrap()
                .address
                .city,
            "Neu"
        );
    }

    #[tokio::test]
    async fn changes_to_different_records_do_not_conflict_and_leave_others_untouched() {
        let store = filled().await;
        let club = example::club();
        let rowid = |id: i64| {
            let store = &store;
            async move {
                sqlx::query("SELECT rowid AS rid FROM members WHERE id = ?")
                    .bind(id)
                    .fetch_one(store.db().pool())
                    .await
                    .unwrap()
                    .get::<i64, _>("rid")
            }
        };
        let before = rowid(103).await;
        let mut anna = club.member(ANNA).unwrap().clone();
        let mut ben = club.member(BEN).unwrap().clone();
        anna.address.city = "A".into();
        ben.address.city = "B".into();
        store
            .commit(
                vec![Change::update(Record::Member(anna), Version(1))],
                vec![],
            )
            .await
            .unwrap();
        store
            .commit(
                vec![Change::update(Record::Member(ben), Version(1))],
                vec![],
            )
            .await
            .unwrap();
        assert_eq!(rowid(103).await, before);
        let exported = store.export().await.unwrap();
        assert_eq!(exported.member(ANNA).unwrap().address.city, "A");
        assert_eq!(exported.member(BEN).unwrap().address.city, "B");
    }

    #[tokio::test]
    async fn create_and_delete_a_work_entry() {
        let store = filled().await;
        let club = example::club();
        let template = club.work_entries[0].clone();
        let mut fresh: WorkEntry = template.clone();
        fresh.id = vereinsakte_core::ids::WorkEntryId::new();
        let key = Record::WorkEntry(fresh.clone()).key();
        store
            .commit(
                vec![Change::create(Record::WorkEntry(fresh.clone()))],
                vec![],
            )
            .await
            .unwrap();
        assert_eq!(
            store.export().await.unwrap().work_entries.len(),
            club.work_entries.len() + 1
        );
        // Doppeltes Anlegen ist ein Konflikt.
        let again = store
            .commit(vec![Change::create(Record::WorkEntry(fresh))], vec![])
            .await;
        assert_eq!(again, Err(AppError::Conflict));
        store
            .commit(
                vec![Change::Delete {
                    kind: RecordKind::WorkEntry,
                    key,
                    expected: Version(1),
                }],
                vec![],
            )
            .await
            .unwrap();
        assert_eq!(
            store.export().await.unwrap().work_entries,
            club.work_entries
        );
    }

    #[tokio::test]
    async fn duplicate_login_email_is_rejected_and_database_unchanged() {
        let store = filled().await;
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
                vec![entry(EVA, "member.edit", 0)],
            )
            .await;
        assert!(matches!(result, Err(AppError::Invalid(_))), "{result:?}");
        assert_eq!(store.export().await.unwrap(), club);
        assert_eq!(audit_count(&store).await, 0);
    }

    #[tokio::test]
    async fn unknown_reference_fails_without_change() {
        let store = filled().await;
        let club = example::club();
        let mut entry_ = club.work_entries[0].clone();
        entry_.id = vereinsakte_core::ids::WorkEntryId::new();
        entry_.subject =
            vereinsakte_core::work_hours::WorkSubject::Boat(vereinsakte_core::ids::BoatId::new());
        let result = store
            .commit(vec![Change::create(Record::WorkEntry(entry_))], vec![])
            .await;
        assert!(result.is_err());
        assert_eq!(store.export().await.unwrap(), club);
        let _ = TRIGON;
    }
}
