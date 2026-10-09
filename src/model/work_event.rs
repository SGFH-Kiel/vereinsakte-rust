//! Angekündigte gemeinsame Arbeitstermine mit Anmeldung.

use std::fmt;

use serde::{Deserialize, Serialize};

use super::ids::{BoatId, MemberId, WorkEventId};
use super::time::Timestamp;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkEvent {
    pub id: WorkEventId,
    pub title: String,
    pub start: Timestamp,
    pub end: Timestamp,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Betroffenes Boot, falls der Termin einem Boot gilt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boat: Option<BoatId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_participants: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub registrations: Vec<MemberId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventError {
    EndNotAfterStart,
}

impl fmt::Display for EventError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Ende des Termins liegt nicht nach dem Beginn")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegistrationError {
    AlreadyRegistered,
    Full,
}

impl fmt::Display for RegistrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::AlreadyRegistered => "Das Mitglied ist bereits angemeldet",
            Self::Full => "Der Termin ist ausgebucht",
        })
    }
}

impl WorkEvent {
    pub fn new(title: impl Into<String>, start: Timestamp, end: Timestamp) -> Self {
        Self {
            id: WorkEventId::new(),
            title: title.into(),
            start,
            end,
            location: None,
            description: None,
            boat: None,
            max_participants: None,
            registrations: Vec::new(),
        }
    }

    pub fn validate(&self) -> Result<(), EventError> {
        if self.end > self.start {
            Ok(())
        } else {
            Err(EventError::EndNotAfterStart)
        }
    }

    pub fn register(&mut self, member: MemberId) -> Result<(), RegistrationError> {
        if self.registrations.contains(&member) {
            return Err(RegistrationError::AlreadyRegistered);
        }
        if self
            .max_participants
            .is_some_and(|max| self.registrations.len() >= max as usize)
        {
            return Err(RegistrationError::Full);
        }
        self.registrations.push(member);
        Ok(())
    }

    /// Meldet ab; liefert `false`, wenn das Mitglied nicht angemeldet war.
    pub fn unregister(&mut self, member: MemberId) -> bool {
        let before = self.registrations.len();
        self.registrations.retain(|&m| m != member);
        self.registrations.len() != before
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::example::{self, at, *};

    fn kranen() -> WorkEvent {
        example::work_events()
            .into_iter()
            .find(|e| e.id == EINKRANEN)
            .unwrap()
    }

    #[test]
    fn event_survives_round_trip() {
        let event = kranen();
        assert_eq!(event.title, "Boote kranen");
        assert_eq!(event.location.as_deref(), Some("Sporthafen Düsternbrook"));
        assert_eq!(event.start, at(2026, 4, 18, 9, 0));
        let json = serde_json::to_string(&event).unwrap();
        assert_eq!(serde_json::from_str::<WorkEvent>(&json).unwrap(), event);
    }

    #[test]
    fn end_must_follow_start() {
        let mut event = kranen();
        assert_eq!(event.validate(), Ok(()));
        event.end = event.start;
        assert_eq!(event.validate(), Err(EventError::EndNotAfterStart));
    }

    #[test]
    fn registration_rules() {
        let mut event = WorkEvent::new(
            "Winterlager aufräumen",
            at(2026, 11, 7, 10, 0),
            at(2026, 11, 7, 14, 0),
        );
        event.max_participants = Some(2);
        assert_eq!(event.register(ANNA), Ok(()));
        assert_eq!(
            event.register(ANNA),
            Err(RegistrationError::AlreadyRegistered)
        );
        assert_eq!(event.registrations, vec![ANNA]);
        assert_eq!(event.register(BEN), Ok(()));
        assert_eq!(event.register(CLARA), Err(RegistrationError::Full));
        assert!(event.unregister(ANNA));
        assert!(!event.unregister(ANNA));
        assert_eq!(event.registrations, vec![BEN]);
    }
}
