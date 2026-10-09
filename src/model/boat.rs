//! Boote mit Stammdaten, Regeln der Segelordnung, Saisonfreigabe und Sperren.

use std::fmt;

use chrono::Datelike;
use serde::{Deserialize, Serialize};

use super::ids::{BoatId, MemberId};
use super::requirement::Requirement;
use super::time::Timestamp;

/// Ein Vereinsboot.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Boat {
    pub id: BoatId,
    pub name: String,
    pub boat_type: BoatType,
    /// Bootsklasse, z. B. „Matcher 37“.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub year_built: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub length_m: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub beam_m: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub draft_m: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sail_area_m2: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub berths: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mooring: Option<String>,
    /// Anforderungen an den Skipper zusätzlich zur Bootserlaubnis für dieses Boot.
    #[serde(default, skip_serializing_if = "Requirement::is_empty")]
    pub skipper_requirement: Requirement,
    /// Höchstens eine Regel je Fahrtgebiet.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub crew_rules: Vec<CrewRule>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub releases: Vec<Release>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<Block>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BoatType {
    /// Jolle
    Dinghy,
    /// Kielboot
    Keelboat,
}

impl fmt::Display for BoatType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Dinghy => "Jolle",
            Self::Keelboat => "Kielboot",
        })
    }
}

/// Fahrtgebiet nach Segelordnung 2.2/2.3.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SailingArea {
    /// Heimrevier (Kieler Förde bzw. Bülk – Kiel Leuchtturm – Wendtorf)
    HomeWaters,
    /// außerhalb des Heimreviers
    Outside,
}

impl fmt::Display for SailingArea {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::HomeWaters => "Heimrevier",
            Self::Outside => "außerhalb des Heimreviers",
        })
    }
}

/// Besatzungsregel eines Bootes für ein Fahrtgebiet.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CrewRule {
    pub area: SailingArea,
    /// Mindestzahl an Vereinsmitgliedern an Bord.
    pub min_members: u8,
    /// Mindestzahl an Personen insgesamt (inkl. Gäste).
    pub min_persons: u8,
    #[serde(default, skip_serializing_if = "is_false")]
    pub co_skipper_required: bool,
    #[serde(default, skip_serializing_if = "Requirement::is_empty")]
    pub co_skipper_requirement: Requirement,
}

fn is_false(value: &bool) -> bool {
    !*value
}

/// Freigabe eines Bootes für eine Saison.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Release {
    pub season: i32,
    pub by: MemberId,
    pub at: Timestamp,
}

/// Sperre eines Bootes; `end` ist exklusiv, ohne Ende gilt die Sperre unbefristet.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Block {
    pub start: Timestamp,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<Timestamp>,
    pub reason: String,
    pub by: MemberId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lifted_by: Option<MemberId>,
}

impl Block {
    pub fn is_active_at(&self, t: Timestamp) -> bool {
        self.start <= t && self.end.is_none_or(|end| t < end)
    }
}

/// Grund, warum ein Boot zu einem Zeitpunkt nicht nutzbar ist.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BoatUnavailable {
    NotReleased { season: i32 },
    Blocked { reason: String },
}

impl fmt::Display for BoatUnavailable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotReleased { season } => {
                write!(f, "Boot ist für die Saison {season} nicht freigegeben")
            }
            Self::Blocked { reason } => write!(f, "Boot ist gesperrt: {reason}"),
        }
    }
}

impl Boat {
    /// Neues Boot mit Name und Typ; alle weiteren Angaben sind leer.
    pub fn new(id: BoatId, name: impl Into<String>, boat_type: BoatType) -> Self {
        Self {
            id,
            name: name.into(),
            boat_type,
            class: None,
            year_built: None,
            length_m: None,
            beam_m: None,
            draft_m: None,
            sail_area_m2: None,
            berths: None,
            engine: None,
            mooring: None,
            skipper_requirement: Requirement::none(),
            crew_rules: Vec::new(),
            releases: Vec::new(),
            blocks: Vec::new(),
        }
    }

    /// Besatzungsregel für ein Fahrtgebiet; `None` heißt: ohne
    /// Vorstandsentscheidung nicht freigegeben.
    pub fn crew_rule(&self, area: SailingArea) -> Option<&CrewRule> {
        self.crew_rules.iter().find(|rule| rule.area == area)
    }

    pub fn is_released_for(&self, season: i32) -> bool {
        self.releases.iter().any(|r| r.season == season)
    }

    /// Ist das Boot zum Zeitpunkt nutzbar (freigegeben und nicht gesperrt)?
    pub fn availability_at(&self, t: Timestamp) -> Result<(), Vec<BoatUnavailable>> {
        let mut issues = Vec::new();
        if !self.is_released_for(t.year()) {
            issues.push(BoatUnavailable::NotReleased { season: t.year() });
        }
        issues.extend(self.blocks.iter().filter(|b| b.is_active_at(t)).map(|b| {
            BoatUnavailable::Blocked {
                reason: b.reason.clone(),
            }
        }));
        if issues.is_empty() {
            Ok(())
        } else {
            Err(issues)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::example::{self, at};
    use crate::model::requirement::Qualification;

    #[test]
    fn boats_survive_round_trip() {
        let trigon = example::trigon();
        assert_eq!(trigon.class.as_deref(), Some("Matcher 37"));
        assert_eq!(trigon.year_built, Some(1996));
        assert_eq!(trigon.length_m, Some(11.10));
        let json = serde_json::to_string(&trigon).unwrap();
        assert_eq!(serde_json::from_str::<Boat>(&json).unwrap(), trigon);

        let filou = Boat::new(example::FILOU, "Filou", BoatType::Dinghy);
        let value = serde_json::to_value(&filou).unwrap();
        assert_eq!(
            value.as_object().unwrap().len(),
            3,
            "nur id, name, boat_type: {value}"
        );
        assert_eq!(serde_json::from_value::<Boat>(value).unwrap(), filou);
    }

    #[test]
    fn dehler_requirement_is_data() {
        let dehler = example::schruvtrekker();
        assert_eq!(dehler.skipper_requirement.0.len(), 3);
        assert!(
            dehler.skipper_requirement.0[2]
                .0
                .contains(&Qualification::RadioLicense(
                    crate::model::member::RadioLicenseType::Src
                ))
        );
    }

    #[test]
    fn crew_rules_per_area() {
        let folkeboot = example::harald_blauzahn();
        let outside = folkeboot.crew_rule(SailingArea::Outside).unwrap();
        assert_eq!(outside.min_members, 2);
        assert!(outside.co_skipper_required);
        assert!(example::filou().crew_rule(SailingArea::Outside).is_none());
    }

    #[test]
    fn release_is_per_season() {
        let mut boat = example::filou();
        boat.releases.retain(|r| r.season == 2025);
        assert_eq!(
            boat.availability_at(at(2026, 5, 1, 12, 0)),
            Err(vec![BoatUnavailable::NotReleased { season: 2026 }])
        );
    }

    #[test]
    fn blocks_with_and_without_end() {
        let mut boat = example::harald_blauzahn();
        boat.blocks.push(Block {
            start: at(2026, 6, 3, 9, 0),
            end: None,
            reason: "Ruderschaden".into(),
            by: example::BEN,
            lifted_by: None,
        });
        let issues = boat.availability_at(at(2026, 6, 10, 12, 0)).unwrap_err();
        assert_eq!(
            issues,
            vec![BoatUnavailable::Blocked {
                reason: "Ruderschaden".into()
            }]
        );
        assert_eq!(issues[0].to_string(), "Boot ist gesperrt: Ruderschaden");

        boat.blocks[0].end = Some(at(2026, 6, 15, 0, 0));
        assert!(boat.availability_at(at(2026, 6, 14, 23, 0)).is_err());
        assert_eq!(boat.availability_at(at(2026, 6, 16, 12, 0)), Ok(()));
    }
}
