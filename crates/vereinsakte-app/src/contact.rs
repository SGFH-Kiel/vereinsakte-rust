//! Sichtbarkeit der Kontaktdaten (E-Mail, Telefonnummern): vereinsintern für
//! alle aktiven Mitglieder, außer das Mitglied hat einen Sperrvermerk gesetzt.
//! Dann sehen sie nur das Mitglied selbst und der Vorstand. Die Werte gelangen
//! gar nicht erst in die Sichtmodelle, wenn sie nicht sichtbar sind.

use vereinsakte_core::ids::MemberId;
use vereinsakte_core::member::PhoneNumbers;

use crate::principal::Principal;

/// E-Mail und Telefonnummern eines Mitglieds aus Sicht des Betrachters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContactInfo {
    Visible {
        email: Option<String>,
        phones: Vec<String>,
    },
    /// Sperrvermerk gesetzt: keine Angaben.
    Blocked,
}

/// Darf der Betrachter die Kontaktdaten des Mitglieds sehen?
pub fn contact_visible(principal: &Principal, subject: MemberId, blocked: bool) -> bool {
    !blocked || principal.member == subject || principal.is_board()
}

impl ContactInfo {
    pub fn of(
        principal: &Principal,
        subject: MemberId,
        blocked: bool,
        email: Option<&String>,
        phone: &PhoneNumbers,
    ) -> Self {
        if contact_visible(principal, subject, blocked) {
            Self::Visible {
                email: email.cloned(),
                phones: [&phone.landline, &phone.business, &phone.mobile]
                    .into_iter()
                    .flatten()
                    .cloned()
                    .collect(),
            }
        } else {
            Self::Blocked
        }
    }
}

impl ContactInfo {
    pub fn is_blocked(&self) -> bool {
        matches!(self, Self::Blocked)
    }

    /// Kontakt-E-Mail, falls sichtbar und vorhanden.
    pub fn email(&self) -> Option<&str> {
        match self {
            Self::Visible { email, .. } => email.as_deref(),
            Self::Blocked => None,
        }
    }

    /// Telefonnummern, falls sichtbar.
    pub fn phones(&self) -> &[String] {
        match self {
            Self::Visible { phones, .. } => phones,
            Self::Blocked => &[],
        }
    }
}
