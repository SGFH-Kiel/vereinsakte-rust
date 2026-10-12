//! Einzelaufgaben, die Mitglieder allein erledigen.

use serde::{Deserialize, Serialize};

use super::club_record::ClubRecord;
use super::ids::{BoatId, MemberId, TaskId};
use super::time::Timestamp;
use super::work_hours::WorkReference;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Open,
    InProgress,
    /// Wartet auf etwas oder jemanden.
    Waiting,
    Done,
}

/// Kommentar zu einer Aufgabe (Markdown).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskComment {
    pub id: uuid::Uuid,
    pub task: TaskId,
    pub author: MemberId,
    pub at: Timestamp,
    pub text: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttachmentKind {
    Image,
    Pdf,
}

impl AttachmentKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Image => "image",
            Self::Pdf => "pdf",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "image" => Some(Self::Image),
            "pdf" => Some(Self::Pdf),
            _ => None,
        }
    }
}

/// Metadaten eines Anhangs; die Dateidaten werden getrennt gelesen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskAttachment {
    pub id: uuid::Uuid,
    pub task: TaskId,
    pub uploader: MemberId,
    pub at: Timestamp,
    pub kind: AttachmentKind,
    pub filename: String,
    pub content_type: String,
    /// Bildbeschreibung (Bild) bzw. Bezeichnung (PDF).
    pub alt: Option<String>,
    pub size: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Task {
    pub id: TaskId,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Betroffenes Boot; Aufgaben wie Webseite oder Vereinszeitung haben keins.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boat: Option<BoatId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assignees: Vec<MemberId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_hours: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due_at: Option<Timestamp>,
    pub status: TaskStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub done_at: Option<Timestamp>,
}

impl Task {
    /// Neue, offene Aufgabe.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            id: TaskId::new(),
            title: title.into(),
            description: None,
            boat: None,
            assignees: Vec::new(),
            estimated_hours: None,
            due_at: None,
            status: TaskStatus::Open,
            done_at: None,
        }
    }

    /// Weist ein Mitglied zu; liefert `false`, wenn es schon zugewiesen war.
    pub fn assign(&mut self, member: MemberId) -> bool {
        if self.assignees.contains(&member) {
            return false;
        }
        self.assignees.push(member);
        true
    }

    /// Entzieht die Zuständigkeit; liefert `false`, wenn das Mitglied nicht zugewiesen war.
    pub fn unassign(&mut self, member: MemberId) -> bool {
        let before = self.assignees.len();
        self.assignees.retain(|&m| m != member);
        self.assignees.len() != before
    }

    /// Setzt den Status; „erledigt“ hält den Zeitpunkt fest, jeder andere
    /// Status (auch „wartet“) löscht ihn.
    pub fn set_status(&mut self, status: TaskStatus, t: Timestamp) {
        self.status = status;
        self.done_at = (status == TaskStatus::Done).then_some(t);
    }
}

impl ClubRecord {
    pub fn task(&self, id: TaskId) -> Option<&Task> {
        self.tasks.iter().find(|t| t.id == id)
    }

    /// Summe aller Arbeitseinsätze (bestätigt oder nicht) mit Bezug auf die Aufgabe.
    pub fn hours_for_task(&self, task: TaskId) -> f64 {
        self.work_entries
            .iter()
            .filter(|e| e.reference == Some(WorkReference::Task(task)))
            .map(|e| e.hours())
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::example::{self, at, *};
    use crate::work_hours::{WorkEntry, WorkExtent, WorkSubject};

    #[test]
    fn new_task_is_open() {
        let mut task = Task::new("Fock von Harald Blauzahn nähen");
        task.boat = Some(HARALD_BLAUZAHN);
        task.estimated_hours = Some(4.0);
        assert!(task.assign(BEN));
        assert_eq!(task.status, TaskStatus::Open);
        assert_eq!(task.assignees, vec![BEN]);
        assert_eq!(task.boat, Some(HARALD_BLAUZAHN));
    }

    #[test]
    fn assignment_is_unique() {
        let mut task = Task::new("Rettungswesten prüfen");
        assert!(task.assign(BEN));
        assert!(!task.assign(BEN));
        assert_eq!(task.assignees, vec![BEN]);
        assert!(task.unassign(BEN));
        assert!(!task.unassign(BEN));
    }

    #[test]
    fn done_sets_and_reset_clears_timestamp() {
        let mut task = Task::new("Pinne der Filou lackieren");
        task.set_status(TaskStatus::Done, at(2026, 5, 20, 18, 0));
        assert_eq!(task.status, TaskStatus::Done);
        assert_eq!(task.done_at, Some(at(2026, 5, 20, 18, 0)));
        task.set_status(TaskStatus::InProgress, at(2026, 5, 21, 9, 0));
        assert_eq!(task.done_at, None);
    }

    #[test]
    fn waiting_clears_done_timestamp() {
        let mut task = Task::new("Segeltuch bestellen");
        task.set_status(TaskStatus::Done, at(2026, 5, 20, 18, 0));
        task.set_status(TaskStatus::Waiting, at(2026, 5, 21, 9, 0));
        assert_eq!(task.status, TaskStatus::Waiting);
        assert_eq!(task.done_at, None);
        assert_eq!(
            serde_json::to_string(&TaskStatus::Waiting).unwrap(),
            "\"waiting\""
        );
    }

    #[test]
    fn hours_per_task() {
        let mut club = example::club();
        assert_eq!(club.hours_for_task(TRIGON_ANTIFOULING), 10.0);

        let task = Task::new("Großschot erneuern");
        let id = task.id;
        club.tasks.push(task);
        for (member, hours) in [(BEN, 2.5), (CLARA, 1.5)] {
            let mut entry = WorkEntry::new(
                member,
                WorkExtent::Amount {
                    date: at(2026, 6, 1, 0, 0),
                    hours,
                },
                WorkSubject::Club,
            );
            entry.reference = Some(WorkReference::Task(id));
            club.work_entries.push(entry);
        }
        assert_eq!(club.hours_for_task(id), 4.0);
    }

    #[test]
    fn tasks_survive_round_trip() {
        for task in example::tasks() {
            let json = serde_json::to_string(&task).unwrap();
            assert_eq!(serde_json::from_str::<Task>(&json).unwrap(), task);
        }
    }
}
