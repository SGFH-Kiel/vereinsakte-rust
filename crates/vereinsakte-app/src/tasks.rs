//! Aufgabenboard: Spalten, Aufgabenseite, Beschreibung, Kommentare und Anhänge.

use chrono::{Datelike, Duration};
use uuid::Uuid;
use vereinsakte_core::boat::Boat;
use vereinsakte_core::ids::{BoatId, MemberId, TaskId};
use vereinsakte_core::rights::Capability;
use vereinsakte_core::task::{AttachmentKind, Task, TaskAttachment, TaskComment, TaskStatus};
use vereinsakte_core::time::{Timestamp, club_time, year_start};
use vereinsakte_core::work_hours::WorkReference;

use crate::audit::AuditFilter;
use crate::changes::RecordKind;
use crate::commands::audit;
use crate::error::{AppError, FieldError};
use crate::format;
use crate::people::PersonRef;
use crate::principal::Principal;
use crate::scope::MemberScope;
use crate::service::{ClubService, CurrentUser};
use crate::task_extras::TaskCounts;
use crate::work::{TaskInput, boat_name, parse_date_text, trimmed};

/// Größte erlaubte Datei (10 MB) und Zahl der Anhänge je Aufgabe.
pub const MAX_ATTACHMENT_BYTES: usize = 10 * 1024 * 1024;
pub const MAX_ATTACHMENTS: usize = 20;
pub const MAX_DESCRIPTION_CHARS: usize = 20_000;
pub const MAX_COMMENT_CHARS: usize = 5_000;
/// So lange darf der Autor seinen Kommentar ändern oder löschen.
pub const COMMENT_WINDOW_MINUTES: i64 = 15;
pub const FORMAT_ERROR: &str = "Erlaubt sind Bilder (JPEG, PNG, WebP, GIF) und PDF-Dokumente.";

/// Die vier Spalten des Boards: Schlüssel, Kürzel, Name, Erklärung.
pub const COLUMNS: [(&str, &str, &str, &str); 4] = [
    ("open", "TODO", "Offen", "noch nicht begonnen"),
    ("in_progress", "WIP", "In Arbeit", "wird gerade bearbeitet"),
    (
        "waiting",
        "WAIT",
        "Wartet",
        "wartet auf etwas oder jemanden",
    ),
    ("done", "DONE", "Erledigt", "in dieser Saison erledigt"),
];

pub fn status_key(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Open => "open",
        TaskStatus::InProgress => "in_progress",
        TaskStatus::Waiting => "waiting",
        TaskStatus::Done => "done",
    }
}

pub fn parse_status(key: &str) -> Option<TaskStatus> {
    Some(match key {
        "open" => TaskStatus::Open,
        "in_progress" => TaskStatus::InProgress,
        "waiting" => TaskStatus::Waiting,
        "done" => TaskStatus::Done,
        _ => return None,
    })
}

fn status_label(status: TaskStatus) -> &'static str {
    COLUMNS
        .iter()
        .find(|c| c.0 == status_key(status))
        .map_or("", |c| c.2)
}

/// Erkennt Bilder und PDFs an der Dateikennung (nicht am Namen oder Typ).
/// Liefert Art und festen Inhaltstyp; alles andere (auch SVG) ist ungültig.
pub fn detect_upload(data: &[u8]) -> Option<(AttachmentKind, &'static str)> {
    if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some((AttachmentKind::Image, "image/jpeg"))
    } else if data.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        Some((AttachmentKind::Image, "image/png"))
    } else if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
        Some((AttachmentKind::Image, "image/gif"))
    } else if data.len() >= 12 && &data[..4] == b"RIFF" && &data[8..12] == b"WEBP" {
        Some((AttachmentKind::Image, "image/webp"))
    } else if data.starts_with(b"%PDF-") {
        Some((AttachmentKind::Pdf, "application/pdf"))
    } else {
        None
    }
}

/// Dateiname ohne Pfadteile und Steuerzeichen, höchstens 100 Zeichen.
pub fn sanitize_filename(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let cleaned: String = base
        .chars()
        .filter(|c| !c.is_control() && !matches!(c, '"' | ';' | '<' | '>' | ':' | '*' | '?' | '|'))
        .collect();
    let cleaned = cleaned.trim().trim_start_matches('.');
    let cut: String = cleaned.chars().take(100).collect();
    if cut.trim().is_empty() {
        "datei".to_string()
    } else {
        cut.trim().to_string()
    }
}

fn file_size(bytes: usize) -> String {
    if bytes >= 1_000_000 {
        format!("{} MB", format::number(bytes as f64 / 1_000_000.0))
    } else {
        format!("{} KB", bytes.div_ceil(1000))
    }
}

fn date_time(t: Timestamp) -> String {
    t.format("%d.%m.%Y, %H:%M").to_string()
}

/// Beginn der laufenden Saison (1. Januar).
fn season_start(now: Timestamp) -> Timestamp {
    year_start(now.year())
}

fn is_archived(task: &Task, now: Timestamp) -> bool {
    task.status == TaskStatus::Done && task.done_at.is_some_and(|t| t < season_start(now))
}

fn is_overdue(task: &Task, now: Timestamp) -> bool {
    task.status != TaskStatus::Done
        && task
            .due_at
            .is_some_and(|due| due + Duration::days(1) <= now)
}

/// Gemeinsame Prüfung der Aufgabenfelder (Anlegen und Ändern).
pub(crate) struct TaskFields {
    pub title: String,
    pub boat: Option<BoatId>,
    pub due: Option<Timestamp>,
    pub estimated: Option<f64>,
}

pub(crate) fn parse_task_fields(input: &TaskInput, boats: &[Boat]) -> Result<TaskFields, AppError> {
    let mut errors = vec![];
    if input.title.trim().is_empty() {
        errors.push(FieldError::new("title", "Bitte einen Titel angeben."));
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
    let due = match trimmed(&input.due) {
        None => None,
        Some(d) => match parse_date_text(&d) {
            Some(d) => Some(club_time(d.year(), d.month(), d.day(), 0, 0)),
            None => {
                errors.push(FieldError::new("due", "Bitte ein gültiges Datum angeben."));
                None
            }
        },
    };
    let estimated = match trimmed(&input.hours) {
        None => None,
        Some(h) => match format::parse_decimal(&h).filter(|h| *h > 0.0) {
            Some(h) => Some(h),
            None => {
                errors.push(FieldError::new(
                    "hours",
                    "Bitte eine positive Stundenzahl eingeben.",
                ));
                None
            }
        },
    };
    if !errors.is_empty() {
        return Err(AppError::Rejected(errors));
    }
    Ok(TaskFields {
        title: input.title.trim().to_string(),
        boat,
        due,
        estimated,
    })
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BoardFilter {
    /// Nur Aufgaben, denen der Angemeldete zugewiesen ist.
    pub mine: bool,
    pub boat: Option<BoatId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoardCard {
    pub id: String,
    pub title: String,
    pub boat: String,
    pub assignees: Vec<PersonRef>,
    pub due: String,
    pub overdue: bool,
    pub comments: u32,
    pub images: u32,
    pub attachments: u32,
    pub version: i64,
    pub can_move: bool,
    pub status_key: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoardColumn {
    pub key: &'static str,
    pub tag: &'static str,
    pub label: &'static str,
    pub hint: &'static str,
    pub cards: Vec<BoardCard>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoardView {
    pub columns: Vec<BoardColumn>,
    pub can_manage: bool,
    pub mine: bool,
    pub boat: String,
    pub boats: Vec<(String, String)>,
    pub members: Vec<(u32, String)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommentLine {
    pub id: String,
    pub author: PersonRef,
    pub at: String,
    pub text: String,
    pub can_edit: bool,
    pub can_delete: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttachmentLine {
    pub id: String,
    pub filename: String,
    pub is_image: bool,
    pub alt: String,
    pub size: String,
    pub by: PersonRef,
    pub at: String,
    pub can_delete: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistoryLine {
    pub at: String,
    pub who: PersonRef,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskPage {
    pub id: String,
    pub title: String,
    pub status_key: &'static str,
    pub status_label: &'static str,
    pub version: i64,
    pub assignees: Vec<PersonRef>,
    pub boat_id: String,
    pub boat: String,
    pub due: String,
    pub due_input: String,
    pub overdue: bool,
    pub estimated: String,
    pub estimated_input: String,
    pub hours: String,
    /// Markdown-Quelltext.
    pub description: String,
    pub can_move: bool,
    pub can_edit: bool,
    pub can_manage: bool,
    pub archived: bool,
    pub comments: Vec<CommentLine>,
    pub attachments: Vec<AttachmentLine>,
    pub history: Vec<HistoryLine>,
    pub boats: Vec<(String, String)>,
    pub members: Vec<(u32, String)>,
    pub max_attachments: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArchivedTask {
    pub id: String,
    pub title: String,
    pub boat: String,
    pub assignees: Vec<PersonRef>,
    pub done: String,
    pub hours: String,
}

impl ClubService {
    fn person_lookup(
        record: &vereinsakte_core::club_record::ClubRecord,
    ) -> impl Fn(MemberId) -> PersonRef + '_ {
        move |m| {
            record.member(m).map_or_else(
                || PersonRef::text(format!("Mitglied {m}")),
                |x| PersonRef::member(m, x.full_name()),
            )
        }
    }

    /// Das Board: Spalten mit Karten der laufenden Saison.
    pub async fn board(
        &self,
        user: &CurrentUser,
        filter: &BoardFilter,
    ) -> Result<BoardView, AppError> {
        let principal = self.principal(user).await?;
        let scoped = self.load(&self.work_scope(MemberScope::All)).await?;
        let counts = self.store.counts().await?;
        let record = &scoped.record;
        let person = Self::person_lookup(record);
        let mut columns: Vec<BoardColumn> = COLUMNS
            .iter()
            .map(|(key, tag, label, hint)| BoardColumn {
                key,
                tag,
                label,
                hint,
                cards: vec![],
            })
            .collect();
        let mut tasks: Vec<&Task> = record
            .tasks
            .iter()
            .filter(|t| !is_archived(t, principal.now))
            .filter(|t| !filter.mine || t.assignees.contains(&user.member))
            .filter(|t| filter.boat.is_none_or(|b| t.boat == Some(b)))
            .collect();
        tasks.sort_by_key(|t| (t.due_at.is_none(), t.due_at, t.title.to_lowercase()));
        for t in tasks {
            let c: TaskCounts = counts.get(&t.id).copied().unwrap_or_default();
            let key = status_key(t.status);
            let card = BoardCard {
                id: t.id.to_string(),
                title: t.title.clone(),
                boat: t
                    .boat
                    .map(|b| boat_name(&record.boats, b))
                    .unwrap_or_default(),
                assignees: t.assignees.iter().map(|m| person(*m)).collect(),
                due: t.due_at.map(format::date).unwrap_or_default(),
                overdue: is_overdue(t, principal.now),
                comments: c.comments,
                images: c.images,
                attachments: c.attachments,
                version: scoped.version(RecordKind::Task, &t.id.to_string()).0,
                can_move: principal.can(&Capability::UpdateTask {
                    assigned: t.assignees.contains(&user.member),
                }),
                status_key: key,
            };
            if let Some(col) = columns.iter_mut().find(|c| c.key == key) {
                col.cards.push(card);
            }
        }
        Ok(BoardView {
            columns,
            can_manage: principal.can(&Capability::ManageTasks),
            mine: filter.mine,
            boat: filter.boat.map(|b| b.to_string()).unwrap_or_default(),
            boats: boat_options(&record.boats),
            members: active_members(record, &principal),
        })
    }

    /// Erledigte Aufgaben früherer Saisons (Archiv).
    pub async fn archive_tasks(&self, user: &CurrentUser) -> Result<Vec<ArchivedTask>, AppError> {
        let principal = self.principal(user).await?;
        principal.require(&Capability::ReadArchive)?;
        let scoped = self.load(&self.work_scope(MemberScope::All)).await?;
        let record = &scoped.record;
        let person = Self::person_lookup(record);
        let mut rows: Vec<(Option<Timestamp>, ArchivedTask)> = record
            .tasks
            .iter()
            .filter(|t| is_archived(t, principal.now))
            .map(|t| {
                (
                    t.done_at,
                    ArchivedTask {
                        id: t.id.to_string(),
                        title: t.title.clone(),
                        boat: t
                            .boat
                            .map(|b| boat_name(&record.boats, b))
                            .unwrap_or_default(),
                        assignees: t.assignees.iter().map(|m| person(*m)).collect(),
                        done: t.done_at.map(format::date).unwrap_or_default(),
                        hours: format::hours(record.hours_for_task(t.id)),
                    },
                )
            })
            .collect();
        rows.sort_by_key(|r| std::cmp::Reverse(r.0));
        Ok(rows.into_iter().map(|r| r.1).collect())
    }

    /// Aufgabenseite mit Beschreibung, Kommentaren, Anhängen und Verlauf.
    pub async fn task_page(&self, user: &CurrentUser, id: TaskId) -> Result<TaskPage, AppError> {
        let principal = self.principal(user).await?;
        let scoped = self.load(&self.work_scope(MemberScope::All)).await?;
        let record = &scoped.record;
        let task = record
            .task(id)
            .ok_or_else(|| AppError::NotFound("Aufgabe".into()))?;
        let person = Self::person_lookup(record);
        let comments = self
            .store
            .comments(id)
            .await?
            .into_iter()
            .map(|c| {
                let own = c.author == user.member;
                let recent = principal.now - c.at <= Duration::minutes(COMMENT_WINDOW_MINUTES);
                CommentLine {
                    id: c.id.to_string(),
                    author: person(c.author),
                    at: date_time(c.at),
                    text: c.text,
                    can_edit: own && recent,
                    can_delete: (own && recent) || principal.is_board(),
                }
            })
            .collect();
        let attachments = self
            .store
            .attachments(id)
            .await?
            .into_iter()
            .map(|a| AttachmentLine {
                id: a.id.to_string(),
                filename: a.filename,
                is_image: a.kind == AttachmentKind::Image,
                alt: a.alt.unwrap_or_default(),
                size: file_size(a.size),
                by: person(a.uploader),
                at: date_time(a.at),
                can_delete: a.uploader == user.member || principal.is_board(),
            })
            .collect();
        let history = self
            .store
            .audit(&AuditFilter {
                object_id: Some(id.to_string()),
                ..AuditFilter::default()
            })
            .await?
            .into_iter()
            .filter(|r| r.entry.object_kind == "task" && r.entry.action != "task.comment_added")
            .map(|r| HistoryLine {
                at: date_time(r.entry.at),
                who: person(r.entry.actor),
                text: r.entry.summary,
            })
            .collect();
        let assigned = task.assignees.contains(&user.member);
        Ok(TaskPage {
            id: id.to_string(),
            title: task.title.clone(),
            status_key: status_key(task.status),
            status_label: status_label(task.status),
            version: scoped.version(RecordKind::Task, &id.to_string()).0,
            assignees: task.assignees.iter().map(|m| person(*m)).collect(),
            boat_id: task.boat.map(|b| b.to_string()).unwrap_or_default(),
            boat: task
                .boat
                .map(|b| boat_name(&record.boats, b))
                .unwrap_or_default(),
            due: task.due_at.map(format::date).unwrap_or_default(),
            due_input: task
                .due_at
                .map(|d| d.format("%Y-%m-%d").to_string())
                .unwrap_or_default(),
            overdue: is_overdue(task, principal.now),
            estimated: task.estimated_hours.map(format::hours).unwrap_or_default(),
            estimated_input: task.estimated_hours.map(format::number).unwrap_or_default(),
            hours: format::hours(
                record
                    .work_entries
                    .iter()
                    .filter(|e| e.reference == Some(WorkReference::Task(id)))
                    .map(|e| e.hours())
                    .sum(),
            ),
            description: task.description.clone().unwrap_or_default(),
            can_move: principal.can(&Capability::UpdateTask { assigned }),
            can_edit: principal.can(&Capability::UpdateTask { assigned }),
            can_manage: principal.can(&Capability::ManageTasks),
            archived: is_archived(task, principal.now),
            comments,
            attachments,
            history,
            boats: boat_options(&record.boats),
            members: active_members(record, &principal),
            max_attachments: MAX_ATTACHMENTS,
        })
    }

    /// Verschiebt eine Aufgabe in eine andere Spalte. Mit `expected` (Version
    /// aus der Anzeige) wird ein zwischenzeitlicher Stand als Konflikt abgelehnt.
    pub async fn move_task(
        &self,
        user: &CurrentUser,
        id: TaskId,
        status: TaskStatus,
        expected: Option<i64>,
    ) -> Result<(), AppError> {
        let principal = self.principal(user).await?;
        let scoped = self.load(&self.work_scope(MemberScope::None)).await?;
        if let Some(v) = expected
            && scoped.version(RecordKind::Task, &id.to_string()).0 != v
        {
            return Err(AppError::Conflict);
        }
        let mut new = scoped.record.clone();
        let task = new
            .tasks
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| AppError::NotFound("Aufgabe".into()))?;
        principal.require(&Capability::UpdateTask {
            assigned: task.assignees.contains(&user.member),
        })?;
        if task.status == status {
            return Ok(());
        }
        task.set_status(status, principal.now);
        let log = audit(
            &principal,
            "task.status",
            "task",
            id,
            None,
            format!("Aufgabenstatus geändert: {}", status_label(status)),
        );
        self.save(&scoped, &new, vec![log]).await
    }

    /// Titel, Boot, Fälligkeit und Schätzung ändern (Amtsträger).
    pub async fn edit_task(
        &self,
        user: &CurrentUser,
        id: TaskId,
        input: &TaskInput,
        expected: Option<i64>,
    ) -> Result<(), AppError> {
        let principal = self.principal(user).await?;
        principal.require(&Capability::ManageTasks)?;
        let scoped = self.load(&self.work_scope(MemberScope::None)).await?;
        if let Some(v) = expected
            && scoped.version(RecordKind::Task, &id.to_string()).0 != v
        {
            return Err(AppError::Conflict);
        }
        let fields = parse_task_fields(input, &scoped.record.boats)?;
        let mut new = scoped.record.clone();
        let task = new
            .tasks
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| AppError::NotFound("Aufgabe".into()))?;
        task.title = fields.title;
        task.boat = fields.boat;
        task.due_at = fields.due;
        task.estimated_hours = fields.estimated;
        let log = audit(
            &principal,
            "task.edit",
            "task",
            id,
            None,
            "Aufgabe geändert",
        );
        self.save(&scoped, &new, vec![log]).await
    }

    /// Entzieht einem Mitglied die Zuständigkeit (Amtsträger).
    pub async fn unassign_task(
        &self,
        user: &CurrentUser,
        id: TaskId,
        member: MemberId,
    ) -> Result<(), AppError> {
        let principal = self.principal(user).await?;
        principal.require(&Capability::ManageTasks)?;
        let scoped = self.load(&self.work_scope(MemberScope::None)).await?;
        let mut new = scoped.record.clone();
        let task = new
            .tasks
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| AppError::NotFound("Aufgabe".into()))?;
        if !task.unassign(member) {
            return Err(AppError::rejected(
                "member",
                "Das Mitglied war nicht zuständig.",
            ));
        }
        let log = audit(
            &principal,
            "task.unassign",
            "task",
            id,
            Some(member),
            "Zuständigkeit entzogen",
        );
        self.save(&scoped, &new, vec![log]).await
    }

    /// Ändert die Markdown-Beschreibung (Zuständige und Amtsträger).
    pub async fn set_task_description(
        &self,
        user: &CurrentUser,
        id: TaskId,
        text: &str,
        expected: Option<i64>,
    ) -> Result<(), AppError> {
        let principal = self.principal(user).await?;
        let scoped = self.load(&self.work_scope(MemberScope::None)).await?;
        if let Some(v) = expected
            && scoped.version(RecordKind::Task, &id.to_string()).0 != v
        {
            return Err(AppError::Conflict);
        }
        let mut new = scoped.record.clone();
        let task = new
            .tasks
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| AppError::NotFound("Aufgabe".into()))?;
        principal.require(&Capability::UpdateTask {
            assigned: task.assignees.contains(&user.member),
        })?;
        if text.chars().count() > MAX_DESCRIPTION_CHARS {
            return Err(AppError::rejected(
                "description",
                format!(
                    "Die Beschreibung darf höchstens {MAX_DESCRIPTION_CHARS} Zeichen lang sein."
                ),
            ));
        }
        let text = text.replace("\r\n", "\n");
        task.description = trimmed(&text);
        let log = audit(
            &principal,
            "task.description_changed",
            "task",
            id,
            None,
            "Beschreibung geändert",
        );
        self.save(&scoped, &new, vec![log]).await
    }

    async fn require_task(&self, id: TaskId) -> Result<(), AppError> {
        let scoped = self
            .load(&crate::scope::Scope {
                tasks: true,
                ..Default::default()
            })
            .await?;
        scoped
            .record
            .task(id)
            .map(|_| ())
            .ok_or_else(|| AppError::NotFound("Aufgabe".into()))
    }

    fn check_comment_text(text: &str) -> Result<String, AppError> {
        let text = text.replace("\r\n", "\n");
        let text = text.trim();
        if text.is_empty() {
            return Err(AppError::rejected(
                "text",
                "Bitte einen Kommentar eingeben.",
            ));
        }
        if text.chars().count() > MAX_COMMENT_CHARS {
            return Err(AppError::rejected(
                "text",
                format!("Ein Kommentar darf höchstens {MAX_COMMENT_CHARS} Zeichen lang sein."),
            ));
        }
        Ok(text.to_string())
    }

    pub async fn add_comment(
        &self,
        user: &CurrentUser,
        task: TaskId,
        text: &str,
    ) -> Result<(), AppError> {
        let principal = self.principal(user).await?;
        let text = Self::check_comment_text(text)?;
        self.require_task(task).await?;
        let comment = TaskComment {
            id: Uuid::new_v4(),
            task,
            author: user.member,
            at: principal.now,
            text,
        };
        let log = audit(
            &principal,
            "task.comment_added",
            "task",
            task,
            None,
            "Aufgabe kommentiert",
        );
        self.store.add_comment(&comment, log).await
    }

    async fn own_or_board_comment(
        &self,
        principal: &Principal,
        id: Uuid,
        board_may: bool,
    ) -> Result<TaskComment, AppError> {
        let comment = self
            .store
            .comment(id)
            .await?
            .ok_or_else(|| AppError::NotFound("Kommentar".into()))?;
        let own = comment.author == principal.member
            && principal.now - comment.at <= Duration::minutes(COMMENT_WINDOW_MINUTES);
        if own || (board_may && principal.is_board()) {
            Ok(comment)
        } else {
            Err(AppError::Forbidden)
        }
    }

    /// Korrigiert den eigenen Kommentar innerhalb von 15 Minuten.
    pub async fn edit_comment(
        &self,
        user: &CurrentUser,
        id: Uuid,
        text: &str,
    ) -> Result<TaskId, AppError> {
        let principal = self.principal(user).await?;
        let comment = self.own_or_board_comment(&principal, id, false).await?;
        let text = Self::check_comment_text(text)?;
        let log = audit(
            &principal,
            "task.comment_edited",
            "task",
            comment.task,
            None,
            "Kommentar korrigiert",
        );
        self.store.edit_comment(id, &text, log).await?;
        Ok(comment.task)
    }

    /// Löscht einen Kommentar: der Autor innerhalb von 15 Minuten, der Vorstand jederzeit.
    pub async fn delete_comment(&self, user: &CurrentUser, id: Uuid) -> Result<TaskId, AppError> {
        let principal = self.principal(user).await?;
        let comment = self.own_or_board_comment(&principal, id, true).await?;
        let log = audit(
            &principal,
            "task.comment_deleted",
            "task",
            comment.task,
            None,
            "Kommentar gelöscht",
        );
        self.store.delete_comment(id, log).await?;
        Ok(comment.task)
    }

    /// Lädt einen Anhang hoch; Inhalt, Größe und Anzahl werden geprüft.
    pub async fn add_attachment(
        &self,
        user: &CurrentUser,
        task: TaskId,
        filename: &str,
        data: Vec<u8>,
        alt: &str,
    ) -> Result<(), AppError> {
        let principal = self.principal(user).await?;
        self.require_task(task).await?;
        if data.is_empty() {
            return Err(AppError::rejected("file", "Bitte eine Datei auswählen."));
        }
        if data.len() > MAX_ATTACHMENT_BYTES {
            return Err(AppError::rejected(
                "file",
                format!(
                    "Die Datei ist zu groß (höchstens {} MB).",
                    MAX_ATTACHMENT_BYTES / 1_000_000
                ),
            ));
        }
        let Some((kind, content_type)) = detect_upload(&data) else {
            return Err(AppError::rejected("file", FORMAT_ERROR));
        };
        if self.store.attachments(task).await?.len() >= MAX_ATTACHMENTS {
            return Err(AppError::rejected(
                "file",
                format!("Je Aufgabe sind höchstens {MAX_ATTACHMENTS} Anhänge möglich."),
            ));
        }
        let alt: String = alt.trim().chars().take(200).collect();
        let attachment = TaskAttachment {
            id: Uuid::new_v4(),
            task,
            uploader: user.member,
            at: principal.now,
            kind,
            filename: sanitize_filename(filename),
            content_type: content_type.to_string(),
            alt: (!alt.is_empty()).then_some(alt),
            size: data.len(),
        };
        let log = audit(
            &principal,
            "task.attachment_added",
            "task",
            task,
            None,
            "Anhang hinzugefügt",
        );
        self.store.add_attachment(&attachment, data, log).await
    }

    /// Anhang zur Auslieferung (nur angemeldete aktive Mitglieder).
    pub async fn attachment_file(
        &self,
        user: &CurrentUser,
        id: Uuid,
    ) -> Result<(TaskAttachment, Vec<u8>), AppError> {
        self.principal(user).await?;
        self.store
            .attachment(id)
            .await?
            .ok_or_else(|| AppError::NotFound("Anhang".into()))
    }

    /// Löscht einen Anhang: der Hochladende oder der Vorstand.
    pub async fn delete_attachment(
        &self,
        user: &CurrentUser,
        id: Uuid,
    ) -> Result<TaskId, AppError> {
        let principal = self.principal(user).await?;
        let meta = self
            .store
            .attachment_meta(id)
            .await?
            .ok_or_else(|| AppError::NotFound("Anhang".into()))?;
        if meta.uploader != user.member && !principal.is_board() {
            return Err(AppError::Forbidden);
        }
        let log = audit(
            &principal,
            "task.attachment_removed",
            "task",
            meta.task,
            None,
            "Anhang entfernt",
        );
        self.store.delete_attachment(id, log).await?;
        Ok(meta.task)
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

fn active_members(
    record: &vereinsakte_core::club_record::ClubRecord,
    principal: &Principal,
) -> Vec<(u32, String)> {
    let mut members: Vec<(u32, String)> = record
        .members
        .iter()
        .filter(|m| m.is_active_at(principal.now))
        .map(|m| (m.id.get(), m.full_name()))
        .collect();
    members.sort_by(|a, b| a.1.cmp(&b.1));
    members
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use vereinsakte_core::example::{
        BEN, CLARA, EVA, FOCK_NAEHEN, GRETA, SEGELTUCH, TRIGON_ANTIFOULING, WEBSEITE,
    };
    use vereinsakte_core::time::club_time;

    use super::*;
    use crate::ports::FixedClock;
    use crate::service::tests::{service, service_at};

    fn user(member: MemberId) -> CurrentUser {
        CurrentUser { member }
    }

    fn column<'a>(board: &'a BoardView, key: &str) -> &'a BoardColumn {
        board.columns.iter().find(|c| c.key == key).unwrap()
    }

    #[test]
    fn uploads_are_recognised_by_content() {
        let png = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0];
        assert_eq!(detect_upload(&png).unwrap().1, "image/png");
        assert_eq!(
            detect_upload(&[0xFF, 0xD8, 0xFF, 0xE0]).unwrap().1,
            "image/jpeg"
        );
        assert_eq!(detect_upload(b"GIF89a....").unwrap().1, "image/gif");
        assert_eq!(
            detect_upload(b"RIFF\0\0\0\0WEBPVP8 ").unwrap().1,
            "image/webp"
        );
        assert_eq!(
            detect_upload(b"%PDF-1.7\n").unwrap(),
            (AttachmentKind::Pdf, "application/pdf")
        );
        assert!(detect_upload(b"hello world, not a png").is_none());
        assert!(detect_upload(b"<svg xmlns='http://www.w3.org/2000/svg'/>").is_none());
        assert!(detect_upload(b"PK\x03\x04 word document").is_none());
        assert!(detect_upload(b"").is_none());
    }

    #[test]
    fn filenames_are_cleaned() {
        assert_eq!(sanitize_filename("../../etc/passwd"), "passwd");
        assert_eq!(
            sanitize_filename("C:\\Users\\x\\Angebot Segeltuch.pdf"),
            "Angebot Segeltuch.pdf"
        );
        assert_eq!(sanitize_filename("a\"b;c\n.pdf"), "abc.pdf");
        assert_eq!(sanitize_filename("   "), "datei");
        assert_eq!(sanitize_filename(&"x".repeat(300)).chars().count(), 100);
    }

    #[tokio::test]
    async fn board_has_four_columns_and_hides_old_done_tasks() {
        let (service, _) = service();
        let board = service
            .board(&user(GRETA), &BoardFilter::default())
            .await
            .unwrap();
        let keys: Vec<_> = board.columns.iter().map(|c| c.key).collect();
        assert_eq!(keys, ["open", "in_progress", "waiting", "done"]);
        assert_eq!(column(&board, "waiting").cards.len(), 1);
        assert_eq!(column(&board, "in_progress").cards.len(), 1);
        // Im März 2026 erledigt, jetzt Oktober 2026: gleiche Saison, bleibt in DONE.
        assert_eq!(column(&board, "done").cards.len(), 3);

        let (next_year, _) = service_at(club_time(2027, 1, 12, 9, 0));
        let board = next_year
            .board(&user(GRETA), &BoardFilter::default())
            .await
            .unwrap();
        assert!(column(&board, "done").cards.is_empty());
        assert_eq!(column(&board, "waiting").cards.len(), 1);
        assert_eq!(next_year.archive_tasks(&user(EVA)).await.unwrap().len(), 3);
        assert_eq!(
            next_year.archive_tasks(&user(GRETA)).await,
            Err(AppError::Forbidden)
        );
    }

    #[tokio::test]
    async fn reopening_brings_an_archived_task_back() {
        let (service, store) = service_at(club_time(2027, 1, 12, 9, 0));
        service
            .move_task(&user(BEN), FOCK_NAEHEN, TaskStatus::InProgress, None)
            .await
            .unwrap();
        let board = service
            .board(&user(BEN), &BoardFilter::default())
            .await
            .unwrap();
        assert!(
            column(&board, "in_progress")
                .cards
                .iter()
                .any(|c| c.id == FOCK_NAEHEN.to_string())
        );
        assert_eq!(service.archive_tasks(&user(EVA)).await.unwrap().len(), 2);
        assert_eq!(store.snapshot().task(FOCK_NAEHEN).unwrap().done_at, None);
    }

    #[tokio::test]
    async fn filters_mine_and_boat() {
        let (service, _) = service();
        let mine = service
            .board(
                &user(BEN),
                &BoardFilter {
                    mine: true,
                    boat: None,
                },
            )
            .await
            .unwrap();
        let all: Vec<_> = mine.columns.iter().flat_map(|c| &c.cards).collect();
        assert_eq!(all.len(), 2);
        assert!(
            all.iter()
                .all(|c| c.assignees.iter().any(|p| p.id == Some(BEN.get())))
        );
        let boat = vereinsakte_core::example::TRIGON;
        let by_boat = service
            .board(
                &user(BEN),
                &BoardFilter {
                    mine: false,
                    boat: Some(boat),
                },
            )
            .await
            .unwrap();
        assert_eq!(by_boat.columns.iter().flat_map(|c| &c.cards).count(), 1);
    }

    #[tokio::test]
    async fn moving_needs_rights_and_current_version() {
        let (service, store) = service();
        // Greta ist weder zuständig noch Amtsträgerin.
        assert_eq!(
            service
                .move_task(&user(GRETA), SEGELTUCH, TaskStatus::InProgress, None)
                .await,
            Err(AppError::Forbidden)
        );
        let board = service
            .board(&user(BEN), &BoardFilter::default())
            .await
            .unwrap();
        let card = column(&board, "waiting").cards[0].clone();
        assert!(card.can_move);
        service
            .move_task(
                &user(BEN),
                SEGELTUCH,
                TaskStatus::InProgress,
                Some(card.version),
            )
            .await
            .unwrap();
        // Zweiter Verschiebeversuch auf Basis des alten Standes.
        assert_eq!(
            service
                .move_task(&user(BEN), SEGELTUCH, TaskStatus::Done, Some(card.version))
                .await,
            Err(AppError::Conflict)
        );
        assert_eq!(
            store.snapshot().task(SEGELTUCH).unwrap().status,
            TaskStatus::InProgress
        );
    }

    #[tokio::test]
    async fn description_rules() {
        let (service, _) = service();
        assert_eq!(
            service
                .set_task_description(&user(GRETA), SEGELTUCH, "x", None)
                .await,
            Err(AppError::Forbidden)
        );
        let long = "a".repeat(MAX_DESCRIPTION_CHARS + 1);
        assert!(matches!(
            service
                .set_task_description(&user(BEN), SEGELTUCH, &long, None)
                .await,
            Err(AppError::Rejected(_))
        ));
        service
            .set_task_description(&user(BEN), SEGELTUCH, "**Wichtig:** Fock", None)
            .await
            .unwrap();
        let page = service.task_page(&user(GRETA), SEGELTUCH).await.unwrap();
        assert_eq!(page.description, "**Wichtig:** Fock");
        assert!(!page.can_edit && !page.can_manage);
        // Verlauf nennt Änderungen, aber nie den Text.
        assert!(page.history.iter().all(|h| !h.text.contains("Wichtig")));
        assert!(!page.history.is_empty());
    }

    #[tokio::test]
    async fn comments_follow_the_fifteen_minute_rule() {
        let (service, store) = service();
        assert!(matches!(
            service.add_comment(&user(GRETA), WEBSEITE, "   ").await,
            Err(AppError::Rejected(e)) if e[0].field == "text"
        ));
        service
            .add_comment(&user(GRETA), WEBSEITE, "Segeltuch ist bestellt")
            .await
            .unwrap();
        let page = service.task_page(&user(GRETA), WEBSEITE).await.unwrap();
        assert_eq!(page.comments.len(), 1);
        let line = &page.comments[0];
        assert!(line.can_edit && line.can_delete);
        let id: Uuid = line.id.parse().unwrap();
        // Fremde dürfen weder ändern noch löschen; der Vorstand darf löschen.
        assert_eq!(
            service.edit_comment(&user(CLARA), id, "x").await,
            Err(AppError::Forbidden)
        );
        assert_eq!(
            service.delete_comment(&user(CLARA), id).await,
            Err(AppError::Forbidden)
        );
        // Nach 30 Minuten ist die Korrektur nicht mehr möglich.
        let later = ClubService::new(
            store.clone(),
            Arc::new(FixedClock(club_time(2026, 10, 9, 12, 30))),
        );
        assert_eq!(
            later.edit_comment(&user(GRETA), id, "neu").await,
            Err(AppError::Forbidden)
        );
        assert_eq!(
            later.delete_comment(&user(GRETA), id).await,
            Err(AppError::Forbidden)
        );
        let page = later.task_page(&user(GRETA), WEBSEITE).await.unwrap();
        assert!(!page.comments[0].can_edit && !page.comments[0].can_delete);
        let board = later
            .board(&user(GRETA), &BoardFilter::default())
            .await
            .unwrap();
        let card = column(&board, "in_progress")
            .cards
            .iter()
            .find(|c| c.id == WEBSEITE.to_string())
            .unwrap()
            .clone();
        assert_eq!(card.comments, 1);
        later.delete_comment(&user(EVA), id).await.unwrap();
        assert!(
            later
                .task_page(&user(GRETA), WEBSEITE)
                .await
                .unwrap()
                .comments
                .is_empty()
        );
        // Das Protokoll enthält keinen Kommentartext.
        assert!(
            store
                .audit_entries()
                .iter()
                .all(|e| !e.summary.contains("Segeltuch"))
        );
    }

    #[tokio::test]
    async fn attachment_rules() {
        let (service, _) = service();
        let png = || {
            let mut d = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
            d.extend_from_slice(&[0; 32]);
            d
        };
        let field_error = |r: Result<(), AppError>| match r {
            Err(AppError::Rejected(e)) => e[0].message.clone(),
            other => panic!("erwartet abgelehnt, war {other:?}"),
        };
        assert_eq!(
            field_error(
                service
                    .add_attachment(&user(BEN), SEGELTUCH, "bild.png", b"just text".to_vec(), "")
                    .await
            ),
            FORMAT_ERROR
        );
        assert_eq!(
            field_error(
                service
                    .add_attachment(
                        &user(BEN),
                        SEGELTUCH,
                        "a.svg",
                        b"<svg onload=x>".to_vec(),
                        ""
                    )
                    .await
            ),
            FORMAT_ERROR
        );
        let mut huge = png();
        huge.resize(MAX_ATTACHMENT_BYTES + 1, 0);
        assert!(
            field_error(
                service
                    .add_attachment(&user(BEN), SEGELTUCH, "g.png", huge, "")
                    .await
            )
            .contains("zu groß")
        );
        service
            .add_attachment(&user(BEN), SEGELTUCH, "Schaden.png", png(), "Riss im Segel")
            .await
            .unwrap();
        service
            .add_attachment(
                &user(BEN),
                SEGELTUCH,
                "../Angebot Segeltuch.pdf",
                b"%PDF-1.4 x".to_vec(),
                "",
            )
            .await
            .unwrap();
        let page = service.task_page(&user(GRETA), SEGELTUCH).await.unwrap();
        assert_eq!(page.attachments.len(), 2);
        assert!(page.attachments[0].is_image && !page.attachments[1].is_image);
        assert_eq!(page.attachments[1].filename, "Angebot Segeltuch.pdf");
        let board = service
            .board(&user(GRETA), &BoardFilter::default())
            .await
            .unwrap();
        let card = &column(&board, "waiting").cards[0];
        assert_eq!((card.attachments, card.images), (2, 1));
        // Auslieferung mit festem Typ; fremde Löschversuche scheitern, Vorstand darf.
        let id: Uuid = page.attachments[1].id.parse().unwrap();
        let (meta, data) = service.attachment_file(&user(GRETA), id).await.unwrap();
        assert_eq!(meta.content_type, "application/pdf");
        assert!(data.starts_with(b"%PDF-"));
        assert_eq!(
            service.delete_attachment(&user(GRETA), id).await,
            Err(AppError::Forbidden)
        );
        service.delete_attachment(&user(EVA), id).await.unwrap();
        // Höchstens 20 Anhänge.
        for i in 0..MAX_ATTACHMENTS - 1 {
            service
                .add_attachment(&user(BEN), SEGELTUCH, &format!("{i}.png"), png(), "")
                .await
                .unwrap();
        }
        assert!(
            field_error(
                service
                    .add_attachment(&user(BEN), SEGELTUCH, "z.png", png(), "")
                    .await
            )
            .contains("höchstens 20")
        );
    }

    #[tokio::test]
    async fn editing_and_assigning_is_for_office_holders() {
        let (service, _) = service();
        let input = TaskInput {
            title: "Fock nähen".into(),
            boat: String::new(),
            due: "2026-12-01".into(),
            hours: "3,5".into(),
            ..Default::default()
        };
        assert_eq!(
            service
                .edit_task(&user(GRETA), TRIGON_ANTIFOULING, &input, None)
                .await,
            Err(AppError::Forbidden)
        );
        service
            .edit_task(&user(BEN), TRIGON_ANTIFOULING, &input, None)
            .await
            .unwrap();
        let page = service
            .task_page(&user(BEN), TRIGON_ANTIFOULING)
            .await
            .unwrap();
        assert_eq!(
            (
                page.title.as_str(),
                page.due.as_str(),
                page.estimated.as_str()
            ),
            ("Fock nähen", "01.12.2026", "3,5")
        );
        assert_eq!(page.due_input, "2026-12-01");
        let bad = TaskInput {
            title: String::new(),
            ..input
        };
        assert!(matches!(
            service
                .edit_task(&user(BEN), TRIGON_ANTIFOULING, &bad, None)
                .await,
            Err(AppError::Rejected(_))
        ));
        service
            .unassign_task(&user(BEN), TRIGON_ANTIFOULING, CLARA)
            .await
            .unwrap();
        assert!(matches!(
            service
                .unassign_task(&user(BEN), TRIGON_ANTIFOULING, CLARA)
                .await,
            Err(AppError::Rejected(_))
        ));
    }

    #[tokio::test]
    async fn overdue_open_tasks_are_marked() {
        let (service, _) = service_at(club_time(2026, 11, 20, 9, 0));
        let board = service
            .board(&user(BEN), &BoardFilter::default())
            .await
            .unwrap();
        assert!(column(&board, "waiting").cards[0].overdue);
        assert!(column(&board, "done").cards.iter().all(|c| !c.overdue));
    }
}
