//! Arbeitstermine mit Anmeldung und Aufgaben.

use vereinsakte_core::ids::{BoatId, MemberId, TaskId, WorkEventId};
use vereinsakte_core::rights::Capability;
use vereinsakte_core::task::{Task, TaskStatus};
use vereinsakte_core::work_event::{RegistrationError, WorkEvent};
use vereinsakte_core::work_hours::WorkReference;

use crate::changes::RecordKind;
use crate::commands::audit;
use crate::error::{AppError, FieldError};
use crate::format;
use crate::ports::Version;
use crate::scope::MemberScope;
use crate::service::{ClubService, CurrentUser};
use crate::work::{
    EventInput, EventLine, EventsView, TaskInput, TaskLine, TasksView, at_time, boat_name,
    parse_date_text, parse_time, trimmed,
};

fn boat_options(boats: &[vereinsakte_core::boat::Boat]) -> Vec<(String, String)> {
    let mut list: Vec<(String, String)> = boats
        .iter()
        .map(|b| (b.id.to_string(), b.name.clone()))
        .collect();
    list.sort_by(|a, b| a.1.cmp(&b.1));
    list
}

fn event_from(
    input: &EventInput,
    boats: &[vereinsakte_core::boat::Boat],
    base: Option<&WorkEvent>,
) -> Result<WorkEvent, AppError> {
    let mut errors = vec![];
    if input.title.trim().is_empty() {
        errors.push(FieldError::new("title", "Bitte einen Titel angeben."));
    }
    let date = parse_date_text(&input.date);
    if date.is_none() {
        errors.push(FieldError::new("date", "Bitte ein gültiges Datum angeben."));
    }
    let (start, end) = (parse_time(&input.start), parse_time(&input.end));
    if start.is_none() || end.is_none() {
        errors.push(FieldError::new(
            "start",
            "Bitte Beginn und Ende als Uhrzeit (HH:MM) angeben.",
        ));
    }
    let boat = match trimmed(&input.boat) {
        None => None,
        Some(id) => match id
            .parse::<BoatId>()
            .ok()
            .filter(|b| boats.iter().any(|x| x.id == *b))
        {
            Some(b) => Some(b),
            None => {
                errors.push(FieldError::new("boat", "Dieses Boot gibt es nicht."));
                None
            }
        },
    };
    let max = match trimmed(&input.max) {
        None => None,
        Some(n) => match n.parse::<u32>().ok().filter(|n| *n > 0) {
            Some(n) => Some(n),
            None => {
                errors.push(FieldError::new(
                    "max",
                    "Bitte eine positive ganze Zahl angeben.",
                ));
                None
            }
        },
    };
    let (Some(date), Some(start), Some(end)) = (date, start, end) else {
        return Err(AppError::Rejected(errors));
    };
    let mut event = base.cloned().unwrap_or_else(|| {
        WorkEvent::new(input.title.trim(), at_time(date, start), at_time(date, end))
    });
    event.title = input.title.trim().to_string();
    event.start = at_time(date, start);
    event.end = at_time(date, end);
    event.location = trimmed(&input.location);
    event.description = trimmed(&input.description);
    event.boat = boat;
    event.max_participants = max;
    if event.validate().is_err() {
        errors.push(FieldError::new(
            "end",
            "Das Ende muss nach dem Beginn liegen.",
        ));
    }
    if let Some(max) = max
        && (event.registrations.len() as u32) > max
    {
        errors.push(FieldError::new(
            "max",
            "Es sind bereits mehr Mitglieder angemeldet.",
        ));
    }
    if errors.is_empty() {
        Ok(event)
    } else {
        Err(AppError::Rejected(errors))
    }
}

impl ClubService {
    /// Arbeitstermine (kommende zuerst) mit Anmeldestand.
    pub async fn events_view(&self, user: &CurrentUser) -> Result<EventsView, AppError> {
        let principal = self.principal(user).await?;
        let scoped = self
            .load(&self.work_scope(MemberScope::Some(vec![user.member])))
            .await?;
        let members = self.store.active_members(principal.now).await?;
        let name = |m: MemberId| {
            members
                .iter()
                .find(|x| x.id == m)
                .map_or_else(|| m.to_string(), |x| x.full_name())
        };
        let record = &scoped.record;
        let mut events: Vec<&WorkEvent> = record.work_events.iter().collect();
        events.sort_by_key(|e| (e.start < principal.now, e.start));
        Ok(EventsView {
            events: events
                .into_iter()
                .map(|e| EventLine {
                    id: e.id.to_string(),
                    title: e.title.clone(),
                    when: format!(
                        "{}, {}–{} Uhr",
                        format::date(e.start),
                        e.start.format("%H:%M"),
                        e.end.format("%H:%M")
                    ),
                    start_date: e.start.date_naive(),
                    end_date: e.end.date_naive(),
                    times: format!("{}–{}", e.start.format("%H:%M"), e.end.format("%H:%M")),
                    location: e.location.clone().unwrap_or_default(),
                    description: e.description.clone().unwrap_or_default(),
                    boat: e
                        .boat
                        .map(|b| boat_name(&record.boats, b))
                        .unwrap_or_default(),
                    registered: e.registrations.len(),
                    max: e.max_participants,
                    participants: e
                        .registrations
                        .iter()
                        .map(|m| crate::people::PersonRef::member(*m, name(*m)))
                        .collect(),
                    mine: e.registrations.contains(&user.member),
                    full: e
                        .max_participants
                        .is_some_and(|m| e.registrations.len() as u32 >= m),
                    past: e.end < principal.now,
                })
                .collect(),
            can_manage: principal.can(&Capability::ManageWorkEvents),
            boats: boat_options(&record.boats),
        })
    }

    pub async fn create_event(
        &self,
        user: &CurrentUser,
        input: &EventInput,
    ) -> Result<WorkEventId, AppError> {
        let principal = self.principal(user).await?;
        principal.require(&Capability::ManageWorkEvents)?;
        let scoped = self.load(&self.work_scope(MemberScope::None)).await?;
        let event = event_from(input, &scoped.record.boats, None)?;
        let id = event.id;
        let mut new = scoped.record.clone();
        new.work_events.push(event);
        let log = audit(
            &principal,
            "event.create",
            "work_event",
            id,
            None,
            "Arbeitstermin angelegt",
        );
        self.save(&scoped, &new, vec![log]).await?;
        Ok(id)
    }

    /// Vorbelegung zum Bearbeiten eines Termins samt Version.
    pub async fn event_input(
        &self,
        user: &CurrentUser,
        id: WorkEventId,
    ) -> Result<(EventInput, i64), AppError> {
        let principal = self.principal(user).await?;
        principal.require(&Capability::ManageWorkEvents)?;
        let scoped = self.load(&self.work_scope(MemberScope::None)).await?;
        let e = scoped
            .record
            .work_events
            .iter()
            .find(|e| e.id == id)
            .ok_or_else(|| AppError::NotFound("Arbeitstermin".into()))?;
        Ok((
            EventInput {
                title: e.title.clone(),
                date: e.start.format("%Y-%m-%d").to_string(),
                start: e.start.format("%H:%M").to_string(),
                end: e.end.format("%H:%M").to_string(),
                location: e.location.clone().unwrap_or_default(),
                description: e.description.clone().unwrap_or_default(),
                boat: e.boat.map(|b| b.to_string()).unwrap_or_default(),
                max: e
                    .max_participants
                    .map(|n| n.to_string())
                    .unwrap_or_default(),
            },
            scoped.version(RecordKind::WorkEvent, &id.to_string()).0,
        ))
    }

    pub async fn update_event(
        &self,
        user: &CurrentUser,
        id: WorkEventId,
        version: i64,
        input: &EventInput,
    ) -> Result<(), AppError> {
        let principal = self.principal(user).await?;
        principal.require(&Capability::ManageWorkEvents)?;
        let scoped = self.load(&self.work_scope(MemberScope::None)).await?;
        if scoped.version(RecordKind::WorkEvent, &id.to_string()) != Version(version) {
            return Err(AppError::Conflict);
        }
        let mut new = scoped.record.clone();
        let slot = new
            .work_events
            .iter_mut()
            .find(|e| e.id == id)
            .ok_or_else(|| AppError::NotFound("Arbeitstermin".into()))?;
        *slot = event_from(input, &scoped.record.boats, Some(slot))?;
        let log = audit(
            &principal,
            "event.edit",
            "work_event",
            id,
            None,
            "Arbeitstermin geändert",
        );
        self.save(&scoped, &new, vec![log]).await
    }

    /// Meldet das angemeldete Mitglied an; ausgebucht oder doppelt wird abgelehnt.
    pub async fn register_event(
        &self,
        user: &CurrentUser,
        id: WorkEventId,
    ) -> Result<(), AppError> {
        let principal = self.principal(user).await?;
        let scoped = self.load(&self.work_scope(MemberScope::None)).await?;
        let mut new = scoped.record.clone();
        let event = new
            .work_events
            .iter_mut()
            .find(|e| e.id == id)
            .ok_or_else(|| AppError::NotFound("Arbeitstermin".into()))?;
        event.register(user.member).map_err(|e| match e {
            RegistrationError::Full => AppError::rejected("", "Der Termin ist ausgebucht."),
            RegistrationError::AlreadyRegistered => {
                AppError::rejected("", "Du bist bereits angemeldet.")
            }
        })?;
        let log = audit(
            &principal,
            "event.register",
            "work_event",
            id,
            Some(user.member),
            "Zum Arbeitstermin angemeldet",
        );
        self.save(&scoped, &new, vec![log]).await
    }

    pub async fn unregister_event(
        &self,
        user: &CurrentUser,
        id: WorkEventId,
    ) -> Result<(), AppError> {
        let principal = self.principal(user).await?;
        let scoped = self.load(&self.work_scope(MemberScope::None)).await?;
        let mut new = scoped.record.clone();
        let event = new
            .work_events
            .iter_mut()
            .find(|e| e.id == id)
            .ok_or_else(|| AppError::NotFound("Arbeitstermin".into()))?;
        event.registrations.retain(|m| *m != user.member);
        let log = audit(
            &principal,
            "event.unregister",
            "work_event",
            id,
            Some(user.member),
            "Vom Arbeitstermin abgemeldet",
        );
        self.save(&scoped, &new, vec![log]).await
    }

    // ---- Aufgaben ---------------------------------------------------------------

    pub async fn tasks_view(&self, user: &CurrentUser) -> Result<TasksView, AppError> {
        let principal = self.principal(user).await?;
        let scoped = self.load(&self.work_scope(MemberScope::All)).await?;
        let record = &scoped.record;
        let name = |m: MemberId| {
            record
                .member(m)
                .map_or_else(|| m.to_string(), |x| x.full_name())
        };
        let mut tasks: Vec<&Task> = record.tasks.iter().collect();
        tasks.sort_by_key(|t| (t.status == TaskStatus::Done, t.due_at));
        let lines = tasks
            .into_iter()
            .map(|t| {
                let hours: f64 = record
                    .work_entries
                    .iter()
                    .filter(|e| e.reference == Some(WorkReference::Task(t.id)))
                    .map(|e| e.hours())
                    .sum();
                let (status, key) = match t.status {
                    TaskStatus::Open => ("offen", "open"),
                    TaskStatus::InProgress => ("in Arbeit", "in_progress"),
                    TaskStatus::Waiting => ("wartet", "waiting"),
                    TaskStatus::Done => ("erledigt", "done"),
                };
                TaskLine {
                    id: t.id.to_string(),
                    title: t.title.clone(),
                    description: t.description.clone().unwrap_or_default(),
                    status,
                    status_key: key,
                    assignees: t
                        .assignees
                        .iter()
                        .map(|m| crate::people::PersonRef::member(*m, name(*m)))
                        .collect(),
                    boat: t
                        .boat
                        .map(|b| boat_name(&record.boats, b))
                        .unwrap_or_default(),
                    due: t.due_at.map(format::date).unwrap_or_default(),
                    hours: format::hours(hours),
                    can_update: principal.can(&Capability::UpdateTask {
                        assigned: t.assignees.contains(&user.member),
                    }),
                }
            })
            .collect();
        let mut members: Vec<(u32, String)> = record
            .members
            .iter()
            .filter(|m| m.is_active_at(principal.now))
            .map(|m| (m.id.get(), m.full_name()))
            .collect();
        members.sort_by(|a, b| a.1.cmp(&b.1));
        Ok(TasksView {
            tasks: lines,
            can_manage: principal.can(&Capability::ManageTasks),
            boats: boat_options(&record.boats),
            members,
        })
    }

    pub async fn create_task(
        &self,
        user: &CurrentUser,
        input: &TaskInput,
    ) -> Result<TaskId, AppError> {
        let principal = self.principal(user).await?;
        principal.require(&Capability::ManageTasks)?;
        let scoped = self.load(&self.work_scope(MemberScope::None)).await?;
        let fields = crate::tasks::parse_task_fields(input, &scoped.record.boats)?;
        let mut task = Task::new(fields.title);
        task.description = trimmed(&input.description);
        task.boat = fields.boat;
        task.due_at = fields.due;
        task.estimated_hours = fields.estimated;
        let id = task.id;
        let mut new = scoped.record.clone();
        new.tasks.push(task);
        let log = audit(
            &principal,
            "task.create",
            "task",
            id,
            None,
            "Aufgabe angelegt",
        );
        self.save(&scoped, &new, vec![log]).await?;
        Ok(id)
    }

    /// Weist ein aktives Mitglied zu (Amtsträger).
    pub async fn assign_task(
        &self,
        user: &CurrentUser,
        id: TaskId,
        member: MemberId,
    ) -> Result<(), AppError> {
        let principal = self.principal(user).await?;
        principal.require(&Capability::ManageTasks)?;
        let scoped = self
            .load(&self.work_scope(MemberScope::Some(vec![member])))
            .await?;
        if scoped
            .record
            .member(member)
            .is_none_or(|m| !m.is_active_at(principal.now))
        {
            return Err(AppError::rejected(
                "member",
                "Dieses Mitglied ist nicht aktiv.",
            ));
        }
        let mut new = scoped.record.clone();
        let task = new
            .tasks
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| AppError::NotFound("Aufgabe".into()))?;
        if !task.assign(member) {
            return Err(AppError::rejected(
                "member",
                "Das Mitglied ist bereits zuständig.",
            ));
        }
        let log = audit(
            &principal,
            "task.assign",
            "task",
            id,
            Some(member),
            "Aufgabe zugewiesen",
        );
        self.save(&scoped, &new, vec![log]).await
    }

    /// Setzt den Status; Zuständige für ihre Aufgabe, Amtsträger für alle.
    pub async fn set_task_status(
        &self,
        user: &CurrentUser,
        id: TaskId,
        status: TaskStatus,
    ) -> Result<(), AppError> {
        let principal = self.principal(user).await?;
        let scoped = self.load(&self.work_scope(MemberScope::None)).await?;
        let mut new = scoped.record.clone();
        let task = new
            .tasks
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| AppError::NotFound("Aufgabe".into()))?;
        principal.require(&Capability::UpdateTask {
            assigned: task.assignees.contains(&user.member),
        })?;
        task.set_status(status, principal.now);
        let log = audit(
            &principal,
            "task.status",
            "task",
            id,
            None,
            "Aufgabenstatus geändert",
        );
        self.save(&scoped, &new, vec![log]).await
    }
}
