//! Gemeinsame Bausteine der schreibenden Anwendungsfälle: Teildaten laden,
//! Änderungen als Unterschied zum geladenen Stand speichern, Protokolleinträge.

use vereinsakte_core::club_record::ClubRecord;
use vereinsakte_core::ids::MemberId;

use crate::audit::AuditEntry;
use crate::changes::Scoped;
use crate::error::AppError;
use crate::principal::Principal;
use crate::scope::Scope;
use crate::service::ClubService;

/// Protokolleintrag des handelnden Mitglieds zum aktuellen Zeitpunkt.
pub fn audit(
    by: &Principal,
    action: &str,
    object_kind: &str,
    object_id: impl ToString,
    subject: Option<MemberId>,
    summary: impl Into<String>,
) -> AuditEntry {
    AuditEntry {
        at: by.now,
        actor: by.member,
        action: action.to_string(),
        object_kind: object_kind.to_string(),
        object_id: object_id.to_string(),
        subject,
        summary: summary.into(),
    }
}

impl ClubService {
    /// Lädt die Teildaten samt Versionen.
    pub(crate) async fn load(&self, scope: &Scope) -> Result<Scoped, AppError> {
        self.store.scope(scope).await
    }

    /// Speichert den Unterschied zwischen geladenem Stand und `new` samt
    /// Protokolleinträgen in einer Transaktion.
    pub(crate) async fn save(
        &self,
        old: &Scoped,
        new: &ClubRecord,
        entries: Vec<AuditEntry>,
    ) -> Result<(), AppError> {
        let changes = old.diff(new);
        if changes.is_empty() && entries.is_empty() {
            return Ok(());
        }
        self.store.commit(changes, entries).await
    }
}
