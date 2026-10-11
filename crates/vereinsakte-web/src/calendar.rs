//! Bereich „Kalender“: Reservierungen und Arbeitstermine gemeinsam, als Liste
//! (nach Woche und Tag gruppiert) oder als Kalender (Monat, Woche).

use std::collections::HashMap;

use askama::Template;
use axum::extract::{Query, State};
use axum::response::Response;
use chrono::{Datelike, Duration, NaiveDate, Weekday};
use tower_sessions::Session;
use vereinsakte_app::reservations::{ReservationFilter, ReservationRow};
use vereinsakte_app::work::EventLine;
use vereinsakte_core::ids::BoatId;

use crate::error::WebError;
use crate::pages::{Layout, render};
use crate::session::{RequireUser, encode};
use crate::state::AppState;

/// Standardzeitraum der Liste ab heute.
const LIST_DAYS: i64 = 90;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Reservation,
    Event,
}

/// Ein Eintrag des Kalenders (Reservierung oder Arbeitstermin).
#[derive(Clone, Debug)]
pub struct Entry {
    pub kind: &'static str,
    /// Beschriftung der Art: „Reservierung“ oder „Arbeitstermin“.
    pub label: &'static str,
    pub icon: &'static str,
    pub tip: String,
    pub title: String,
    pub sub: String,
    pub href: String,
    /// „dabei“ bzw. „angemeldet“, leer wenn nicht beteiligt.
    pub mine: &'static str,
    pub tag: String,
    pub tag_kind: &'static str,
    pub tag_icon: &'static str,
    /// Fortsetzung eines mehrtägigen Eintrags: „weiter bis 07.07.“.
    pub cont: String,
    start: NaiveDate,
    end: NaiveDate,
}

impl Entry {
    fn from_reservation(r: &ReservationRow) -> Self {
        let mut sub = r.skipper.clone();
        if !r.times.is_empty() {
            sub = format!("{sub} · {}", r.times);
        }
        let _ = Kind::Reservation;
        Self {
            kind: "reservation",
            label: "Reservierung",
            icon: r.boat_icon,
            tip: r.boat_tip.clone(),
            title: r.boat.clone(),
            sub,
            href: format!("/reservierungen/{}", r.id),
            mine: if r.mine { "dabei" } else { "" },
            tag: if r.tour { "Tour".into() } else { String::new() },
            tag_kind: "info",
            tag_icon: "anchor",
            cont: String::new(),
            start: r.start_date,
            end: r.end_date,
        }
    }

    fn from_event(e: &EventLine) -> Self {
        let (tag, tag_kind, tag_icon) = if e.full {
            ("ausgebucht".to_string(), "warn", "alert")
        } else if let Some(max) = e.max {
            (
                format!(
                    "{} Plätze frei",
                    (max as usize).saturating_sub(e.registered)
                ),
                "info",
                "check",
            )
        } else {
            (String::new(), "info", "check")
        };
        let _ = Kind::Event;
        let mut sub = e.times.clone();
        if !e.location.is_empty() {
            sub = format!("{sub} · {}", e.location);
        }
        Self {
            kind: "event",
            label: "Arbeitstermin",
            icon: "wrench",
            tip: "Arbeitstermin".into(),
            title: e.title.clone(),
            sub,
            href: format!("/arbeit/termine/{}", e.id),
            mine: if e.mine { "angemeldet" } else { "" },
            tag,
            tag_kind,
            tag_icon,
            cont: String::new(),
            start: e.start_date,
            end: e.end_date,
        }
    }

    fn on(&self, day: NaiveDate) -> bool {
        self.start <= day && day <= self.end
    }

    fn for_day(&self, day: NaiveDate) -> Self {
        let mut entry = self.clone();
        if day > self.start {
            entry.cont = format!("weiter bis {}", self.end.format("%d.%m."));
        }
        entry
    }
}

pub struct ListDay {
    pub label: String,
    pub entries: Vec<Entry>,
}

pub struct ListWeek {
    pub title: String,
    pub days: Vec<ListDay>,
}

pub struct CalDay {
    pub label: String,
    pub number: u32,
    pub other_month: bool,
    pub today: bool,
    pub entries: Vec<Entry>,
}

pub struct CalWeek {
    pub days: Vec<CalDay>,
}

const WEEKDAYS: [&str; 7] = ["Mo", "Di", "Mi", "Do", "Fr", "Sa", "So"];
const WEEKDAYS_LONG: [&str; 7] = [
    "Montag",
    "Dienstag",
    "Mittwoch",
    "Donnerstag",
    "Freitag",
    "Samstag",
    "Sonntag",
];
const MONTHS: [&str; 12] = [
    "Januar",
    "Februar",
    "März",
    "April",
    "Mai",
    "Juni",
    "Juli",
    "August",
    "September",
    "Oktober",
    "November",
    "Dezember",
];

fn weekday_index(day: NaiveDate) -> usize {
    day.weekday().num_days_from_monday() as usize
}

fn monday(day: NaiveDate) -> NaiveDate {
    day - Duration::days(day.weekday().num_days_from_monday() as i64)
}

fn day_label(day: NaiveDate) -> String {
    format!(
        "{}, {}. {}",
        WEEKDAYS_LONG[weekday_index(day)],
        day.day(),
        MONTHS[day.month0() as usize]
    )
}

fn short_label(day: NaiveDate) -> String {
    format!("{}, {}", WEEKDAYS[weekday_index(day)], day.format("%d.%m."))
}

#[derive(Template)]
#[template(path = "calendar.html")]
struct CalendarPage {
    layout: Layout,
    view: String,
    kind: String,
    boat: String,
    mine: bool,
    boats: Vec<(String, String)>,
    active_filters: usize,
    /// Verknüpfungen der Ansichtsumschaltung und des Blätterns.
    switch: Vec<(String, String, bool)>,
    title: String,
    previous: String,
    next: String,
    today_link: String,
    weeks: Vec<ListWeek>,
    grid: Vec<CalWeek>,
    weekdays: [&'static str; 7],
    empty: bool,
    can_create_event: bool,
    archive_hint: bool,
}

/// Filter und Ansicht aus der Adresse; Grundlage der Verknüpfungen.
struct Params {
    view: String,
    kind: String,
    boat: String,
    mine: bool,
    month: Option<NaiveDate>,
    week: Option<NaiveDate>,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
}

impl Params {
    fn from_query(query: &HashMap<String, String>) -> Self {
        let get = |k: &str| query.get(k).map(String::as_str).unwrap_or("");
        let date = |k: &str| NaiveDate::parse_from_str(get(k), "%Y-%m-%d").ok();
        let view = match get("ansicht") {
            "monat" => "monat",
            "woche" => "woche",
            _ => "liste",
        };
        let kind = match get("art") {
            "reservierungen" => "reservierungen",
            "termine" => "termine",
            _ => "alle",
        };
        let month = NaiveDate::parse_from_str(&format!("{}-01", get("monat")), "%Y-%m-%d").ok();
        Self {
            view: view.into(),
            kind: kind.into(),
            boat: get("boot").to_string(),
            mine: !get("meine").is_empty(),
            month,
            week: date("woche"),
            from: date("von"),
            to: date("bis"),
        }
    }

    /// Adresse mit denselben Filtern und anderen Angaben zur Ansicht.
    fn url(&self, view: &str, extra: &[(&str, String)]) -> String {
        let mut parts: Vec<String> = vec![format!("ansicht={view}")];
        if self.kind != "alle" {
            parts.push(format!("art={}", self.kind));
        }
        if !self.boat.is_empty() {
            parts.push(format!("boot={}", encode(&self.boat)));
        }
        if self.mine {
            parts.push("meine=1".into());
        }
        for (key, value) in extra {
            parts.push(format!("{key}={}", encode(value)));
        }
        format!("/kalender?{}", parts.join("&"))
    }

    fn filter_count(&self) -> usize {
        usize::from(self.kind != "alle")
            + usize::from(!self.boat.is_empty())
            + usize::from(self.mine)
    }
}

pub async fn show(
    State(state): State<AppState>,
    session: Session,
    RequireUser(user): RequireUser,
    Query(query): Query<HashMap<String, String>>,
) -> Result<Response, WebError> {
    let params = Params::from_query(&query);
    let principal = state.club.principal(&user).await?;
    let today = principal.now.date_naive();

    // Zeitraum der Ansicht; vergangene Einträge zeigt nur das Archiv.
    let (from, to) = match params.view.as_str() {
        "monat" => {
            let first = params.month.unwrap_or_else(|| {
                NaiveDate::from_ymd_opt(today.year(), today.month(), 1).expect("Monat")
            });
            let next = if first.month() == 12 {
                NaiveDate::from_ymd_opt(first.year() + 1, 1, 1)
            } else {
                NaiveDate::from_ymd_opt(first.year(), first.month() + 1, 1)
            }
            .expect("Monat");
            (
                monday(first),
                monday(next - Duration::days(1)) + Duration::days(6),
            )
        }
        "woche" => {
            let start = monday(params.week.unwrap_or(today));
            (start, start + Duration::days(6))
        }
        _ => {
            let from = params.from.unwrap_or(today).max(today);
            let to = params
                .to
                .unwrap_or(from + Duration::days(LIST_DAYS))
                .max(from);
            (from, to)
        }
    };

    let mut entries: Vec<Entry> = vec![];
    let mut boats: Vec<(String, String)> = vec![];
    if params.kind != "termine" {
        let filter = ReservationFilter {
            boat: params.boat.parse::<BoatId>().ok(),
            from: Some(from),
            to: Some(to),
            mine: params.mine,
        };
        let list = state.club.reservation_list(&user, &filter).await?;
        boats = list.boats.clone();
        entries.extend(list.rows.iter().map(Entry::from_reservation));
    }
    if params.kind != "reservierungen" {
        let events = state.club.events_view(&user).await?;
        if boats.is_empty() {
            boats = events.boats.clone();
        }
        let boat_name = boats
            .iter()
            .find(|(key, _)| *key == params.boat)
            .map(|(_, name)| name.clone());
        entries.extend(
            events
                .events
                .iter()
                .filter(|e| !params.mine || e.mine)
                .filter(|e| boat_name.as_ref().is_none_or(|n| &e.boat == n))
                .map(Entry::from_event),
        );
    }
    if boats.is_empty() {
        boats = state.club.reservation_form_data(&user).await?.boats;
    }
    // Nur Aktuelles und Kommendes; Vergangenes steht im Archiv.
    entries.retain(|e| e.end >= today && e.end >= from && e.start <= to);
    entries.sort_by(|a, b| (a.start, a.kind, &a.title).cmp(&(b.start, b.kind, &b.title)));

    let mut weeks = vec![];
    let mut grid = vec![];
    let title;
    let (previous, next, today_link);
    let switch = vec![
        (
            "Liste".to_string(),
            params.url("liste", &[]),
            params.view == "liste",
        ),
        (
            "Monat".to_string(),
            params.url("monat", &[]),
            params.view == "monat",
        ),
        (
            "Woche".to_string(),
            params.url("woche", &[]),
            params.view == "woche",
        ),
    ];
    match params.view.as_str() {
        "liste" => {
            title = format!("Ab {}", from.format("%d.%m.%Y"));
            let mut day = from;
            let this_week = monday(today);
            let mut current: Option<ListWeek> = None;
            while day <= to {
                let day_entries: Vec<Entry> = entries
                    .iter()
                    .filter(|e| e.on(day))
                    .map(|e| e.for_day(day))
                    .collect();
                if !day_entries.is_empty() {
                    let week_start = monday(day);
                    if current
                        .as_ref()
                        .is_none_or(|w| w.title != week_title(week_start, this_week))
                    {
                        weeks.extend(current.take());
                        current = Some(ListWeek {
                            title: week_title(week_start, this_week),
                            days: vec![],
                        });
                    }
                    if let Some(w) = current.as_mut() {
                        w.days.push(ListDay {
                            label: day_label(day),
                            entries: day_entries,
                        });
                    }
                }
                day += Duration::days(1);
            }
            weeks.extend(current);
            let step = Duration::days(LIST_DAYS);
            previous = if from > today {
                params.url(
                    "liste",
                    &[
                        ("von", (from - step).max(today).to_string()),
                        ("bis", (from - Duration::days(1)).to_string()),
                    ],
                )
            } else {
                String::new()
            };
            next = params.url(
                "liste",
                &[
                    ("von", (to + Duration::days(1)).to_string()),
                    ("bis", (to + step).to_string()),
                ],
            );
            today_link = params.url("liste", &[]);
        }
        "monat" => {
            let first = params.month.unwrap_or_else(|| {
                NaiveDate::from_ymd_opt(today.year(), today.month(), 1).expect("Monat")
            });
            title = format!("{} {}", MONTHS[first.month0() as usize], first.year());
            let mut day = from;
            while day <= to {
                let mut week = CalWeek { days: vec![] };
                for _ in 0..7 {
                    week.days.push(CalDay {
                        label: short_label(day),
                        number: day.day(),
                        other_month: day.month() != first.month(),
                        today: day == today,
                        entries: entries
                            .iter()
                            .filter(|e| e.on(day))
                            .map(|e| e.for_day(day))
                            .collect(),
                    });
                    day += Duration::days(1);
                }
                grid.push(week);
            }
            let shift = |delta: i32| {
                let months = first.year() * 12 + first.month0() as i32 + delta;
                format!("{}-{:02}", months.div_euclid(12), months.rem_euclid(12) + 1)
            };
            previous = params.url("monat", &[("monat", shift(-1))]);
            next = params.url("monat", &[("monat", shift(1))]);
            today_link = params.url("monat", &[]);
        }
        _ => {
            let start = from;
            title = format!(
                "Woche vom {} bis {}",
                start.format("%d.%m."),
                to.format("%d.%m.%Y")
            );
            let mut week = CalWeek { days: vec![] };
            let mut day = start;
            while day <= to {
                week.days.push(CalDay {
                    label: short_label(day),
                    number: day.day(),
                    other_month: false,
                    today: day == today,
                    entries: entries
                        .iter()
                        .filter(|e| e.on(day))
                        .map(|e| e.for_day(day))
                        .collect(),
                });
                day += Duration::days(1);
            }
            grid.push(week);
            previous = params.url(
                "woche",
                &[("woche", (start - Duration::days(7)).to_string())],
            );
            next = params.url(
                "woche",
                &[("woche", (start + Duration::days(7)).to_string())],
            );
            today_link = params.url("woche", &[]);
        }
    }

    let empty = if params.view == "liste" {
        weeks.is_empty()
    } else {
        grid.iter()
            .all(|w| w.days.iter().all(|d| d.entries.is_empty()))
    };
    let can_create_event = principal.can(&vereinsakte_core::rights::Capability::ManageWorkEvents);
    let page_title = "Kalender";
    let layout = Layout::build(
        &state,
        &session,
        Some(&user),
        page_title,
        &params.url(&params.view, &[]),
    )
    .await?;
    render(&CalendarPage {
        layout,
        active_filters: params.filter_count(),
        view: params.view.clone(),
        kind: params.kind.clone(),
        boat: params.boat.clone(),
        mine: params.mine,
        boats,
        switch,
        title,
        previous,
        next,
        today_link,
        weeks,
        grid,
        weekdays: WEEKDAYS,
        empty,
        can_create_event,
        archive_hint: from < today || params.view != "liste",
    })
}

fn week_title(week_start: NaiveDate, this_week: NaiveDate) -> String {
    if week_start <= this_week {
        "Diese Woche".into()
    } else if week_start == this_week + Duration::days(7) {
        "Nächste Woche".into()
    } else {
        format!("Woche ab {}", week_start.format("%d.%m."))
    }
}

#[allow(dead_code)]
fn is_weekend(day: NaiveDate) -> bool {
    matches!(day.weekday(), Weekday::Sat | Weekday::Sun)
}
