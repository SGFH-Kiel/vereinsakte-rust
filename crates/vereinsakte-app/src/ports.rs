//! Ports: anwendungsfallbezogene Schnittstellen der App-Schicht zur
//! Datenbank. Jede Methode liefert genau die Daten, die ein Anwendungsfall
//! braucht – die ganze Vereinsakte wird nie geladen (außer für Sicherungen in
//! der Speicherschicht). Tests nutzen [`crate::memory::MemoryStore`].

use async_trait::async_trait;
use chrono::Utc;
use vereinsakte_core::boat::Boat;
use vereinsakte_core::boat_permit::{BoatCheckout, PermitHistory, Revocation};
use vereinsakte_core::fees::Season;
use vereinsakte_core::ids::MemberId;
use vereinsakte_core::member::{Member, MembershipType, PhoneNumbers};
use vereinsakte_core::office::Office;
use vereinsakte_core::sailing_permit::{AnnualRecord, MemberYear};
use vereinsakte_core::time::{Timestamp, club_offset};
use vereinsakte_core::work_hours::{HourTransfer, WorkEntry};

use crate::error::AppError;

/// Versionsstand eines Datensatzes (optimistische Sperre je Datensatz).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version(pub i64);

impl Version {
    pub fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

/// Ein Datensatz mit seinem Versionsstand.
#[derive(Clone, Debug, PartialEq)]
pub struct Versioned<T> {
    pub value: T,
    pub version: Version,
}

/// Name und Kontaktangaben eines Mitglieds (für Listen; ohne Scheine und Konto).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemberSummary {
    pub id: MemberId,
    pub first_name: String,
    pub last_name: String,
    pub membership_type: MembershipType,
    pub email: Option<String>,
    pub phone: PhoneNumbers,
    /// Sperrvermerk für E-Mail und Telefonnummern.
    pub contact_blocked: bool,
    /// Segelscheine als Text, z. B. „SKS“.
    pub sailing_licenses: Vec<String>,
    /// Funklizenzen als Text, z. B. „SRC“.
    pub radio_licenses: Vec<String>,
}

impl MemberSummary {
    pub fn full_name(&self) -> String {
        format!("{} {}", self.first_name, self.last_name)
    }
}

impl From<&Member> for MemberSummary {
    fn from(m: &Member) -> Self {
        Self {
            id: m.id,
            first_name: m.first_name.clone(),
            last_name: m.last_name.clone(),
            membership_type: m.membership_type,
            email: m.email.clone(),
            phone: m.phone.clone(),
            contact_blocked: m.contact_blocked,
            sailing_licenses: m
                .sailing_licenses
                .iter()
                .map(|l| l.kind.to_string())
                .collect(),
            radio_licenses: m
                .radio_licenses
                .iter()
                .map(|l| l.kind.to_string())
                .collect(),
        }
    }
}

#[async_trait]
pub trait MemberRepository: Send + Sync {
    /// Mitglied mit Scheinen, Lizenzen und Konto.
    async fn member(&self, id: MemberId) -> Result<Option<Versioned<Member>>, AppError>;

    /// Mitglied mit diesem Anmeldekonto (ohne Beachtung der Groß-/Kleinschreibung).
    async fn member_by_login_email(&self, login: &str) -> Result<Option<Member>, AppError>;

    /// Zum Zeitpunkt aktive Mitglieder, sortiert nach Nach- und Vorname.
    async fn active_members(&self, t: Timestamp) -> Result<Vec<MemberSummary>, AppError>;

    /// Speichert ein Mitglied samt Scheinen und Konto, wenn sein gespeicherter
    /// Stand `expected` ist; sonst [`AppError::Conflict`].
    async fn save_member(&self, member: &Member, expected: Version) -> Result<Version, AppError>;

    /// Nächste freie Mitgliedsnummer (größte vergebene plus eins).
    async fn next_member_number(&self) -> Result<u32, AppError>;
}

#[async_trait]
pub trait OfficeRepository: Send + Sync {
    /// Zum Zeitpunkt aktive Ämter.
    async fn active_offices(&self, t: Timestamp) -> Result<Vec<Office>, AppError>;
}

#[async_trait]
pub trait BoatRepository: Send + Sync {
    async fn boats(&self) -> Result<Vec<Boat>, AppError>;
}

/// Jahresdaten eines Mitglieds aus der Datenbank.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MemberYearData {
    pub season: Option<Season>,
    pub annual_record: Option<AnnualRecord>,
    /// Einsätze des Mitglieds in diesem Jahr.
    pub work_entries: Vec<WorkEntry>,
    /// Überträge dieses Jahres, an denen das Mitglied beteiligt ist.
    pub hour_transfers: Vec<HourTransfer>,
}

impl MemberYearData {
    pub fn view<'a>(&'a self, member: &'a Member, year: i32) -> MemberYear<'a> {
        MemberYear {
            member,
            year,
            season: self.season.as_ref(),
            annual_record: self.annual_record.as_ref(),
            work_entries: self.work_entries.iter().collect(),
            hour_transfers: self.hour_transfers.iter().collect(),
        }
    }
}

#[async_trait]
pub trait YearRepository: Send + Sync {
    async fn member_year(&self, member: MemberId, year: i32) -> Result<MemberYearData, AppError>;
}

/// Abnahmen und Widerrufe eines Mitglieds aus der Datenbank.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PermitData {
    pub checkouts: Vec<BoatCheckout>,
    pub revocations: Vec<Revocation>,
}

impl PermitData {
    pub fn view<'a>(&'a self, member: &Member) -> PermitHistory<'a> {
        PermitHistory {
            member: member.id,
            address_valid: member.address_valid,
            checkouts: self.checkouts.iter().collect(),
            revocations: self.revocations.iter().collect(),
        }
    }
}

#[async_trait]
pub trait PermitRepository: Send + Sync {
    async fn permit_history(&self, member: MemberId) -> Result<PermitData, AppError>;
}

#[async_trait]
pub trait Health: Send + Sync {
    /// Ist die Datenbank erreichbar?
    async fn ping(&self) -> Result<(), AppError>;
}

/// Alle Ports der Vereinsakte zusammen (eine Datenbank).
pub trait ClubStore:
    MemberRepository
    + OfficeRepository
    + BoatRepository
    + YearRepository
    + PermitRepository
    + Health
    + crate::changes::ChangeRepository
    + crate::audit::AuditRepository
    + crate::scope::ScopeRepository
    + crate::task_extras::TaskExtrasRepository
{
}

impl<T> ClubStore for T where
    T: MemberRepository
        + OfficeRepository
        + BoatRepository
        + YearRepository
        + PermitRepository
        + Health
        + crate::changes::ChangeRepository
        + crate::audit::AuditRepository
        + crate::scope::ScopeRepository
        + crate::task_extras::TaskExtrasRepository
{
}

/// Verknüpfung externer Anmeldeidentitäten mit Mitgliedern.
#[async_trait]
pub trait IdentityRepository: Send + Sync {
    async fn find(&self, provider: &str, subject: &str) -> Result<Option<MemberId>, AppError>;

    /// Verknüpft eine Identität mit einem Mitglied; [`AppError::Conflict`], wenn
    /// die Identität oder das Mitglied bereits verknüpft ist.
    async fn link(&self, provider: &str, subject: &str, member: MemberId) -> Result<(), AppError>;
}

/// Liefert die aktuelle Zeit in Vereinszeit.
pub trait Clock: Send + Sync {
    fn now(&self) -> Timestamp;
}

/// Systemuhr in Vereinszeit (MEZ/MESZ).
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Timestamp {
        let utc = Utc::now();
        utc.with_timezone(&club_offset(utc.date_naive()))
    }
}

/// Feste Uhr für Tests und Vorführungen.
#[derive(Clone, Copy, Debug)]
pub struct FixedClock(pub Timestamp);

impl Clock for FixedClock {
    fn now(&self) -> Timestamp {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use vereinsakte_core::time::club_time;

    use super::*;

    #[test]
    fn clocks() {
        let fixed = FixedClock(club_time(2026, 10, 9, 12, 0));
        assert_eq!(fixed.now(), club_time(2026, 10, 9, 12, 0));
        let offset = SystemClock.now().offset().local_minus_utc();
        assert!(offset == 3600 || offset == 7200);
    }

    #[test]
    fn version_increments() {
        assert_eq!(Version(3).next(), Version(4));
    }
}
