//! Änderungsprotokoll: unveränderliche Einträge über jede schreibende Aktion.

use async_trait::async_trait;
use vereinsakte_core::ids::MemberId;
use vereinsakte_core::time::Timestamp;

use crate::error::AppError;

/// Ein Eintrag im Änderungsprotokoll. Enthält nur Kennungen und Feldnamen,
/// keine Werte personenbezogener Felder und keine Anmeldedaten.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditEntry {
    pub at: Timestamp,
    /// Handelndes Mitglied.
    pub actor: MemberId,
    /// Aktionsart, z. B. `boat.block`.
    pub action: String,
    /// Art des betroffenen Objekts, z. B. `boat`.
    pub object_kind: String,
    /// Kennung des betroffenen Objekts.
    pub object_id: String,
    /// Betroffenes Mitglied, falls vorhanden.
    pub subject: Option<MemberId>,
    /// Kurze deutsche Beschreibung.
    pub summary: String,
}

/// Gespeicherter Eintrag mit laufender Nummer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditRecord {
    pub id: i64,
    pub entry: AuditEntry,
}

/// Filter und Seitenwahl für die Protokollansicht (neueste zuerst).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AuditFilter {
    pub from: Option<Timestamp>,
    /// Exklusive Obergrenze.
    pub to: Option<Timestamp>,
    pub actor: Option<MemberId>,
    pub action: Option<String>,
    /// Nur Einträge, in denen das Mitglied handelt oder betroffen ist.
    pub involving: Option<MemberId>,
    pub limit: Option<u32>,
    pub offset: u32,
}

#[async_trait]
pub trait AuditRepository: Send + Sync {
    async fn audit(&self, filter: &AuditFilter) -> Result<Vec<AuditRecord>, AppError>;
}
