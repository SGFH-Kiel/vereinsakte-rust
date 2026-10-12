//! Kommentare und Anhänge zu Aufgaben. Sie liegen außerhalb der Vereinsakte
//! (`ClubRecord`), weil Dateidaten nie in Listen- oder Teildatenabfragen
//! geladen werden sollen; jeder Schreibvorgang trägt seinen Protokolleintrag
//! und wird mit ihm in einer Transaktion gespeichert.

use std::collections::HashMap;

use async_trait::async_trait;
use uuid::Uuid;
use vereinsakte_core::ids::TaskId;
use vereinsakte_core::task::{TaskAttachment, TaskComment};

use crate::audit::AuditEntry;
use crate::error::AppError;

/// Zähler je Aufgabe für die Karten des Boards.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TaskCounts {
    pub comments: u32,
    pub attachments: u32,
    pub images: u32,
}

#[async_trait]
pub trait TaskExtrasRepository: Send + Sync {
    /// Zähler aller Aufgaben mit mindestens einem Kommentar oder Anhang.
    async fn counts(&self) -> Result<HashMap<TaskId, TaskCounts>, AppError>;
    /// Kommentare einer Aufgabe in der Reihenfolge des Eingangs.
    async fn comments(&self, task: TaskId) -> Result<Vec<TaskComment>, AppError>;
    async fn comment(&self, id: Uuid) -> Result<Option<TaskComment>, AppError>;
    async fn add_comment(&self, comment: &TaskComment, audit: AuditEntry) -> Result<(), AppError>;
    async fn edit_comment(&self, id: Uuid, text: &str, audit: AuditEntry) -> Result<(), AppError>;
    async fn delete_comment(&self, id: Uuid, audit: AuditEntry) -> Result<(), AppError>;
    /// Metadaten der Anhänge, ohne Dateidaten.
    async fn attachments(&self, task: TaskId) -> Result<Vec<TaskAttachment>, AppError>;
    async fn attachment(&self, id: Uuid) -> Result<Option<(TaskAttachment, Vec<u8>)>, AppError>;
    async fn attachment_meta(&self, id: Uuid) -> Result<Option<TaskAttachment>, AppError>;
    async fn add_attachment(
        &self,
        attachment: &TaskAttachment,
        data: Vec<u8>,
        audit: AuditEntry,
    ) -> Result<(), AppError>;
    async fn delete_attachment(&self, id: Uuid, audit: AuditEntry) -> Result<(), AppError>;
}
