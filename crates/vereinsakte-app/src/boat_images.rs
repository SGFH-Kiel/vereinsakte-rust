//! Bootsbild: ein Bild je Boot, getrennt von der Vereinsakte gespeichert
//! (Dateidaten werden nie in Listenabfragen geladen).

use std::collections::HashMap;

use async_trait::async_trait;
use vereinsakte_core::ids::BoatId;
use vereinsakte_core::rights::Capability;
use vereinsakte_core::time::Timestamp;

use crate::audit::AuditEntry;
use crate::commands::audit;
use crate::error::AppError;
use crate::service::{ClubService, CurrentUser};
use crate::tasks::detect_upload;

pub const MAX_BOAT_IMAGE_BYTES: usize = 8 * 1024 * 1024;
pub const BOAT_IMAGE_FORMAT_ERROR: &str = "Nur JPEG, PNG, WebP oder GIF sind erlaubt.";

/// Metadaten eines Bootsbildes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoatImageMeta {
    pub content_type: String,
    pub alt: Option<String>,
    pub size: usize,
    pub updated_at: Timestamp,
}

impl BoatImageMeta {
    /// Versionszahl für die Adresse (Cache-Busting).
    pub fn version(&self) -> i64 {
        self.updated_at.timestamp_millis()
    }
}

#[async_trait]
pub trait BoatImageRepository: Send + Sync {
    /// Metadaten aller Boote mit Bild, ohne Dateidaten.
    async fn boat_images(&self) -> Result<HashMap<BoatId, BoatImageMeta>, AppError>;
    async fn boat_image(&self, boat: BoatId) -> Result<Option<(BoatImageMeta, Vec<u8>)>, AppError>;
    async fn set_boat_image(
        &self,
        boat: BoatId,
        meta: &BoatImageMeta,
        data: Vec<u8>,
        audit: AuditEntry,
    ) -> Result<(), AppError>;
    async fn remove_boat_image(&self, boat: BoatId, audit: AuditEntry) -> Result<(), AppError>;
}

impl ClubService {
    async fn require_boat_edit(
        &self,
        user: &CurrentUser,
        boat: BoatId,
    ) -> Result<crate::principal::Principal, AppError> {
        let principal = self.principal(user).await?;
        let boats = self.store.boats().await?;
        if !boats.iter().any(|b| b.id == boat) {
            return Err(AppError::NotFound("Boot".into()));
        }
        principal.require(&Capability::EditBoat(boat))?;
        Ok(principal)
    }

    /// Setzt oder ersetzt das Bild (Bearbeitungsrecht für dieses Boot).
    pub async fn set_boat_image(
        &self,
        user: &CurrentUser,
        boat: BoatId,
        data: Vec<u8>,
        alt: &str,
    ) -> Result<(), AppError> {
        let principal = self.require_boat_edit(user, boat).await?;
        if data.is_empty() {
            return Err(AppError::rejected("file", "Bitte eine Datei auswählen."));
        }
        if data.len() > MAX_BOAT_IMAGE_BYTES {
            return Err(AppError::rejected(
                "file",
                format!(
                    "Das Bild ist zu groß (höchstens {} MB).",
                    MAX_BOAT_IMAGE_BYTES / (1024 * 1024)
                ),
            ));
        }
        let content_type = match detect_upload(&data) {
            Some((vereinsakte_core::task::AttachmentKind::Image, ct)) => ct,
            _ => return Err(AppError::rejected("file", BOAT_IMAGE_FORMAT_ERROR)),
        };
        let alt: String = alt.trim().chars().take(200).collect();
        let meta = BoatImageMeta {
            content_type: content_type.to_string(),
            alt: (!alt.is_empty()).then_some(alt),
            size: data.len(),
            updated_at: principal.now,
        };
        let log = audit(
            &principal,
            "boat.image_set",
            "boat",
            boat,
            None,
            "Bootsbild gesetzt",
        );
        self.store.set_boat_image(boat, &meta, data, log).await
    }

    pub async fn remove_boat_image(
        &self,
        user: &CurrentUser,
        boat: BoatId,
    ) -> Result<(), AppError> {
        let principal = self.require_boat_edit(user, boat).await?;
        let log = audit(
            &principal,
            "boat.image_removed",
            "boat",
            boat,
            None,
            "Bootsbild entfernt",
        );
        self.store.remove_boat_image(boat, log).await
    }

    /// Bild zur Auslieferung (nur aktive Mitglieder).
    pub async fn boat_image_file(
        &self,
        user: &CurrentUser,
        boat: BoatId,
    ) -> Result<(BoatImageMeta, Vec<u8>), AppError> {
        self.principal(user).await?;
        self.store
            .boat_image(boat)
            .await?
            .ok_or_else(|| AppError::NotFound("Bild".into()))
    }
}

#[cfg(test)]
mod tests {
    use vereinsakte_core::example::{BEN, FILOU, GRETA, HARALD_BLAUZAHN, TRIGON};

    use super::*;
    use crate::service::tests::service;

    fn user(member: vereinsakte_core::ids::MemberId) -> CurrentUser {
        CurrentUser { member }
    }

    fn png() -> Vec<u8> {
        let mut d = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        d.extend_from_slice(&[0; 16]);
        d
    }

    fn rejected(r: Result<(), AppError>) -> String {
        match r {
            Err(AppError::Rejected(e)) => e[0].message.clone(),
            other => panic!("erwartet abgelehnt: {other:?}"),
        }
    }

    #[tokio::test]
    async fn warden_sets_replaces_and_removes_only_for_own_boat() {
        let (service, store) = service();
        service
            .set_boat_image(&user(BEN), HARALD_BLAUZAHN, png(), "Harald")
            .await
            .unwrap();
        // Bootswart eines anderen Bootes und Mitglied ohne Amt: 403, nichts ändert sich.
        assert_eq!(
            service.set_boat_image(&user(BEN), FILOU, png(), "").await,
            Err(AppError::Forbidden)
        );
        assert_eq!(
            service
                .set_boat_image(&user(GRETA), HARALD_BLAUZAHN, png(), "")
                .await,
            Err(AppError::Forbidden)
        );
        assert_eq!(
            service
                .remove_boat_image(&user(GRETA), HARALD_BLAUZAHN)
                .await,
            Err(AppError::Forbidden)
        );
        // Ersetzen: weiterhin genau ein Bild.
        let mut second = png();
        second.push(1);
        service
            .set_boat_image(&user(BEN), HARALD_BLAUZAHN, second.clone(), "")
            .await
            .unwrap();
        let (meta, data) = service
            .boat_image_file(&user(GRETA), HARALD_BLAUZAHN)
            .await
            .unwrap();
        assert_eq!(data, second);
        assert_eq!(meta.content_type, "image/png");
        assert_eq!(store.boat_images().await.unwrap().len(), 1);
        let list = service.boat_list(&user(GRETA)).await.unwrap();
        let with: Vec<_> = list.rows.iter().filter(|r| r.image.is_some()).collect();
        assert_eq!(with.len(), 1);
        assert!(
            list.rows
                .iter()
                .find(|r| r.id == TRIGON.to_string())
                .unwrap()
                .image
                .is_none()
        );
        service
            .remove_boat_image(&user(BEN), HARALD_BLAUZAHN)
            .await
            .unwrap();
        assert!(store.boat_images().await.unwrap().is_empty());
        // Protokoll ohne Bilddaten
        let actions: Vec<_> = store
            .audit_entries()
            .iter()
            .map(|e| e.action.clone())
            .collect();
        assert_eq!(
            actions,
            ["boat.image_set", "boat.image_set", "boat.image_removed"]
        );
    }

    #[tokio::test]
    async fn bad_files_are_rejected_and_old_image_stays() {
        let (service, _) = service();
        service
            .set_boat_image(&user(BEN), HARALD_BLAUZAHN, png(), "")
            .await
            .unwrap();
        for data in [
            b"<svg xmlns='http://www.w3.org/2000/svg'/>".to_vec(),
            b"nur text".to_vec(),
            b"%PDF-1.4".to_vec(),
        ] {
            assert_eq!(
                rejected(
                    service
                        .set_boat_image(&user(BEN), HARALD_BLAUZAHN, data, "")
                        .await
                ),
                BOAT_IMAGE_FORMAT_ERROR
            );
        }
        let mut big = png();
        big.resize(MAX_BOAT_IMAGE_BYTES + 1, 0);
        assert!(
            rejected(
                service
                    .set_boat_image(&user(BEN), HARALD_BLAUZAHN, big, "")
                    .await
            )
            .contains("8 MB")
        );
        let (_, data) = service
            .boat_image_file(&user(BEN), HARALD_BLAUZAHN)
            .await
            .unwrap();
        assert_eq!(data, png());
    }
}
