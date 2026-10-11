//! Verweis auf eine Person für die Anzeige: mit Mitgliedsnummer (dann ist der
//! Name ein Link zur Profilseite) oder ohne (Gäste, ausgeschiedene Mitglieder).

use vereinsakte_core::ids::MemberId;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PersonRef {
    /// Nur für aktive Mitglieder mit Profilseite.
    pub id: Option<u32>,
    pub name: String,
}

impl PersonRef {
    pub fn member(id: MemberId, name: impl Into<String>) -> Self {
        Self {
            id: Some(id.get()),
            name: name.into(),
        }
    }

    pub fn text(name: impl Into<String>) -> Self {
        Self {
            id: None,
            name: name.into(),
        }
    }
}
