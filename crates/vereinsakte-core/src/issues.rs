//! Meldungen der Prüfung einer Vereinsakte.

use std::fmt;

use super::boat::{BoatUnavailable, SailingArea};
use super::boat_permit::EligibilityIssue;
use super::ids::{BoatId, HourTransferId, MemberId, WorkEntryId};
use super::office::{Action, Role};
use super::reservation::ReservationError;
use super::sailing_permit::SailingPermitMissing;
use super::time::Timestamp;
use super::work_hours::{HourTransferError, WorkEntryIssue};

/// Verstoß gegen eine Akte-weite Regel.
#[derive(Clone, Debug, PartialEq)]
pub enum RecordIssue {
    DuplicateMemberId(MemberId),
    /// Anmeldekonto bei mehreren Mitgliedern (ohne Beachtung der Groß-/Kleinschreibung).
    DuplicateLoginEmail(String),
    /// Vorstandsamt mit nicht stimmberechtigtem Mitglied besetzt (Satzung §10 Abs. 1).
    BoardMemberWithoutVote {
        member: MemberId,
        role: Role,
    },
    /// Verschiedene Vorstandsämter in einer Person (Satzung §10 Abs. 5).
    BoardOfficesAccumulated {
        member: MemberId,
        roles: (Role, Role),
    },
    /// Ein Vorstandsamt ist gleichzeitig mehrfach besetzt.
    BoardOfficeTwice {
        role: Role,
    },
    InvalidWorkEntry {
        entry: WorkEntryId,
        issue: WorkEntryIssue,
    },
    InvalidHourTransfer {
        transfer: HourTransferId,
        error: HourTransferError,
    },
    /// Gespeicherte Handlung, zu der der Urheber nicht berechtigt war.
    Unauthorized {
        action: Action,
        actor: MemberId,
        at: Timestamp,
    },
}

impl fmt::Display for RecordIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateMemberId(id) => write!(f, "Mitgliedsnummer {id} ist mehrfach vergeben"),
            Self::DuplicateLoginEmail(login) => {
                write!(f, "Anmeldekonto {login} ist mehrfach vergeben")
            }
            Self::BoardMemberWithoutVote { member, role } => {
                write!(f, "Mitglied {member} ist als {role} nicht stimmberechtigt")
            }
            Self::BoardOfficesAccumulated {
                member,
                roles: (a, b),
            } => {
                write!(
                    f,
                    "Mitglied {member} hat zugleich die Vorstandsämter {a} und {b}"
                )
            }
            Self::BoardOfficeTwice { role } => {
                write!(f, "Das Amt {role} ist gleichzeitig mehrfach besetzt")
            }
            Self::InvalidWorkEntry { entry, issue } => write!(f, "Arbeitseinsatz {entry}: {issue}"),
            Self::InvalidHourTransfer { transfer, error } => {
                write!(f, "Stundenübertrag {transfer}: {error}")
            }
            Self::Unauthorized { action, actor, at } => {
                write!(
                    f,
                    "Mitglied {actor} war am {} nicht berechtigt zur {action}",
                    at.format("%d.%m.%Y")
                )
            }
        }
    }
}

/// Beanstandung einer Reservierung nach der Segelordnung.
#[derive(Clone, Debug, PartialEq)]
pub enum ReservationIssue {
    Invalid(ReservationError),
    UnknownBoat(BoatId),
    UnknownMember(MemberId),
    BoatUnavailable(BoatUnavailable),
    /// Mitglied der Crew ohne Segelerlaubnis.
    NoSailingPermit {
        member: MemberId,
        reasons: Vec<SailingPermitMissing>,
    },
    Skipper(Vec<EligibilityIssue>),
    CoSkipper(Vec<EligibilityIssue>),
    AreaNotReleased(SailingArea),
    CoSkipperMissing,
    TooFewMembers {
        actual: u32,
        min: u32,
    },
    TooFewPersons {
        actual: u32,
        min: u32,
    },
}

fn join<T: fmt::Display>(items: &[T]) -> String {
    items
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

impl fmt::Display for ReservationIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(error) => write!(f, "{error}"),
            Self::UnknownBoat(id) => write!(f, "unbekanntes Boot {id}"),
            Self::UnknownMember(id) => write!(f, "unbekanntes Mitglied {id}"),
            Self::BoatUnavailable(reason) => write!(f, "{reason}"),
            Self::NoSailingPermit { member, reasons } => {
                write!(
                    f,
                    "Mitglied {member} hat keine Segelerlaubnis: {}",
                    join(reasons)
                )
            }
            Self::Skipper(issues) => write!(f, "Skipper nicht geeignet: {}", join(issues)),
            Self::CoSkipper(issues) => write!(f, "Co-Skipper nicht geeignet: {}", join(issues)),
            Self::AreaNotReleased(area) => write!(f, "Fahrtgebiet nicht freigegeben ({area})"),
            Self::CoSkipperMissing => f.write_str("Co-Skipper fehlt"),
            Self::TooFewMembers { actual, min } => {
                write!(
                    f,
                    "zu wenige Vereinsmitglieder an Bord ({actual} statt mindestens {min})"
                )
            }
            Self::TooFewPersons { actual, min } => {
                write!(
                    f,
                    "zu wenige Personen an Bord ({actual} statt mindestens {min})"
                )
            }
        }
    }
}
