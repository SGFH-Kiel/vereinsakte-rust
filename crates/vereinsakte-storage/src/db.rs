//! Verbindung zur SQLite-Datenbank und Migrationen.

use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use tower_sessions_sqlx_store::SqliteStore;
use vereinsakte_app::AppError;
use vereinsakte_core::issues::RecordIssue;

/// Fehler der Speicherschicht.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("Datenbankfehler: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Migration fehlgeschlagen: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
    #[error("die Daten wurden zwischenzeitlich geändert")]
    Conflict,
    #[error("ungültige Vereinsakte: {}", issues.iter().map(ToString::to_string).collect::<Vec<_>>().join("; "))]
    Invalid { issues: Vec<RecordIssue> },
    /// Verweis auf ein nicht vorhandenes Objekt (Fremdschlüssel verletzt).
    #[error("unbekannter Verweis in {table}.{column} auf {parent}")]
    UnknownReference {
        table: String,
        column: String,
        parent: String,
    },
    #[error("die Datenbank enthält bereits Daten; Import nur mit Ersetzen")]
    NotEmpty,
    /// Gespeicherte Daten lassen sich nicht in Fachobjekte umwandeln.
    #[error("ungültige gespeicherte Daten: {0}")]
    Corrupt(String),
}

impl From<StorageError> for AppError {
    fn from(error: StorageError) -> Self {
        match error {
            StorageError::Conflict => AppError::Conflict,
            StorageError::Invalid { issues } => AppError::Invalid(issues),
            other => AppError::Storage(other.to_string()),
        }
    }
}

/// Verbindungspool zur Datenbank.
#[derive(Clone, Debug)]
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    /// Öffnet (und legt bei Bedarf an) die Datenbankdatei.
    pub async fn connect(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .foreign_keys(true)
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(Duration::from_secs(5));
        let pool = SqlitePoolOptions::new()
            .max_connections(8)
            .connect_with(options)
            .await?;
        Ok(Self { pool })
    }

    /// Datenbank im Arbeitsspeicher (für Tests): genau eine dauerhafte Verbindung.
    pub async fn in_memory() -> Result<Self, StorageError> {
        let options = SqliteConnectOptions::from_str("sqlite::memory:")?.foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .min_connections(1)
            .idle_timeout(None)
            .max_lifetime(None)
            .connect_with(options)
            .await?;
        Ok(Self { pool })
    }

    /// Bringt das Schema (inkl. Sitzungstabelle) auf den aktuellen Stand.
    pub async fn migrate(&self) -> Result<(), StorageError> {
        sqlx::migrate!("./migrations").run(&self.pool).await?;
        self.session_store().migrate().await?;
        Ok(())
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Sitzungsspeicher für `tower-sessions` in derselben Datenbank.
    pub fn session_store(&self) -> SqliteStore {
        SqliteStore::new(self.pool.clone())
    }
}

#[cfg(test)]
mod tests {
    use sqlx::Row;

    use super::*;

    async fn tables(db: &Database) -> Vec<String> {
        sqlx::query("SELECT name FROM sqlite_schema WHERE type = 'table' AND name NOT LIKE 'sqlite_%' AND name NOT LIKE '_sqlx%'")
            .fetch_all(db.pool())
            .await
            .unwrap()
            .into_iter()
            .map(|row| row.get::<String, _>("name"))
            .collect()
    }

    #[tokio::test]
    async fn new_file_is_created_and_migrations_are_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("akte.db");
        assert!(!path.exists());
        let db = Database::connect(&path).await.unwrap();
        db.migrate().await.unwrap();
        assert!(path.exists());
        let before = tables(&db).await;
        assert!(before.contains(&"members".to_string()));
        db.migrate().await.unwrap();
        assert_eq!(tables(&db).await, before);
    }

    /// Migration 0004 baut `tasks` neu auf: Zuständige und Arbeitseinsatz-Verweise bleiben.
    #[tokio::test]
    async fn task_table_rebuild_keeps_references() {
        use std::borrow::Cow;
        let db = Database::in_memory().await.unwrap();
        let all = sqlx::migrate!("./migrations");
        let old = sqlx::migrate::Migrator {
            migrations: Cow::Owned(
                all.migrations
                    .iter()
                    .filter(|m| m.version < 4)
                    .cloned()
                    .collect(),
            ),
            ..sqlx::migrate::Migrator::DEFAULT
        };
        old.run(db.pool()).await.unwrap();
        for sql in [
            "INSERT INTO members (id, position, last_name, first_name, street, postal_code, city,
                 address_valid, membership_type, board_consent, joined_at)
             VALUES (1, 0, 'Test', 'Tina', 'Weg 1', '24103', 'Kiel', 1, 'regular', 0, '2026-01-01T00:00:00+01:00')",
            "INSERT INTO tasks (id, position, title, status) VALUES ('t1', 0, 'Fender', 'open')",
            "INSERT INTO task_assignees (task_id, position, member_id) VALUES ('t1', 0, 1)",
            "INSERT INTO work_entries (id, position, member_id, date_at, hours, task_id)
             VALUES ('w1', 0, 1, '2026-05-01T00:00:00+02:00', 2.0, 't1')",
        ] {
            sqlx::query(sql).execute(db.pool()).await.unwrap();
        }
        db.migrate().await.unwrap();
        let count = |sql: &'static str| {
            let pool = db.pool().clone();
            async move {
                sqlx::query(sql)
                    .fetch_one(&pool)
                    .await
                    .unwrap()
                    .get::<i64, _>(0)
            }
        };
        assert_eq!(
            count("SELECT COUNT(*) FROM task_assignees WHERE task_id = 't1'").await,
            1
        );
        assert_eq!(
            count("SELECT COUNT(*) FROM work_entries WHERE task_id = 't1'").await,
            1
        );
        sqlx::query("UPDATE tasks SET status = 'waiting' WHERE id = 't1'")
            .execute(db.pool())
            .await
            .unwrap();
        assert!(
            sqlx::query("UPDATE tasks SET status = 'bogus' WHERE id = 't1'")
                .execute(db.pool())
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn check_and_unique_constraints() {
        let db = Database::in_memory().await.unwrap();
        db.migrate().await.unwrap();
        let insert_member = |id: i64, membership_type: &'static str, login: &'static str| {
            sqlx::query(
                "INSERT INTO members (id, position, last_name, first_name, street, postal_code, city,
                     login_email, address_valid, membership_type, board_consent, joined_at)
                 VALUES (?, 0, 'Test', 'Tina', 'Weg 1', '24103', 'Kiel', ?, 1, ?, 0, '2026-01-01T00:00:00+01:00')",
            )
            .bind(id)
            .bind(login)
            .bind(membership_type)
        };
        assert!(
            insert_member(1, "vip", "a@sgfh.example")
                .execute(db.pool())
                .await
                .is_err()
        );
        insert_member(1, "regular", "anna.petersen@sgfh.example")
            .execute(db.pool())
            .await
            .unwrap();
        let duplicate = insert_member(2, "regular", "ANNA.PETERSEN@sgfh.example")
            .execute(db.pool())
            .await;
        assert!(duplicate.unwrap_err().to_string().contains("UNIQUE"));

        let both_forms = sqlx::query(
            "INSERT INTO work_entries (id, position, member_id, date_at, hours, start_at, end_at)
             VALUES ('x', 0, 1, '2026-04-12T00:00:00+02:00', 3.5, '2026-04-12T09:00:00+02:00', '2026-04-12T12:00:00+02:00')",
        )
        .execute(db.pool())
        .await;
        assert!(both_forms.unwrap_err().to_string().contains("CHECK"));
    }

    #[tokio::test]
    async fn indices_and_versions_exist() {
        let db = Database::in_memory().await.unwrap();
        db.migrate().await.unwrap();
        let indices: Vec<String> =
            sqlx::query("SELECT name FROM sqlite_schema WHERE type = 'index'")
                .fetch_all(db.pool())
                .await
                .unwrap()
                .iter()
                .map(|r| r.get("name"))
                .collect();
        for index in ["work_entries_member", "claims_member", "offices_member"] {
            assert!(indices.contains(&index.to_string()), "{index} fehlt");
        }
        for table in [
            "members",
            "boats",
            "offices",
            "work_entries",
            "reservations",
        ] {
            let has_version =
                sqlx::query("SELECT 1 FROM pragma_table_info(?) WHERE name = 'version'")
                    .bind(table)
                    .fetch_optional(db.pool())
                    .await
                    .unwrap()
                    .is_some();
            assert!(has_version, "{table}.version fehlt");
        }
        let meta = sqlx::query("SELECT 1 FROM sqlite_schema WHERE name = 'record_meta'")
            .fetch_optional(db.pool())
            .await
            .unwrap();
        assert!(meta.is_none(), "keine globale Version");
    }

    #[tokio::test]
    async fn schema_has_no_json_columns() {
        let db = Database::in_memory().await.unwrap();
        db.migrate().await.unwrap();
        for table in tables(&db).await {
            // Die Sitzungstabelle von tower-sessions speichert Sitzungsdaten als BLOB;
            // sie enthält keine Fachdaten.
            if table == "tower_sessions" {
                continue;
            }
            let columns = sqlx::query("SELECT name, type FROM pragma_table_info(?)")
                .bind(&table)
                .fetch_all(db.pool())
                .await
                .unwrap();
            assert!(!columns.is_empty(), "{table}");
            for column in columns {
                let (name, kind): (String, String) = (column.get("name"), column.get("type"));
                // Hochgeladene Dateien (Bilder, PDFs) liegen als BLOB, nicht als JSON.
                if table == "task_attachments" && name == "data" {
                    assert_eq!(kind, "BLOB");
                    continue;
                }
                assert_ne!(name, "data", "{table}");
                assert!(
                    ["INTEGER", "TEXT", "REAL"].contains(&kind.as_str()),
                    "{table}.{name}: {kind}"
                );
            }
        }
    }
}
