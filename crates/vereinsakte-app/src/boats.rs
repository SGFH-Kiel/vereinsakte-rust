//! Boote ansehen und pflegen, sperren, freigeben sowie Abnahmen und Widerrufe.

use chrono::{Datelike, NaiveDate};
use vereinsakte_core::boat::{Boat, BoatType, CrewRule, SailingArea};
use vereinsakte_core::boat_permit::CheckoutResult;
use vereinsakte_core::ids::{BoatId, MemberId};
use vereinsakte_core::member::{RadioLicenseType, SailingLicenseType};
use vereinsakte_core::office::{ActionError, NotAuthorized, Offices, Role};
use vereinsakte_core::requirement::{AnyOf, Qualification, Requirement};
use vereinsakte_core::rights::Capability;
use vereinsakte_core::time::{Timestamp, club_time};

use crate::changes::RecordKind;
use crate::commands::audit;
use crate::error::{AppError, FieldError};
use crate::format;
use crate::ports::Version;
use crate::scope::{BoatScope, MemberScope, Scope};
use crate::service::{ClubService, CurrentUser};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoatRow {
    pub id: String,
    pub name: String,
    pub boat_type: String,
    /// Symbol („jolle“ oder „dickschiff“) und Tooltip („Kielboot · Matcher 37“).
    pub type_icon: &'static str,
    pub type_tip: String,
    pub class: String,
    /// Für die laufende Saison freigegeben?
    pub released: bool,
    /// Grund der aktuellen Sperre.
    pub blocked: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoatList {
    pub season: i32,
    pub rows: Vec<BoatRow>,
    pub can_create: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockLine {
    pub period: String,
    pub reason: String,
    pub by: String,
    pub open: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrewRuleLine {
    pub area: String,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Eligibility {
    pub skipper: bool,
    pub reasons: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoatPage {
    pub id: String,
    pub name: String,
    pub boat_type: String,
    pub type_icon: &'static str,
    pub type_tip: String,
    pub facts: Vec<(&'static str, String)>,
    pub wardens: Vec<String>,
    pub examiners: Vec<String>,
    pub requirement: String,
    pub crew_rules: Vec<CrewRuleLine>,
    pub released_seasons: Vec<i32>,
    pub season: i32,
    pub released: bool,
    pub blocks: Vec<BlockLine>,
    pub eligibility: Eligibility,
    pub can_edit: bool,
    pub can_block: bool,
    pub can_lift: bool,
    pub can_release: bool,
    pub can_checkout: bool,
    pub can_revoke: bool,
    pub members: Vec<(u32, String)>,
}

/// Eingaben des Bootsformulars (Texte, wie sie aus dem Formular kommen).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BoatInput {
    pub name: String,
    pub dinghy: bool,
    pub class: String,
    pub year_built: String,
    pub length: String,
    pub beam: String,
    pub draft: String,
    pub sail_area: String,
    pub berths: String,
    pub engine: String,
    pub mooring: String,
    pub skipper_requirement: String,
    pub home_min_members: String,
    pub home_min_persons: String,
    pub home_co_skipper: bool,
    pub outside_min_members: String,
    pub outside_min_persons: String,
    pub outside_co_skipper: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoatForm {
    pub id: String,
    pub version: i64,
    pub input: BoatInput,
}

/// Symbol und Tooltip des Bootstyps für die Oberfläche.
pub fn boat_type_display(boat: &Boat) -> (&'static str, String) {
    let icon = match boat.boat_type {
        BoatType::Dinghy => "jolle",
        BoatType::Keelboat => "dickschiff",
    };
    let tip = match boat.class.as_deref().filter(|c| !c.is_empty()) {
        Some(class) => format!("{} · {class}", boat.boat_type),
        None => boat.boat_type.to_string(),
    };
    (icon, tip)
}

fn trimmed(text: &str) -> Option<String> {
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_string())
}

fn midnight(date: NaiveDate) -> Timestamp {
    club_time(date.year(), date.month(), date.day(), 0, 0)
}

fn action_error(error: ActionError) -> AppError {
    match error {
        ActionError::NotAuthorized(NotAuthorized::MissingOffice | NotAuthorized::OwnWorkEntry) => {
            AppError::Forbidden
        }
        ActionError::NotAuthorized(NotAuthorized::UnknownWorkEntry) => {
            AppError::NotFound("Arbeitseinsatz".into())
        }
        ActionError::UnknownBoat(_) => AppError::NotFound("Boot".into()),
        ActionError::UnknownWorkEntry(_) => AppError::NotFound("Arbeitseinsatz".into()),
        ActionError::NoOpenBlock => AppError::rejected("", "Es gibt keine offene Sperre."),
    }
}

/// Anforderung als Text: je Zeile eine Gruppe, Alternativen mit „|“,
/// z. B. `SKS | SSS`, `Funk:SRC`, `Boot:Filou`, `Typ:Jolle`.
pub fn requirement_text(requirement: &Requirement, boats: &[Boat]) -> String {
    requirement
        .0
        .iter()
        .map(|group| {
            group
                .0
                .iter()
                .map(|q| match q {
                    Qualification::SailingLicense(kind) => kind.to_string(),
                    Qualification::RadioLicense(kind) => format!("Funk:{kind}"),
                    Qualification::BoatPermit(id) => format!(
                        "Boot:{}",
                        boats
                            .iter()
                            .find(|b| b.id == *id)
                            .map_or_else(|| id.to_string(), |b| b.name.clone())
                    ),
                    Qualification::BoatPermitForType(t) => format!("Typ:{t}"),
                })
                .collect::<Vec<_>>()
                .join(" | ")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn parse_requirement(text: &str, boats: &[Boat]) -> Result<Requirement, String> {
    let mut groups = vec![];
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let mut any = vec![];
        for part in line.split('|').map(str::trim).filter(|p| !p.is_empty()) {
            let q = if let Some(kind) = part.strip_prefix("Funk:") {
                Qualification::RadioLicense(RadioLicenseType::from(kind.trim()))
            } else if let Some(name) = part.strip_prefix("Boot:") {
                let boat = boats
                    .iter()
                    .find(|b| b.name.eq_ignore_ascii_case(name.trim()))
                    .ok_or_else(|| format!("Unbekanntes Boot „{}“.", name.trim()))?;
                Qualification::BoatPermit(boat.id)
            } else if let Some(kind) = part.strip_prefix("Typ:") {
                match kind.trim().to_lowercase().as_str() {
                    "jolle" => Qualification::BoatPermitForType(BoatType::Dinghy),
                    "kielboot" => Qualification::BoatPermitForType(BoatType::Keelboat),
                    other => {
                        return Err(format!(
                            "Unbekannter Bootstyp „{other}“ (Jolle oder Kielboot)."
                        ));
                    }
                }
            } else {
                Qualification::SailingLicense(SailingLicenseType::from(part))
            };
            any.push(q);
        }
        if !any.is_empty() {
            groups.push(AnyOf(any));
        }
    }
    Ok(Requirement(groups))
}

fn crew_rule_text(rule: &CrewRule) -> String {
    let plural = |n: u8, one: &str, many: &str| {
        if n == 1 {
            format!("1 {one}")
        } else {
            format!("{n} {many}")
        }
    };
    let mut text = format!(
        "mindestens {}, insgesamt mindestens {}",
        plural(rule.min_members, "Mitglied", "Mitglieder"),
        plural(rule.min_persons, "Person", "Personen"),
    );
    if rule.co_skipper_required {
        text.push_str(", Co-Skipper erforderlich");
    }
    text
}

impl BoatInput {
    fn from_boat(b: &Boat, all: &[Boat]) -> Self {
        let rule = |area| b.crew_rule(area);
        let num = |v: Option<f64>| v.map(format::number).unwrap_or_default();
        Self {
            name: b.name.clone(),
            dinghy: b.boat_type == BoatType::Dinghy,
            class: b.class.clone().unwrap_or_default(),
            year_built: b.year_built.map(|y| y.to_string()).unwrap_or_default(),
            length: num(b.length_m),
            beam: num(b.beam_m),
            draft: num(b.draft_m),
            sail_area: num(b.sail_area_m2),
            berths: b.berths.map(|n| n.to_string()).unwrap_or_default(),
            engine: b.engine.clone().unwrap_or_default(),
            mooring: b.mooring.clone().unwrap_or_default(),
            skipper_requirement: requirement_text(&b.skipper_requirement, all),
            home_min_members: rule(SailingArea::HomeWaters)
                .map(|r| r.min_members.to_string())
                .unwrap_or_default(),
            home_min_persons: rule(SailingArea::HomeWaters)
                .map(|r| r.min_persons.to_string())
                .unwrap_or_default(),
            home_co_skipper: rule(SailingArea::HomeWaters).is_some_and(|r| r.co_skipper_required),
            outside_min_members: rule(SailingArea::Outside)
                .map(|r| r.min_members.to_string())
                .unwrap_or_default(),
            outside_min_persons: rule(SailingArea::Outside)
                .map(|r| r.min_persons.to_string())
                .unwrap_or_default(),
            outside_co_skipper: rule(SailingArea::Outside).is_some_and(|r| r.co_skipper_required),
        }
    }

    /// Wendet die Eingaben auf das Boot an; Fehler je Feld.
    fn apply(&self, boat: &mut Boat, all: &[Boat]) -> Result<(), Vec<FieldError>> {
        let mut errors = vec![];
        if self.name.trim().is_empty() {
            errors.push(FieldError::new("name", "Bitte den Namen angeben."));
        } else if all
            .iter()
            .any(|b| b.id != boat.id && b.name.eq_ignore_ascii_case(self.name.trim()))
        {
            errors.push(FieldError::new(
                "name",
                "Ein Boot mit diesem Namen gibt es bereits.",
            ));
        }
        let mut number = |field: &str, text: &str| -> Option<f64> {
            let text = text.trim();
            if text.is_empty() {
                return None;
            }
            match format::parse_decimal(text).filter(|v| *v > 0.0) {
                Some(v) => Some(v),
                None => {
                    errors.push(FieldError::new(field, "Bitte eine positive Zahl eingeben."));
                    None
                }
            }
        };
        let length = number("length", &self.length);
        let beam = number("beam", &self.beam);
        let draft = number("draft", &self.draft);
        let sail_area = number("sail_area", &self.sail_area);
        let year_built = match trimmed(&self.year_built) {
            None => None,
            Some(y) => match y.parse::<u16>().ok().filter(|y| (1850..=2100).contains(y)) {
                Some(y) => Some(y),
                None => {
                    errors.push(FieldError::new(
                        "year_built",
                        "Bitte ein Baujahr zwischen 1850 und 2100 angeben.",
                    ));
                    None
                }
            },
        };
        let berths = match trimmed(&self.berths) {
            None => None,
            Some(n) => match n.parse::<u8>() {
                Ok(n) => Some(n),
                Err(_) => {
                    errors.push(FieldError::new("berths", "Bitte eine ganze Zahl angeben."));
                    None
                }
            },
        };
        let requirement = parse_requirement(&self.skipper_requirement, all)
            .map_err(|m| errors.push(FieldError::new("skipper_requirement", m)))
            .ok();
        let mut rules = vec![];
        for (area, prefix, members, persons, co) in [
            (
                SailingArea::HomeWaters,
                "home",
                &self.home_min_members,
                &self.home_min_persons,
                self.home_co_skipper,
            ),
            (
                SailingArea::Outside,
                "outside",
                &self.outside_min_members,
                &self.outside_min_persons,
                self.outside_co_skipper,
            ),
        ] {
            if members.trim().is_empty() && persons.trim().is_empty() && !co {
                continue;
            }
            let min_members = members.trim().parse::<u8>().ok();
            let min_persons = persons.trim().parse::<u8>().ok();
            match (min_members, min_persons) {
                (Some(m), Some(p)) if p >= m => rules.push(CrewRule {
                    area,
                    min_members: m,
                    min_persons: p,
                    co_skipper_required: co,
                    co_skipper_requirement: boat
                        .crew_rule(area)
                        .map(|r| r.co_skipper_requirement.clone())
                        .unwrap_or_default(),
                }),
                (Some(_), Some(_)) => errors.push(FieldError::new(
                    &format!("{prefix}_min_persons"),
                    "Die Mindestzahl der Personen darf nicht unter der der Mitglieder liegen.",
                )),
                _ => errors.push(FieldError::new(
                    &format!("{prefix}_min_members"),
                    "Bitte Mindestzahl Mitglieder und Personen als ganze Zahlen angeben.",
                )),
            }
        }
        if !errors.is_empty() {
            return Err(errors);
        }
        boat.name = self.name.trim().to_string();
        boat.boat_type = if self.dinghy {
            BoatType::Dinghy
        } else {
            BoatType::Keelboat
        };
        boat.class = trimmed(&self.class);
        boat.year_built = year_built;
        boat.length_m = length;
        boat.beam_m = beam;
        boat.draft_m = draft;
        boat.sail_area_m2 = sail_area;
        boat.berths = berths;
        boat.engine = trimmed(&self.engine);
        boat.mooring = trimmed(&self.mooring);
        boat.skipper_requirement = requirement.unwrap_or_default();
        boat.crew_rules = rules;
        Ok(())
    }
}

impl ClubService {
    async fn load_boats_with(&self, extra: Scope) -> Result<crate::changes::Scoped, AppError> {
        self.load(&Scope {
            boats: BoatScope::All,
            offices: true,
            seasons: true,
            ..extra
        })
        .await
    }

    /// Bootsliste mit Freigabe- und Sperrstatus.
    pub async fn boat_list(&self, user: &CurrentUser) -> Result<BoatList, AppError> {
        let principal = self.principal(user).await?;
        let scoped = self.load_boats_with(Scope::default()).await?;
        let mut boats: Vec<&Boat> = scoped.record.boats.iter().collect();
        boats.sort_by(|a, b| a.name.cmp(&b.name));
        let season = principal.now.year();
        let rows = boats
            .into_iter()
            .map(|b| BoatRow {
                id: b.id.to_string(),
                name: b.name.clone(),
                boat_type: b.boat_type.to_string(),
                type_icon: boat_type_display(b).0,
                type_tip: boat_type_display(b).1,
                class: b.class.clone().unwrap_or_default(),
                released: b.is_released_for(season),
                blocked: b
                    .blocks
                    .iter()
                    .find(|x| x.is_active_at(principal.now))
                    .map(|x| x.reason.clone()),
            })
            .collect();
        Ok(BoatList {
            season,
            rows,
            can_create: principal.can(&Capability::CreateBoat),
        })
    }

    /// Bootsseite mit Stammdaten, Ämtern, Sperren und eigener Eignung.
    pub async fn boat_page(&self, user: &CurrentUser, id: BoatId) -> Result<BoatPage, AppError> {
        let principal = self.principal(user).await?;
        let scoped = self
            .load_boats_with(Scope {
                members: MemberScope::Some(vec![user.member]),
                ..Scope::default()
            })
            .await?;
        let record = &scoped.record;
        let boat = record
            .boat(id)
            .ok_or_else(|| AppError::NotFound("Boot".into()))?;
        let names = self.store.active_members(principal.now).await?;
        let name_of = |m: MemberId| {
            names
                .iter()
                .find(|x| x.id == m)
                .map_or_else(|| format!("Mitglied {m}"), |x| x.full_name())
        };
        let offices = Offices::new(&record.offices);
        let holders = |role: Role| -> Vec<String> {
            offices
                .holders(role, principal.now)
                .into_iter()
                .map(&name_of)
                .collect()
        };
        let mut facts: Vec<(&'static str, String)> = vec![("Typ", boat.boat_type.to_string())];
        let mut push = |label: &'static str, value: Option<String>| {
            if let Some(v) = value {
                facts.push((label, v));
            }
        };
        push("Klasse", boat.class.clone());
        push("Baujahr", boat.year_built.map(|y| y.to_string()));
        push(
            "Länge",
            boat.length_m.map(|v| format!("{} m", format::number(v))),
        );
        push(
            "Breite",
            boat.beam_m.map(|v| format!("{} m", format::number(v))),
        );
        push(
            "Tiefgang",
            boat.draft_m.map(|v| format!("{} m", format::number(v))),
        );
        push(
            "Segelfläche",
            boat.sail_area_m2
                .map(|v| format!("{} m²", format::number(v))),
        );
        push("Kojen", boat.berths.map(|n| n.to_string()));
        push("Motor", boat.engine.clone());
        push("Liegeplatz", boat.mooring.clone());
        let eligibility = match record.skipper_eligibility(user.member, id, principal.now) {
            Ok(()) => Eligibility {
                skipper: true,
                reasons: vec![],
            },
            Err(issues) => Eligibility {
                skipper: false,
                reasons: issues.iter().map(ToString::to_string).collect(),
            },
        };
        let members = if principal.can(&Capability::RecordCheckout(id))
            || principal.can(&Capability::Revoke(id))
        {
            names.iter().map(|m| (m.id.get(), m.full_name())).collect()
        } else {
            vec![]
        };
        let season = principal.now.year();
        Ok(BoatPage {
            id: id.to_string(),
            name: boat.name.clone(),
            boat_type: boat.boat_type.to_string(),
            type_icon: boat_type_display(&boat).0,
            type_tip: boat_type_display(&boat).1,
            facts,
            wardens: holders(Role::Boatwarden(id)),
            examiners: holders(Role::Examiner(id)),
            requirement: requirement_text(&boat.skipper_requirement, &record.boats),
            crew_rules: boat
                .crew_rules
                .iter()
                .map(|r| CrewRuleLine {
                    area: r.area.to_string(),
                    text: crew_rule_text(r),
                })
                .collect(),
            released_seasons: boat.releases.iter().map(|r| r.season).collect(),
            season,
            released: boat.is_released_for(season),
            blocks: boat
                .blocks
                .iter()
                .map(|b| BlockLine {
                    period: match b.end {
                        Some(end) => format!("{} bis {}", format::date(b.start), format::date(end)),
                        None => format!("seit {}", format::date(b.start)),
                    },
                    reason: b.reason.clone(),
                    by: name_of(b.by),
                    open: b.end.is_none(),
                })
                .collect(),
            eligibility,
            can_edit: principal.can(&Capability::EditBoat(id)),
            can_block: principal.can(&Capability::BlockBoat(id)),
            can_lift: principal.can(&Capability::LiftBlock(id))
                && boat.blocks.iter().any(|b| b.end.is_none()),
            can_release: principal.can(&Capability::ReleaseBoat(id))
                && !boat.is_released_for(season),
            can_checkout: principal.can(&Capability::RecordCheckout(id)),
            can_revoke: principal.can(&Capability::Revoke(id)),
            members,
        })
    }

    pub async fn boat_form(&self, user: &CurrentUser, id: BoatId) -> Result<BoatForm, AppError> {
        let principal = self.principal(user).await?;
        principal.require(&Capability::EditBoat(id))?;
        let scoped = self.load_boats_with(Scope::default()).await?;
        let boat = scoped
            .record
            .boat(id)
            .ok_or_else(|| AppError::NotFound("Boot".into()))?;
        Ok(BoatForm {
            id: id.to_string(),
            version: scoped.version(RecordKind::Boat, &id.to_string()).0,
            input: BoatInput::from_boat(boat, &scoped.record.boats),
        })
    }

    pub async fn create_boat(
        &self,
        user: &CurrentUser,
        input: &BoatInput,
    ) -> Result<BoatId, AppError> {
        let principal = self.principal(user).await?;
        principal.require(&Capability::CreateBoat)?;
        let scoped = self.load_boats_with(Scope::default()).await?;
        let mut boat = Boat::new(BoatId::new(), input.name.trim(), BoatType::Keelboat);
        input
            .apply(&mut boat, &scoped.record.boats)
            .map_err(AppError::Rejected)?;
        let id = boat.id;
        let entry = audit(
            &principal,
            "boat.create",
            "boat",
            id,
            None,
            format!("Boot {} angelegt", boat.name),
        );
        let mut new = scoped.record.clone();
        new.boats.push(boat);
        self.save(&scoped, &new, vec![entry]).await?;
        Ok(id)
    }

    pub async fn update_boat(
        &self,
        user: &CurrentUser,
        id: BoatId,
        version: i64,
        input: &BoatInput,
    ) -> Result<(), AppError> {
        let principal = self.principal(user).await?;
        principal.require(&Capability::EditBoat(id))?;
        let scoped = self.load_boats_with(Scope::default()).await?;
        if scoped.version(RecordKind::Boat, &id.to_string()) != Version(version) {
            return Err(AppError::Conflict);
        }
        let mut new = scoped.record.clone();
        let all = scoped.record.boats.clone();
        let boat = new
            .boat_mut(id)
            .ok_or_else(|| AppError::NotFound("Boot".into()))?;
        input.apply(boat, &all).map_err(AppError::Rejected)?;
        let entry = audit(
            &principal,
            "boat.edit",
            "boat",
            id,
            None,
            format!("Boot {} geändert", boat.name),
        );
        self.save(&scoped, &new, vec![entry]).await
    }

    /// Sperrt das Boot ab `start` (Standard: jetzt), optional befristet.
    pub async fn block_boat(
        &self,
        user: &CurrentUser,
        id: BoatId,
        reason: &str,
        start: Option<NaiveDate>,
        end: Option<NaiveDate>,
    ) -> Result<(), AppError> {
        let principal = self.principal(user).await?;
        principal.require(&Capability::BlockBoat(id))?;
        if reason.trim().is_empty() {
            return Err(AppError::rejected("reason", "Bitte einen Grund angeben."));
        }
        let start_at = start.map_or(principal.now, midnight);
        let end_at = end.map(midnight);
        if end_at.is_some_and(|e| e <= start_at) {
            return Err(AppError::rejected(
                "end",
                "Das Ende muss nach dem Beginn liegen.",
            ));
        }
        let scoped = self.load_boats_with(Scope::default()).await?;
        let mut new = scoped.record.clone();
        new.block_boat(user.member, id, start_at, end_at, reason.trim())
            .map_err(action_error)?;
        let entry = audit(&principal, "boat.block", "boat", id, None, "Boot gesperrt");
        self.save(&scoped, &new, vec![entry]).await
    }

    /// Hebt die offene Sperre auf.
    pub async fn lift_block(&self, user: &CurrentUser, id: BoatId) -> Result<(), AppError> {
        let principal = self.principal(user).await?;
        principal.require(&Capability::LiftBlock(id))?;
        let scoped = self.load_boats_with(Scope::default()).await?;
        let mut new = scoped.record.clone();
        new.lift_block(user.member, id, principal.now)
            .map_err(action_error)?;
        let entry = audit(
            &principal,
            "boat.unblock",
            "boat",
            id,
            None,
            "Sperre aufgehoben",
        );
        self.save(&scoped, &new, vec![entry]).await
    }

    /// Gibt das Boot für die Saison frei (Standard: laufende Saison).
    pub async fn release_boat(
        &self,
        user: &CurrentUser,
        id: BoatId,
        season: Option<i32>,
    ) -> Result<(), AppError> {
        let principal = self.principal(user).await?;
        principal.require(&Capability::ReleaseBoat(id))?;
        let season = season.unwrap_or_else(|| principal.now.year());
        let scoped = self.load_boats_with(Scope::default()).await?;
        if scoped
            .record
            .boat(id)
            .is_some_and(|b| b.is_released_for(season))
        {
            return Err(AppError::rejected(
                "season",
                "Das Boot ist für diese Saison bereits freigegeben.",
            ));
        }
        let mut new = scoped.record.clone();
        new.release_boat(user.member, id, season, principal.now)
            .map_err(action_error)?;
        let entry = audit(
            &principal,
            "boat.release",
            "boat",
            id,
            None,
            format!("Freigabe für Saison {season}"),
        );
        self.save(&scoped, &new, vec![entry]).await
    }

    /// Erfasst eine Bootsabnahme.
    pub async fn record_checkout(
        &self,
        user: &CurrentUser,
        boat: BoatId,
        member: MemberId,
        passed: bool,
        remark: &str,
    ) -> Result<(), AppError> {
        let principal = self.principal(user).await?;
        principal.require(&Capability::RecordCheckout(boat))?;
        let scoped = self
            .load_boats_with(Scope {
                members: MemberScope::Some(vec![member]),
                ..Scope::default()
            })
            .await?;
        if scoped.record.member(member).is_none() {
            return Err(AppError::rejected(
                "member",
                "Dieses Mitglied gibt es nicht.",
            ));
        }
        let mut new = scoped.record.clone();
        let id = new
            .record_checkout(
                user.member,
                member,
                boat,
                principal.now,
                if passed {
                    CheckoutResult::Passed
                } else {
                    CheckoutResult::Failed
                },
                trimmed(remark),
            )
            .map_err(action_error)?;
        let entry = audit(
            &principal,
            "boat.checkout",
            "checkout",
            id,
            Some(member),
            if passed {
                "Bootsabnahme bestanden"
            } else {
                "Bootsabnahme nicht bestanden"
            },
        );
        self.save(&scoped, &new, vec![entry]).await
    }

    /// Widerruft die Bootserlaubnis (Vorstand).
    pub async fn revoke_permit(
        &self,
        user: &CurrentUser,
        boat: BoatId,
        member: MemberId,
        reason: &str,
    ) -> Result<(), AppError> {
        let principal = self.principal(user).await?;
        principal.require(&Capability::Revoke(boat))?;
        if reason.trim().is_empty() {
            return Err(AppError::rejected(
                "reason",
                "Bitte eine Begründung angeben.",
            ));
        }
        let scoped = self
            .load_boats_with(Scope {
                members: MemberScope::Some(vec![member]),
                ..Scope::default()
            })
            .await?;
        if scoped.record.member(member).is_none() {
            return Err(AppError::rejected(
                "member",
                "Dieses Mitglied gibt es nicht.",
            ));
        }
        let mut new = scoped.record.clone();
        let id = new
            .revoke(user.member, member, boat, principal.now, reason.trim())
            .map_err(action_error)?;
        let entry = audit(
            &principal,
            "boat.revoke",
            "revocation",
            id,
            Some(member),
            "Bootserlaubnis widerrufen",
        );
        self.save(&scoped, &new, vec![entry]).await
    }
}

#[cfg(test)]
mod tests {
    use vereinsakte_core::example::{self, BEN, DAVID, EVA, FILOU, FRANK, GRETA, HARALD_BLAUZAHN};

    use super::*;
    use crate::service::tests::service;

    fn user(member: MemberId) -> CurrentUser {
        CurrentUser { member }
    }

    #[test]
    fn boat_input_round_trips_all_example_boats() {
        let club = example::club();
        for boat in &club.boats {
            let mut copy = boat.clone();
            let input = BoatInput::from_boat(boat, &club.boats);
            input.apply(&mut copy, &club.boats).unwrap();
            assert_eq!(&copy, boat, "{}", boat.name);
        }
    }

    #[test]
    fn requirement_text_parses_all_kinds_of_qualifications() {
        let club = example::club();
        let text = "SKS | SSS\nFunk:SRC\nBoot:Filou | Typ:Jolle";
        let requirement = parse_requirement(text, &club.boats).unwrap();
        assert_eq!(requirement.0.len(), 3);
        assert_eq!(requirement_text(&requirement, &club.boats), text);
        assert!(parse_requirement("Boot:Gibtsnicht", &club.boats).is_err());
        assert!(parse_requirement("Typ:Floß", &club.boats).is_err());
    }

    #[tokio::test]
    async fn list_and_page_show_status_and_own_eligibility() {
        let (service, _) = service();
        let list = service.boat_list(&user(GRETA)).await.unwrap();
        assert_eq!(list.season, 2026);
        assert!(list.rows.iter().all(|r| r.released && r.blocked.is_none()));
        assert!(!list.can_create);
        let page = service.boat_page(&user(GRETA), FILOU).await.unwrap();
        assert_eq!(page.name, "Filou");
        assert!(page.wardens.contains(&"David Schulz".to_string()));
        assert!(page.examiners.contains(&"David Schulz".to_string()));
        assert!(!page.eligibility.skipper && !page.eligibility.reasons.is_empty());
        assert!(!page.can_edit && !page.can_block && !page.can_checkout && !page.can_revoke);
        let david = service.boat_page(&user(DAVID), FILOU).await.unwrap();
        assert!(david.can_edit && david.can_block && david.can_checkout && !david.can_revoke);
    }

    #[tokio::test]
    async fn warden_blocks_own_boat_only_and_lifts_again() {
        let (service, store) = service();
        assert_eq!(
            service
                .block_boat(&user(BEN), FILOU, "Riss im Segel", None, None)
                .await,
            Err(AppError::Forbidden)
        );
        service
            .block_boat(&user(BEN), HARALD_BLAUZAHN, "Mast defekt", None, None)
            .await
            .unwrap();
        let list = service.boat_list(&user(GRETA)).await.unwrap();
        let row = list
            .rows
            .iter()
            .find(|r| r.id == HARALD_BLAUZAHN.to_string())
            .unwrap();
        assert_eq!(row.blocked.as_deref(), Some("Mast defekt"));
        let page = service
            .boat_page(&user(BEN), HARALD_BLAUZAHN)
            .await
            .unwrap();
        assert!(page.blocks[0].open && page.blocks[0].by == "Ben Hansen" && page.can_lift);
        service
            .lift_block(&user(BEN), HARALD_BLAUZAHN)
            .await
            .unwrap();
        let again = service.lift_block(&user(BEN), HARALD_BLAUZAHN).await;
        assert!(
            matches!(again, Err(AppError::Rejected(_))),
            "keine offene Sperre"
        );
        assert_eq!(store.audit_entries().len(), 2);
        let empty = service
            .block_boat(&user(BEN), HARALD_BLAUZAHN, " ", None, None)
            .await;
        assert!(matches!(empty, Err(AppError::Rejected(_))));
    }

    #[tokio::test]
    async fn release_for_a_new_season_and_not_twice() {
        let (service, store) = service();
        let twice = service.release_boat(&user(FRANK), FILOU, None).await;
        assert!(matches!(twice, Err(AppError::Rejected(_))));
        service
            .release_boat(&user(FRANK), FILOU, Some(2027))
            .await
            .unwrap();
        assert!(store.snapshot().boat(FILOU).unwrap().is_released_for(2027));
        assert_eq!(
            service.release_boat(&user(GRETA), FILOU, Some(2028)).await,
            Err(AppError::Forbidden)
        );
    }

    #[tokio::test]
    async fn checkout_grants_and_revocation_removes_the_boat_permit() {
        let (service, store) = service();
        let reasons = |p: crate::boats::BoatPage| p.eligibility.reasons.join(" ");
        assert!(
            reasons(service.boat_page(&user(GRETA), FILOU).await.unwrap()).contains("Bootsabnahme")
        );
        assert_eq!(
            service
                .record_checkout(&user(GRETA), FILOU, GRETA, true, "")
                .await,
            Err(AppError::Forbidden)
        );
        service
            .record_checkout(&user(DAVID), FILOU, GRETA, true, "Probefahrt")
            .await
            .unwrap();
        assert!(
            !reasons(service.boat_page(&user(GRETA), FILOU).await.unwrap())
                .contains("keine bestandene Bootsabnahme")
        );
        assert_eq!(
            service.revoke_permit(&user(DAVID), FILOU, GRETA, "x").await,
            Err(AppError::Forbidden)
        );
        let empty = service.revoke_permit(&user(EVA), FILOU, GRETA, "").await;
        assert!(matches!(empty, Err(AppError::Rejected(_))));
        service
            .revoke_permit(&user(EVA), FILOU, GRETA, "Unfall")
            .await
            .unwrap();
        assert!(
            reasons(service.boat_page(&user(GRETA), FILOU).await.unwrap()).contains("widerrufen")
        );
        let log: Vec<_> = store
            .audit_entries()
            .into_iter()
            .map(|e| e.action)
            .collect();
        assert_eq!(log, ["boat.checkout", "boat.revoke"]);
        let subject = store.audit_entries()[0].subject;
        assert_eq!(subject, Some(GRETA));
    }

    #[tokio::test]
    async fn create_and_update_boats_with_validation_and_conflicts() {
        let (service, _) = service();
        let input = BoatInput {
            name: "Neptun".into(),
            dinghy: true,
            length: "4,5".into(),
            skipper_requirement: "SKS | SSS".into(),
            home_min_members: "1".into(),
            home_min_persons: "2".into(),
            ..Default::default()
        };
        assert_eq!(
            service.create_boat(&user(GRETA), &input).await,
            Err(AppError::Forbidden)
        );
        let id = service.create_boat(&user(FRANK), &input).await.unwrap();
        let form = service.boat_form(&user(FRANK), id).await.unwrap();
        assert_eq!(form.input.length, "4,5");
        let duplicate = service.create_boat(&user(FRANK), &input).await;
        let Err(AppError::Rejected(e)) = duplicate else {
            panic!()
        };
        assert_eq!(e[0].field, "name");
        let mut bad = input.clone();
        bad.name = "Andere".into();
        bad.length = "abc".into();
        bad.home_min_members = "3".into();
        bad.home_min_persons = "2".into();
        let Err(AppError::Rejected(errors)) = service.create_boat(&user(FRANK), &bad).await else {
            panic!()
        };
        let fields: Vec<_> = errors.iter().map(|e| e.field.as_str()).collect();
        assert!(fields.contains(&"length") && fields.contains(&"home_min_persons"));
        let mut changed = form.input.clone();
        changed.class = "Pirat".into();
        service
            .update_boat(&user(FRANK), id, form.version, &changed)
            .await
            .unwrap();
        let stale = service
            .update_boat(&user(FRANK), id, form.version, &changed)
            .await;
        assert_eq!(stale, Err(AppError::Conflict));
        let foreign = service
            .update_boat(&user(DAVID), id, form.version + 1, &changed)
            .await;
        assert_eq!(
            foreign,
            Err(AppError::Forbidden),
            "DAVID ist nicht Bootswart der Neptun"
        );
    }
}
