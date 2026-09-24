//! Dated host tasks — the `workspace-timeline-task` surface.
//!
//! A module declaring that surface serves [`TIMELINE_TASKS`]: the platform hands it the stays of
//! the window ([`TimelineTasksArgs::stays`], sorted, one stay past each end so « before the next
//! arrival » can be computed) and places the returned tasks between them. Tasks are computed,
//! not stored; only what was ticked is. The module then serves [`TASK_TOGGLE`] and
//! [`TASK_COMPLETE`], refuses them with [`PHOTO_REQUIRED`] when an item needs a photo it did not
//! get, and emits [`TASK_UPDATED`] with a [`TaskUpdated`] payload. Schema:
//! `contracts/timeline-tasks.v1.json`.
//!
//! ```
//! use chrono::{TimeZone, Utc};
//! use portaki_sdk::contracts::i18n::I18nText;
//! use portaki_sdk::contracts::timeline::{self, TimelineTaskItem};
//! use uuid::Uuid;
//!
//! let at = Utc.with_ymd_and_hms(2026, 9, 26, 10, 0, 0).unwrap();
//! let stay = Uuid::nil();
//! let task = timeline::task(
//!     format!("cleaning:{stay}"),
//!     at,
//!     Uuid::nil(),
//!     I18nText::new("Ménage", "Cleaning"),
//!     I18nText::new("Avant l'arrivée de Liam", "Before Liam arrives"),
//! )
//! .stay(stay)
//! .items(vec![TimelineTaskItem::new("floors", I18nText::new("Sols", "Floors")).photo_required()]);
//!
//! assert!(task.items[0].check_toggle(true, None).is_err());
//! assert_eq!(serde_json::to_value(&task).unwrap()["at"], "2026-09-26T10:00:00Z");
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::contracts::i18n::I18nText;
use crate::error::{PortakiError, Result};
use crate::ids::{EventType, OperationName};

/// Query name (`timelineTasks`).
pub const TIMELINE_TASKS: OperationName = OperationName::new("timelineTasks");

/// Command ticking or unticking one item (`taskToggle`), args [`TaskToggleArgs`].
pub const TASK_TOGGLE: OperationName = OperationName::new("taskToggle");

/// Command ticking every item of a task (`taskComplete`), args [`TaskCompleteArgs`].
pub const TASK_COMPLETE: OperationName = OperationName::new("taskComplete");

/// Event the module emits after a toggle or a completion (`checklist.task-updated`), payload
/// [`TaskUpdated`]. The platform pushes it to the workspace members.
pub const TASK_UPDATED: EventType = EventType::new("checklist.task-updated");

/// Error code refusing a tick without the photo the item requires.
pub const PHOTO_REQUIRED: &str = "photo_required";

/// Args of [`TIMELINE_TASKS`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineTasksArgs {
    /// The property.
    pub property_id: Uuid,
    /// Window start.
    pub from: DateTime<Utc>,
    /// Window end.
    pub to: DateTime<Utc>,
    /// Stays of the window, sorted by check-in, with one more on each side.
    #[serde(default)]
    pub stays: Vec<TimelineStay>,
}

/// A stay as [`TimelineTasksArgs`] carries it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineStay {
    /// The stay.
    pub id: Uuid,
    /// Check-in instant.
    pub check_in: DateTime<Utc>,
    /// Check-out instant.
    pub check_out: DateTime<Utc>,
    /// Guest display name.
    pub guest_name: String,
    /// Platform stay status (`UPCOMING`, `ACTIVE`, …).
    pub status: String,
}

/// Answer of [`TIMELINE_TASKS`].
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineTasks {
    /// The tasks of the window.
    pub tasks: Vec<TimelineTask>,
}

/// A dated host task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineTask {
    /// Stable id, the same on every computation (`cleaning:<stayId>`).
    pub id: String,
    /// Where the task sits on the timeline.
    pub at: DateTime<Utc>,
    /// When it must be done by, if it must.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due_at: Option<DateTime<Utc>>,
    /// The stay it follows or prepares, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stay_id: Option<Uuid>,
    /// The property.
    pub property_id: Uuid,
    /// Title (`Ménage`).
    pub title: I18nText,
    /// One line of context (`Avant l'arrivée de Liam`).
    pub context: I18nText,
    /// Who does it, if anyone is named.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assignee: Option<TaskAssignee>,
    /// What to tick — never empty.
    pub items: Vec<TimelineTaskItem>,
}

/// Who a [`TimelineTask`] is assigned to — free text, not necessarily a workspace member.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskAssignee {
    /// Display name.
    pub name: String,
    /// Role (`Ménage`).
    pub role: I18nText,
}

/// One item of a [`TimelineTask`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineTaskItem {
    /// Stable id within the task.
    pub id: String,
    /// What to do.
    pub label: I18nText,
    /// Ticked.
    pub done: bool,
    /// Can only be ticked with a photo.
    #[serde(default)]
    pub photo_required: bool,
    /// The photo attached when ticked (`portaki-file:<id>`, see [`crate::files::FileRef`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub photo: Option<String>,
}

/// Args of [`TASK_TOGGLE`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskToggleArgs {
    /// The property.
    pub property_id: Uuid,
    /// [`TimelineTask::id`].
    pub task_id: String,
    /// [`TimelineTaskItem::id`].
    pub item_id: String,
    /// Tick or untick.
    pub done: bool,
    /// Photo uploaded with the tick (`portaki-file:<id>`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub photo: Option<String>,
}

/// Args of [`TASK_COMPLETE`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskCompleteArgs {
    /// The property.
    pub property_id: Uuid,
    /// [`TimelineTask::id`].
    pub task_id: String,
}

/// Payload of [`TASK_UPDATED`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskUpdated {
    /// The property.
    pub property_id: Uuid,
    /// The stay of the task, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stay_id: Option<Uuid>,
    /// [`TimelineTask::id`].
    pub task_id: String,
    /// Items ticked.
    pub done: u32,
    /// Items in the task.
    pub total: u32,
    /// [`TaskAssignee::name`], if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assignee_name: Option<String>,
}

/// A task at `at`, with no stay, deadline, assignee or item yet.
pub fn task(
    id: impl Into<String>,
    at: DateTime<Utc>,
    property_id: Uuid,
    title: I18nText,
    context: I18nText,
) -> TimelineTask {
    TimelineTask {
        id: id.into(),
        at,
        due_at: None,
        stay_id: None,
        property_id,
        title,
        context,
        assignee: None,
        items: Vec::new(),
    }
}

impl TimelineTask {
    /// Ties the task to a stay.
    pub fn stay(mut self, stay_id: Uuid) -> Self {
        self.stay_id = Some(stay_id);
        self
    }

    /// Sets the deadline.
    pub fn due_at(mut self, due_at: DateTime<Utc>) -> Self {
        self.due_at = Some(due_at);
        self
    }

    /// Names who does it.
    pub fn assignee(mut self, name: impl Into<String>, role: I18nText) -> Self {
        self.assignee = Some(TaskAssignee {
            name: name.into(),
            role,
        });
        self
    }

    /// Sets the items.
    pub fn items(mut self, items: Vec<TimelineTaskItem>) -> Self {
        self.items = items;
        self
    }
}

impl TimelineTaskItem {
    /// An unticked item that needs no photo.
    pub fn new(id: impl Into<String>, label: I18nText) -> Self {
        Self {
            id: id.into(),
            label,
            done: false,
            photo_required: false,
            photo: None,
        }
    }

    /// Requires a photo to be ticked.
    pub fn photo_required(mut self) -> Self {
        self.photo_required = true;
        self
    }

    /// Refuses ticking this item without the photo it requires — [`PHOTO_REQUIRED`].
    ///
    /// Call it from [`TASK_TOGGLE`] with the args, and from [`TASK_COMPLETE`] with `done: true`
    /// and the photo already stored, for every item.
    pub fn check_toggle(&self, done: bool, photo: Option<&str>) -> Result<()> {
        let has_photo = photo.is_some_and(|p| !p.trim().is_empty());
        if done && self.photo_required && !has_photo {
            return Err(PortakiError::Host(format!(
                "{PHOTO_REQUIRED}: item `{}` needs a photo to be ticked",
                self.id
            )));
        }
        Ok(())
    }
}
