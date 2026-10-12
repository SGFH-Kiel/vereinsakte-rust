//! Bootsbilder (Migration 0005).

use std::collections::HashMap;

use async_trait::async_trait;
use sqlx::Row;
use vereinsakte_app::AppError;
use vereinsakte_app::audit::AuditEntry;
use vereinsakte_app::boat_images::{BoatImageMeta, BoatImageRepository};
use vereinsakte_core::ids::BoatId;

use crate::commit::insert_audit;
use crate::db::StorageError;
use crate::repo::SqliteClubStore;
use crate::values::{get, get_ts};

fn db(error: sqlx::Error) -> AppError {
    StorageError::from(error).into()
}

fn meta_from(row: &sqlx::sqlite::SqliteRow) -> Result<BoatImageMeta, StorageError> {
    let size: i64 = get(row, "size")?;
    Ok(BoatImageMeta {
        content_type: get(row, "content_type")?,
        alt: get(row, "alt")?,
        size: usize::try_from(size).map_err(|e| StorageError::Corrupt(format!("size: {e}")))?,
        updated_at: get_ts(row, "updated_at")?,
    })
}

#[async_trait]
impl BoatImageRepository for SqliteClubStore {
    async fn boat_images(&self) -> Result<HashMap<BoatId, BoatImageMeta>, AppError> {
        let rows =
            sqlx::query("SELECT boat_id, content_type, alt, size, updated_at FROM boat_images")
                .fetch_all(self.db().pool())
                .await
                .map_err(db)?;
        let mut map = HashMap::new();
        for row in &rows {
            let id: String = row.get("boat_id");
            let id: BoatId = id
                .parse()
                .map_err(|_| AppError::Storage("ungültige Boots-Kennung".into()))?;
            map.insert(id, meta_from(row).map_err(AppError::from)?);
        }
        Ok(map)
    }

    async fn boat_image(&self, boat: BoatId) -> Result<Option<(BoatImageMeta, Vec<u8>)>, AppError> {
        let row = sqlx::query(
            "SELECT content_type, alt, size, updated_at, data FROM boat_images WHERE boat_id = ?",
        )
        .bind(boat.to_string())
        .fetch_optional(self.db().pool())
        .await
        .map_err(db)?;
        row.map(|r| {
            let data: Vec<u8> = get(&r, "data").map_err(AppError::from)?;
            Ok((meta_from(&r).map_err(AppError::from)?, data))
        })
        .transpose()
    }

    async fn set_boat_image(
        &self,
        boat: BoatId,
        meta: &BoatImageMeta,
        data: Vec<u8>,
        audit: AuditEntry,
    ) -> Result<(), AppError> {
        let mut tx = self.db().pool().begin().await.map_err(db)?;
        sqlx::query(
            "INSERT INTO boat_images (boat_id, content_type, alt, size, updated_at, data)
             VALUES (?, ?, ?, ?, ?, ?)
             ON CONFLICT (boat_id) DO UPDATE SET content_type = excluded.content_type,
                alt = excluded.alt, size = excluded.size, updated_at = excluded.updated_at,
                data = excluded.data",
        )
        .bind(boat.to_string())
        .bind(&meta.content_type)
        .bind(&meta.alt)
        .bind(i64::try_from(meta.size).unwrap_or(i64::MAX))
        .bind(meta.updated_at.to_rfc3339())
        .bind(data)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        insert_audit(&mut tx, &audit)
            .await
            .map_err(AppError::from)?;
        tx.commit().await.map_err(db)
    }

    async fn remove_boat_image(&self, boat: BoatId, audit: AuditEntry) -> Result<(), AppError> {
        let mut tx = self.db().pool().begin().await.map_err(db)?;
        let done = sqlx::query("DELETE FROM boat_images WHERE boat_id = ?")
            .bind(boat.to_string())
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        if done.rows_affected() == 0 {
            return Err(AppError::NotFound("Bild".into()));
        }
        insert_audit(&mut tx, &audit)
            .await
            .map_err(AppError::from)?;
        tx.commit().await.map_err(db)
    }
}

#[cfg(test)]
mod tests {
    use vereinsakte_core::example::{self, BEN, FILOU, TRIGON, at};

    use super::*;
    use crate::db::Database;

    fn entry() -> AuditEntry {
        AuditEntry {
            at: at(2026, 10, 9, 12, 0),
            actor: BEN,
            action: "boat.image_set".into(),
            object_kind: "boat".into(),
            object_id: FILOU.to_string(),
            subject: None,
            summary: "x".into(),
        }
    }

    fn meta(size: usize) -> BoatImageMeta {
        BoatImageMeta {
            content_type: "image/png".into(),
            alt: Some("Filou".into()),
            size,
            updated_at: at(2026, 10, 9, 12, 0),
        }
    }

    #[tokio::test]
    async fn upsert_replaces_and_lists_without_data() {
        let db = Database::in_memory().await.unwrap();
        db.migrate().await.unwrap();
        let store = SqliteClubStore::new(db);
        store.import(&example::club(), false).await.unwrap();
        store
            .set_boat_image(FILOU, &meta(2), vec![1, 2], entry())
            .await
            .unwrap();
        store
            .set_boat_image(FILOU, &meta(3), vec![1, 2, 3], entry())
            .await
            .unwrap();
        let all = store.boat_images().await.unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[&FILOU].size, 3);
        assert!(!all.contains_key(&TRIGON));
        assert_eq!(
            store.boat_image(FILOU).await.unwrap().unwrap().1,
            vec![1, 2, 3]
        );
        store.remove_boat_image(FILOU, entry()).await.unwrap();
        assert!(store.boat_image(FILOU).await.unwrap().is_none());
        assert_eq!(
            store.remove_boat_image(FILOU, entry()).await,
            Err(AppError::NotFound("Bild".into()))
        );
    }
}
