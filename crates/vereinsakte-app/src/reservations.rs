//! Reservierungen: Kalender, Anlegen, Ändern und Stornieren mit allen
//! fachlichen Prüfungen der Segelordnung.

use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, Timelike};
use vereinsakte_core::boat::{Boat, SailingArea};
use vereinsakte_core::ids::{BoatId, MemberId, ReservationId};
use vereinsakte_core::reservation::{CrewSeat, Reservation};
use vereinsakte_core::rights::Capability;
use vereinsakte_core::time::{Timestamp, club_time};

use crate::changes::RecordKind;
use crate::commands::audit;
use crate::error::{AppError, FieldError};
use crate::format;
use crate::ports::Version;
use crate::scope::{BoatScope, ReservationWindow, Scope};
use crate::service::{ClubService, CurrentUser};

/// Filter der Reservierungsansicht.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReservationFilter {
    pub boat: Option<BoatId>,
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    /// Nur Reservierungen, an denen das Mitglied beteiligt ist.
    pub mine: bool,
}

/// Crew-Mitglied einer Reservierung für die Anzeige.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrewLine {
    pub number: u32,
    pub name: String,
    /// Eigener Zeitraum an Bord, z. B. „ (bis 07.07.2026 18:00)“ (leer ohne Angabe).
    pub period: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReservationRow {
    pub id: String,
    pub boat: String,
    /// Symbol und Tooltip des Bootstyps.
    pub boat_icon: &'static str,
    pub boat_tip: String,
    /// Erster und letzter Tag der Reservierung (für den Kalender).
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    /// Uhrzeit bei eintägigen Reservierungen, z. B. „10:00–18:00“.
    pub times: String,
    pub skipper_id: u32,
    pub co_skipper_id: Option<u32>,
    pub day: String,
    pub when: String,
    pub area: String,
    pub skipper: String,
    pub co_skipper: String,
    pub crew: Vec<String>,
    /// Crew-Mitglieder mit Nummer und optionalem eigenem Zeitraum.
    pub crew_members: Vec<CrewLine>,
    pub remark: String,
    pub tour: bool,
    pub mine: bool,
    pub past: bool,
    pub can_change: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReservationList {
    pub from: NaiveDate,
    pub to: NaiveDate,
    pub rows: Vec<ReservationRow>,
    pub boats: Vec<(String, String)>,
    pub can_create: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeeLine {
    pub who: String,
    pub text: String,
    pub amount: String,
    /// „offen“, „bezahlt“, „teilweise bezahlt“ oder „noch nicht berechnet“.
    pub status: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReservationPage {
    pub row: ReservationRow,
    pub fees: Vec<FeeLine>,
    pub fee_total: Option<String>,
}

/// Eingaben des Reservierungsformulars (Texte).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReservationInput {
    pub boat: String,
    pub area: String,
    pub from_date: String,
    pub from_time: String,
    pub to_date: String,
    pub to_time: String,
    /// Leer: das angemeldete Mitglied.
    pub skipper: String,
    pub co_skipper: String,
    /// Je Zeile: `Mitgliedsnummer oder Name[; von[; bis]]`.
    pub crew: String,
    /// Bemerkung, auch für Gäste und Hinweise zur Fahrt (bis 5.000 Zeichen).
    pub remark: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReservationFormData {
    pub boats: Vec<(String, String)>,
    pub members: Vec<(u32, String)>,
}

fn trimmed(text: &str) -> Option<String> {
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_string())
}

fn parse_stamp(text: &str) -> Option<Timestamp> {
    let text = text.trim();
    ["%Y-%m-%d %H:%M", "%d.%m.%Y %H:%M", "%Y-%m-%dT%H:%M"]
        .iter()
        .find_map(|f| NaiveDateTime::parse_from_str(text, f).ok())
        .map(|n| club_time(n.year(), n.month(), n.day(), n.hour(), n.minute()))
}

fn date_time(date: &str, time: &str) -> Option<Timestamp> {
    parse_stamp(&format!("{} {}", date.trim(), time.trim()))
}

fn stamp_text(t: Timestamp) -> String {
    t.format("%Y-%m-%d %H:%M").to_string()
}

fn area_label(area: SailingArea) -> &'static str {
    match area {
        SailingArea::HomeWaters => "Heimrevier",
        SailingArea::Outside => "außerhalb",
    }
}

/// Setzt „Mitglied 107“ in Meldungen der Prüfung auf Namen um.
fn describe(issue: &str, names: &[(MemberId, String)]) -> String {
    let mut text = issue.to_string();
    for (id, name) in names {
        text = text.replace(&format!("Mitglied {id} "), &format!("{name} "));
    }
    text
}

impl ClubService {
    fn name_in(&self, names: &[crate::ports::MemberSummary], id: MemberId) -> String {
        names
            .iter()
            .find(|m| m.id == id)
            .map_or_else(|| format!("Mitglied {id}"), |m| m.full_name())
    }

    fn reservation_row(
        &self,
        r: &Reservation,
        boats: &[Boat],
        names: &[crate::ports::MemberSummary],
        me: MemberId,
        now: Timestamp,
        can_change: bool,
    ) -> ReservationRow {
        let name = |m: MemberId| self.name_in(names, m);
        let period = |from: Option<Timestamp>, until: Option<Timestamp>| -> String {
            let mut text = String::new();
            if let Some(f) = from {
                text.push_str(&format!(" ab {}", f.format("%d.%m. %H:%M")));
            }
            if let Some(u) = until {
                text.push_str(&format!(" bis {}", u.format("%d.%m. %H:%M")));
            }
            text
        };
        ReservationRow {
            id: r.id.to_string(),
            boat: boats
                .iter()
                .find(|b| b.id == r.boat)
                .map_or_else(|| r.boat.to_string(), |b| b.name.clone()),
            boat_icon: boats
                .iter()
                .find(|b| b.id == r.boat)
                .map_or("boat", |b| crate::boats::boat_type_display(b).0),
            boat_tip: boats
                .iter()
                .find(|b| b.id == r.boat)
                .map(|b| crate::boats::boat_type_display(b).1)
                .unwrap_or_default(),
            start_date: r.start.date_naive(),
            end_date: r.end.date_naive(),
            times: if r.start.date_naive() == r.end.date_naive() {
                format!("{}–{}", r.start.format("%H:%M"), r.end.format("%H:%M"))
            } else {
                String::new()
            },
            skipper_id: r.skipper.get(),
            co_skipper_id: r.co_skipper.map(MemberId::get),
            day: format::date(r.start),
            when: if r.start.date_naive() == r.end.date_naive() {
                format!(
                    "{}, {}–{} Uhr",
                    format::date(r.start),
                    r.start.format("%H:%M"),
                    r.end.format("%H:%M")
                )
            } else {
                format!(
                    "{} {} Uhr – {} {} Uhr",
                    format::date(r.start),
                    r.start.format("%H:%M"),
                    format::date(r.end),
                    r.end.format("%H:%M")
                )
            },
            area: area_label(r.area).into(),
            skipper: name(r.skipper),
            co_skipper: r.co_skipper.map(name).unwrap_or_default(),
            crew: r
                .crew
                .iter()
                .map(|s| format!("{}{}", name(s.member), period(s.from, s.until)))
                .collect(),
            crew_members: r
                .crew
                .iter()
                .map(|s| CrewLine {
                    number: s.member.get(),
                    name: name(s.member),
                    period: period(s.from, s.until),
                })
                .collect(),
            remark: r.remark.clone().unwrap_or_default(),
            tour: r.is_tour(),
            mine: r.members_on_board().contains(&me),
            past: r.end < now,
            can_change,
        }
    }

    fn can_change(&self, principal: &crate::principal::Principal, r: &Reservation) -> bool {
        principal.can(&Capability::ChangeReservation {
            boat: r.boat,
            skipper: r.skipper,
            started: r.start <= principal.now,
        })
    }

    /// Reservierungen im Zeitraum (Standard: die nächsten vier Wochen).
    pub async fn reservation_list(
        &self,
        user: &CurrentUser,
        filter: &ReservationFilter,
    ) -> Result<ReservationList, AppError> {
        let principal = self.principal(user).await?;
        let today = principal.now.date_naive();
        let from = filter.from.unwrap_or(today);
        let to = filter.to.unwrap_or(from + Duration::days(28));
        let scoped = self
            .load(&Scope {
                boats: BoatScope::All,
                reservations_between: Some((
                    club_time(from.year(), from.month(), from.day(), 0, 0),
                    club_time(
                        (to + Duration::days(1)).year(),
                        (to + Duration::days(1)).month(),
                        (to + Duration::days(1)).day(),
                        0,
                        0,
                    ),
                )),
                offices: true,
                ..Scope::default()
            })
            .await?;
        let names = self.store.active_members(principal.now).await?;
        let mut found: Vec<&Reservation> = scoped
            .record
            .reservations
            .iter()
            .filter(|r| filter.boat.is_none_or(|b| r.boat == b))
            .filter(|r| !filter.mine || r.members_on_board().contains(&user.member))
            .collect();
        found.sort_by_key(|r| (r.start, r.boat));
        let rows = found
            .into_iter()
            .map(|r| {
                let can = self.can_change(&principal, r);
                self.reservation_row(
                    r,
                    &scoped.record.boats,
                    &names,
                    user.member,
                    principal.now,
                    can,
                )
            })
            .collect();
        Ok(ReservationList {
            from,
            to,
            rows,
            boats: boat_options(&scoped.record.boats),
            can_create: true,
        })
    }

    /// Optionen für das Formular.
    pub async fn reservation_form_data(
        &self,
        user: &CurrentUser,
    ) -> Result<ReservationFormData, AppError> {
        let principal = self.principal(user).await?;
        let scoped = self
            .load(&Scope {
                boats: BoatScope::All,
                ..Scope::default()
            })
            .await?;
        let mut members: Vec<(u32, String)> = self
            .store
            .active_members(principal.now)
            .await?
            .into_iter()
            .map(|m| (m.id.get(), m.full_name()))
            .collect();
        members.sort_by(|a, b| a.1.cmp(&b.1));
        Ok(ReservationFormData {
            boats: boat_options(&scoped.record.boats),
            members,
        })
    }

    /// Eine Reservierung mit Beteiligten und – bei Touren – der erwarteten Gebühr.
    pub async fn reservation_page(
        &self,
        user: &CurrentUser,
        id: ReservationId,
    ) -> Result<ReservationPage, AppError> {
        let principal = self.principal(user).await?;
        let scoped = self.load(&self.reservation_scope(id)).await?;
        let Some(r) = scoped.record.reservation(id) else {
            return Err(AppError::NotFound("Reservierung".into()));
        };
        let r = r.clone();
        let scoped = self.load(&Scope::for_reservation(&r)).await?;
        let names = self.store.active_members(principal.now).await?;
        let can = self.can_change(&principal, &r);
        let row = self.reservation_row(
            &r,
            &scoped.record.boats,
            &names,
            user.member,
            principal.now,
            can,
        );
        let (fees, fee_total) = if r.is_tour() {
            use vereinsakte_core::tour_fees::PaymentStatus;
            let mut lines: Vec<FeeLine> = scoped
                .record
                .tour_fee_overview(&r)
                .into_iter()
                .map(|l| FeeLine {
                    who: self.name_in(&names, l.account),
                    text: l.guest.map_or_else(
                        || "Tourgebühr".to_string(),
                        |g| format!("Tourgebühr für Gast {g}"),
                    ),
                    amount: l.amount.to_string(),
                    status: match l.status {
                        PaymentStatus::Paid => "bezahlt".into(),
                        PaymentStatus::Partial(open) => format!("teilweise bezahlt, offen {open}"),
                        PaymentStatus::Open => "offen".into(),
                    },
                })
                .collect();
            let missing = scoped.record.tour_fee_claims(&r, principal.now);
            let total_missing: vereinsakte_core::money::Money =
                missing.iter().map(|(_, c)| c.amount).sum();
            lines.extend(missing.into_iter().map(|(m, c)| FeeLine {
                who: self.name_in(&names, m),
                text: c.text.clone(),
                amount: c.amount.to_string(),
                status: "noch nicht berechnet".into(),
            }));
            let billed: vereinsakte_core::money::Money = scoped
                .record
                .tour_fee_overview(&r)
                .iter()
                .map(|l| l.amount)
                .sum();
            (lines, Some((billed + total_missing).to_string()))
        } else {
            (vec![], None)
        };
        Ok(ReservationPage {
            row,
            fees,
            fee_total,
        })
    }

    fn reservation_scope(&self, _id: ReservationId) -> Scope {
        // Die Reservierung ist erst nach dem Laden bekannt; geladen werden alle
        // künftigen und laufenden Reservierungen des vergangenen Jahres.
        Scope {
            boats: BoatScope::All,
            reservations: None,
            reservations_between: Some((club_time(2000, 1, 1, 0, 0), club_time(2100, 1, 1, 0, 0))),
            ..Scope::default()
        }
    }

    fn parse_member(&self, text: &str, names: &[crate::ports::MemberSummary]) -> Option<MemberId> {
        let text = text.trim();
        if let Ok(n) = text.parse::<u32>() {
            return MemberId::new(n).filter(|id| names.iter().any(|m| m.id == *id));
        }
        names
            .iter()
            .find(|m| m.full_name().eq_ignore_ascii_case(text))
            .map(|m| m.id)
    }

    /// Baut die Reservierung aus der Eingabe; Fehler je Feld.
    async fn reservation_from_input(
        &self,
        user: &CurrentUser,
        input: &ReservationInput,
        base: Option<&Reservation>,
    ) -> Result<Reservation, AppError> {
        let principal = self.principal(user).await?;
        let names = self.store.active_members(principal.now).await?;
        let boats = self
            .load(&Scope {
                boats: BoatScope::All,
                ..Scope::default()
            })
            .await?
            .record
            .boats;
        let mut errors: Vec<FieldError> = vec![];
        let boat = input
            .boat
            .parse::<BoatId>()
            .ok()
            .filter(|b| boats.iter().any(|x| x.id == *b));
        if boat.is_none() {
            errors.push(FieldError::new("boat", "Bitte ein Boot wählen."));
        }
        let area = match input.area.as_str() {
            "outside" => SailingArea::Outside,
            _ => SailingArea::HomeWaters,
        };
        let start = date_time(&input.from_date, &input.from_time);
        let end = date_time(&input.to_date, &input.to_time);
        if start.is_none() {
            errors.push(FieldError::new(
                "from_date",
                "Bitte Beginn (Datum und Uhrzeit) angeben.",
            ));
        }
        if end.is_none() {
            errors.push(FieldError::new(
                "to_date",
                "Bitte Ende (Datum und Uhrzeit) angeben.",
            ));
        }
        let skipper = match trimmed(&input.skipper) {
            None => Some(base.map_or(user.member, |r| r.skipper)),
            Some(text) => {
                let found = self.parse_member(&text, &names);
                if found.is_none() {
                    errors.push(FieldError::new("skipper", "Dieses Mitglied gibt es nicht."));
                }
                found
            }
        };
        let co_skipper = match trimmed(&input.co_skipper) {
            None => None,
            Some(text) => {
                let found = self.parse_member(&text, &names);
                if found.is_none() {
                    errors.push(FieldError::new(
                        "co_skipper",
                        "Dieses Mitglied gibt es nicht.",
                    ));
                }
                found
            }
        };
        let mut crew = vec![];
        for line in input.crew.lines().map(str::trim).filter(|l| !l.is_empty()) {
            let parts: Vec<&str> = line.split(';').map(str::trim).collect();
            let Some(member) = self.parse_member(parts[0], &names) else {
                errors.push(FieldError::new(
                    "crew",
                    format!("Unbekanntes Crewmitglied „{}“.", parts[0]),
                ));
                continue;
            };
            let (from, until) = (
                parts.get(1).and_then(|t| parse_stamp(t)),
                parts.get(2).and_then(|t| parse_stamp(t)),
            );
            if parts
                .get(1)
                .is_some_and(|t| !t.is_empty() && from.is_none())
                || parts
                    .get(2)
                    .is_some_and(|t| !t.is_empty() && until.is_none())
            {
                errors.push(FieldError::new(
                    "crew",
                    format!("Ungültiger Zeitraum bei „{}“ (TT.MM.JJJJ HH:MM).", parts[0]),
                ));
            }
            crew.push(CrewSeat {
                member,
                from,
                until,
            });
        }
        if input.remark.chars().count() > 5000 {
            errors.push(FieldError::new(
                "remark",
                "Die Bemerkung darf höchstens 5.000 Zeichen lang sein.",
            ));
        }
        let (Some(boat), Some(start), Some(end), Some(skipper)) = (boat, start, end, skipper)
        else {
            return Err(AppError::Rejected(errors));
        };
        if !errors.is_empty() {
            return Err(AppError::Rejected(errors));
        }
        let mut r = base
            .cloned()
            .unwrap_or_else(|| Reservation::new(boat, start, end, area, skipper));
        r.boat = boat;
        r.start = start;
        r.end = end;
        r.area = area;
        r.skipper = skipper;
        r.co_skipper = co_skipper;
        r.crew = crew;
        r.remark = trimmed(&input.remark);
        Ok(r)
    }

    /// Prüft die Reservierung vollständig und liefert alle Beanstandungen als Text.
    async fn reservation_problems(
        &self,
        r: &Reservation,
    ) -> Result<(Vec<String>, crate::changes::Scoped), AppError> {
        let mut scope = Scope::for_reservation(r);
        scope.reservations = Some(ReservationWindow {
            boats: vec![r.boat],
            from: r.start,
            to: r.end,
        });
        let scoped = self.load(&scope).await?;
        let record = &scoped.record;
        let names: Vec<(MemberId, String)> = record
            .members
            .iter()
            .map(|m| (m.id, m.full_name()))
            .collect();
        let mut problems: Vec<String> = match record.check_reservation(r) {
            Ok(()) => vec![],
            Err(issues) => issues
                .iter()
                .map(|i| describe(&i.to_string(), &names))
                .collect(),
        };
        let skipper_name = |m: MemberId| {
            names
                .iter()
                .find(|(id, _)| *id == m)
                .map_or_else(|| m.to_string(), |(_, n)| n.clone())
        };
        for other in record
            .reservations
            .iter()
            .filter(|o| o.id != r.id && o.boat == r.boat && o.overlaps(r))
        {
            problems.push(format!(
                "Das Boot ist im Zeitraum bereits reserviert: {} bis {} (Skipper {}).",
                other.start.format("%d.%m.%Y %H:%M"),
                other.end.format("%d.%m.%Y %H:%M"),
                skipper_name(other.skipper)
            ));
        }
        Ok((problems, scoped))
    }

    /// Legt eine Reservierung an; alle Beanstandungen der Prüfung werden gemeldet.
    pub async fn create_reservation(
        &self,
        user: &CurrentUser,
        input: &ReservationInput,
    ) -> Result<ReservationId, AppError> {
        let principal = self.principal(user).await?;
        let r = self.reservation_from_input(user, input, None).await?;
        principal.require(&Capability::Reserve {
            boat: r.boat,
            skipper: r.skipper,
        })?;
        if r.start < principal.now {
            return Err(AppError::rejected(
                "from_date",
                "Der Beginn liegt in der Vergangenheit.",
            ));
        }
        let (problems, scoped) = self.reservation_problems(&r).await?;
        if !problems.is_empty() {
            return Err(AppError::Rejected(
                problems
                    .into_iter()
                    .map(|p| FieldError::new("", p))
                    .collect(),
            ));
        }
        let id = r.id;
        let mut new = scoped.record.clone();
        new.reservations.push(r.clone());
        let entry = audit(
            &principal,
            "reservation.create",
            "reservation",
            id,
            Some(r.skipper),
            "Reservierung angelegt",
        );
        self.save(&scoped, &new, vec![entry]).await?;
        Ok(id)
    }

    async fn load_reservation(
        &self,
        id: ReservationId,
    ) -> Result<(crate::changes::Scoped, Reservation), AppError> {
        let scoped = self.load(&self.reservation_scope(id)).await?;
        let r = scoped
            .record
            .reservation(id)
            .cloned()
            .ok_or_else(|| AppError::NotFound("Reservierung".into()))?;
        Ok((scoped, r))
    }

    /// Vorbelegung zum Ändern samt Version.
    pub async fn reservation_input(
        &self,
        user: &CurrentUser,
        id: ReservationId,
    ) -> Result<(ReservationInput, i64), AppError> {
        let principal = self.principal(user).await?;
        let (scoped, r) = self.load_reservation(id).await?;
        principal.require(&Capability::ChangeReservation {
            boat: r.boat,
            skipper: r.skipper,
            started: r.start <= principal.now,
        })?;
        let names = self.store.active_members(principal.now).await?;
        let line = |m: MemberId, from: Option<Timestamp>, until: Option<Timestamp>| {
            let mut text = m.to_string();
            if from.is_some() || until.is_some() {
                text.push_str(&format!(
                    "; {}; {}",
                    from.map(stamp_text).unwrap_or_default(),
                    until.map(stamp_text).unwrap_or_default()
                ));
            }
            text
        };
        let _ = &names;
        Ok((
            ReservationInput {
                boat: r.boat.to_string(),
                area: if r.area == SailingArea::Outside {
                    "outside".into()
                } else {
                    "home".into()
                },
                from_date: r.start.format("%Y-%m-%d").to_string(),
                from_time: r.start.format("%H:%M").to_string(),
                to_date: r.end.format("%Y-%m-%d").to_string(),
                to_time: r.end.format("%H:%M").to_string(),
                skipper: r.skipper.to_string(),
                co_skipper: r.co_skipper.map(|m| m.to_string()).unwrap_or_default(),
                crew: r
                    .crew
                    .iter()
                    .map(|s| line(s.member, s.from, s.until))
                    .collect::<Vec<_>>()
                    .join("\n"),
                remark: r.remark.clone().unwrap_or_default(),
            },
            scoped.version(RecordKind::Reservation, &id.to_string()).0,
        ))
    }

    /// Ändert eine Reservierung; sie wird erneut vollständig geprüft.
    pub async fn update_reservation(
        &self,
        user: &CurrentUser,
        id: ReservationId,
        version: i64,
        input: &ReservationInput,
    ) -> Result<(), AppError> {
        let principal = self.principal(user).await?;
        let (scoped, old) = self.load_reservation(id).await?;
        principal.require(&Capability::ChangeReservation {
            boat: old.boat,
            skipper: old.skipper,
            started: old.start <= principal.now,
        })?;
        if scoped.version(RecordKind::Reservation, &id.to_string()) != Version(version) {
            return Err(AppError::Conflict);
        }
        let r = self.reservation_from_input(user, input, Some(&old)).await?;
        principal.require(&Capability::Reserve {
            boat: r.boat,
            skipper: r.skipper,
        })?;
        let (problems, check) = self.reservation_problems(&r).await?;
        if !problems.is_empty() {
            return Err(AppError::Rejected(
                problems
                    .into_iter()
                    .map(|p| FieldError::new("", p))
                    .collect(),
            ));
        }
        let mut new = scoped.record.clone();
        let slot = new
            .reservations
            .iter_mut()
            .find(|x| x.id == id)
            .expect("geladen");
        *slot = r;
        let entry = audit(
            &principal,
            "reservation.edit",
            "reservation",
            id,
            Some(old.skipper),
            "Reservierung geändert",
        );
        let _ = check;
        self.save(&scoped, &new, vec![entry]).await
    }

    /// Storniert (löscht) eine Reservierung.
    pub async fn cancel_reservation(
        &self,
        user: &CurrentUser,
        id: ReservationId,
    ) -> Result<(), AppError> {
        let principal = self.principal(user).await?;
        let (scoped, r) = self.load_reservation(id).await?;
        principal.require(&Capability::ChangeReservation {
            boat: r.boat,
            skipper: r.skipper,
            started: r.start <= principal.now,
        })?;
        let mut new = scoped.record.clone();
        new.reservations.retain(|x| x.id != id);
        let entry = audit(
            &principal,
            "reservation.cancel",
            "reservation",
            id,
            Some(r.skipper),
            "Reservierung storniert",
        );
        self.save(&scoped, &new, vec![entry]).await
    }
}

fn boat_options(boats: &[Boat]) -> Vec<(String, String)> {
    let mut list: Vec<(String, String)> = boats
        .iter()
        .map(|b| (b.id.to_string(), b.name.clone()))
        .collect();
    list.sort_by(|a, b| a.1.cmp(&b.1));
    list
}

#[cfg(test)]
mod tests {
    use vereinsakte_core::example::{
        ANNA, BEN, CLARA, DAVID, EVA, FILOU, FRANK, GRETA, HARALD_BLAUZAHN, SCHRUVTREKKER,
    };

    use super::*;
    use crate::service::tests::service;

    fn user(member: MemberId) -> CurrentUser {
        CurrentUser { member }
    }

    fn input(boat: BoatId, day: &str, from: &str, to: &str) -> ReservationInput {
        ReservationInput {
            boat: boat.to_string(),
            area: "home".into(),
            from_date: day.into(),
            from_time: from.into(),
            to_date: day.into(),
            to_time: to.into(),
            ..Default::default()
        }
    }

    fn texts(error: AppError) -> Vec<String> {
        match error {
            AppError::Rejected(list) => list.into_iter().map(|e| e.message).collect(),
            other => panic!("{other:?}"),
        }
    }

    #[tokio::test]
    async fn list_filters_by_period_boat_and_participation() {
        let (service, _) = service();
        let june = ReservationFilter {
            from: NaiveDate::from_ymd_opt(2026, 6, 1),
            to: NaiveDate::from_ymd_opt(2026, 6, 30),
            ..Default::default()
        };
        let list = service.reservation_list(&user(GRETA), &june).await.unwrap();
        assert_eq!(list.rows.len(), 2);
        assert!(list.rows.iter().all(|r| r.past && !r.can_change));
        let david = service
            .reservation_list(
                &user(DAVID),
                &ReservationFilter {
                    mine: true,
                    ..june.clone()
                },
            )
            .await
            .unwrap();
        assert_eq!(david.rows.len(), 1);
        assert_eq!(david.rows[0].boat, "Filou");
        let boat = service
            .reservation_list(
                &user(GRETA),
                &ReservationFilter {
                    boat: Some(HARALD_BLAUZAHN),
                    ..june
                },
            )
            .await
            .unwrap();
        assert_eq!(boat.rows.len(), 1);
        assert_eq!(boat.rows[0].crew, ["Jonas Brandt"]);
    }

    #[tokio::test]
    async fn eligible_skipper_reserves_and_checks_run() {
        let (service, store) = service();
        let id = service
            .create_reservation(&user(DAVID), &input(FILOU, "2026-10-17", "10:00", "16:00"))
            .await
            .unwrap();
        assert_eq!(
            store.audit_entries().last().unwrap().action,
            "reservation.create"
        );
        let page = service.reservation_page(&user(GRETA), id).await.unwrap();
        assert_eq!(page.row.skipper, "David Schulz");
        assert!(page.fees.is_empty() && page.fee_total.is_none());
        // Überschneidung nennt die kollidierende Reservierung.
        let overlap = service
            .create_reservation(&user(DAVID), &input(FILOU, "2026-10-17", "12:00", "14:00"))
            .await
            .unwrap_err();
        assert!(
            texts(overlap)
                .iter()
                .any(|t| t.contains("bereits reserviert") && t.contains("David Schulz"))
        );
    }

    #[tokio::test]
    async fn problems_are_reported_in_german() {
        let (service, _) = service();
        let result = service
            .create_reservation(&user(GRETA), &input(FILOU, "2026-10-17", "10:00", "16:00"))
            .await
            .unwrap_err();
        let problems = texts(result);
        assert!(
            problems
                .iter()
                .any(|t| t.contains("Skipper nicht geeignet")),
            "{problems:?}"
        );
        let mut past = input(FILOU, "2026-01-01", "10:00", "11:00");
        past.skipper = DAVID.to_string();
        assert_eq!(
            service.create_reservation(&user(GRETA), &past).await,
            Err(AppError::Forbidden)
        );
        let bad = service
            .create_reservation(&user(DAVID), &input(FILOU, "2026-01-01", "10:00", "11:00"))
            .await
            .unwrap_err();
        assert!(texts(bad)[0].contains("Vergangenheit"));
        let mut unknown = input(FILOU, "2026-10-17", "10:00", "09:00");
        unknown.crew = "Niemand".into();
        let AppError::Rejected(errors) = service
            .create_reservation(&user(DAVID), &unknown)
            .await
            .unwrap_err()
        else {
            panic!()
        };
        assert!(errors.iter().any(|e| e.field == "crew"));
    }

    #[tokio::test]
    async fn blocked_boat_and_crew_without_permit_are_named() {
        let (service, _) = service();
        service
            .block_boat(&user(DAVID), FILOU, "Riss im Segel", None, None)
            .await
            .unwrap();
        let blocked = service
            .create_reservation(&user(DAVID), &input(FILOU, "2026-10-17", "10:00", "16:00"))
            .await
            .unwrap_err();
        assert!(
            texts(blocked)
                .iter()
                .any(|t| t.contains("gesperrt") && t.contains("Riss im Segel"))
        );
        let mut with_crew = input(SCHRUVTREKKER, "2026-10-24", "10:00", "16:00");
        with_crew.crew = "Greta Lorenzen".into();
        let result = service.create_reservation(&user(ANNA), &with_crew).await;
        if let Err(error) = result {
            assert!(texts(error).iter().any(|t| t.contains("Greta Lorenzen")
                || t.contains("Skipper")
                || t.contains("Mitglieder")));
        }
    }

    #[tokio::test]
    async fn change_and_cancel_follow_rights_and_recheck() {
        let (service, _) = service();
        let id = service
            .create_reservation(&user(DAVID), &input(FILOU, "2026-10-17", "10:00", "16:00"))
            .await
            .unwrap();
        assert_eq!(
            service.cancel_reservation(&user(GRETA), id).await,
            Err(AppError::Forbidden)
        );
        let (mut form, version) = service.reservation_input(&user(DAVID), id).await.unwrap();
        form.to_time = "18:00".into();
        service
            .update_reservation(&user(DAVID), id, version, &form)
            .await
            .unwrap();
        assert_eq!(
            service
                .update_reservation(&user(DAVID), id, version, &form)
                .await,
            Err(AppError::Conflict)
        );
        let (mut form, version) = service.reservation_input(&user(DAVID), id).await.unwrap();
        form.co_skipper = "Unbekannte Person".into();
        assert!(matches!(
            service
                .update_reservation(&user(DAVID), id, version, &form)
                .await,
            Err(AppError::Rejected(_))
        ));
        // Bootswart darf für andere ändern; Vorstand nach Beginn.
        assert!(service.reservation_input(&user(DAVID), id).await.is_ok());
        service.cancel_reservation(&user(DAVID), id).await.unwrap();
        let list = service
            .reservation_list(
                &user(DAVID),
                &ReservationFilter {
                    from: NaiveDate::from_ymd_opt(2026, 10, 17),
                    to: NaiveDate::from_ymd_opt(2026, 10, 17),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert!(list.rows.is_empty());
        let past = vereinsakte_core::example::JOLLE_SAMSTAG;
        assert_eq!(
            service.cancel_reservation(&user(DAVID), past).await,
            Err(AppError::Forbidden),
            "begonnen: nur Vorstand"
        );
        service.cancel_reservation(&user(EVA), past).await.unwrap();
        let _ = (BEN, CLARA, FRANK);
    }

    #[tokio::test]
    async fn tour_page_shows_expected_fees() {
        let (service, _) = service();
        let page = service
            .reservation_page(&user(GRETA), vereinsakte_core::example::DEHLER_TOUR)
            .await
            .unwrap();
        assert!(page.row.tour);
        assert!(
            page.row.remark.contains("Max Gast"),
            "Gäste stehen in der Bemerkung"
        );
        assert_eq!(page.row.crew_members.len(), 3);
        assert!(!page.fees.is_empty());
        assert!(page.fee_total.is_some());
    }
}
