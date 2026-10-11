//! SQLite-Implementierung der App-Ports mit gezielten Abfragen sowie
//! Sicherung (Export) und Wiederherstellung (Import) der ganzen Akte.

use async_trait::async_trait;
use sqlx::{Row, SqliteConnection};
use vereinsakte_app::AppError;
use vereinsakte_app::changes::{RecordKind, Scoped};
use vereinsakte_app::ports::*;
use vereinsakte_core::boat::Boat;
use vereinsakte_core::club_record::ClubRecord;
use vereinsakte_core::ids::MemberId;
use vereinsakte_core::issues::RecordIssue;
use vereinsakte_core::member::{Member, PhoneNumbers};
use vereinsakte_core::office::Office;
use vereinsakte_core::time::Timestamp;

use crate::db::{Database, StorageError};
use crate::read::*;
use crate::values::{Filter, get, get_bool, get_choice, get_member, int, member, rows, text, ts};
use crate::write::{Aggregate, write_diff};

/// Zum Zeitpunkt `?1` aktive Mitgliedschaft (Vergleich über `julianday`, weil
/// die gespeicherten Zeitpunkte unterschiedliche Offsets tragen).
const ACTIVE_MEMBER: &str = "julianday(joined_at) <= julianday(?1)
    AND (limited_until IS NULL OR julianday(?1) < julianday(limited_until))
    AND (left_at IS NULL OR julianday(?1) < julianday(left_at))
    AND (expelled_at IS NULL OR julianday(?1) < julianday(expelled_at))";

/// Vereinsakte in SQLite.
#[derive(Clone, Debug)]
pub struct SqliteClubStore {
    db: Database,
}

impl SqliteClubStore {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub(crate) fn db(&self) -> &Database {
        &self.db
    }

    /// Sicherung: die gesamte Akte (ohne Anmeldeidentitäten).
    pub async fn export(&self) -> Result<ClubRecord, StorageError> {
        let mut tx = self.db.pool().begin().await?;
        let record = load_record(&mut tx).await?;
        tx.commit().await?;
        Ok(record)
    }

    /// Wiederherstellung einer Sicherung. Ohne `replace` nur in eine Datenbank
    /// ohne Mitglieder; mit `replace` wird der Bestand an die Akte angeglichen
    /// (Anmeldeidentitäten bleiben für weiterhin vorhandene Mitglieder erhalten).
    pub async fn import(&self, record: &ClubRecord, replace: bool) -> Result<(), StorageError> {
        record
            .validate()
            .map_err(|issues| StorageError::Invalid { issues })?;
        let mut tx = self.db.pool().begin().await?;
        sqlx::query("PRAGMA defer_foreign_keys = ON")
            .execute(&mut *tx)
            .await?;
        let current = load_record(&mut tx).await?;
        if !replace && !current.members.is_empty() {
            return Err(StorageError::NotEmpty);
        }
        write_diff(&mut tx, &current, record).await?;
        check_foreign_keys(&mut tx).await?;
        tx.commit().await?;
        Ok(())
    }
}

/// Meldet die erste Verletzung eines Fremdschlüssels als unbekannten Verweis.
pub(crate) async fn check_foreign_keys(conn: &mut SqliteConnection) -> Result<(), StorageError> {
    let Some(violation) = sqlx::query("PRAGMA foreign_key_check")
        .fetch_optional(&mut *conn)
        .await?
    else {
        return Ok(());
    };
    let table: String = violation.get("table");
    let parent: String = violation.get("parent");
    let fkid: i64 = violation.get("fkid");
    let columns: Vec<String> =
        sqlx::query(r#"SELECT "from" FROM pragma_foreign_key_list(?) WHERE id = ? ORDER BY seq"#)
            .bind(&table)
            .bind(fkid)
            .fetch_all(&mut *conn)
            .await?
            .iter()
            .map(|r| r.get::<String, _>("from"))
            .collect();
    Err(StorageError::UnknownReference {
        table,
        column: columns.join(", "),
        parent,
    })
}

#[async_trait]
impl MemberRepository for SqliteClubStore {
    async fn member(&self, id: MemberId) -> Result<Option<Versioned<Member>>, AppError> {
        let mut tx = self.db.pool().begin().await.map_err(StorageError::from)?;
        let filter = Filter::new("WHERE id = ?", vec![member(id)]);
        let found = load_members(&mut tx, &filter).await?.into_iter().next();
        let version = match &found {
            Some(_) => Some(member_version(&mut tx, id).await?),
            None => None,
        };
        tx.commit().await.map_err(StorageError::from)?;
        Ok(found
            .zip(version)
            .map(|(value, version)| Versioned { value, version }))
    }

    async fn member_by_login_email(&self, login: &str) -> Result<Option<Member>, AppError> {
        let mut tx = self.db.pool().begin().await.map_err(StorageError::from)?;
        // Die Spalte hat COLLATE NOCASE: Vergleich ohne Groß-/Kleinschreibung.
        let filter = Filter::new("WHERE login_email = ?", vec![text(login)]);
        let found = load_members(&mut tx, &filter).await?.into_iter().next();
        tx.commit().await.map_err(StorageError::from)?;
        Ok(found)
    }

    async fn active_members(&self, t: Timestamp) -> Result<Vec<MemberSummary>, AppError> {
        let mut conn = self.db.pool().acquire().await.map_err(StorageError::from)?;
        let sql = format!(
            "SELECT id, first_name, last_name, membership_type, email, phone_landline, phone_business, phone_mobile, contact_blocked
             FROM members WHERE {ACTIVE_MEMBER} ORDER BY last_name, first_name"
        );
        let mut licenses: std::collections::HashMap<MemberId, Vec<String>> = Default::default();
        for r in rows(
            &mut conn,
            "SELECT member_id, kind FROM member_sailing_licenses ORDER BY member_id, position",
            &[],
        )
        .await?
        {
            licenses
                .entry(get_member(&r, "member_id")?)
                .or_default()
                .push(
                    vereinsakte_core::member::SailingLicenseType::from(
                        get::<String>(&r, "kind")?.as_str(),
                    )
                    .to_string(),
                );
        }
        let mut radio: std::collections::HashMap<MemberId, Vec<String>> = Default::default();
        for r in rows(
            &mut conn,
            "SELECT member_id, kind FROM member_radio_licenses ORDER BY member_id, position",
            &[],
        )
        .await?
        {
            radio.entry(get_member(&r, "member_id")?).or_default().push(
                vereinsakte_core::member::RadioLicenseType::from(
                    get::<String>(&r, "kind")?.as_str(),
                )
                .to_string(),
            );
        }
        let result: Result<Vec<MemberSummary>, StorageError> = rows(&mut conn, &sql, &[ts(t)])
            .await?
            .iter()
            .map(|r| {
                Ok(MemberSummary {
                    sailing_licenses: licenses.remove(&get_member(r, "id")?).unwrap_or_default(),
                    radio_licenses: radio.remove(&get_member(r, "id")?).unwrap_or_default(),
                    id: get_member(r, "id")?,
                    first_name: get(r, "first_name")?,
                    last_name: get(r, "last_name")?,
                    membership_type: get_choice(r, "membership_type")?,
                    email: get(r, "email")?,
                    phone: PhoneNumbers {
                        landline: get(r, "phone_landline")?,
                        business: get(r, "phone_business")?,
                        mobile: get(r, "phone_mobile")?,
                    },
                    contact_blocked: get_bool(r, "contact_blocked")?,
                })
            })
            .collect();
        Ok(result?)
    }

    async fn next_member_number(&self) -> Result<u32, AppError> {
        let row = sqlx::query("SELECT COALESCE(MAX(id), 0) + 1 AS next FROM members")
            .fetch_one(self.db.pool())
            .await
            .map_err(StorageError::from)?;
        Ok(u32::try_from(row.get::<i64, _>("next")).unwrap_or(u32::MAX))
    }

    async fn save_member(&self, changed: &Member, expected: Version) -> Result<Version, AppError> {
        let mut tx = self.db.pool().begin().await.map_err(StorageError::from)?;
        sqlx::query("PRAGMA defer_foreign_keys = ON")
            .execute(&mut *tx)
            .await
            .map_err(StorageError::from)?;
        let row = sqlx::query("SELECT version, position FROM members WHERE id = ?")
            .bind(i64::from(changed.id.get()))
            .fetch_optional(&mut *tx)
            .await
            .map_err(StorageError::from)?
            .ok_or_else(|| AppError::NotFound(format!("Mitglied {}", changed.id)))?;
        if Version(row.get("version")) != expected {
            return Err(AppError::Conflict);
        }
        let position: i64 = row.get("position");
        if let Err(error) = changed.write(&mut tx, position).await {
            return Err(match error {
                StorageError::Database(sqlx::Error::Database(e))
                    if e.is_unique_violation() && e.message().contains("login_email") =>
                {
                    let login = changed
                        .login_email
                        .clone()
                        .unwrap_or_default()
                        .to_lowercase();
                    AppError::Invalid(vec![RecordIssue::DuplicateLoginEmail(login)])
                }
                other => other.into(),
            });
        }
        check_foreign_keys(&mut tx).await?;
        let version = member_version(&mut tx, changed.id).await?;
        tx.commit().await.map_err(StorageError::from)?;
        Ok(version)
    }
}

async fn member_version(
    conn: &mut SqliteConnection,
    id: MemberId,
) -> Result<Version, StorageError> {
    let row = sqlx::query("SELECT version FROM members WHERE id = ?")
        .bind(i64::from(id.get()))
        .fetch_one(conn)
        .await?;
    Ok(Version(row.get("version")))
}

#[async_trait]
impl OfficeRepository for SqliteClubStore {
    async fn active_offices(&self, t: Timestamp) -> Result<Vec<Office>, AppError> {
        let mut conn = self.db.pool().acquire().await.map_err(StorageError::from)?;
        let filter = Filter::new(
            "WHERE julianday(start_at) <= julianday(?1) AND (end_at IS NULL OR julianday(?1) < julianday(end_at))",
            vec![ts(t)],
        );
        Ok(load_offices(&mut conn, &filter).await?)
    }
}

#[async_trait]
impl BoatRepository for SqliteClubStore {
    async fn boats(&self) -> Result<Vec<Boat>, AppError> {
        let mut tx = self.db.pool().begin().await.map_err(StorageError::from)?;
        let boats = load_boats(&mut tx).await?;
        tx.commit().await.map_err(StorageError::from)?;
        Ok(boats)
    }
}

#[async_trait]
impl YearRepository for SqliteClubStore {
    async fn member_year(&self, id: MemberId, year: i32) -> Result<MemberYearData, AppError> {
        let mut tx = self.db.pool().begin().await.map_err(StorageError::from)?;
        let season = load_seasons(&mut tx, &Filter::new("WHERE year = ?", vec![int(year)])).await?;
        let record = load_annual_records(
            &mut tx,
            &Filter::new(
                "WHERE member_id = ? AND year = ?",
                vec![member(id), int(year)],
            ),
        )
        .await?;
        // Jahr im gespeicherten Offset = die ersten vier Zeichen des Zeitpunkts.
        let entries = load_work_entries(
            &mut tx,
            &Filter::new(
                "WHERE member_id = ? AND CAST(substr(coalesce(date_at, start_at), 1, 4) AS INTEGER) = ?",
                vec![member(id), int(year)],
            ),
        )
        .await?;
        let transfers = load_hour_transfers(
            &mut tx,
            &Filter::new(
                "WHERE year = ?1 AND (from_member = ?2 OR to_member = ?2)",
                vec![int(year), member(id)],
            ),
        )
        .await?;
        tx.commit().await.map_err(StorageError::from)?;
        Ok(MemberYearData {
            season: season.into_iter().next(),
            annual_record: record.into_iter().next(),
            work_entries: entries,
            hour_transfers: transfers,
        })
    }
}

#[async_trait]
impl PermitRepository for SqliteClubStore {
    async fn permit_history(&self, id: MemberId) -> Result<PermitData, AppError> {
        let mut tx = self.db.pool().begin().await.map_err(StorageError::from)?;
        let filter = Filter::new("WHERE member_id = ?", vec![member(id)]);
        let checkouts = load_checkouts(&mut tx, &filter).await?;
        let revocations = load_revocations(&mut tx, &filter).await?;
        tx.commit().await.map_err(StorageError::from)?;
        Ok(PermitData {
            checkouts,
            revocations,
        })
    }
}

#[async_trait]
impl Health for SqliteClubStore {
    async fn ping(&self) -> Result<(), AppError> {
        sqlx::query("SELECT 1")
            .execute(self.db.pool())
            .await
            .map_err(StorageError::from)?;
        Ok(())
    }
}

/// Anmeldeidentitäten in SQLite.
#[derive(Clone, Debug)]
pub struct SqliteIdentityRepository {
    db: Database,
}

impl SqliteIdentityRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
}

#[async_trait]
impl IdentityRepository for SqliteIdentityRepository {
    async fn find(&self, provider: &str, subject: &str) -> Result<Option<MemberId>, AppError> {
        let row =
            sqlx::query("SELECT member_id FROM identities WHERE provider = ? AND subject = ?")
                .bind(provider)
                .bind(subject)
                .fetch_optional(self.db.pool())
                .await
                .map_err(StorageError::from)?;
        Ok(row
            .and_then(|r| u32::try_from(r.get::<i64, _>("member_id")).ok())
            .and_then(MemberId::new))
    }

    async fn link(&self, provider: &str, subject: &str, member: MemberId) -> Result<(), AppError> {
        let result =
            sqlx::query("INSERT INTO identities (provider, subject, member_id) VALUES (?, ?, ?)")
                .bind(provider)
                .bind(subject)
                .bind(i64::from(member.get()))
                .execute(self.db.pool())
                .await;
        match result {
            Ok(_) => Ok(()),
            Err(sqlx::Error::Database(e)) if e.is_unique_violation() => Err(AppError::Conflict),
            Err(sqlx::Error::Database(e)) if e.is_foreign_key_violation() => {
                Err(AppError::NotFound(format!("Mitglied {member}")))
            }
            Err(e) => Err(StorageError::from(e).into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::Datelike;
    use sqlx::Row;
    use vereinsakte_core::account::{Claim, ClaimKind, MemberAccount};
    use vereinsakte_core::example::{self, ANNA, BEN, GRETA, TRIGON, at};
    use vereinsakte_core::ids::{BoatId, ReservationId, TaskId};
    use vereinsakte_core::member::{SailingLicense, SailingLicenseType};
    use vereinsakte_core::requirement::AnyOf;
    use vereinsakte_core::work_hours::{WorkReference, WorkSubject};

    use super::*;
    use crate::SqliteIdentityRepository;

    const FIXTURE: &str = include_str!("../../vereinsakte-core/testdata/vereinsakte_beispiel.json");

    async fn empty() -> SqliteClubStore {
        let db = Database::in_memory().await.unwrap();
        db.migrate().await.unwrap();
        SqliteClubStore::new(db)
    }

    async fn filled() -> SqliteClubStore {
        let store = empty().await;
        store.import(&example::club(), false).await.unwrap();
        store
    }

    #[tokio::test]
    async fn contact_block_round_trips_and_reaches_summaries() {
        let store = filled().await;
        let mut record = store.export().await.unwrap();
        record.member_mut(GRETA).unwrap().contact_blocked = true;
        let store = empty().await;
        store.import(&record, false).await.unwrap();
        assert!(
            store
                .export()
                .await
                .unwrap()
                .member(GRETA)
                .unwrap()
                .contact_blocked
        );
        let summaries = store.active_members(at(2026, 10, 9, 12, 0)).await.unwrap();
        let greta = summaries.iter().find(|m| m.id == GRETA).unwrap();
        assert!(greta.contact_blocked);
        assert!(
            summaries
                .iter()
                .filter(|m| m.id != GRETA)
                .all(|m| !m.contact_blocked)
        );
    }

    #[tokio::test]
    async fn legacy_guest_rows_are_read_into_the_remark_and_dropped_on_save() {
        use vereinsakte_core::example::JOLLE_SAMSTAG;
        let store = filled().await;
        sqlx::query(
            "INSERT INTO reservation_guests (reservation_id, position, name, phone)
             VALUES (?, 0, 'Max Gast', '0160 1'), (?, 1, 'Paula Gast', NULL)",
        )
        .bind(JOLLE_SAMSTAG.to_string())
        .bind(JOLLE_SAMSTAG.to_string())
        .execute(store.db.pool())
        .await
        .unwrap();
        let record = store.export().await.unwrap();
        let reservation = record.reservation(JOLLE_SAMSTAG).unwrap().clone();
        assert!(
            reservation
                .remark
                .as_deref()
                .unwrap()
                .ends_with("Gäste: Max Gast (0160 1), Paula Gast")
        );
        // Beim Speichern geht die Angabe in die Bemerkung über, die alten Zeilen entfallen.
        let mut changed = record.clone();
        changed.reservations.iter_mut().for_each(|r| {
            if r.id == JOLLE_SAMSTAG {
                r.remark = Some(format!("{} ", r.remark.clone().unwrap()));
            }
        });
        store.import(&changed, true).await.unwrap();
        let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM reservation_guests")
            .fetch_one(store.db.pool())
            .await
            .unwrap();
        assert_eq!(rows, 0);
        assert!(
            store
                .export()
                .await
                .unwrap()
                .reservation(JOLLE_SAMSTAG)
                .unwrap()
                .remark
                .as_deref()
                .unwrap()
                .contains("Max Gast")
        );
    }

    #[tokio::test]
    async fn empty_database_exports_empty_record() {
        assert_eq!(empty().await.export().await.unwrap(), ClubRecord::default());
    }

    #[tokio::test]
    async fn example_club_survives_import_and_export() {
        let store = filled().await;
        let record = store.export().await.unwrap();
        assert_eq!(record, example::club());
        let anna = record.member(ANNA).unwrap();
        assert_eq!(
            anna.membership.joined_at.to_rfc3339(),
            "2015-03-01T00:00:00+01:00"
        );
        assert_eq!(
            anna.sailing_licenses[0].acquired_at.unwrap().to_rfc3339(),
            "2014-06-20T00:00:00+02:00"
        );
    }

    /// Jede gezielte Abfrage liefert genau das, was die ganze Akte für diesen
    /// Anwendungsfall enthält.
    #[tokio::test]
    async fn targeted_queries_match_whole_record() {
        let store = filled().await;
        let club = example::club();
        for t in [
            at(2026, 6, 13, 10, 0),
            at(2026, 11, 15, 12, 0),
            at(2025, 7, 1, 12, 0),
        ] {
            let year = t.year();
            let active: Vec<_> = club
                .members
                .iter()
                .filter(|m| m.is_active_at(t))
                .map(|m| m.id)
                .collect();
            let mut loaded: Vec<_> = store
                .active_members(t)
                .await
                .unwrap()
                .into_iter()
                .map(|m| m.id)
                .collect();
            let mut expected = active.clone();
            loaded.sort();
            expected.sort();
            assert_eq!(loaded, expected, "{t}");
            let offices: Vec<_> = club.active_offices(t).cloned().collect();
            assert_eq!(store.active_offices(t).await.unwrap(), offices, "{t}");

            for m in &club.members {
                assert_eq!(store.member(m.id).await.unwrap().unwrap().value, *m);
                let data = store.member_year(m.id, year).await.unwrap();
                let scoped = data.view(m, year);
                let whole = club.member_year(m.id, year).unwrap();
                assert_eq!(scoped.work_entries, whole.work_entries, "{} {year}", m.id);
                assert_eq!(
                    scoped.hour_transfers, whole.hour_transfers,
                    "{} {year}",
                    m.id
                );
                assert_eq!(scoped.annual_record, whole.annual_record, "{} {year}", m.id);
                assert_eq!(scoped.season, whole.season, "{year}");
                assert_eq!(scoped.sailing_permit_at(t), club.sailing_permit_at(m.id, t));

                let permits = store.permit_history(m.id).await.unwrap();
                let history = permits.view(m);
                for boat in &club.boats {
                    assert_eq!(
                        history.boat_permit(boat.id, t),
                        club.boat_permit(m.id, boat.id, t)
                    );
                }
            }
        }
        assert_eq!(store.boats().await.unwrap(), club.boats);
    }

    #[tokio::test]
    async fn login_email_lookup_ignores_case() {
        let store = filled().await;
        let anna = store
            .member_by_login_email("ANNA.Petersen@sgfh.example")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(anna.id, ANNA);
        assert!(
            store
                .member_by_login_email("anna.petersen@example.org")
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn versions_per_member() {
        let store = filled().await;
        let anna = store.member(ANNA).await.unwrap().unwrap();
        let ben = store.member(BEN).await.unwrap().unwrap();

        let mut first = anna.value.clone();
        first.phone.mobile = Some("0151 11111111".into());
        let next = store.save_member(&first, anna.version).await.unwrap();
        assert_eq!(next, anna.version.next());

        // Zweite Bearbeitung von Anna auf altem Stand: Konflikt, nichts geändert.
        let mut second = anna.value.clone();
        second.phone.mobile = Some("0151 22222222".into());
        assert!(matches!(
            store.save_member(&second, anna.version).await,
            Err(AppError::Conflict)
        ));
        let stored = store.member(ANNA).await.unwrap().unwrap();
        assert_eq!(stored.value.phone.mobile.as_deref(), Some("0151 11111111"));

        // Ben mit seinem eigenen Stand: unberührt von Annas Version.
        let mut ben_changed = ben.value.clone();
        ben_changed
            .sailing_licenses
            .push(SailingLicense::new("Jüngstenschein"));
        store.save_member(&ben_changed, ben.version).await.unwrap();
        let ben_stored = store.member(BEN).await.unwrap().unwrap().value;
        assert_eq!(
            ben_stored.sailing_licenses.last().unwrap().kind,
            SailingLicenseType::Other("Jüngstenschein".into())
        );
    }

    async fn rowids(store: &SqliteClubStore, sql: &str) -> Vec<i64> {
        sqlx::query(sql)
            .fetch_all(store.db.pool())
            .await
            .unwrap()
            .iter()
            .map(|r| r.get(0))
            .collect()
    }

    #[tokio::test]
    async fn saving_a_member_touches_only_its_rows() {
        let store = filled().await;
        let others = "SELECT rowid FROM member_sailing_licenses WHERE member_id <> 101
                      UNION ALL SELECT rowid FROM claims WHERE member_id <> 101
                      UNION ALL SELECT rowid FROM requirement_qualifications";
        let before = rowids(&store, others).await;
        let versions = "SELECT version FROM members WHERE id <> 101";
        let versions_before = rowids(&store, versions).await;

        let anna = store.member(ANNA).await.unwrap().unwrap();
        let mut changed = anna.value.clone();
        changed.address_valid = false;
        store.save_member(&changed, anna.version).await.unwrap();

        assert_eq!(rowids(&store, others).await, before);
        assert_eq!(rowids(&store, versions).await, versions_before);
        assert!(
            !store
                .member(ANNA)
                .await
                .unwrap()
                .unwrap()
                .value
                .address_valid
        );
    }

    #[tokio::test]
    async fn duplicate_login_account_is_rejected() {
        let store = filled().await;
        let greta = store.member(GRETA).await.unwrap().unwrap();
        let mut changed = greta.value.clone();
        changed.login_email = Some("ANNA.PETERSEN@sgfh.example".into());
        match store.save_member(&changed, greta.version).await {
            Err(AppError::Invalid(issues)) => {
                assert_eq!(
                    issues,
                    vec![RecordIssue::DuplicateLoginEmail(
                        "anna.petersen@sgfh.example".into()
                    )]
                );
            }
            other => panic!("erwartet Invalid, erhalten {other:?}"),
        }
        assert_eq!(store.member(GRETA).await.unwrap().unwrap(), greta);
    }

    #[tokio::test]
    async fn free_license_names_and_empty_requirement_groups_survive_backup() {
        let store = filled().await;
        let mut record = example::club();
        record
            .member_mut(BEN)
            .unwrap()
            .sailing_licenses
            .push(SailingLicense::new("Jüngstenschein"));
        let trigon = record.boat_mut(TRIGON).unwrap();
        trigon.skipper_requirement.0.push(AnyOf::default());
        trigon.crew_rules[0].co_skipper_requirement.0.clear();
        store.import(&record, true).await.unwrap();
        assert_eq!(store.export().await.unwrap(), record);
    }

    #[tokio::test]
    async fn plain_sql_join_finds_sks_holders() {
        let store = filled().await;
        let names: Vec<String> = sqlx::query(
            "SELECT m.first_name FROM members m
             JOIN member_sailing_licenses l ON l.member_id = m.id
             WHERE l.kind = 'SKS' ORDER BY m.id",
        )
        .fetch_all(store.db.pool())
        .await
        .unwrap()
        .iter()
        .map(|r| r.get("first_name"))
        .collect();
        assert_eq!(names, vec!["Anna", "Ben", "Clara", "Eva", "Frank"]);
        let cents: i64 = sqlx::query(
            "SELECT amount_cents FROM payments WHERE member_id = 101 ORDER BY position",
        )
        .fetch_one(store.db.pool())
        .await
        .unwrap()
        .get("amount_cents");
        assert_eq!(cents, 25000);
    }

    #[tokio::test]
    async fn identities() {
        let store = filled().await;
        let ids = SqliteIdentityRepository::new(store.db.clone());
        ids.link("entra", "oid-anna", ANNA).await.unwrap();
        assert_eq!(ids.find("entra", "oid-anna").await.unwrap(), Some(ANNA));
        assert!(matches!(
            ids.link("entra", "oid-anna", BEN).await,
            Err(AppError::Conflict)
        ));
        assert!(matches!(
            ids.link("entra", "oid-zweit", ANNA).await,
            Err(AppError::Conflict)
        ));
        let unknown = MemberId::new(999).unwrap();
        assert!(matches!(
            ids.link("entra", "oid-x", unknown).await,
            Err(AppError::NotFound(_))
        ));
        assert_eq!(store.export().await.unwrap(), example::club());
        store.import(&example::club(), true).await.unwrap();
        assert_eq!(ids.find("entra", "oid-anna").await.unwrap(), Some(ANNA));
    }

    #[tokio::test]
    async fn import_and_export_of_fixture_file() {
        let record: ClubRecord = serde_json::from_str(FIXTURE).unwrap();
        let store = empty().await;
        store.import(&record, false).await.unwrap();
        let exported = store.export().await.unwrap();
        assert_eq!(exported, record);
        assert_eq!(
            serde_json::to_string_pretty(&exported).unwrap(),
            FIXTURE.trim_end()
        );
        assert!(matches!(
            store.import(&record, false).await,
            Err(StorageError::NotEmpty)
        ));
        store.import(&record, true).await.unwrap();
    }

    #[tokio::test]
    async fn invalid_records_are_not_imported() {
        let store = empty().await;
        let mut duplicate = example::club();
        duplicate.members[1].id = ANNA;
        duplicate.members[1].login_email = None;
        match store.import(&duplicate, false).await {
            Err(StorageError::Invalid { issues }) => {
                assert!(issues.contains(&RecordIssue::DuplicateMemberId(ANNA)));
            }
            other => panic!("erwartet Invalid, erhalten {other:?}"),
        }

        // Verweis auf eine unbekannte Aufgabe: meldet schon die Prüfung der Akte.
        let mut unknown_task = example::club();
        unknown_task.work_entries[0].reference = Some(WorkReference::Task(TaskId::new()));
        let error = store.import(&unknown_task, false).await.unwrap_err();
        assert!(error.to_string().contains("existiert nicht"), "{error}");

        // Verweise, die nur die Datenbank prüft, melden Tabelle und Spalte.
        let mut unknown_boat = example::club();
        unknown_boat.work_entries[0].subject = WorkSubject::Boat(BoatId::new());
        match store.import(&unknown_boat, false).await {
            Err(StorageError::UnknownReference {
                table,
                column,
                parent,
            }) => {
                assert_eq!(
                    (table.as_str(), column.as_str(), parent.as_str()),
                    ("work_entries", "subject_boat_id", "boats")
                );
            }
            other => panic!("erwartet UnknownReference, erhalten {other:?}"),
        }
        let mut unknown_reservation = example::club();
        let anna = unknown_reservation.member_mut(ANNA).unwrap();
        anna.account = MemberAccount::default();
        anna.account.claims.push(Claim::new(
            ClaimKind::TourFee {
                reservation: ReservationId::new(),
                guest: None,
            },
            "12".parse().unwrap(),
            "Tourgebühr",
            at(2026, 7, 1, 0, 0),
            None,
        ));
        assert!(matches!(
            store.import(&unknown_reservation, false).await,
            Err(StorageError::UnknownReference { .. })
        ));
        assert_eq!(store.export().await.unwrap(), ClubRecord::default());
    }

    #[tokio::test]
    async fn sessions_are_stored_and_expire() {
        use tower_sessions::SessionStore;
        use tower_sessions::session::{Id, Record};

        let db = Database::in_memory().await.unwrap();
        db.migrate().await.unwrap();
        let sessions = db.session_store();
        let mut record = Record {
            id: Id::default(),
            data: [("member".to_string(), serde_json::json!(101))]
                .into_iter()
                .collect(),
            expiry_date: time::OffsetDateTime::now_utc() + time::Duration::hours(1),
        };
        sessions.create(&mut record).await.unwrap();
        assert_eq!(
            sessions.load(&record.id).await.unwrap().unwrap().data,
            record.data
        );
        sessions.delete(&record.id).await.unwrap();
        assert!(sessions.load(&record.id).await.unwrap().is_none());

        let mut expired = Record {
            id: Id::default(),
            data: Default::default(),
            expiry_date: time::OffsetDateTime::now_utc() - time::Duration::minutes(1),
        };
        sessions.create(&mut expired).await.unwrap();
        assert!(sessions.load(&expired.id).await.unwrap().is_none());
    }
}

/// Versionsstände aller Zeilen einer Tabelle (Schlüsseltext → Version).
async fn versions_of(
    conn: &mut SqliteConnection,
    versions: &mut std::collections::HashMap<(RecordKind, String), Version>,
    kind: RecordKind,
    table: &str,
    key_sql: &str,
) -> Result<(), StorageError> {
    let sql = format!("SELECT {key_sql} AS k, version FROM {table}");
    for row in sqlx::query(&sql).fetch_all(conn).await? {
        versions.insert((kind, row.get("k")), Version(row.get("version")));
    }
    Ok(())
}

#[async_trait]
impl vereinsakte_app::scope::ScopeRepository for SqliteClubStore {
    async fn scope(&self, scope: &vereinsakte_app::scope::Scope) -> Result<Scoped, AppError> {
        use vereinsakte_app::scope::{BoatScope, MemberScope};

        let mut tx = self.db.pool().begin().await.map_err(StorageError::from)?;
        let mut record = ClubRecord::default();

        // Filter auf die gewünschten Mitglieder; `column` ist die Spalte mit der Mitgliedsnummer.
        let by_member = |column: &str| -> Option<Filter> {
            match &scope.members {
                MemberScope::None => None,
                MemberScope::All => Some(Filter::all()),
                MemberScope::Some(ids) if ids.is_empty() => None,
                MemberScope::Some(ids) => Some(Filter::new(
                    &format!("WHERE {column} IN ({})", vec!["?"; ids.len()].join(", ")),
                    ids.iter().map(|id| member(*id)).collect(),
                )),
            }
        };
        if let Some(filter) = by_member("id") {
            record.members = load_members(&mut tx, &filter).await?;
        }
        if let Some(filter) = by_member("member_id") {
            record.annual_records = load_annual_records(&mut tx, &filter).await?;
            record.boat_checkouts = load_checkouts(&mut tx, &filter).await?;
            record.revocations = load_revocations(&mut tx, &filter).await?;
            record.work_entries = load_work_entries(&mut tx, &filter).await?;
        }
        let transfers = match &scope.members {
            MemberScope::Some(ids) if !ids.is_empty() => {
                let marks = vec!["?"; ids.len()].join(", ");
                let binds = ids.iter().chain(ids.iter()).map(|id| member(*id)).collect();
                Some(Filter::new(
                    &format!("WHERE from_member IN ({marks}) OR to_member IN ({marks})"),
                    binds,
                ))
            }
            MemberScope::All => Some(Filter::all()),
            _ => None,
        };
        if let Some(filter) = transfers {
            record.hour_transfers = load_hour_transfers(&mut tx, &filter).await?;
        }
        match &scope.boats {
            BoatScope::None => {}
            BoatScope::All => record.boats = load_boats(&mut tx).await?,
            BoatScope::Some(ids) => {
                record.boats = load_boats(&mut tx)
                    .await?
                    .into_iter()
                    .filter(|b| ids.contains(&b.id))
                    .collect();
            }
        }
        if scope.seasons {
            record.seasons = load_seasons(&mut tx, &Filter::all()).await?;
        }
        if scope.offices {
            record.offices = load_offices(&mut tx, &Filter::all()).await?;
        }
        if scope.work_events {
            record.work_events = load_work_events(&mut tx).await?;
        }
        if scope.tasks {
            record.tasks = load_tasks(&mut tx).await?;
        }
        if scope.reservations.is_some() || scope.reservations_between.is_some() {
            record.reservations = load_reservations(&mut tx)
                .await?
                .into_iter()
                .filter(|r| {
                    scope.reservations.as_ref().is_some_and(|w| {
                        w.boats.contains(&r.boat) && r.start < w.to && r.end > w.from
                    }) || scope
                        .reservations_between
                        .is_some_and(|(from, to)| r.start < to && r.end > from)
                })
                .collect();
        }

        let mut versions = std::collections::HashMap::new();
        let v = &mut versions;
        let c = &mut *tx;
        if !record.members.is_empty() {
            versions_of(c, v, RecordKind::Member, "members", "CAST(id AS TEXT)").await?;
        }
        if !record.boats.is_empty() {
            versions_of(c, v, RecordKind::Boat, "boats", "id").await?;
        }
        if !record.offices.is_empty() {
            versions_of(c, v, RecordKind::Office, "offices", "id").await?;
        }
        if !record.boat_checkouts.is_empty() {
            versions_of(c, v, RecordKind::Checkout, "boat_checkouts", "id").await?;
        }
        if !record.revocations.is_empty() {
            versions_of(c, v, RecordKind::Revocation, "revocations", "id").await?;
        }
        if !record.work_events.is_empty() {
            versions_of(c, v, RecordKind::WorkEvent, "work_events", "id").await?;
        }
        if !record.tasks.is_empty() {
            versions_of(c, v, RecordKind::Task, "tasks", "id").await?;
        }
        if !record.work_entries.is_empty() {
            versions_of(c, v, RecordKind::WorkEntry, "work_entries", "id").await?;
        }
        if !record.hour_transfers.is_empty() {
            versions_of(c, v, RecordKind::HourTransfer, "hour_transfers", "id").await?;
        }
        if !record.seasons.is_empty() {
            versions_of(c, v, RecordKind::Season, "seasons", "CAST(year AS TEXT)").await?;
        }
        if !record.annual_records.is_empty() {
            let key = "member_id || ':' || year";
            versions_of(c, v, RecordKind::AnnualRecord, "annual_records", key).await?;
        }
        if !record.reservations.is_empty() {
            versions_of(c, v, RecordKind::Reservation, "reservations", "id").await?;
        }
        tx.commit().await.map_err(StorageError::from)?;
        Ok(Scoped { record, versions })
    }
}

#[cfg(test)]
mod scope_tests {
    use vereinsakte_app::memory::MemoryStore;
    use vereinsakte_app::scope::{Scope, ScopeRepository};
    use vereinsakte_core::example::{self, at};

    use super::*;

    async fn filled() -> SqliteClubStore {
        let db = Database::in_memory().await.unwrap();
        db.migrate().await.unwrap();
        let store = SqliteClubStore::new(db);
        store.import(&example::club(), false).await.unwrap();
        store
    }

    #[tokio::test]
    async fn reservation_check_on_scope_equals_check_on_whole_record() {
        let store = filled().await;
        let full = example::club();
        assert!(!full.reservations.is_empty());
        for reservation in &full.reservations {
            let scope = store
                .scope(&Scope::for_reservation(reservation))
                .await
                .unwrap()
                .record;
            assert_eq!(
                scope.check_reservation(reservation),
                full.check_reservation(reservation),
                "Reservierung {}",
                reservation.id
            );
        }
    }

    #[tokio::test]
    async fn fee_claims_on_scope_equal_claims_on_whole_record() {
        let store = filled().await;
        let full = example::club();
        let now = at(2026, 10, 9, 12, 0);
        // Forderungen tragen frische Kennungen; verglichen wird alles übrige.
        let plain = |claims: Vec<(MemberId, vereinsakte_core::account::Claim)>| {
            claims
                .into_iter()
                .map(|(m, c)| (m, c.kind, c.amount, c.text, c.due_at))
                .collect::<Vec<_>>()
        };
        let all = store.scope(&Scope::all_members()).await.unwrap().record;
        assert_eq!(
            plain(all.annual_fee_claims(2027, now)),
            plain(full.annual_fee_claims(2027, now))
        );
        assert_eq!(
            plain(all.admission_fee_claims(2026, now)),
            plain(full.admission_fee_claims(2026, now))
        );
        for tour in full.reservations.iter().filter(|r| r.is_tour()) {
            let scope = store
                .scope(&Scope::for_reservation(tour))
                .await
                .unwrap()
                .record;
            assert_eq!(
                plain(scope.tour_fee_claims(tour, now)),
                plain(full.tour_fee_claims(tour, now))
            );
            assert_eq!(scope.tour_fee_overview(tour), full.tour_fee_overview(tour));
        }
    }

    #[tokio::test]
    async fn sqlite_and_memory_scopes_are_equal_and_small() {
        let store = filled().await;
        let memory = MemoryStore::new(example::club());
        let full = example::club();
        let reservation = full.reservations.first().unwrap();
        let wanted = Scope::for_reservation(reservation);
        let sqlite = store.scope(&wanted).await.unwrap().record;
        let mem = memory.scope(&wanted).await.unwrap().record;
        assert_eq!(sqlite, mem);
        assert!(sqlite.members.len() < full.members.len(), "nur Beteiligte");
        assert_eq!(sqlite.boats.len(), 1);
        assert!(sqlite.offices.is_empty());
    }
}
