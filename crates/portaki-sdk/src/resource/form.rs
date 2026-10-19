//! The guest's form of a record, derived from its item: one field per declared field (a choice
//! as a list of options, a short text as an input, a long one as an area, a photo as an upload),
//! required marks and placeholders from the declaration, sent by the record's `submit`.
//!
//! The module keeps what is its own: which options it offers, a header, a footer, a prefill.

use std::collections::BTreeMap;

use super::{FieldKind, FieldSpec, Item};
use crate::sdui::common::ChoiceListLayout;
use crate::sdui::component::Component;
use crate::sdui::primitives::{Button, ChoiceList, Field, Form, ImageUpload, TextArea, TextInput};
use crate::Context;

/// The form; [`Self::build`] gives it.
pub struct FormFlow<'a, T: Item> {
    ctx: &'a Context,
    only: BTreeMap<&'static str, Vec<String>>,
    without: Vec<&'static str>,
    prefill: BTreeMap<&'static str, String>,
    header: Vec<Component>,
    footer: Vec<Component>,
    submit_label: String,
    _item: std::marker::PhantomData<T>,
}

impl<'a, T: Item> FormFlow<'a, T> {
    /// The form of `T`, sent by `submit`; its button labelled `form.submit`.
    pub fn new(ctx: &'a Context) -> Self {
        Self {
            ctx,
            only: BTreeMap::new(),
            without: Vec::new(),
            prefill: BTreeMap::new(),
            header: Vec::new(),
            footer: Vec::new(),
            submit_label: "i18n:form.submit".into(),
            _item: std::marker::PhantomData,
        }
    }

    /// Offers only these values of a choice field; a single one is sent without being asked.
    pub fn only(
        mut self,
        key: &'static str,
        values: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.only
            .insert(key, values.into_iter().map(Into::into).collect());
        self
    }

    /// Leaves a field out.
    pub fn without(mut self, key: &'static str) -> Self {
        self.without.push(key);
        self
    }

    /// A value already in the field.
    pub fn prefill(mut self, key: &'static str, value: impl Into<String>) -> Self {
        self.prefill.insert(key, value.into());
        self
    }

    /// Components before the fields.
    pub fn header(mut self, component: impl Into<Component>) -> Self {
        self.header.push(component.into());
        self
    }

    /// Components after the button.
    pub fn footer(mut self, component: impl Into<Component>) -> Self {
        self.footer.push(component.into());
        self
    }

    /// The form.
    pub fn build(self) -> Form {
        let mut form = Form::new();
        for component in &self.header {
            form = form.child(component.clone());
        }
        for spec in T::fields()
            .iter()
            .filter(|f| !self.without.contains(&f.key))
        {
            if let Some(component) = self.field(spec) {
                form = form.child(component);
            }
        }
        form = form.child(
            Button::new().label(self.submit_label.clone()).action(
                crate::ids::current_module(self.ctx)
                    .command_empty(crate::ids::OperationName::new("submit")),
            ),
        );
        for component in self.footer {
            form = form.child(component);
        }
        form
    }

    fn field(&self, spec: &FieldSpec) -> Option<Component> {
        let prefill = self.prefill.get(spec.key).cloned();
        let control: Component = match spec.kind {
            FieldKind::Choice => {
                let mut options = spec.options.map(|options| options()).unwrap_or_default();
                if let Some(only) = self.only.get(spec.key) {
                    options.retain(|o| only.contains(&o.value));
                }
                match options.as_slice() {
                    [] => return None,
                    [one] => {
                        return Some(
                            TextInput::new()
                                .name(spec.key)
                                .value(one.value.clone())
                                .into(),
                        )
                    }
                    _ => {}
                }
                let mut list = ChoiceList::new()
                    .name(spec.key)
                    .layout(ChoiceListLayout::Compact)
                    .choices(options);
                if let Some(value) = prefill.filter(|v| !v.is_empty()) {
                    list = list.value(value);
                }
                list.into()
            }
            FieldKind::Photo => ImageUpload::new().name(spec.key).into(),
            _ => {
                let long = spec.max_chars.is_some_and(|max| max > 200);
                let placeholder = spec.placeholder.map(|p| format!("i18n:{p}"));
                if long {
                    let mut area = TextArea::new().name(spec.key);
                    if let Some(p) = placeholder {
                        area = area.placeholder(p);
                    }
                    area.into()
                } else {
                    let mut input = TextInput::new().name(spec.key);
                    if let Some(p) = placeholder {
                        input = input.placeholder(p);
                    }
                    if let Some(value) = prefill {
                        input = input.value(value);
                    }
                    input.into()
                }
            }
        };
        let mut field = Field::new()
            .name(spec.key)
            .label(format!("i18n:{}", spec.label));
        if spec.required {
            field = field.required(true);
        }
        Some(field.child(control).into())
    }
}
