//! The host form derived from `#[portaki_sdk::config(form)]` (feature `unstable`).
//!
//! Every declared field becomes its control — a toggle row, a choice list, a text, a phone, a
//! number, a secret kept blank — with its label, its help (`description`, else the bundle's
//! `<label>.hint` / `.help`), its placeholder (`<label>.placeholder`) and its problem. Fields
//! carrying `section = "…"` are grouped, in declaration order, under that title; the page follows
//! the settings rule: a `Section` per group, never a `Card`.
//!
//! The module keeps what is its own: an effective value (`value`), a component before or after a
//! field, a field replaced whole, the problems (`errors`).

use std::collections::BTreeMap;

use serde_json::Value;

use crate::contracts::i18n::I18nText;
use crate::sdui::common::{ChoiceListLayout, ChoiceOption};
use crate::sdui::component::Component;
use crate::sdui::primitives::{
    ChoiceList, Field, FieldHint, Form, NumberInput, PhoneInput, SecretInput, Section, Select,
    Stack, Text, TextArea, TextInput, ToggleRow,
};
use crate::Context;

/// One declared field, as `#[config(form)]` writes it.
#[derive(Debug, Clone, Copy)]
pub struct FormField {
    /// Stored key.
    pub key: &'static str,
    /// The manifest `type`.
    pub kind: &'static str,
    /// Bundle key of the label.
    pub label: &'static str,
    /// Bundle key of the help.
    pub description: Option<&'static str>,
    /// Bundle key of the placeholder.
    pub placeholder: Option<&'static str>,
    /// Bundle key of the section title it goes under.
    pub section: Option<&'static str>,
    /// The `visibleWhen` of its field (`urgent_phone=true`).
    pub visible_when: Option<&'static str>,
    /// The control asked for: `textarea`, `phone`, `segmented`.
    pub input: Option<&'static str>,
    /// The values of a `select`; each label is `<label>.<value>`.
    pub options: &'static [&'static str],
    /// The options of a field picking from a `#[choice]` (a list: several at once).
    pub choice: Option<fn() -> Vec<ChoiceOption>>,
    /// Another bundle prefix for the choice's labels on this form (`host.category`).
    pub choice_labels: Option<&'static str>,
}

type Errors<'a> = Box<dyn Fn(&str) -> Option<I18nText> + 'a>;

/// The form; [`Self::build`] gives it.
pub struct ConfigForm<'a> {
    ctx: &'a Context,
    fields: &'static [FormField],
    values: Value,
    errors: Option<Errors<'a>>,
    before: BTreeMap<&'static str, Vec<Component>>,
    after: BTreeMap<&'static str, Vec<Component>>,
    inside: BTreeMap<&'static str, Vec<Component>>,
    replaced: BTreeMap<&'static str, Component>,
}

impl<'a> ConfigForm<'a> {
    #[doc(hidden)]
    pub fn new(ctx: &'a Context, fields: &'static [FormField], values: Value) -> Self {
        Self {
            ctx,
            fields,
            values,
            errors: None,
            before: BTreeMap::new(),
            after: BTreeMap::new(),
            inside: BTreeMap::new(),
            replaced: BTreeMap::new(),
        }
    }

    /// The problem of a field, shown under it — usually the config's own `error_of`.
    pub fn errors(mut self, errors: impl Fn(&str) -> Option<I18nText> + 'a) -> Self {
        self.errors = Some(Box::new(errors));
        self
    }

    /// The value the form shows for `key` instead of the stored one (an effective default).
    pub fn value(mut self, key: &str, value: impl serde::Serialize) -> Self {
        self.values[key] = serde_json::to_value(value).unwrap_or(Value::Null);
        self
    }

    /// A component before a field.
    pub fn before(mut self, key: &'static str, component: impl Into<Component>) -> Self {
        self.before.entry(key).or_default().push(component.into());
        self
    }

    /// A component after a field.
    pub fn after(mut self, key: &'static str, component: impl Into<Component>) -> Self {
        self.after.entry(key).or_default().push(component.into());
        self
    }

    /// A component inside a field, under its control and help (a hint that depends on the value).
    pub fn inside(mut self, key: &'static str, component: impl Into<Component>) -> Self {
        self.inside.entry(key).or_default().push(component.into());
        self
    }

    /// A field drawn by the module.
    pub fn replace(mut self, key: &'static str, component: impl Into<Component>) -> Self {
        self.replaced.insert(key, component.into());
        self
    }

    /// The form: a `Section` per group of fields.
    pub fn build(mut self) -> Component {
        let mut groups: Vec<(Option<&'static str>, Vec<Component>)> = Vec::new();
        for field in self.fields {
            let mut components = self.before.remove(field.key).unwrap_or_default();
            if let Some(component) = self.replaced.remove(field.key) {
                components.push(component);
            } else if let Some(component) = self.field(field) {
                components.push(component);
            }
            components.extend(self.after.remove(field.key).unwrap_or_default());
            match groups
                .iter_mut()
                .find(|(section, _)| *section == field.section)
            {
                Some((_, children)) => children.extend(components),
                None => groups.push((field.section, components)),
            }
        }
        let sections: Vec<Component> = groups
            .into_iter()
            .filter(|(_, children)| !children.is_empty())
            .map(|(title, children)| {
                let mut section = Section::new().children(children);
                if let Some(title) = title {
                    section = section.title(format!("i18n:{title}"));
                }
                section.into()
            })
            .collect();
        Form::new()
            .child(Stack::new().gap(16.0).children(sections))
            .into()
    }

    fn field(&self, spec: &FormField) -> Option<Component> {
        let value = &self.values[spec.key];
        let text = || match value {
            Value::String(text) => text.clone(),
            Value::Number(n) => n.to_string(),
            Value::Object(_) => serde_json::from_value::<I18nText>(value.clone())
                .map(|t| t.host_value(self.ctx).to_string())
                .unwrap_or_default(),
            _ => String::new(),
        };
        let placeholder = spec.placeholder.map(|p| format!("i18n:{p}"));
        let control: Component = match (spec.kind, spec.input) {
            ("toggle", _) => {
                let mut row = ToggleRow::new()
                    .name(spec.key)
                    .label(format!("i18n:{}", spec.label))
                    .checked(value.as_bool().unwrap_or_default());
                if let Some(description) = spec.description {
                    row = row.description(format!("i18n:{description}"));
                }
                return Some(row.into());
            }
            ("structured", _) => {
                let mut choices = spec.choice?();
                if let Some(prefix) = spec.choice_labels {
                    for choice in &mut choices {
                        choice.label = format!("i18n:{prefix}.{}", choice.value);
                        choice.icon = None;
                    }
                }
                let chosen: Vec<&str> = value
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .collect();
                ChoiceList::new()
                    .name(spec.key)
                    .multi(true)
                    .layout(ChoiceListLayout::Compact)
                    .choices(choices)
                    .value(serde_json::to_string(&chosen).unwrap_or_default())
                    .into()
            }
            ("select", input) => {
                let options = spec
                    .options
                    .iter()
                    .map(|o| ChoiceOption::new(*o, format!("i18n:{}.{o}", spec.label)))
                    .collect();
                if input == Some("segmented") {
                    ChoiceList::new()
                        .name(spec.key)
                        .layout(ChoiceListLayout::Segmented)
                        .emitOnChange(true)
                        .choices(options)
                        .value(text())
                        .into()
                } else {
                    Select::new()
                        .name(spec.key)
                        .options(options)
                        .value(text())
                        .into()
                }
            }
            ("number", _) => NumberInput::new()
                .name(spec.key)
                .value(value.as_f64().unwrap_or_default())
                .into(),
            ("secret", _) => SecretInput::new()
                .name(spec.key)
                .value(String::new())
                .into(),
            ("readonly", _) => Text::new().text(text()).into(),
            (_, Some("phone")) => PhoneInput::new().name(spec.key).value(text()).into(),
            ("textarea", _) | (_, Some("textarea")) => {
                let mut area = TextArea::new().name(spec.key).rows(2).value(text());
                if let Some(p) = placeholder {
                    area = area.placeholder(p);
                }
                area.into()
            }
            _ => {
                let mut input = TextInput::new().name(spec.key).value(text());
                if let Some(p) = placeholder {
                    input = input.placeholder(p);
                }
                input.into()
            }
        };
        let mut field = Field::new()
            .name(spec.key)
            .label(format!("i18n:{}", spec.label))
            .child(control);
        if let Some(when) = spec.visible_when {
            field = field.visibleWhen(when);
        }
        if let Some(description) = spec.description {
            field = field.child(FieldHint::new().text(format!("i18n:{description}")));
        }
        for component in self.inside.get(spec.key).into_iter().flatten() {
            field = field.child(component.clone());
        }
        if let Some(error) = self.errors.as_ref().and_then(|errors| errors(spec.key)) {
            field = field.error(error.get(&self.ctx.locale).to_string());
        }
        Some(field.into())
    }
}
