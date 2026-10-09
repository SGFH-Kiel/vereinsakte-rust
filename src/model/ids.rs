//! Kennungen der Fachobjekte.
//!
//! Mitglieder werden über ihre Mitgliedsnummer identifiziert, alle anderen
//! Fachobjekte über eine UUID.

use std::fmt;
use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Mitgliedsnummer – zugleich die Kennung eines Mitglieds.
///
/// Im JSON eine positive ganze Zahl; `0`, negative Zahlen und Zeichenketten
/// werden beim Einlesen abgelehnt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u32", into = "u32")]
pub struct MemberId(NonZeroU32);

impl MemberId {
    /// Liefert `None` für die Nummer 0.
    pub const fn new(number: u32) -> Option<Self> {
        match NonZeroU32::new(number) {
            Some(n) => Some(Self(n)),
            None => None,
        }
    }

    pub fn get(self) -> u32 {
        self.0.get()
    }
}

impl TryFrom<u32> for MemberId {
    type Error = String;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::new(value).ok_or_else(|| "Mitgliedsnummer muss positiv sein".to_string())
    }
}

impl From<MemberId> for u32 {
    fn from(id: MemberId) -> Self {
        id.get()
    }
}

impl fmt::Display for MemberId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

macro_rules! uuid_id {
    ($($(#[$doc:meta])* $name:ident),* $(,)?) => {$(
        $(#[$doc])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            /// Erzeugt eine neue, zufällige Kennung.
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }

            pub const fn from_uuid(uuid: Uuid) -> Self {
                Self(uuid)
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    )*};
}

uuid_id!(
    /// Kennung eines Bootes.
    BoatId,
    /// Kennung eines Amtes.
    OfficeId,
    /// Kennung einer Bootsabnahme.
    CheckoutId,
    /// Kennung eines Widerrufs.
    RevocationId,
    /// Kennung eines Arbeitseinsatzes.
    WorkEntryId,
    /// Kennung eines Stundenübertrags.
    HourTransferId,
    /// Kennung einer Reservierung.
    ReservationId,
    /// Kennung eines Arbeitstermins.
    WorkEventId,
    /// Kennung einer Aufgabe.
    TaskId,
    /// Kennung einer Forderung.
    ClaimId,
    /// Kennung einer Einzahlung.
    PaymentId,
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn member_id_is_plain_number() {
        let id = MemberId::new(42).unwrap();
        assert_eq!(serde_json::to_string(&id).unwrap(), "42");
        assert_eq!(serde_json::from_str::<MemberId>("42").unwrap(), id);
    }

    #[test]
    fn member_id_rejects_invalid_numbers() {
        for json in ["0", "-5", "\"42\"", "4.5"] {
            assert!(serde_json::from_str::<MemberId>(json).is_err(), "{json}");
        }
    }

    #[test]
    fn uuid_ids_are_unique_strings() {
        let a = BoatId::new();
        let b = BoatId::new();
        assert_ne!(a, b);
        let json = serde_json::to_string(&a).unwrap();
        assert!(json.starts_with('"') && json.len() == 38, "{json}");
        assert_eq!(serde_json::from_str::<BoatId>(&json).unwrap(), a);
    }

    #[test]
    fn invalid_uuid_is_rejected() {
        assert!(serde_json::from_str::<BoatId>("\"keine-uuid\"").is_err());
    }
}
