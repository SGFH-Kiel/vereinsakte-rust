//! Kommentare und Anhänge zu Aufgaben (Migration 0004).

use std::collections::HashMap;

use async_trait::async_trait;
use sqlx::{Row, sqlite::SqliteRow};
use uuid::Uuid;
use vereinsakte_app::AppError;
use vereinsakte_app::audit::AuditEntry;
use vereinsakte_app::task_extras::{TaskCounts, TaskExtrasRepository};
use vereinsakte_core::ids::TaskId;
use vereinsakte_core::task::{AttachmentKind, TaskAttachment, TaskComment};

use crate::commit::insert_audit;
use crate::db::StorageError;
use crate::repo::SqliteClubStore;
use crate::values::{get, get_member, get_ts};

fn uuid(row: &SqliteRow, column: &str) -> Result<Uuid, StorageError> {
    let text: String = get(row, column)?;
    Uuid::parse_str(&text).map_err(|e| StorageError::Corrupt(format!("{column}: {e}")))
}

fn comment_from(row: &SqliteRow) -> Result<TaskComment, StorageError> {
    Ok(TaskComment {
        id: uuid(row, "id")?,
        task: TaskId::from_uuid(uuid(row, "task_id")?),
        author: get_member(row, "author_id")?,
        at: get_ts(row, "at")?,
        text: get(row, "text")?,
    })
}

fn attachment_from(row: &SqliteRow) -> Result<TaskAttachment, StorageError> {
    let kind: String = get(row, "kind")?;
    let size: i64 = get(row, "size")?;
    Ok(TaskAttachment {
        id: uuid(row, "id")?,
        task: TaskId::from_uuid(uuid(row, "task_id")?),
        uploader: get_member(row, "uploader_id")?,
        at: get_ts(row, "at")?,
        kind: AttachmentKind::parse(&kind)
            .ok_or_else(|| StorageError::Corrupt(format!("kind: {kind}")))?,
        filename: get(row, "filename")?,
        content_type: get(row, "content_type")?,
        alt: get(row, "alt")?,
        size: usize::try_from(size).map_err(|e| StorageError::Corrupt(format!("size: {e}")))?,
    })
}

const ATTACHMENT_COLUMNS: &str =
    "id, task_id, uploader_id, at, kind, filename, content_type, alt, size";

fn db(error: sqlx::Error) -> AppError {
    StorageError::from(error).into()
}

fn storage(error: StorageError) -> AppError {
    error.into()
}

#[async_trait]
impl TaskExtrasRepository for SqliteClubStore {
    async fn counts(&self) -> Result<HashMap<TaskId, TaskCounts>, AppError> {
        let pool = self.db().pool();
        let mut map: HashMap<TaskId, TaskCounts> = HashMap::new();
        for row in sqlx::query("SELECT task_id, COUNT(*) AS n FROM task_comments GROUP BY task_id")
            .fetch_all(pool)
            .await
            .map_err(db)?
        {
            let id = TaskId::from_uuid(uuid(&row, "task_id").map_err(storage)?);
            map.entry(id).or_default().comments =
                u32::try_from(row.get::<i64, _>("n")).unwrap_or(0);
        }
        for row in sqlx::query(
            "SELECT task_id, COUNT(*) AS n, SUM(kind = 'image') AS images
             FROM task_attachments GROUP BY task_id",
        )
        .fetch_all(pool)
        .await
        .map_err(db)?
        {
            let id = TaskId::from_uuid(uuid(&row, "task_id").map_err(storage)?);
            let entry = map.entry(id).or_default();
            entry.attachments = u32::try_from(row.get::<i64, _>("n")).unwrap_or(0);
            entry.images = u32::try_from(row.get::<i64, _>("images")).unwrap_or(0);
        }
        Ok(map)
    }

    async fn comments(&self, task: TaskId) -> Result<Vec<TaskComment>, AppError> {
        let rows = sqlx::query(
            "SELECT id, task_id, author_id, at, text FROM task_comments
             WHERE task_id = ? ORDER BY julianday(at), rowid",
        )
        .bind(task.to_string())
        .fetch_all(self.db().pool())
        .await
        .map_err(db)?;
        rows.iter()
            .map(|r| comment_from(r).map_err(storage))
            .collect()
    }

    async fn comment(&self, id: Uuid) -> Result<Option<TaskComment>, AppError> {
        let row =
            sqlx::query("SELECT id, task_id, author_id, at, text FROM task_comments WHERE id = ?")
                .bind(id.to_string())
                .fetch_optional(self.db().pool())
                .await
                .map_err(db)?;
        row.map(|r| comment_from(&r).map_err(storage)).transpose()
    }

    async fn add_comment(&self, c: &TaskComment, audit: AuditEntry) -> Result<(), AppError> {
        let mut tx = self.db().pool().begin().await.map_err(db)?;
        sqlx::query(
            "INSERT INTO task_comments (id, task_id, author_id, at, text) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(c.id.to_string())
        .bind(c.task.to_string())
        .bind(i64::from(c.author.get()))
        .bind(c.at.to_rfc3339())
        .bind(&c.text)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        insert_audit(&mut tx, &audit).await.map_err(storage)?;
        tx.commit().await.map_err(db)
    }

    async fn edit_comment(&self, id: Uuid, text: &str, audit: AuditEntry) -> Result<(), AppError> {
        let mut tx = self.db().pool().begin().await.map_err(db)?;
        let done = sqlx::query("UPDATE task_comments SET text = ? WHERE id = ?")
            .bind(text)
            .bind(id.to_string())
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        if done.rows_affected() == 0 {
            return Err(AppError::NotFound("Kommentar".into()));
        }
        insert_audit(&mut tx, &audit).await.map_err(storage)?;
        tx.commit().await.map_err(db)
    }

    async fn delete_comment(&self, id: Uuid, audit: AuditEntry) -> Result<(), AppError> {
        let mut tx = self.db().pool().begin().await.map_err(db)?;
        let done = sqlx::query("DELETE FROM task_comments WHERE id = ?")
            .bind(id.to_string())
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        if done.rows_affected() == 0 {
            return Err(AppError::NotFound("Kommentar".into()));
        }
        insert_audit(&mut tx, &audit).await.map_err(storage)?;
        tx.commit().await.map_err(db)
    }

    async fn attachments(&self, task: TaskId) -> Result<Vec<TaskAttachment>, AppError> {
        let sql = format!(
            "SELECT {ATTACHMENT_COLUMNS} FROM task_attachments WHERE task_id = ? ORDER BY julianday(at), rowid"
        );
        let rows = sqlx::query(&sql)
            .bind(task.to_string())
            .fetch_all(self.db().pool())
            .await
            .map_err(db)?;
        rows.iter()
            .map(|r| attachment_from(r).map_err(storage))
            .collect()
    }

    async fn attachment(&self, id: Uuid) -> Result<Option<(TaskAttachment, Vec<u8>)>, AppError> {
        let sql = format!("SELECT {ATTACHMENT_COLUMNS}, data FROM task_attachments WHERE id = ?");
        let row = sqlx::query(&sql)
            .bind(id.to_string())
            .fetch_optional(self.db().pool())
            .await
            .map_err(db)?;
        row.map(|r| {
            let data: Vec<u8> = get(&r, "data").map_err(storage)?;
            Ok((attachment_from(&r).map_err(storage)?, data))
        })
        .transpose()
    }

    async fn attachment_meta(&self, id: Uuid) -> Result<Option<TaskAttachment>, AppError> {
        let sql = format!("SELECT {ATTACHMENT_COLUMNS} FROM task_attachments WHERE id = ?");
        let row = sqlx::query(&sql)
            .bind(id.to_string())
            .fetch_optional(self.db().pool())
            .await
            .map_err(db)?;
        row.map(|r| attachment_from(&r).map_err(storage))
            .transpose()
    }

    async fn add_attachment(
        &self,
        a: &TaskAttachment,
        data: Vec<u8>,
        audit: AuditEntry,
    ) -> Result<(), AppError> {
        let mut tx = self.db().pool().begin().await.map_err(db)?;
        sqlx::query(
            "INSERT INTO task_attachments (id, task_id, uploader_id, at, kind, filename, content_type, alt, size, data)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(a.id.to_string())
        .bind(a.task.to_string())
        .bind(i64::from(a.uploader.get()))
        .bind(a.at.to_rfc3339())
        .bind(a.kind.as_str())
        .bind(&a.filename)
        .bind(&a.content_type)
        .bind(&a.alt)
        .bind(i64::try_from(a.size).unwrap_or(i64::MAX))
        .bind(data)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        insert_audit(&mut tx, &audit).await.map_err(storage)?;
        tx.commit().await.map_err(db)
    }

    async fn delete_attachment(&self, id: Uuid, audit: AuditEntry) -> Result<(), AppError> {
        let mut tx = self.db().pool().begin().await.map_err(db)?;
        let done = sqlx::query("DELETE FROM task_attachments WHERE id = ?")
            .bind(id.to_string())
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        if done.rows_affected() == 0 {
            return Err(AppError::NotFound("Anhang".into()));
        }
        insert_audit(&mut tx, &audit).await.map_err(storage)?;
        tx.commit().await.map_err(db)
    }
}

#[cfg(test)]
mod tests {
    use vereinsakte_app::audit::{AuditFilter, AuditRepository};
    use vereinsakte_app::changes::{Change, ChangeRepository, Record, RecordKind};
    use vereinsakte_app::ports::Version;
    use vereinsakte_core::example::{self, BEN, FOCK_NAEHEN, GRETA, SEGELTUCH, WEBSEITE, at};

    use super::*;
    use crate::db::Database;

    async fn filled() -> SqliteClubStore {
        let db = Database::in_memory().await.unwrap();
        db.migrate().await.unwrap();
        let store = SqliteClubStore::new(db);
        store.import(&example::club(), false).await.unwrap();
        store
    }

    fn entry(action: &str) -> AuditEntry {
        AuditEntry {
            at: at(2026, 10, 9, 12, 0),
            actor: BEN,
            action: action.into(),
            object_kind: "task".into(),
            object_id: WEBSEITE.to_string(),
            subject: None,
            summary: "x".into(),
        }
    }

    fn attachment(kind: AttachmentKind, data_len: usize) -> TaskAttachment {
        TaskAttachment {
            id: Uuid::new_v4(),
            task: WEBSEITE,
            uploader: BEN,
            at: at(2026, 10, 9, 12, 0),
            kind,
            filename: "Angebot.pdf".into(),
            content_type: "application/pdf".into(),
            alt: Some("Angebot".into()),
            size: data_len,
        }
    }

    #[tokio::test]
    async fn comments_and_attachments_round_trip() {
        let store = filled().await;
        let comment = TaskComment {
            id: Uuid::new_v4(),
            task: WEBSEITE,
            author: GRETA,
            at: at(2026, 10, 9, 12, 0),
            text: "Segeltuch ist bestellt".into(),
        };
        store
            .add_comment(&comment, entry("task.comment_added"))
            .await
            .unwrap();
        assert_eq!(
            store.comments(WEBSEITE).await.unwrap(),
            vec![comment.clone()]
        );
        store
            .edit_comment(comment.id, "neu", entry("task.comment_edited"))
            .await
            .unwrap();
        assert_eq!(
            store.comment(comment.id).await.unwrap().unwrap().text,
            "neu"
        );

        let pdf = attachment(AttachmentKind::Pdf, 5);
        let img = attachment(AttachmentKind::Image, 3);
        store
            .add_attachment(&pdf, b"%PDF-".to_vec(), entry("task.attachment_added"))
            .await
            .unwrap();
        store
            .add_attachment(&img, vec![1, 2, 3], entry("task.attachment_added"))
            .await
            .unwrap();
        // Listen tragen keine Dateidaten, die Auslieferung schon.
        assert_eq!(
            store.attachments(WEBSEITE).await.unwrap(),
            vec![pdf.clone(), img.clone()]
        );
        assert_eq!(
            store.attachment(pdf.id).await.unwrap().unwrap().1,
            b"%PDF-".to_vec()
        );
        let counts = store.counts().await.unwrap();
        assert_eq!(
            counts[&WEBSEITE],
            TaskCounts {
                comments: 1,
                attachments: 2,
                images: 1
            }
        );
        assert!(!counts.contains_key(&FOCK_NAEHEN));
        // Jeder Schreibvorgang trägt genau einen Protokolleintrag.
        let log = store
            .audit(&AuditFilter {
                object_id: Some(WEBSEITE.to_string()),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(log.len(), 4);

        store
            .delete_attachment(pdf.id, entry("task.attachment_removed"))
            .await
            .unwrap();
        store
            .delete_comment(comment.id, entry("task.comment_deleted"))
            .await
            .unwrap();
        assert!(store.attachment(pdf.id).await.unwrap().is_none());
        assert_eq!(
            store
                .delete_comment(comment.id, entry("task.comment_deleted"))
                .await,
            Err(AppError::NotFound("Kommentar".into()))
        );
    }

    #[tokio::test]
    async fn files_are_removed_with_the_task() {
        let store = filled().await;
        let mut img = attachment(AttachmentKind::Image, 3);
        img.task = SEGELTUCH;
        store
            .add_attachment(&img, vec![1, 2, 3], entry("task.attachment_added"))
            .await
            .unwrap();
        store
            .commit(
                vec![Change::Delete {
                    kind: RecordKind::Task,
                    key: SEGELTUCH.to_string(),
                    expected: Version(1),
                }],
                vec![],
            )
            .await
            .unwrap();
        assert!(store.attachment(img.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn saving_a_task_keeps_its_comments_and_files() {
        let store = filled().await;
        let img = attachment(AttachmentKind::Image, 3);
        store
            .add_attachment(&img, vec![1, 2, 3], entry("task.attachment_added"))
            .await
            .unwrap();
        let mut task = example::tasks()
            .into_iter()
            .find(|t| t.id == WEBSEITE)
            .unwrap();
        task.status = vereinsakte_core::task::TaskStatus::Waiting;
        store
            .commit(vec![Change::update(Record::Task(task), Version(1))], vec![])
            .await
            .unwrap();
        assert_eq!(store.attachments(WEBSEITE).await.unwrap().len(), 1);
    }
}
