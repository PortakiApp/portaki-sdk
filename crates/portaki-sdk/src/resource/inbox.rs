//! The host's inbox of a record: filters « open / done / all », one `FeedItem` per record, the
//! detail in a panel (status chips, the item's fields, the photo, a reply, the history).
//!
//! Texts by convention in the module's bundles, under a prefix (`host`): `{p}.tab.filter.open`
//! (with `{count}`), `{p}.tab.filter.<done>`, `{p}.tab.filter.all`, `{p}.tab.empty.<filter>`,
//! `{p}.reply.label`, `{p}.reply.send`, `{p}.history.title`, `{p}.history.added`,
//! `{p}.history.reported`, `{p}.history.status` (`{status}`), `{p}.history.reply` (`{text}`);
//! status labels are the lifecycle's own (`#[choice(label = …)]`).

use chrono::{DateTime, FixedOffset, Utc};
use serde_json::json;
use uuid::Uuid;

use super::record::{Actor, Entry, HistoryEvent, RecordDecl, RecordHooks};
use super::{Choice, FieldKind, Item, Lifecycle};
use crate::contracts::shell::{SURFACE_INPUT, SURFACE_OVERLAY};
use crate::files::FileRef;
use crate::sdui::action::Action;
use crate::sdui::common::{StackDirection, TextVariant, Tone};
use crate::sdui::component::Component;
use crate::sdui::primitives::{
    Button, Card, EmptyState, FeedItem, Field, FilterBar, FilterChip, Form, Image, Stack, Text,
    TextArea,
};
use crate::sdui::FeedStatus;
use crate::vocab::IconName;
use crate::Context;

type Line<T> = Box<dyn Fn(&T) -> String>;
type Extra<T, S> = Box<dyn Fn(&Entry<T, S>) -> Vec<Component>>;

/// The inbox; [`Self::build`] gives the page, or the detail when the panel asks for one.
pub struct Inbox<'a, H: RecordHooks> {
    ctx: &'a Context,
    decl: RecordDecl,
    keys: String,
    icon: IconName,
    title: Line<H::Item>,
    tag: Option<Line<H::Item>>,
    detail_extra: Option<Extra<H::Item, H::Status>>,
    header: Vec<Component>,
}

impl<'a, H: RecordHooks> Inbox<'a, H> {
    /// The inbox of `decl`; its texts under the bundle prefix `keys`.
    pub fn new(ctx: &'a Context, decl: RecordDecl, keys: &str) -> Self {
        Self {
            ctx,
            decl,
            keys: keys.to_string(),
            icon: IconName::Info,
            title: Box::new(key_text::<H::Item>),
            tag: None,
            detail_extra: None,
            header: Vec::new(),
        }
    }

    /// The icon of the panel.
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = icon;
        self
    }

    /// The tag of a row (a bundle key, `i18n:`-prefixed).
    pub fn tag(mut self, tag: impl Fn(&H::Item) -> String + 'static) -> Self {
        self.tag = Some(Box::new(tag));
        self
    }

    /// Components after the derived ones in the detail.
    pub fn detail_extra(
        mut self,
        extra: impl Fn(&Entry<H::Item, H::Status>) -> Vec<Component> + 'static,
    ) -> Self {
        self.detail_extra = Some(Box::new(extra));
        self
    }

    /// Components next to the filters (an « add » button…).
    pub fn header(mut self, component: impl Into<Component>) -> Self {
        self.header.push(component.into());
        self
    }

    /// The page, or the detail of `{name}Id` when the panel asks for it.
    pub fn build(self) -> crate::Result<Component> {
        let id_key = format!("{}Id", self.decl.name);
        if let Some(id) = self
            .ctx
            .input_str(&id_key)
            .and_then(|raw| Uuid::parse_str(raw).ok())
        {
            if let Some(entry) = super::record::get::<H::Item, H::Status>(self.decl, id)? {
                return Ok(self.detail(&entry));
            }
        }
        let entries = super::record::list::<H::Item, H::Status>(self.decl)?;
        let done = H::Status::DONE.wire();
        let filter = match self.ctx.input_str("filter") {
            Some("all") => "all",
            Some(f) if f == done => done,
            _ => "open",
        };
        let open = entries.iter().filter(|e| !e.is_done()).count();
        let chips: Vec<Component> = ["open", done, "all"]
            .into_iter()
            .map(|key| {
                let label = if key == "open" {
                    crate::t!(
                        &format!("{}.tab.filter.open", self.keys),
                        count = open.to_string()
                    )
                    .unwrap_or_default()
                } else {
                    format!("i18n:{}.tab.filter.{key}", self.keys)
                };
                FilterChip::new()
                    .label(label)
                    .selected(key == filter)
                    .action(Action::emit(SURFACE_INPUT, Some(json!({ "filter": key }))))
                    .into()
            })
            .collect();
        let shown: Vec<&Entry<H::Item, H::Status>> = entries
            .iter()
            .filter(|e| match filter {
                "open" => !e.is_done(),
                "all" => true,
                _ => e.is_done(),
            })
            .collect();
        let body: Component = if shown.is_empty() {
            EmptyState::new()
                .title(format!("i18n:{}.tab.empty.{filter}", self.keys))
                .icon(self.icon)
                .into()
        } else {
            let now = crate::host::time::now()?;
            Stack::new()
                .gap(0.0)
                .children(shown.iter().map(|e| self.row(e, now)).collect())
                .into()
        };
        let mut header = Stack::new()
            .direction(StackDirection::Horizontal)
            .gap(12.0)
            .child(FilterBar::new().children(chips));
        for component in &self.header {
            header = header.child(component.clone());
        }
        Ok(Stack::new()
            .gap(16.0)
            .child(header)
            .child(Card::new().child(body))
            .into())
    }

    fn row(&self, entry: &Entry<H::Item, H::Status>, now: DateTime<Utc>) -> Component {
        let when = crate::host::time::ago(entry.created_at, now, &self.ctx.lang());
        let date = match entry.guest_name.as_str() {
            "" => when,
            name => format!("{name} · {when}"),
        };
        let title = (self.title)(&entry.item);
        let tone = tone(entry);
        let mut row = FeedItem::new()
            .title(title.clone())
            .date(date)
            .dotTone(tone)
            .status(FeedStatus::new(entry.status.label_key(), tone))
            .action(Action::emit(
                SURFACE_OVERLAY,
                Some(json!({
                    "input": { format!("{}Id", self.decl.name): entry.id },
                    "icon": self.icon,
                    "title": title,
                    "status": { "label": crate::t!(&entry.status.bundle_key()).unwrap_or_default(), "tone": tone },
                })),
            ));
        if let Some(tag) = &self.tag {
            row = row.tag(tag(&entry.item));
        }
        row.into()
    }

    /// The detail: status chips, the item's fields, the photo, a reply, the history.
    pub fn detail(&self, entry: &Entry<H::Item, H::Status>) -> Component {
        let module = &crate::ids::current_module(self.ctx);
        let chips = FilterBar::new().children(
            H::Status::ALL
                .iter()
                .map(|to| {
                    FilterChip::new()
                        .label(to.label_key())
                        .selected(*to == entry.status)
                        .action(module.command(
                            crate::ids::OperationName::new("setStatus"),
                            json!({ "id": entry.id, "status": to.wire() }),
                        ))
                        .into()
                })
                .collect(),
        );
        let mut children: Vec<Component> = vec![chips.into()];
        let item = serde_json::to_value(&entry.item).unwrap_or_default();
        for field in H::Item::fields() {
            let value = item[field.key].as_str().unwrap_or_default();
            if value.is_empty() || Some(field.key) == H::Item::KEY {
                continue;
            }
            match field.kind {
                FieldKind::Photo => {
                    if let Some(photo) = FileRef::parse(value) {
                        children.push(Image::new().url(photo.image_url()).into());
                    }
                }
                FieldKind::Text => children.push(Text::new().text(value).into()),
                _ => {}
            }
        }
        if let Some(extra) = &self.detail_extra {
            children.extend(extra(entry));
        }
        children.push(
            Form::new()
                .child(
                    Field::new()
                        .name("text")
                        .label(format!("i18n:{}.reply.label", self.keys))
                        .child(TextArea::new().name("text").rows(3)),
                )
                .child(
                    Button::new()
                        .label(format!("i18n:{}.reply.send", self.keys))
                        .action(module.command(
                            crate::ids::OperationName::new("note"),
                            json!({ "id": entry.id }),
                        )),
                )
                .into(),
        );
        children.push(self.history(entry));
        Stack::new().gap(16.0).children(children).into()
    }

    fn history(&self, entry: &Entry<H::Item, H::Status>) -> Component {
        let mut list = Stack::new().gap(6.0).child(
            Text::new()
                .text(format!("i18n:{}.history.title", self.keys))
                .variant(TextVariant::Title),
        );
        for line in &entry.history {
            let text = match &line.event {
                HistoryEvent::Created if line.actor == Actor::Host => {
                    crate::t!(&format!("{}.history.added", self.keys))
                }
                HistoryEvent::Created => crate::t!(&format!("{}.history.reported", self.keys)),
                HistoryEvent::Status { to } => {
                    let label =
                        crate::t!(&format!("{}.{to}", H::Status::LABEL_PREFIX)).unwrap_or_default();
                    crate::t!(&format!("{}.history.status", self.keys), status = label)
                }
                HistoryEvent::Note { text } => {
                    crate::t!(&format!("{}.history.reply", self.keys), text = text.clone())
                }
            };
            let when = self.local(line.at).format("%d/%m %H:%M");
            list = list.child(
                Text::new()
                    .text(format!("{when} · {}", text.unwrap_or_default()))
                    .variant(TextVariant::Caption),
            );
        }
        list.into()
    }

    fn local(&self, at: DateTime<Utc>) -> DateTime<FixedOffset> {
        match self.ctx.property_tz() {
            Some(tz) => tz.to_local(at),
            None => at.fixed_offset(),
        }
    }
}

/// New is to act on, done is settled, the rest is in progress.
fn tone<T: Item, S: Lifecycle>(entry: &Entry<T, S>) -> Tone {
    if entry.status == S::INITIAL {
        Tone::Warning
    } else if entry.is_done() {
        Tone::Success
    } else {
        Tone::Info
    }
}

/// The item's natural key, as text — its title in lists.
fn key_text<T: Item>(item: &T) -> String {
    T::KEY
        .and_then(|key| {
            serde_json::to_value(item)
                .ok()?
                .get(key)?
                .as_str()
                .map(str::to_string)
        })
        .unwrap_or_default()
}
