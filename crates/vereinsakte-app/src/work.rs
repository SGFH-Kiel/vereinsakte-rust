//! Arbeit: Einsätze erfassen und bestätigen, Arbeitstermine, Aufgaben,
//! Stundenübersicht, Überträge und finanzielle Abgeltung.

use chrono::{Datelike, NaiveDate, NaiveTime};
use vereinsakte_core::boat::Boat;
use vereinsakte_core::ids::{BoatId, MemberId, TaskId, WorkEntryId, WorkEventId};
use vereinsakte_core::office::ActionError;
use vereinsakte_core::rights::Capability;
use vereinsakte_core::task::TaskStatus;
use vereinsakte_core::time::{Timestamp, club_time};
use vereinsakte_core::work_hours::{WorkEntry, WorkExtent, WorkReference, WorkSubject};

use crate::changes::RecordKind;
use crate::commands::audit;
use crate::error::{AppError, FieldError};
use crate::format;
use crate::ports::Version;
use crate::scope::{BoatScope, MemberScope, Scope};
use crate::service::{ClubService, CurrentUser};

// ---- Sichtmodelle ------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntryLine {
    pub id: String,
    pub member: String,
    pub date: String,
    pub hours: String,
    pub subject: String,
    pub description: String,
    pub reference: String,
    pub confirmed: bool,
    pub can_edit: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkView {
    pub year: i32,
    pub required: String,
    pub credited: String,
    pub entries: Vec<EntryLine>,
    /// Auswahl „Gegenstand“: Schlüssel und Bezeichnung.
    pub subjects: Vec<(String, String)>,
    /// Auswahl „Bezug“: Schlüssel und Bezeichnung.
    pub references: Vec<(String, String)>,
    pub can_see_all: bool,
    pub can_confirm: bool,
}

/// Eingabe eines Arbeitseinsatzes (Texte aus dem Formular).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EntryInput {
    pub date: String,
    pub hours: String,
    pub start: String,
    pub end: String,
    pub subject: String,
    pub description: String,
    pub reference: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventLine {
    pub id: String,
    pub title: String,
    pub when: String,
    /// Erster und letzter Tag sowie Uhrzeit (für den Kalender).
    pub start_date: chrono::NaiveDate,
    pub end_date: chrono::NaiveDate,
    pub times: String,
    pub location: String,
    pub description: String,
    pub boat: String,
    pub registered: usize,
    pub max: Option<u32>,
    pub participants: Vec<crate::people::PersonRef>,
    pub mine: bool,
    pub full: bool,
    pub past: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventsView {
    pub events: Vec<EventLine>,
    pub can_manage: bool,
    pub boats: Vec<(String, String)>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EventInput {
    pub title: String,
    pub date: String,
    pub start: String,
    pub end: String,
    pub location: String,
    pub description: String,
    pub boat: String,
    pub max: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskLine {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: &'static str,
    pub status_key: &'static str,
    pub assignees: Vec<String>,
    pub boat: String,
    pub due: String,
    pub hours: String,
    pub can_update: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TasksView {
    pub tasks: Vec<TaskLine>,
    pub can_manage: bool,
    pub boats: Vec<(String, String)>,
    pub members: Vec<(u32, String)>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TaskInput {
    pub title: String,
    pub description: String,
    pub boat: String,
    pub due: String,
    pub hours: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HoursRow {
    pub id: u32,
    pub name: String,
    pub required: String,
    pub credited: String,
    pub missing: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HoursOverview {
    pub year: i32,
    pub rows: Vec<HoursRow>,
    pub can_grant: bool,
    pub members: Vec<(u32, String)>,
    pub rate: String,
}

// ---- Hilfen ---------------------------------------------------------------------

pub(crate) fn trimmed(text: &str) -> Option<String> {
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_string())
}

pub(crate) fn at_time(date: NaiveDate, time: NaiveTime) -> Timestamp {
    use chrono::Timelike;
    club_time(
        date.year(),
        date.month(),
        date.day(),
        time.hour(),
        time.minute(),
    )
}

pub(crate) fn parse_time(text: &str) -> Option<NaiveTime> {
    NaiveTime::parse_from_str(text.trim(), "%H:%M").ok()
}

pub(crate) fn parse_date_text(text: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(text.trim(), "%Y-%m-%d")
        .or_else(|_| NaiveDate::parse_from_str(text.trim(), "%d.%m.%Y"))
        .ok()
}

pub(crate) fn boat_name(boats: &[Boat], id: BoatId) -> String {
    boats
        .iter()
        .find(|b| b.id == id)
        .map_or_else(|| id.to_string(), |b| b.name.clone())
}

fn subject_label(boats: &[Boat], subject: WorkSubject) -> String {
    match subject {
        WorkSubject::Club => "Verein".into(),
        WorkSubject::Boat(id) => boat_name(boats, id),
    }
}

fn subject_key(subject: WorkSubject) -> String {
    match subject {
        WorkSubject::Club => "club".into(),
        WorkSubject::Boat(id) => id.to_string(),
    }
}

fn reference_key(reference: Option<WorkReference>) -> String {
    match reference {
        None => String::new(),
        Some(WorkReference::WorkEvent(id)) => format!("event:{id}"),
        Some(WorkReference::Task(id)) => format!("task:{id}"),
    }
}

fn subject_options(boats: &[Boat]) -> Vec<(String, String)> {
    let mut list = vec![("club".to_string(), "Verein".to_string())];
    let mut sorted: Vec<&Boat> = boats.iter().collect();
    sorted.sort_by(|a, b| a.name.cmp(&b.name));
    list.extend(
        sorted
            .into_iter()
            .map(|b| (b.id.to_string(), b.name.clone())),
    );
    list
}

pub(crate) fn action_error(error: ActionError) -> AppError {
    match error {
        ActionError::NotAuthorized(_) => AppError::Forbidden,
        ActionError::UnknownBoat(_) => AppError::NotFound("Boot".into()),
        ActionError::UnknownWorkEntry(_) => AppError::NotFound("Arbeitseinsatz".into()),
        ActionError::NoOpenBlock => AppError::rejected("", "Es gibt keine offene Sperre."),
    }
}

impl EntryInput {
    fn extent(&self, errors: &mut Vec<FieldError>) -> Option<WorkExtent> {
        let Some(date) = parse_date_text(&self.date) else {
            errors.push(FieldError::new("date", "Bitte ein gültiges Datum angeben."));
            return None;
        };
        let has_hours = !self.hours.trim().is_empty();
        let has_period = !self.start.trim().is_empty() || !self.end.trim().is_empty();
        match (has_hours, has_period) {
            (true, true) => {
                errors.push(FieldError::new(
                    "hours",
                    "Bitte entweder Stunden oder einen Zeitraum angeben, nicht beides.",
                ));
                None
            }
            (false, false) => {
                errors.push(FieldError::new(
                    "hours",
                    "Bitte Stunden oder einen Zeitraum angeben.",
                ));
                None
            }
            (true, false) => match format::parse_decimal(&self.hours).filter(|h| *h > 0.0) {
                Some(hours) => Some(WorkExtent::Amount {
                    date: club_time(date.year(), date.month(), date.day(), 0, 0),
                    hours,
                }),
                None => {
                    errors.push(FieldError::new(
                        "hours",
                        "Bitte eine positive Stundenzahl eingeben (z. B. 2,5).",
                    ));
                    None
                }
            },
            (false, true) => match (parse_time(&self.start), parse_time(&self.end)) {
                (Some(s), Some(e)) if e > s => Some(WorkExtent::Period {
                    start: at_time(date, s),
                    end: at_time(date, e),
                }),
                (Some(_), Some(_)) => {
                    errors.push(FieldError::new(
                        "end",
                        "Das Ende liegt nicht nach dem Beginn.",
                    ));
                    None
                }
                _ => {
                    errors.push(FieldError::new(
                        "start",
                        "Bitte Beginn und Ende als Uhrzeit (HH:MM) angeben.",
                    ));
                    None
                }
            },
        }
    }

    fn subject(&self, boats: &[Boat], errors: &mut Vec<FieldError>) -> Option<WorkSubject> {
        if self.subject == "club" {
            return Some(WorkSubject::Club);
        }
        match self
            .subject
            .parse::<BoatId>()
            .ok()
            .filter(|b| boats.iter().any(|x| x.id == *b))
        {
            Some(boat) => Some(WorkSubject::Boat(boat)),
            None => {
                errors.push(FieldError::new(
                    "subject",
                    "Bitte den Gegenstand wählen (Verein oder Boot).",
                ));
                None
            }
        }
    }

    fn reference(&self, errors: &mut Vec<FieldError>) -> Option<Option<WorkReference>> {
        let text = self.reference.trim();
        if text.is_empty() {
            return Some(None);
        }
        let parsed = text.split_once(':').and_then(|(kind, id)| match kind {
            "event" => id.parse::<WorkEventId>().ok().map(WorkReference::WorkEvent),
            "task" => id.parse::<TaskId>().ok().map(WorkReference::Task),
            _ => None,
        });
        match parsed {
            Some(r) => Some(Some(r)),
            None => {
                errors.push(FieldError::new("reference", "Unbekannter Bezug."));
                None
            }
        }
    }
}

// ---- Anwendungsfälle: Einsätze -------------------------------------------------------

impl ClubService {
    pub(crate) fn work_scope(&self, members: MemberScope) -> Scope {
        Scope {
            members,
            boats: BoatScope::All,
            work_events: true,
            tasks: true,
            seasons: true,
            offices: true,
            ..Scope::default()
        }
    }

    /// Einsätze und Stunden eines Mitglieds im Jahr (eigene; Vorstand und Revisoren fremde).
    pub async fn work_view(
        &self,
        user: &CurrentUser,
        member: Option<MemberId>,
        year: Option<i32>,
    ) -> Result<WorkView, AppError> {
        let principal = self.principal(user).await?;
        let target = member.unwrap_or(user.member);
        if target != user.member {
            principal.require(&Capability::ReadMember(target))?;
        }
        let year = year.unwrap_or_else(|| principal.now.year());
        let scoped = self
            .load(&self.work_scope(MemberScope::Some(vec![target])))
            .await?;
        let record = &scoped.record;
        let my = record
            .member_year(target, year)
            .ok_or_else(|| AppError::NotFound(format!("Mitglied {target}")))?;
        let mut entries: Vec<&WorkEntry> = my.entries().collect();
        entries.sort_by_key(|e| e.date());
        let lines = entries
            .into_iter()
            .map(|e| EntryLine {
                id: e.id.to_string(),
                member: String::new(),
                date: format::date(e.date()),
                hours: format::hours(e.hours()),
                subject: subject_label(&record.boats, e.subject),
                description: e.description.clone().unwrap_or_default(),
                reference: self.reference_label(record, e.reference),
                confirmed: e.is_confirmed(),
                can_edit: principal.can(&Capability::EditWorkEntry {
                    owner: e.member,
                    confirmed: e.is_confirmed(),
                }),
            })
            .collect();
        Ok(WorkView {
            year,
            required: format::hours(my.required_hours()),
            credited: format::hours(my.credited_hours()),
            entries: lines,
            subjects: subject_options(&record.boats),
            references: self.reference_options(record, principal.now),
            can_see_all: principal.can(&Capability::ReadAllHours),
            can_confirm: principal.has_office(),
        })
    }

    fn reference_label(
        &self,
        record: &vereinsakte_core::club_record::ClubRecord,
        r: Option<WorkReference>,
    ) -> String {
        match r {
            None => String::new(),
            Some(WorkReference::WorkEvent(id)) => record
                .work_events
                .iter()
                .find(|e| e.id == id)
                .map_or_else(String::new, |e| format!("Termin: {}", e.title)),
            Some(WorkReference::Task(id)) => record
                .task(id)
                .map_or_else(String::new, |t| format!("Aufgabe: {}", t.title)),
        }
    }

    fn reference_options(
        &self,
        record: &vereinsakte_core::club_record::ClubRecord,
        _now: Timestamp,
    ) -> Vec<(String, String)> {
        let mut list: Vec<(String, String)> = record
            .work_events
            .iter()
            .map(|e| {
                (
                    format!("event:{}", e.id),
                    format!("Termin: {} ({})", e.title, format::date(e.start)),
                )
            })
            .collect();
        list.extend(
            record
                .tasks
                .iter()
                .filter(|t| t.status != TaskStatus::Done)
                .map(|t| (format!("task:{}", t.id), format!("Aufgabe: {}", t.title))),
        );
        list
    }

    fn entry_from_input(
        &self,
        member: MemberId,
        input: &EntryInput,
        record: &vereinsakte_core::club_record::ClubRecord,
    ) -> Result<WorkEntry, AppError> {
        let mut errors = vec![];
        let extent = input.extent(&mut errors);
        let subject = input.subject(&record.boats, &mut errors);
        let reference = input.reference(&mut errors);
        if !errors.is_empty() {
            return Err(AppError::Rejected(errors));
        }
        let mut entry = WorkEntry::new(member, extent.expect("geprüft"), subject.expect("geprüft"));
        entry.description = trimmed(&input.description);
        entry.reference = reference.expect("geprüft");
        if let Err(issues) = record.validate_work_entry(&entry) {
            return Err(AppError::Rejected(
                issues
                    .into_iter()
                    .map(|i| FieldError::new("subject", i.to_string()))
                    .collect(),
            ));
        }
        Ok(entry)
    }

    /// Erfasst einen Einsatz für das angemeldete Mitglied (unbestätigt).
    pub async fn add_work_entry(
        &self,
        user: &CurrentUser,
        input: &EntryInput,
    ) -> Result<WorkEntryId, AppError> {
        let principal = self.principal(user).await?;
        let scoped = self
            .load(&self.work_scope(MemberScope::Some(vec![user.member])))
            .await?;
        let entry = self.entry_from_input(user.member, input, &scoped.record)?;
        let id = entry.id;
        let mut new = scoped.record.clone();
        new.work_entries.push(entry);
        let log = audit(
            &principal,
            "work.add",
            "work_entry",
            id,
            Some(user.member),
            "Arbeitseinsatz erfasst",
        );
        self.save(&scoped, &new, vec![log]).await?;
        Ok(id)
    }

    async fn load_entry(
        &self,
        id: WorkEntryId,
    ) -> Result<(crate::changes::Scoped, WorkEntry), AppError> {
        let scoped = self.load(&self.work_scope(MemberScope::All)).await?;
        let entry = scoped
            .record
            .work_entry(id)
            .cloned()
            .ok_or_else(|| AppError::NotFound("Arbeitseinsatz".into()))?;
        Ok((scoped, entry))
    }

    /// Vorbelegung zum Bearbeiten eines Einsatzes samt Version.
    pub async fn work_entry_input(
        &self,
        user: &CurrentUser,
        id: WorkEntryId,
    ) -> Result<(EntryInput, i64), AppError> {
        let principal = self.principal(user).await?;
        let (scoped, entry) = self.load_entry(id).await?;
        principal.require(&Capability::EditWorkEntry {
            owner: entry.member,
            confirmed: entry.is_confirmed(),
        })?;
        let (date, hours, start, end) = match &entry.extent {
            WorkExtent::Amount { date, hours } => {
                (*date, format::hours(*hours), String::new(), String::new())
            }
            WorkExtent::Period { start, end } => (
                *start,
                String::new(),
                start.format("%H:%M").to_string(),
                end.format("%H:%M").to_string(),
            ),
        };
        Ok((
            EntryInput {
                date: date.format("%Y-%m-%d").to_string(),
                hours,
                start,
                end,
                subject: subject_key(entry.subject),
                description: entry.description.clone().unwrap_or_default(),
                reference: reference_key(entry.reference),
            },
            scoped.version(RecordKind::WorkEntry, &id.to_string()).0,
        ))
    }

    /// Ändert einen Einsatz; die Änderung hebt eine Bestätigung auf.
    pub async fn update_work_entry(
        &self,
        user: &CurrentUser,
        id: WorkEntryId,
        version: i64,
        input: &EntryInput,
    ) -> Result<(), AppError> {
        let principal = self.principal(user).await?;
        let (scoped, old) = self.load_entry(id).await?;
        principal.require(&Capability::EditWorkEntry {
            owner: old.member,
            confirmed: old.is_confirmed(),
        })?;
        if scoped.version(RecordKind::WorkEntry, &id.to_string()) != Version(version) {
            return Err(AppError::Conflict);
        }
        let mut entry = self.entry_from_input(old.member, input, &scoped.record)?;
        entry.id = id;
        let mut new = scoped.record.clone();
        let slot = new
            .work_entries
            .iter_mut()
            .find(|e| e.id == id)
            .expect("geladen");
        *slot = entry;
        let log = audit(
            &principal,
            "work.edit",
            "work_entry",
            id,
            Some(old.member),
            "Arbeitseinsatz geändert",
        );
        self.save(&scoped, &new, vec![log]).await
    }

    /// Löscht einen eigenen unbestätigten Einsatz.
    pub async fn delete_work_entry(
        &self,
        user: &CurrentUser,
        id: WorkEntryId,
    ) -> Result<(), AppError> {
        let principal = self.principal(user).await?;
        let (scoped, entry) = self.load_entry(id).await?;
        principal.require(&Capability::EditWorkEntry {
            owner: entry.member,
            confirmed: entry.is_confirmed(),
        })?;
        let mut new = scoped.record.clone();
        new.work_entries.retain(|e| e.id != id);
        let log = audit(
            &principal,
            "work.delete",
            "work_entry",
            id,
            Some(entry.member),
            "Arbeitseinsatz gelöscht",
        );
        self.save(&scoped, &new, vec![log]).await
    }

    /// Offene Einsätze, die das angemeldete Mitglied bestätigen darf.
    pub async fn open_confirmations(&self, user: &CurrentUser) -> Result<Vec<EntryLine>, AppError> {
        let principal = self.principal(user).await?;
        let scoped = self.load(&self.work_scope(MemberScope::All)).await?;
        let record = &scoped.record;
        let name = |m: MemberId| {
            record
                .member(m)
                .map_or_else(|| m.to_string(), |x| x.full_name())
        };
        let mut open: Vec<&WorkEntry> = record
            .work_entries
            .iter()
            .filter(|e| {
                !e.is_confirmed()
                    && principal.can(&Capability::ConfirmWork {
                        owner: e.member,
                        subject: e.subject,
                    })
            })
            .collect();
        open.sort_by_key(|e| e.date());
        Ok(open
            .into_iter()
            .map(|e| EntryLine {
                id: e.id.to_string(),
                member: name(e.member),
                date: format::date(e.date()),
                hours: format::hours(e.hours()),
                subject: subject_label(&record.boats, e.subject),
                description: e.description.clone().unwrap_or_default(),
                reference: self.reference_label(record, e.reference),
                confirmed: false,
                can_edit: false,
            })
            .collect())
    }

    /// Bestätigt einen Einsatz.
    pub async fn confirm_work(&self, user: &CurrentUser, id: WorkEntryId) -> Result<(), AppError> {
        let principal = self.principal(user).await?;
        let (scoped, entry) = self.load_entry(id).await?;
        let mut new = scoped.record.clone();
        new.confirm_work(user.member, id, principal.now)
            .map_err(action_error)?;
        let log = audit(
            &principal,
            "work.confirm",
            "work_entry",
            id,
            Some(entry.member),
            "Arbeitseinsatz bestätigt",
        );
        self.save(&scoped, &new, vec![log]).await
    }
}
