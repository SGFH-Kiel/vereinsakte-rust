//! Schreibender Port: Änderungen an den Fachobjekten der Vereinsakte, jeweils
//! mit Versionsprüfung und zusammen mit ihren Protokolleinträgen in einer
//! Transaktion.

use async_trait::async_trait;
use vereinsakte_core::boat::Boat;
use vereinsakte_core::boat_permit::{BoatCheckout, Revocation};
use vereinsakte_core::fees::Season;
use vereinsakte_core::member::Member;
use vereinsakte_core::office::Office;
use vereinsakte_core::reservation::Reservation;
use vereinsakte_core::sailing_permit::AnnualRecord;
use vereinsakte_core::task::Task;
use vereinsakte_core::work_event::WorkEvent;
use vereinsakte_core::work_hours::{HourTransfer, WorkEntry};

use crate::audit::AuditEntry;
use crate::error::AppError;
use crate::ports::Version;

/// Art eines unabhängig speicherbaren Fachobjekts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RecordKind {
    Member,
    Boat,
    Office,
    Checkout,
    Revocation,
    WorkEvent,
    Task,
    WorkEntry,
    HourTransfer,
    Season,
    AnnualRecord,
    Reservation,
}

/// Ein vollständiges Fachobjekt samt seinen Kindern.
#[derive(Clone, Debug, PartialEq)]
pub enum Record {
    Member(Member),
    Boat(Boat),
    Office(Office),
    Checkout(BoatCheckout),
    Revocation(Revocation),
    WorkEvent(WorkEvent),
    Task(Task),
    WorkEntry(WorkEntry),
    HourTransfer(HourTransfer),
    Season(Season),
    AnnualRecord(AnnualRecord),
    Reservation(Reservation),
}

impl Record {
    pub fn kind(&self) -> RecordKind {
        match self {
            Self::Member(_) => RecordKind::Member,
            Self::Boat(_) => RecordKind::Boat,
            Self::Office(_) => RecordKind::Office,
            Self::Checkout(_) => RecordKind::Checkout,
            Self::Revocation(_) => RecordKind::Revocation,
            Self::WorkEvent(_) => RecordKind::WorkEvent,
            Self::Task(_) => RecordKind::Task,
            Self::WorkEntry(_) => RecordKind::WorkEntry,
            Self::HourTransfer(_) => RecordKind::HourTransfer,
            Self::Season(_) => RecordKind::Season,
            Self::AnnualRecord(_) => RecordKind::AnnualRecord,
            Self::Reservation(_) => RecordKind::Reservation,
        }
    }

    /// Schlüssel als Text (Kennung bzw. Mitglied und Jahr).
    pub fn key(&self) -> String {
        match self {
            Self::Member(x) => x.id.to_string(),
            Self::Boat(x) => x.id.to_string(),
            Self::Office(x) => x.id.to_string(),
            Self::Checkout(x) => x.id.to_string(),
            Self::Revocation(x) => x.id.to_string(),
            Self::WorkEvent(x) => x.id.to_string(),
            Self::Task(x) => x.id.to_string(),
            Self::WorkEntry(x) => x.id.to_string(),
            Self::HourTransfer(x) => x.id.to_string(),
            Self::Season(x) => x.year.to_string(),
            Self::AnnualRecord(x) => format!("{}:{}", x.member, x.year),
            Self::Reservation(x) => x.id.to_string(),
        }
    }
}

/// Eine Änderung innerhalb einer Transaktion.
#[derive(Clone, Debug, PartialEq)]
pub enum Change {
    /// Anlegen (`expected: None`) oder Ändern (`expected: Some(Version)`).
    Save {
        record: Record,
        expected: Option<Version>,
    },
    /// Löschen eines Fachobjekts mit Versionsprüfung.
    Delete {
        kind: RecordKind,
        key: String,
        expected: Version,
    },
}

impl Change {
    pub fn create(record: Record) -> Self {
        Self::Save {
            record,
            expected: None,
        }
    }

    pub fn update(record: Record, expected: Version) -> Self {
        Self::Save {
            record,
            expected: Some(expected),
        }
    }
}

#[async_trait]
pub trait ChangeRepository: Send + Sync {
    /// Wendet alle Änderungen und Protokolleinträge in einer Transaktion an.
    /// Stimmt eine erwartete Version nicht (oder existiert ein neues Objekt
    /// schon), schlägt alles mit [`AppError::Conflict`] fehl und nichts wird
    /// verändert.
    async fn commit(&self, changes: Vec<Change>, audit: Vec<AuditEntry>) -> Result<(), AppError>;
}
