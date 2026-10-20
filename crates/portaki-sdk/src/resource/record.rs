//! Records — what a guest (or the host for him) deposits during a stay and the host follows:
//! one item, a status on a [`Lifecycle`], a history. Declared with `#[record]`; stored one per
//! row of `PortakiResource`, with the stay as a column so purging a stay takes its records.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use uuid::Uuid;

use super::store::PortakiResource;
use super::{Choice, Item, Lifecycle};
use crate::contracts::i18n::I18nText;
use crate::contracts::timeline::{self, TimelineTaskItem, TimelineTasks, TimelineTasksArgs};
use crate::error::{PortakiError, Result};
use crate::host::repo::{self, eq, Query};
use crate::Context;

/// What `#[record]` declares.
#[derive(Debug, Clone, Copy)]
pub struct RecordDecl {
    /// The resource name.
    pub name: &'static str,
    /// An open record older than this many hours is a task in the host's timeline.
    pub due_hours: Option<i64>,
}

/// Who wrote.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Actor {
    /// The guest, from the booklet.
    Guest,
    /// The host, from the dashboard.
    Host,
}

/// One line of a record's history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryLine {
    /// When.
    pub at: DateTime<Utc>,
    /// Who.
    pub actor: Actor,
    /// What happened.
    #[serde(flatten)]
    pub event: HistoryEvent,
}

/// What a history line records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum HistoryEvent {
    /// The record was made.
    Created,
    /// The status became `to` (its wire value).
    Status {
        /// The new status.
        to: String,
    },
    /// A note — the host's reply to the guest.
    Note {
        /// Its text.
        text: String,
    },
}

/// The bookkeeping stored next to the item.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
struct Bookkeeping {
    status: String,
    actor: Option<Actor>,
    guest_name: String,
    check_in: Option<DateTime<Utc>>,
    check_out: Option<DateTime<Utc>>,
    created_at: Option<DateTime<Utc>>,
    history: Vec<HistoryLine>,
}

/// One record, as stored.
#[derive(Debug, Clone)]
pub struct Entry<T, S> {
    /// Its id.
    pub id: Uuid,
    /// What was deposited.
    pub item: T,
    /// Where it stands.
    pub status: S,
    /// Its stay.
    pub stay_id: Option<Uuid>,
    /// When it was made.
    pub created_at: DateTime<Utc>,
    /// Who made it.
    pub actor: Actor,
    /// The guest's name when it was made; empty when unknown.
    pub guest_name: String,
    /// The stay's dates when it was made.
    pub check_in: Option<DateTime<Utc>>,
    /// The stay's dates when it was made.
    pub check_out: Option<DateTime<Utc>>,
    /// What happened to it, oldest first.
    pub history: Vec<HistoryLine>,
}

impl<T: Item, S: Lifecycle> Entry<T, S> {
    /// Nothing more to do.
    pub fn is_done(&self) -> bool {
        self.status == S::DONE
    }

    /// The notes (the host's replies), oldest first.
    pub fn notes(&self) -> impl Iterator<Item = &str> {
        self.history.iter().filter_map(|line| match &line.event {
            HistoryEvent::Note { text } => Some(text.as_str()),
            _ => None,
        })
    }

    fn from_row(row: PortakiResource) -> Result<Self> {
        let book: Bookkeeping = serde_json::from_str(&row.meta_json).map_err(corrupt)?;
        Ok(Self {
            id: row.id,
            item: serde_json::from_str(&row.rows_json).map_err(corrupt)?,
            status: S::from_wire(&book.status).unwrap_or(S::INITIAL),
            stay_id: row.stay_id,
            created_at: book.created_at.unwrap_or(row.updated_at),
            actor: book.actor.unwrap_or(Actor::Guest),
            guest_name: book.guest_name,
            check_in: book.check_in,
            check_out: book.check_out,
            history: book.history,
        })
    }

    /// Stores the record.
    pub fn save(&self, decl: RecordDecl) -> Result<()> {
        let book = Bookkeeping {
            status: self.status.wire().to_string(),
            actor: Some(self.actor),
            guest_name: self.guest_name.clone(),
            check_in: self.check_in,
            check_out: self.check_out,
            created_at: Some(self.created_at),
            history: self.history.clone(),
        };
        let row = PortakiResource {
            id: self.id,
            resource: decl.name.to_string(),
            rows_json: serde_json::to_string(&self.item)?,
            meta_json: serde_json::to_string(&book)?,
            stay_id: self.stay_id,
            updated_at: crate::host::time::now()?,
        };
        let _: PortakiResource = repo::create::<PortakiResource, _, _>(row)?;
        Ok(())
    }
}

/// Every record of `decl`, newest first.
pub fn list<T: Item, S: Lifecycle>(decl: RecordDecl) -> Result<Vec<Entry<T, S>>> {
    rows(decl, Query::new().r#where(eq("resource", decl.name)))
}

/// The records of one stay, newest first.
pub fn for_stay<T: Item, S: Lifecycle>(decl: RecordDecl, stay: Uuid) -> Result<Vec<Entry<T, S>>> {
    rows(
        decl,
        Query::new()
            .r#where(eq("resource", decl.name))
            .r#where(eq("stay_id", stay)),
    )
}

/// One record.
pub fn get<T: Item, S: Lifecycle>(decl: RecordDecl, id: Uuid) -> Result<Option<Entry<T, S>>> {
    Ok(list::<T, S>(decl)?.into_iter().find(|entry| entry.id == id))
}

fn rows<T: Item, S: Lifecycle>(
    _decl: RecordDecl,
    query: Query<PortakiResource>,
) -> Result<Vec<Entry<T, S>>> {
    let page = repo::find::<PortakiResource, PortakiResource>(query)?;
    let mut entries = page
        .items
        .into_iter()
        .map(Entry::from_row)
        .collect::<Result<Vec<_>>>()?;
    entries.sort_by_key(|entry| std::cmp::Reverse(entry.created_at));
    Ok(entries)
}

/// What a module adds to the generated operations of a record — every hook optional.
pub trait RecordHooks {
    /// The item.
    type Item: Item;
    /// Its lifecycle.
    type Status: Lifecycle;

    /// Before a guest's deposit is stored: refuse it (`Err`) when the module's settings say no.
    fn guard(_ctx: &Context, _item: &Self::Item) -> Result<()> {
        Ok(())
    }

    /// After a record is stored — the email to the host. An error is logged, never undone.
    fn on_created(_ctx: &Context, _entry: &Entry<Self::Item, Self::Status>) -> Result<()> {
        Ok(())
    }

    /// After the host adds a note — the email to the guest. An error is logged, never undone.
    fn on_note(
        _ctx: &Context,
        _entry: &Entry<Self::Item, Self::Status>,
        _text: &str,
    ) -> Result<()> {
        Ok(())
    }
}

/// The arguments of `setStatus`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetStatusArgs {
    /// The record.
    pub id: Uuid,
    /// Its new status (wire value).
    pub status: String,
}

/// The arguments of `note`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteArgs {
    /// The record.
    pub id: Uuid,
    /// The text.
    #[serde(default)]
    pub text: String,
}

/// The generated `submit`: a guest deposits an item for his stay.
pub fn submit_command<H: RecordHooks>(ctx: &Context, decl: RecordDecl, args: &Value) -> Result<()> {
    let stay = ctx
        .guest
        .as_ref()
        .map(|guest| guest.session_id)
        .ok_or_else(|| PortakiError::Host("stay_id_required".into()))?;
    let item: H::Item = item_of(args, &ctx.lang())?;
    H::guard(ctx, &item)?;
    let entry = create::<H>(ctx, decl, item, stay, Actor::Guest)?;
    if let Err(error) = H::on_created(ctx, &entry) {
        hook_failed(decl, &error);
    }
    Ok(())
}

/// The generated `add`: the host records an item for a stay (`stayId`).
pub fn add_command<H: RecordHooks>(ctx: &Context, decl: RecordDecl, args: &Value) -> Result<()> {
    require_host(ctx)?;
    let stay = args
        .get("stayId")
        .and_then(Value::as_str)
        .and_then(|raw| Uuid::parse_str(raw).ok())
        .ok_or_else(|| PortakiError::Host("stay_id_required".into()))?;
    let item: H::Item = item_of(args, &ctx.lang())?;
    let entry = create::<H>(ctx, decl, item, stay, Actor::Host)?;
    if let Err(error) = H::on_created(ctx, &entry) {
        hook_failed(decl, &error);
    }
    Ok(())
}

/// The generated `setStatus`, host only. Setting the current status writes nothing.
pub fn set_status_command<H: RecordHooks>(
    ctx: &Context,
    decl: RecordDecl,
    args: SetStatusArgs,
) -> Result<()> {
    require_host(ctx)?;
    let to = H::Status::from_wire(&args.status)
        .ok_or_else(|| PortakiError::Host("status_unknown".into()))?;
    set_status::<H>(decl, args.id, to, Actor::Host)
}

/// Moves a record to `to`, with its history line; nothing when it is already there.
pub fn set_status<H: RecordHooks>(
    decl: RecordDecl,
    id: Uuid,
    to: H::Status,
    actor: Actor,
) -> Result<()> {
    let mut entry = get::<H::Item, H::Status>(decl, id)?
        .ok_or_else(|| PortakiError::Host("record_not_found".into()))?;
    if entry.status == to {
        return Ok(());
    }
    entry.status = to;
    entry.history.push(HistoryLine {
        at: crate::host::time::now()?,
        actor,
        event: HistoryEvent::Status {
            to: to.wire().to_string(),
        },
    });
    entry.save(decl)
}

/// The generated `note`, host only: a line of history, then [`RecordHooks::on_note`].
pub fn note_command<H: RecordHooks>(ctx: &Context, decl: RecordDecl, args: NoteArgs) -> Result<()> {
    require_host(ctx)?;
    let text = args.text.trim().to_string();
    if text.is_empty() {
        return Err(PortakiError::Host("text_required".into()));
    }
    let mut entry = get::<H::Item, H::Status>(decl, args.id)?
        .ok_or_else(|| PortakiError::Host("record_not_found".into()))?;
    entry.history.push(HistoryLine {
        at: crate::host::time::now()?,
        actor: Actor::Host,
        event: HistoryEvent::Note { text: text.clone() },
    });
    entry.save(decl)?;
    if let Err(error) = H::on_note(ctx, &entry, &text) {
        hook_failed(decl, &error);
    }
    Ok(())
}

/// The generated `timelineTasks`: every open record older than `due_hours` is a task with one
/// item, « done », whose check closes it.
pub fn timeline_tasks<H: RecordHooks>(
    decl: RecordDecl,
    args: &TimelineTasksArgs,
    title: I18nText,
    done_label: I18nText,
) -> Result<TimelineTasks> {
    let Some(hours) = decl.due_hours else {
        return Ok(TimelineTasks::default());
    };
    let now = crate::host::time::now()?;
    let key = <H::Item as Item>::KEY;
    let mut tasks: Vec<_> = list::<H::Item, H::Status>(decl)?
        .into_iter()
        .filter(|entry| !entry.is_done())
        .map(|entry| (entry.created_at + Duration::hours(hours), entry))
        .filter(|(late, _)| *late <= now && *late <= args.to)
        .map(|(late, entry)| {
            let context = key
                .and_then(|key| {
                    serde_json::to_value(&entry.item)
                        .ok()?
                        .get(key)?
                        .as_str()
                        .map(str::to_string)
                })
                .unwrap_or_default();
            let mut task = timeline::task(
                entry.id.to_string(),
                late.max(args.from),
                args.property_id,
                title.clone(),
                I18nText::from(context.as_str()),
            )
            .items(vec![TimelineTaskItem::new("done", done_label.clone())]);
            if let Some(stay) = entry.stay_id {
                task = task.stay(stay);
            }
            task
        })
        .collect();
    tasks.sort_by_key(|task| task.at);
    Ok(TimelineTasks { tasks })
}

/// The generated `taskToggle` / `taskComplete`: checking the task closes the record.
pub fn complete_task<H: RecordHooks>(decl: RecordDecl, task_id: &str) -> Result<()> {
    let id = Uuid::parse_str(task_id).map_err(|_| PortakiError::Host("record_not_found".into()))?;
    set_status::<H>(decl, id, H::Status::DONE, Actor::Host)
}

fn create<H: RecordHooks>(
    ctx: &Context,
    decl: RecordDecl,
    item: H::Item,
    stay: Uuid,
    actor: Actor,
) -> Result<Entry<H::Item, H::Status>> {
    let now = crate::host::time::now()?;
    let stay_ctx = ctx.stay.as_ref().filter(|s| s.stay_id == stay);
    let entry = Entry {
        id: Uuid::new_v4(),
        item,
        status: H::Status::INITIAL,
        stay_id: Some(stay),
        created_at: now,
        actor,
        guest_name: match actor {
            Actor::Guest => ctx
                .guest
                .as_ref()
                .and_then(|g| g.display_name.as_deref())
                .unwrap_or_default()
                .trim()
                .to_string(),
            Actor::Host => String::new(),
        },
        check_in: stay_ctx.and_then(|s| s.checkin_at),
        check_out: stay_ctx.and_then(|s| s.checkout_at),
        history: vec![HistoryLine {
            at: now,
            actor,
            event: HistoryEvent::Created,
        }],
    };
    entry.save(decl)?;
    Ok(entry)
}

/// The item a form or a call sends, normalized like a host save, then checked: a required
/// field empty is `{key}_required`, any other problem `{key}_invalid`.
fn item_of<T: Item>(args: &Value, lang: &str) -> Result<T> {
    let row = super::merge::normalize(&Map::new(), args, T::fields(), lang);
    for field in T::fields() {
        if let Some(_problem) = field.problem(&row) {
            let empty = field.words(&row[field.key]).trim().is_empty();
            let code = if empty { "required" } else { "invalid" };
            return Err(PortakiError::Host(format!("{}_{code}", field.key)));
        }
    }
    Ok(serde_json::from_value(row)?)
}

fn require_host(ctx: &Context) -> Result<()> {
    if ctx.guest.is_some() {
        return Err(PortakiError::Host("host_only".into()));
    }
    Ok(())
}

fn corrupt(error: serde_json::Error) -> PortakiError {
    PortakiError::Storage(format!("record_unreadable: {error}"))
}

/// A hook failed after the record was stored: logged, the record stays.
fn hook_failed(decl: RecordDecl, error: &PortakiError) {
    let mut fields = crate::host::log::Fields::new();
    fields.insert("record", &decl.name);
    fields.insert("error", &error.to_string());
    let _ = crate::host::log::info("record_hook_failed", &fields);
}

/// The generated `listForStay`: a guest's records for his stay — item, status, notes.
pub fn list_for_stay_query<H: RecordHooks>(ctx: &Context, decl: RecordDecl) -> Result<Vec<Value>> {
    let Some(stay) = ctx.guest.as_ref().map(|guest| guest.session_id) else {
        return Ok(Vec::new());
    };
    for_stay::<H::Item, H::Status>(decl, stay)?
        .into_iter()
        .map(|entry| {
            Ok(serde_json::json!({
                "id": entry.id,
                "item": serde_json::to_value(&entry.item)?,
                "status": entry.status.wire(),
                "createdAt": entry.created_at,
                "notes": entry.notes().collect::<Vec<_>>(),
            }))
        })
        .collect()
}
