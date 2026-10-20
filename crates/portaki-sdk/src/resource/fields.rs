//! The controls of the field kinds that are more than one input: a link, a file, a list of
//! labels, and the frame of nested rows.

use serde_json::{Map, Value};

use super::FieldSpec;
use crate::contracts::shell::SURFACE_INPUT;
use crate::sdui::action::Action;
use crate::sdui::component::Component;
use crate::sdui::primitives::{EditableList, FileUpload, StepList, TextInput};
use crate::sdui::EditableListItem;

/// The labels a [`super::FieldKind::Labels`] value holds.
pub fn labels(value: &Value) -> Vec<String> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

/// The control of a link, a file or a list of labels.
pub fn control(spec: &FieldSpec, name: &str, value: &Value) -> Component {
    let text = value.as_str().unwrap_or_default();
    match spec.kind {
        super::FieldKind::File => {
            let mut upload = FileUpload::new().name(name).value(text);
            if let Some(accept) = spec.accept {
                upload = upload.accept(accept);
            }
            if let Some(max) = spec.max_bytes {
                upload = upload.maxBytes(max);
            }
            upload.into()
        }
        super::FieldKind::Labels => EditableList::new()
            .name(name)
            .items(labels(value).iter().map(EditableListItem::new).collect())
            .into(),
        _ => {
            let input = TextInput::new().name(name).value(text);
            match spec.placeholder {
                Some(key) => input.placeholder(format!("i18n:{key}")).into(),
                None => input.placeholder("https://…").into(),
            }
        }
    }
}

/// The frame of nested rows: help, « add » (`<label>.add`) while `add` says how many to open.
pub fn step_list(
    spec: &FieldSpec,
    name: &str,
    rows: Vec<Component>,
    add: Option<(String, usize)>,
) -> Component {
    let mut list = StepList::new()
        .itemKeyPrefix(name)
        .addLabel(format!("i18n:{}.add", spec.label))
        .children(rows);
    if let Some(hint) = spec.description {
        list = list.hint(format!("i18n:{hint}"));
    }
    if let Some((key, count)) = add {
        let mut input = Map::new();
        input.insert(key, Value::from(count));
        list = list.addAction(Action::emit(SURFACE_INPUT, Some(Value::Object(input))));
    }
    list.into()
}
