//! Arbeitseinsätze und Stundenüberträge.

use std::fmt;

use chrono::Datelike;
use serde::{Deserialize, Serialize};

use super::club_record::ClubRecord;
use super::ids::{BoatId, HourTransferId, MemberId, TaskId, WorkEntryId, WorkEventId};
use super::time::Timestamp;

/// Ein Arbeitseinsatz eines Mitglieds.
///
/// Im JSON steht der Umfang flach im Eintrag: entweder `date` + `hours` oder
/// `start` + `end`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "RawWorkEntry", into = "RawWorkEntry")]
pub struct WorkEntry {
    pub id: WorkEntryId,
    pub member: MemberId,
    pub extent: WorkExtent,
    /// Boot oder Verein, für das die Arbeit geleistet wurde.
    pub subject: WorkSubject,
    pub description: Option<String>,
    /// Optionaler Bezug auf einen Arbeitstermin oder eine Aufgabe.
    pub reference: Option<WorkReference>,
    /// Bestätigung durch ein berechtigtes Amt; nur bestätigte Stunden zählen.
    pub confirmation: Option<Confirmation>,
}

/// Umfang eines Arbeitseinsatzes.
#[derive(Clone, Debug, PartialEq)]
pub enum WorkExtent {
    /// Datum und geleistete Stunden.
    Amount { date: Timestamp, hours: f64 },
    /// Beginn und Ende.
    Period { start: Timestamp, end: Timestamp },
}

/// Gegenstand eines Arbeitseinsatzes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkSubject {
    Boat(BoatId),
    /// Arbeiten ohne Boot, z. B. Webseite, IT oder Vereinszeitung.
    Club,
}

/// Bezug eines Arbeitseinsatzes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkReference {
    WorkEvent(WorkEventId),
    Task(TaskId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Confirmation {
    pub by: MemberId,
    pub at: Timestamp,
}

/// JSON-Darstellung eines Arbeitseinsatzes mit flachem Umfang.
#[derive(Serialize, Deserialize)]
struct RawWorkEntry {
    id: WorkEntryId,
    member: MemberId,
    subject: WorkSubject,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    date: Option<Timestamp>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    hours: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    start: Option<Timestamp>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    end: Option<Timestamp>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    reference: Option<WorkReference>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    confirmation: Option<Confirmation>,
}

impl TryFrom<RawWorkEntry> for WorkEntry {
    type Error = String;

    fn try_from(raw: RawWorkEntry) -> Result<Self, Self::Error> {
        let extent = match (raw.date, raw.hours, raw.start, raw.end) {
            (Some(date), Some(hours), None, None) => WorkExtent::Amount { date, hours },
            (None, None, Some(start), Some(end)) => WorkExtent::Period { start, end },
            (None, None, None, None) => {
                return Err(
                    "Arbeitseinsatz ohne Umfang: `date`/`hours` oder `start`/`end` fehlt".into(),
                );
            }
            (date, hours, start, end)
                if (date.is_some() || hours.is_some()) && (start.is_some() || end.is_some()) =>
            {
                return Err("Arbeitseinsatz mit `hours` und `start`/`end` zugleich".into());
            }
            _ => return Err("Arbeitseinsatz mit unvollständigem Umfang".into()),
        };
        Ok(Self {
            id: raw.id,
            member: raw.member,
            extent,
            subject: raw.subject,
            description: raw.description,
            reference: raw.reference,
            confirmation: raw.confirmation,
        })
    }
}

impl From<WorkEntry> for RawWorkEntry {
    fn from(entry: WorkEntry) -> Self {
        let (date, hours, start, end) = match entry.extent {
            WorkExtent::Amount { date, hours } => (Some(date), Some(hours), None, None),
            WorkExtent::Period { start, end } => (None, None, Some(start), Some(end)),
        };
        Self {
            id: entry.id,
            member: entry.member,
            subject: entry.subject,
            date,
            hours,
            start,
            end,
            description: entry.description,
            reference: entry.reference,
            confirmation: entry.confirmation,
        }
    }
}

/// Fehler eines einzelnen Arbeitseinsatzes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkEntryError {
    NegativeHours,
    EndNotAfterStart,
}

impl fmt::Display for WorkEntryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NegativeHours => "Stunden dürfen nicht negativ sein",
            Self::EndNotAfterStart => "Ende liegt nicht nach dem Beginn",
        })
    }
}

impl WorkEntry {
    /// Neuer, unbestätigter Einsatz.
    pub fn new(member: MemberId, extent: WorkExtent, subject: WorkSubject) -> Self {
        Self {
            id: WorkEntryId::new(),
            member,
            extent,
            subject,
            description: None,
            reference: None,
            confirmation: None,
        }
    }

    /// Geleistete Stunden.
    pub fn hours(&self) -> f64 {
        match &self.extent {
            WorkExtent::Amount { hours, .. } => *hours,
            WorkExtent::Period { start, end } => (*end - *start).num_seconds() as f64 / 3600.0,
        }
    }

    /// Kalenderjahr des Datums bzw. Beginns im angegebenen Offset.
    pub fn year(&self) -> i32 {
        match &self.extent {
            WorkExtent::Amount { date, .. } => date.year(),
            WorkExtent::Period { start, .. } => start.year(),
        }
    }

    /// Zeitpunkt des Einsatzes (Datum bzw. Beginn).
    pub fn date(&self) -> Timestamp {
        match &self.extent {
            WorkExtent::Amount { date, .. } => *date,
            WorkExtent::Period { start, .. } => *start,
        }
    }

    pub fn is_confirmed(&self) -> bool {
        self.confirmation.is_some()
    }

    pub fn validate(&self) -> Result<(), WorkEntryError> {
        match &self.extent {
            WorkExtent::Amount { hours, .. } if *hours < 0.0 => Err(WorkEntryError::NegativeHours),
            WorkExtent::Period { start, end } if end <= start => {
                Err(WorkEntryError::EndNotAfterStart)
            }
            _ => Ok(()),
        }
    }
}

/// Befund zu einem Arbeitseinsatz im Kontext der Vereinsakte.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorkEntryIssue {
    Invalid(WorkEntryError),
    /// Der Termin bzw. die Aufgabe, auf die verwiesen wird, existiert nicht.
    UnknownReference,
    /// Termin bzw. Aufgabe gilt einem Boot, der Gegenstand nennt etwas anderes.
    SubjectMismatch {
        expected: BoatId,
    },
}

impl fmt::Display for WorkEntryIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(error) => write!(f, "{error}"),
            Self::UnknownReference => f.write_str("Termin bzw. Aufgabe existiert nicht"),
            Self::SubjectMismatch { expected } => {
                write!(
                    f,
                    "Gegenstand passt nicht zum Termin bzw. zur Aufgabe (Boot {expected})"
                )
            }
        }
    }
}

impl ClubRecord {
    pub fn work_entry(&self, id: WorkEntryId) -> Option<&WorkEntry> {
        self.work_entries.iter().find(|e| e.id == id)
    }

    /// Prüft einen Einsatz einschließlich seines Bezugs auf Termin oder Aufgabe.
    pub fn validate_work_entry(&self, entry: &WorkEntry) -> Result<(), Vec<WorkEntryIssue>> {
        let mut issues = Vec::new();
        if let Err(error) = entry.validate() {
            issues.push(WorkEntryIssue::Invalid(error));
        }
        if let Some(reference) = entry.reference {
            let boat = match reference {
                WorkReference::WorkEvent(id) => {
                    self.work_events.iter().find(|e| e.id == id).map(|e| e.boat)
                }
                WorkReference::Task(id) => self.task(id).map(|t| t.boat),
            };
            match boat {
                None => issues.push(WorkEntryIssue::UnknownReference),
                Some(Some(expected)) if entry.subject != WorkSubject::Boat(expected) => {
                    issues.push(WorkEntryIssue::SubjectMismatch { expected });
                }
                Some(_) => {}
            }
        }
        if issues.is_empty() {
            Ok(())
        } else {
            Err(issues)
        }
    }
}

/// Übertrag von Arbeitsstunden zwischen Mitgliedern (z. B. Paare).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HourTransfer {
    pub id: HourTransferId,
    pub year: i32,
    pub from: MemberId,
    pub to: MemberId,
    pub hours: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HourTransferError {
    SelfTransfer,
    NonPositiveHours,
}

impl fmt::Display for HourTransferError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::SelfTransfer => "Stunden können nicht an sich selbst übertragen werden",
            Self::NonPositiveHours => "Übertragene Stunden müssen positiv sein",
        })
    }
}

impl HourTransfer {
    pub fn validate(&self) -> Result<(), Vec<HourTransferError>> {
        let mut errors = Vec::new();
        if self.from == self.to {
            errors.push(HourTransferError::SelfTransfer);
        }
        if self.hours <= 0.0 {
            errors.push(HourTransferError::NonPositiveHours);
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::example::{self, at, *};

    fn find<'a>(entries: &'a [WorkEntry], description: &str) -> &'a WorkEntry {
        entries
            .iter()
            .find(|e| e.description.as_deref() == Some(description))
            .expect(description)
    }

    #[test]
    fn amount_and_period_hours() {
        let entry = WorkEntry {
            description: Some("Unterwasserschiff geschliffen".into()),
            ..WorkEntry::new(
                CLARA,
                WorkExtent::Amount {
                    date: at(2026, 4, 12, 0, 0),
                    hours: 3.5,
                },
                WorkSubject::Boat(TRIGON),
            )
        };
        assert_eq!(entry.hours(), 3.5);
        let period = WorkEntry::new(
            BEN,
            WorkExtent::Period {
                start: at(2026, 4, 12, 9, 0),
                end: at(2026, 4, 12, 13, 30),
            },
            WorkSubject::Club,
        );
        assert_eq!(period.hours(), 4.5);
    }

    #[test]
    fn json_is_flat() {
        let entries = example::work_entries();
        let amount = serde_json::to_value(find(&entries, "Fock genäht")).unwrap();
        assert!(amount.get("date").is_some() && amount.get("hours").is_some());
        assert!(amount.get("start").is_none() && amount.get("extent").is_none());
        let period = serde_json::to_value(find(&entries, "Einkranen")).unwrap();
        assert!(period.get("start").is_some() && period.get("end").is_some());
        assert_eq!(
            serde_json::to_value(WorkSubject::Club).unwrap(),
            json!("club")
        );
        assert_eq!(
            serde_json::to_value(WorkSubject::Boat(TRIGON)).unwrap(),
            json!({"boat": TRIGON.to_string()})
        );
        for entry in &entries {
            let json = serde_json::to_string(entry).unwrap();
            assert_eq!(&serde_json::from_str::<WorkEntry>(&json).unwrap(), entry);
        }
    }

    #[test]
    fn both_or_no_extent_is_rejected() {
        let mut value = serde_json::to_value(&example::work_entries()[0]).unwrap();
        let obj = value.as_object_mut().unwrap();
        obj.insert("date".into(), json!("2026-04-18T09:00:00+02:00"));
        obj.insert("hours".into(), json!(6.0));
        obj.insert("start".into(), json!("2026-04-18T09:00:00+02:00"));
        obj.insert("end".into(), json!("2026-04-18T15:00:00+02:00"));
        assert!(serde_json::from_value::<WorkEntry>(value.clone()).is_err());
        for key in ["date", "hours", "start", "end"] {
            value.as_object_mut().unwrap().remove(key);
        }
        assert!(serde_json::from_value::<WorkEntry>(value).is_err());
    }

    #[test]
    fn subject_is_required() {
        let mut value = serde_json::to_value(&example::work_entries()[0]).unwrap();
        value.as_object_mut().unwrap().remove("subject");
        assert!(serde_json::from_value::<WorkEntry>(value).is_err());
    }

    #[test]
    fn plausibility() {
        let negative = WorkEntry::new(
            BEN,
            WorkExtent::Amount {
                date: at(2026, 5, 1, 0, 0),
                hours: -2.0,
            },
            WorkSubject::Club,
        );
        assert_eq!(negative.validate(), Err(WorkEntryError::NegativeHours));
        let backwards = WorkEntry::new(
            BEN,
            WorkExtent::Period {
                start: at(2026, 5, 1, 13, 0),
                end: at(2026, 5, 1, 9, 0),
            },
            WorkSubject::Club,
        );
        assert_eq!(backwards.validate(), Err(WorkEntryError::EndNotAfterStart));
    }

    #[test]
    fn year_uses_given_offset() {
        let entries = example::work_entries();
        let new_year = find(&entries, "Silvesterwache Bootshalle");
        assert_eq!(new_year.date().to_rfc3339(), "2025-12-31T22:00:00+01:00");
        assert_eq!(new_year.year(), 2025);
        let start = at(2026, 12, 31, 22, 0);
        let entry = WorkEntry::new(
            FRANK,
            WorkExtent::Period {
                start,
                end: at(2027, 1, 1, 1, 0),
            },
            WorkSubject::Club,
        );
        assert_eq!(entry.year(), 2026);
        assert_eq!(entry.hours(), 3.0);
    }

    #[test]
    fn entries_without_event_or_task_and_new_entries() {
        let entries = example::work_entries();
        let flicken = find(&entries, "Segel der Filou geflickt");
        assert_eq!(flicken.reference, None);
        assert_eq!(flicken.subject, WorkSubject::Boat(FILOU));
        assert!(!flicken.is_confirmed());
        assert_eq!(flicken.validate(), Ok(()));
        let website = find(&entries, "Webseite aktualisiert");
        assert_eq!(website.subject, WorkSubject::Club);
        assert_eq!(website.validate(), Ok(()));
        let fresh = WorkEntry::new(
            DAVID,
            WorkExtent::Amount {
                date: at(2026, 6, 1, 0, 0),
                hours: 1.0,
            },
            WorkSubject::Club,
        );
        assert!(!fresh.is_confirmed());
    }

    #[test]
    fn subject_must_match_referenced_boat() {
        let club = example::club();
        for entry in &club.work_entries {
            assert_eq!(
                club.validate_work_entry(entry),
                Ok(()),
                "{:?}",
                entry.description
            );
        }
        let mut entry = WorkEntry::new(
            CLARA,
            WorkExtent::Amount {
                date: at(2026, 3, 29, 0, 0),
                hours: 2.0,
            },
            WorkSubject::Boat(FILOU),
        );
        entry.reference = Some(WorkReference::Task(TRIGON_ANTIFOULING));
        assert_eq!(
            club.validate_work_entry(&entry),
            Err(vec![WorkEntryIssue::SubjectMismatch { expected: TRIGON }])
        );
        entry.subject = WorkSubject::Boat(TRIGON);
        assert_eq!(club.validate_work_entry(&entry), Ok(()));
        // Termin ohne Boot: jeder Gegenstand passt.
        entry.reference = Some(WorkReference::WorkEvent(EINKRANEN));
        entry.subject = WorkSubject::Boat(FILOU);
        assert_eq!(club.validate_work_entry(&entry), Ok(()));
        entry.reference = Some(WorkReference::Task(TaskId::new()));
        assert_eq!(
            club.validate_work_entry(&entry),
            Err(vec![WorkEntryIssue::UnknownReference])
        );
    }

    #[test]
    fn invalid_entries_are_reported_by_validate() {
        let mut club = example::club();
        let mut entry = WorkEntry::new(
            CLARA,
            WorkExtent::Amount {
                date: at(2026, 3, 29, 0, 0),
                hours: 2.0,
            },
            WorkSubject::Boat(FILOU),
        );
        entry.reference = Some(WorkReference::Task(TRIGON_ANTIFOULING));
        let id = entry.id;
        club.work_entries.push(entry);
        let issues = club.validate().unwrap_err();
        assert_eq!(
            issues,
            vec![crate::issues::RecordIssue::InvalidWorkEntry {
                entry: id,
                issue: WorkEntryIssue::SubjectMismatch { expected: TRIGON },
            }]
        );
    }

    #[test]
    fn hour_transfer_plausibility() {
        for transfer in example::hour_transfers() {
            assert_eq!(transfer.validate(), Ok(()));
        }
        let invalid = HourTransfer {
            id: HourTransferId::new(),
            year: 2026,
            from: BEN,
            to: BEN,
            hours: 0.0,
            description: None,
        };
        assert_eq!(
            invalid.validate(),
            Err(vec![
                HourTransferError::SelfTransfer,
                HourTransferError::NonPositiveHours
            ])
        );
        let negative = HourTransfer {
            hours: -3.0,
            to: CLARA,
            ..invalid
        };
        assert_eq!(
            negative.validate(),
            Err(vec![HourTransferError::NonPositiveHours])
        );
    }
}
