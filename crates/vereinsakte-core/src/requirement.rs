//! Anforderungen an Skipper und Co-Skipper als Daten.
//!
//! Eine [`Requirement`] ist eine Liste von Gruppen ([`AnyOf`]); jede Gruppe
//! ist erfüllt, wenn mindestens einer ihrer Nachweise vorliegt, und die
//! Anforderung ist erfüllt, wenn alle Gruppen erfüllt sind.

use std::fmt;

use serde::{Deserialize, Serialize};

use super::boat::BoatType;
use super::ids::BoatId;
use super::member::{RadioLicenseType, SailingLicenseType};

/// Ein einzelner Nachweis.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Qualification {
    SailingLicense(SailingLicenseType),
    RadioLicense(RadioLicenseType),
    /// Bootserlaubnis für genau dieses Boot.
    BoatPermit(BoatId),
    /// Bootserlaubnis für irgendein Boot dieses Typs (z. B. „Jollenerlaubnis“).
    BoatPermitForType(BoatType),
}

/// Gruppe von Nachweisen, von denen mindestens einer vorliegen muss.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AnyOf(pub Vec<Qualification>);

/// Anforderung: alle Gruppen müssen erfüllt sein. Leer = keine Anforderung.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Requirement(pub Vec<AnyOf>);

impl Requirement {
    pub fn none() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Fügt eine Gruppe hinzu.
    pub fn and(mut self, group: AnyOf) -> Self {
        self.0.push(group);
        self
    }
}

impl AnyOf {
    pub fn sailing_licenses(kinds: &[SailingLicenseType]) -> Self {
        Self(
            kinds
                .iter()
                .cloned()
                .map(Qualification::SailingLicense)
                .collect(),
        )
    }

    pub fn radio_licenses(kinds: &[RadioLicenseType]) -> Self {
        Self(
            kinds
                .iter()
                .cloned()
                .map(Qualification::RadioLicense)
                .collect(),
        )
    }

    pub fn or(mut self, qualification: Qualification) -> Self {
        self.0.push(qualification);
        self
    }
}

impl fmt::Display for Qualification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SailingLicense(kind) => write!(f, "{kind}"),
            Self::RadioLicense(kind) => write!(f, "{kind}"),
            Self::BoatPermit(boat) => write!(f, "Bootserlaubnis für Boot {boat}"),
            Self::BoatPermitForType(boat_type) => write!(f, "Bootserlaubnis für eine {boat_type}"),
        }
    }
}

impl fmt::Display for AnyOf {
    /// z. B. „SKS | SSS | SHS | BR“
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parts: Vec<String> = self.0.iter().map(ToString::to_string).collect();
        write!(f, "[{}]", parts.join(" | "))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn qualification_json_shape() {
        assert_eq!(
            serde_json::to_value(Qualification::SailingLicense(SailingLicenseType::Sks)).unwrap(),
            json!({"sailing_license": "SKS"})
        );
        assert_eq!(
            serde_json::to_value(Qualification::BoatPermitForType(BoatType::Dinghy)).unwrap(),
            json!({"boat_permit_for_type": "dinghy"})
        );
    }

    #[test]
    fn group_display() {
        let group = AnyOf::radio_licenses(&[RadioLicenseType::Src, RadioLicenseType::Lrc]);
        assert_eq!(group.to_string(), "[SRC | LRC]");
    }
}
