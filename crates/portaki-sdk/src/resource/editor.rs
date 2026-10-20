//! The host editor of a collection, derived from its declaration.
//!
//! A `Section` holding the settings (if the collection has any), then a `DataTable` in the
//! dashboard's wire format: rows `{ id, …cells }`, one panel per row whose fields are named
//! `{name}.{i}.{key}`, « Add » through `host.surface.input` (`{name}New`: the ids added and not
//! saved yet). Every panel field comes from the item's `#[field]`: control from the kind, label,
//! help, required mark, and the problem under it.
//!
//! Texts by convention, in the module's bundles: `host.{name}.title`, `host.{name}.add`,
//! `host.{name}.tooMany`.

use serde_json::{json, Map, Value};

use super::{Collection, FieldKind, FieldSpec, Item, Meta};
use crate::context::Context;
use crate::contracts::i18n::I18nText;
use crate::contracts::shell::SURFACE_INPUT;
use crate::sdui::action::Action;
use crate::sdui::common::{ChoiceListLayout, Tone};
use crate::sdui::component::Component;
use crate::sdui::primitives::{
    ChoiceList, DataTable, Field, FieldHint, InlineNotice, NumberInput, RichTextEditor, Section,
    Select, Stack, TextArea, TextInput, TimeRange, Toggle,
};

type Cell<T> = Box<dyn Fn(&T) -> String>;
type Panel<T> = Box<dyn Fn(usize, &T) -> Vec<Component>>;

struct Column {
    key: String,
    label: String,
    kind: Option<&'static str>,
    sub: Option<String>,
    mono: bool,
    /// A check column: at most this many ticked, and what to say past it.
    max: Option<(usize, String)>,
}

/// The editor; [`Self::build`] gives the component.
pub struct CollectionEditor<'a, T: Item, M: Meta> {
    ctx: &'a Context,
    collection: &'a Collection<T, M>,
    columns: Vec<Column>,
    cells: Vec<(String, Cell<T>)>,
    panel_extra: Option<Panel<T>>,
    divider: Option<(u32, String)>,
    hint: Option<String>,
    when_empty: Vec<Component>,
    settings: Option<(String, Vec<Component>)>,
    search: Option<String>,
    filter: Option<String>,
    settings_last: bool,
}

impl<'a, T: Item, M: Meta> CollectionEditor<'a, T, M> {
    /// The editor of `collection`; every field is a column until [`Self::column`] says otherwise.
    pub fn new(ctx: &'a Context, collection: &'a Collection<T, M>) -> Self {
        Self {
            ctx,
            collection,
            columns: Vec::new(),
            cells: Vec::new(),
            panel_extra: None,
            divider: None,
            hint: None,
            when_empty: Vec::new(),
            settings: None,
            search: None,
            filter: None,
            settings_last: false,
        }
    }

    /// A column showing `key` (a field, or a cell given by [`Self::cell`]).
    pub fn column(mut self, key: &str) -> Self {
        self.columns.push(self.column_of(key));
        self
    }

    /// A column showing `key` with `sub` underneath.
    pub fn column_with(mut self, key: &str, sub: &str) -> Self {
        let mut column = self.column_of(key);
        column.sub = Some(sub.to_string());
        self.columns.push(column);
        self
    }

    /// A column the module computes with [`Self::cell`], titled `label` (bundle key).
    pub fn computed(mut self, key: &str, label: &str) -> Self {
        let mut column = self.column_of(key);
        column.label = format!("i18n:{label}");
        self.columns.push(column);
        self
    }

    /// A column drawn as a pill.
    pub fn pill(mut self, key: &str, label: &str) -> Self {
        self.columns.push(Column {
            key: key.to_string(),
            label: format!("i18n:{label}"),
            kind: Some("pill"),
            sub: None,
            mono: false,
            max: None,
        });
        self
    }

    /// A toggle field as a column of check boxes, ticked in the table itself.
    pub fn check(mut self, key: &str) -> Self {
        let mut column = self.column_of(key);
        column.kind = Some("check");
        self.columns.push(column);
        self
    }

    /// [`Self::check`] with at most `max` rows ticked; past it the table says `message`.
    pub fn check_at_most(mut self, key: &str, max: usize, message: impl Into<String>) -> Self {
        let mut column = self.column_of(key);
        column.kind = Some("check");
        column.max = Some((max, message.into()));
        self.columns.push(column);
        self
    }

    /// The settings section after the rows instead of before.
    pub fn settings_last(mut self) -> Self {
        self.settings_last = true;
        self
    }

    /// A search box above the table (placeholder: bundle key).
    pub fn search(mut self, placeholder: &str) -> Self {
        self.search = Some(format!("i18n:{placeholder}"));
        self
    }

    /// Filter chips on the values of the column `key`.
    pub fn filter_by(mut self, key: &str) -> Self {
        self.filter = Some(key.to_string());
        self
    }

    /// A column in the monospaced face (times, codes).
    pub fn mono(mut self, key: &str) -> Self {
        let mut column = self.column_of(key);
        column.mono = true;
        self.columns.push(column);
        self
    }

    /// What a cell shows, computed by the module instead of the stored value.
    pub fn cell(mut self, key: &str, cell: impl Fn(&T) -> String + 'static) -> Self {
        self.cells.push((key.to_string(), Box::new(cell)));
        self
    }

    /// Components added under the derived fields of a row's panel.
    pub fn panel_extra(mut self, extra: impl Fn(usize, &T) -> Vec<Component> + 'static) -> Self {
        self.panel_extra = Some(Box::new(extra));
        self
    }

    /// A line after the first `count` rows, labelled `label` (bundle key).
    pub fn divider_after(mut self, count: usize, label: &str) -> Self {
        self.divider = Some((count as u32, format!("i18n:{label}")));
        self
    }

    /// A help line above the table (bundle key).
    pub fn hint(mut self, key: &str) -> Self {
        self.hint = Some(format!("i18n:{key}"));
        self
    }

    /// What follows the table while there is no row (a « start from a template » button…).
    pub fn when_empty(mut self, component: impl Into<Component>) -> Self {
        self.when_empty.push(component.into());
        self
    }

    /// The settings section, titled `title` (bundle key), with `extra` after the derived fields.
    pub fn settings(mut self, title: &str, extra: Vec<Component>) -> Self {
        self.settings = Some((format!("i18n:{title}"), extra));
        self
    }

    /// The editor: the settings section (if any), then the rows section.
    pub fn build(self) -> Vec<Component> {
        let mut out = Vec::new();
        if let Some((title, extra)) = &self.settings {
            let meta = serde_json::to_value(self.collection.meta()).unwrap_or(Value::Null);
            let problems = self.collection.problems();
            let mut fields: Vec<Component> = M::fields()
                .iter()
                .map(|field| {
                    let error = problems
                        .iter()
                        .find(|p| p.row.is_none() && p.field == field.key)
                        .map(|p| p.message.clone());
                    self.field(field, field.key.to_string(), &meta[field.key], error)
                })
                .collect();
            fields.extend(extra.iter().cloned());
            out.push(Section::new().title(title.clone()).children(fields).into());
        }
        let rows = self.rows_section();
        if self.settings_last {
            out.insert(0, rows);
        } else {
            out.push(rows);
        }
        out
    }

    fn rows_section(&self) -> Component {
        let name = self.collection.decl().name;
        let max = self.collection.decl().max;
        let rows = self.rows();
        let mut children: Vec<Component> = Vec::new();
        if let Some(hint) = &self.hint {
            children.push(FieldHint::new().text(hint.clone()).into());
        }
        if max.is_some_and(|max| rows.len() > max) {
            children.push(
                InlineNotice::new()
                    .tone(Tone::Danger)
                    .message(format!("i18n:host.{name}.tooMany"))
                    .into(),
            );
        }
        let columns: Vec<Value> = self.columns().iter().map(column_json).collect();
        let mut table = DataTable::new()
            .rowName(name)
            .sortable(true)
            .panel("dialog")
            .columns(Value::Array(columns))
            .rows(Value::Array(
                rows.iter().map(|row| self.row_json(row)).collect(),
            ))
            .children(
                rows.iter()
                    .enumerate()
                    .map(|(index, row)| self.panel(index, row))
                    .collect(),
            );
        if let Some(placeholder) = &self.search {
            table = table.searchPlaceholder(placeholder.clone());
        }
        if let Some(key) = &self.filter {
            table = table.filterKey(key.clone());
        }
        if let Some((count, label)) = &self.divider {
            table = table.dividerAfter(*count).dividerLabel(label.clone());
        }
        if max.map_or(true, |max| rows.len() < max) {
            let mut added: Vec<String> = rows
                .iter()
                .filter(|row| self.collection.find(row.id()).is_none())
                .map(|row| row.id().to_string())
                .collect();
            added.push(uuid::Uuid::new_v4().to_string());
            let mut input = Map::new();
            input.insert(format!("{name}New"), Value::String(added.join(",")));
            table = table
                .addLabel(format!("i18n:host.{name}.add"))
                .addAction(Action::emit(SURFACE_INPUT, Some(Value::Object(input))));
        }
        children.push(table.into());
        if rows.is_empty() {
            children.extend(self.when_empty.iter().cloned());
        }
        let mut section = Section::new()
            .title(format!("i18n:host.{name}.title"))
            .children(children);
        if let Some(max) = max {
            section = section.subtitle(format!("{} / {max}", rows.len()));
        }
        section.into()
    }

    /// The stored rows, then the rows « Add » opened and not saved yet.
    fn rows(&self) -> Vec<T> {
        let name = self.collection.decl().name;
        let mut rows = self.collection.rows().to_vec();
        let added = self
            .ctx
            .input_str(&format!("{name}New"))
            .unwrap_or_default();
        for id in added.split(',').map(str::trim).filter(|id| !id.is_empty()) {
            if self.collection.find(id).is_none() {
                if let Ok(row) = serde_json::from_value::<T>(json!({ "id": id })) {
                    rows.push(row);
                }
            }
        }
        rows
    }

    fn columns(&self) -> Vec<Column> {
        if !self.columns.is_empty() {
            return self.columns.iter().map(Column::clone_shallow).collect();
        }
        T::fields().iter().map(|f| self.column_of(f.key)).collect()
    }

    fn column_of(&self, key: &str) -> Column {
        let label = T::fields()
            .iter()
            .find(|f| f.key == key)
            .map_or(String::new(), |f| format!("i18n:{}", f.label));
        Column {
            key: key.to_string(),
            label,
            kind: None,
            sub: None,
            mono: false,
            max: None,
        }
    }

    fn row_json(&self, row: &T) -> Value {
        let value = serde_json::to_value(row).unwrap_or(Value::Null);
        let mut cells = Map::new();
        cells.insert("id".into(), Value::String(row.id().to_string()));
        for field in T::fields() {
            let checked = field.kind == FieldKind::Toggle
                && self
                    .columns
                    .iter()
                    .any(|c| c.key == field.key && c.kind == Some("check"));
            let cell = if checked {
                Value::Bool(value[field.key].as_bool().unwrap_or_default())
            } else {
                Value::String(self.shown(field, &value[field.key]))
            };
            cells.insert(field.key.into(), cell);
        }
        for (key, cell) in &self.cells {
            cells.insert(key.clone(), Value::String(cell(row)));
        }
        Value::Object(cells)
    }

    /// What a stored value reads as in a cell.
    fn shown(&self, field: &FieldSpec, value: &Value) -> String {
        match field.kind {
            FieldKind::Localized => serde_json::from_value::<I18nText>(value.clone())
                .map(|text| text.host_value(self.ctx).to_string())
                .unwrap_or_default(),
            FieldKind::Choice => {
                let wire = value.as_str().unwrap_or_default();
                field
                    .options
                    .and_then(|options| options().into_iter().find(|o| o.value == wire))
                    .map_or_else(String::new, |o| o.label)
            }
            FieldKind::Toggle => if value.as_bool() == Some(true) {
                "✓"
            } else {
                ""
            }
            .to_string(),
            FieldKind::Number => value.as_f64().map_or_else(String::new, |n| n.to_string()),
            FieldKind::Text | FieldKind::TimeRange | FieldKind::Url => {
                value.as_str().unwrap_or_default().to_string()
            }
            FieldKind::Labels => super::fields::labels(value).join(", "),
            FieldKind::Choices => super::fields::labels(value)
                .iter()
                .filter_map(|wire| {
                    let options = field.options?();
                    options
                        .into_iter()
                        .find(|o| &o.value == wire)
                        .map(|o| o.label)
                })
                .collect::<Vec<_>>()
                .join(", "),
            FieldKind::Rows => match value.as_array().map_or(0, Vec::len) {
                0 => String::new(),
                count => count.to_string(),
            },
            FieldKind::File => if value.as_str().is_some_and(|v| !v.is_empty()) {
                "📄"
            } else {
                ""
            }
            .to_string(),
            FieldKind::RichText => serde_json::from_value::<I18nText>(value.clone())
                .map(|text| super::rich_text::plain_text(text.host_value(self.ctx)))
                .unwrap_or_default(),
            FieldKind::Photo => if value.as_str().is_some_and(|v| !v.is_empty()) {
                "📷"
            } else {
                ""
            }
            .to_string(),
        }
    }

    fn panel(&self, index: usize, row: &T) -> Component {
        let name = self.collection.decl().name;
        let value = serde_json::to_value(row).unwrap_or(Value::Null);
        let mut children: Vec<Component> = T::fields()
            .iter()
            .map(|field| {
                self.field(
                    field,
                    format!("{name}.{index}.{}", field.key),
                    &value[field.key],
                    self.collection.problem_of(row.id(), field.key),
                )
            })
            .collect();
        if let Some(extra) = &self.panel_extra {
            children.extend(extra(index, row));
        }
        Stack::new().gap(14.0).children(children).into()
    }

    /// One field: label, control from the kind, help, problem.
    fn field(
        &self,
        spec: &FieldSpec,
        name: String,
        value: &Value,
        error: Option<I18nText>,
    ) -> Component {
        let fallback = spec
            .default
            .filter(|_| value.is_null())
            .and_then(|json| serde_json::from_str::<Value>(json).ok());
        let value = fallback.as_ref().unwrap_or(value);
        let control: Component = match spec.kind {
            FieldKind::Url | FieldKind::File | FieldKind::Labels | FieldKind::Choices => {
                super::fields::control(spec, &name, value)
            }
            FieldKind::Rows => self.nested(spec, &name, value),
            FieldKind::Localized | FieldKind::Text => {
                let text = match spec.kind {
                    FieldKind::Localized => self.shown(spec, value),
                    _ => value.as_str().unwrap_or_default().to_string(),
                };
                let placeholder = spec.placeholder.map(|key| format!("i18n:{key}"));
                if spec.max_chars.is_some_and(|max| max > 120) {
                    let area = TextArea::new().name(name.clone()).value(text);
                    match placeholder {
                        Some(p) => area.placeholder(p).into(),
                        None => area.into(),
                    }
                } else {
                    let input = TextInput::new().name(name.clone()).value(text);
                    match placeholder {
                        Some(p) => input.placeholder(p).into(),
                        None => input.into(),
                    }
                }
            }
            FieldKind::Choice => {
                let options = spec.options.map(|options| options()).unwrap_or_default();
                let current = value.as_str().unwrap_or_default().to_string();
                if options.iter().any(|o| o.icon.is_some()) {
                    ChoiceList::new()
                        .name(name.clone())
                        .layout(ChoiceListLayout::Cards)
                        .choices(options)
                        .value(current)
                        .into()
                } else {
                    Select::new()
                        .name(name.clone())
                        .options(options)
                        .value(current)
                        .into()
                }
            }
            FieldKind::RichText => {
                let text = serde_json::from_value::<I18nText>(value.clone()).unwrap_or_default();
                RichTextEditor::new()
                    .name(name.clone())
                    .value(super::rich_text::editor_value(text.host_value(self.ctx)))
                    .into()
            }
            FieldKind::Photo => crate::sdui::primitives::ImageUpload::new()
                .name(name.clone())
                .value(value.as_str().unwrap_or_default())
                .into(),
            FieldKind::TimeRange => TimeRange::new()
                .name(name.clone())
                .value(value.as_str().unwrap_or_default())
                .into(),
            FieldKind::Toggle => Toggle::new()
                .name(name.clone())
                .checked(value.as_bool().unwrap_or_default())
                .into(),
            FieldKind::Number => {
                let mut input = NumberInput::new()
                    .name(name.clone())
                    .value(value.as_f64().unwrap_or_default());
                if let Some(min) = spec.min {
                    input = input.min(min);
                }
                if let Some(max) = spec.max {
                    input = input.max(max);
                }
                input.into()
            }
        };
        let mut field = Field::new()
            .name(name)
            .label(format!("i18n:{}", spec.label))
            .required(spec.required)
            .child(control);
        if let Some(description) = spec.description {
            field = field.child(FieldHint::new().text(format!("i18n:{description}")));
        }
        if let Some(error) = error {
            field = field.error(error.get(&self.ctx.locale).to_string());
        }
        if let Some(when) = spec.visible_when {
            field = field.visibleWhen(when);
        }
        field.into()
    }
}

impl<T: Item, M: Meta> CollectionEditor<'_, T, M> {
    /// The rows of a nested field, one block each: its id sent back hidden, then its fields.
    fn nested(&self, spec: &FieldSpec, name: &str, value: &Value) -> Component {
        let stored = value.as_array().cloned().unwrap_or_default();
        let fields = spec.rows.map(|rows| rows()).unwrap_or_default();
        let count_key = format!("{name}.rows");
        let max = spec.max_rows.unwrap_or(usize::MAX);
        let count = self
            .ctx
            .input_u64(&count_key)
            .map_or(0, |n| n as usize)
            .max(stored.len())
            .min(max.max(stored.len()));
        let rows = (0..count)
            .map(|n| {
                let row = stored.get(n).cloned().unwrap_or(Value::Null);
                let prefix = format!("{name}.{n}");
                let mut children = vec![crate::sdui::row_id(name, n, row["id"].as_str())];
                children.extend(fields.iter().map(|field| {
                    self.field(
                        field,
                        format!("{prefix}.{}", field.key),
                        &row[field.key],
                        None,
                    )
                }));
                Stack::new().gap(8.0).children(children).into()
            })
            .collect();
        super::fields::step_list(
            spec,
            name,
            rows,
            (count < max).then_some((count_key, count + 1)),
        )
    }
}

impl Column {
    fn clone_shallow(&self) -> Self {
        Self {
            key: self.key.clone(),
            label: self.label.clone(),
            kind: self.kind,
            sub: self.sub.clone(),
            mono: self.mono,
            max: self.max.clone(),
        }
    }
}

fn column_json(column: &Column) -> Value {
    let mut out = json!({ "key": column.key, "label": column.label });
    if let Some(kind) = column.kind {
        out["kind"] = json!(kind);
    }
    if let Some(sub) = &column.sub {
        out["sub"] = json!(sub);
    }
    if column.mono {
        out["mono"] = json!(true);
    }
    if let Some((max, message)) = &column.max {
        out["max"] = json!(max);
        out["maxMessage"] = json!(message);
    }
    out
}
