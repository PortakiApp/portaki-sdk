//! The two ways rows are written: the host's form, and an import. Pure functions on JSON, so the
//! rule is one, tested once, whatever the module.
//!
//! - **Host** ([`host_save`]): the list sent fixes which rows exist and their order. A row keeps
//!   the fields the form did not send; a field sent empty is cleared; a localized text is written
//!   in the host's language and the others are kept; a row whose texts are all blank goes.
//! - **Import** ([`import`]): rows are only **added** — never replaced, never matched by position.
//!   A row whose natural key is already there is skipped; past `max`, the rest is dropped.

use serde::Serialize;
use serde_json::{Map, Value};

use super::{FieldKind, FieldSpec};

/// What an import did — the answer of `importRows`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct ImportReport {
    /// Rows added.
    pub added: usize,
    /// Rows already there (same natural key) or empty.
    pub skipped: usize,
    /// Rows dropped because the collection was full.
    pub overflow: usize,
}

/// The rows and settings after a host save of `sent` (the form, as `updateConfig` receives it).
///
/// `sent[name]` absent: the rows stay as stored. `[]`: the list is emptied.
pub fn host_save(
    stored: &[Value],
    meta: &Value,
    sent: &Value,
    name: &str,
    row_fields: &[FieldSpec],
    meta_fields: &[FieldSpec],
    lang: &str,
) -> (Vec<Value>, Value) {
    let rows = match sent.get(name).and_then(Value::as_array) {
        Some(sent_rows) => sent_rows
            .iter()
            .filter_map(Value::as_object)
            .map(|sent_row| {
                let id = sent_row
                    .get("id")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|id| !id.is_empty());
                let previous = id.and_then(|id| stored.iter().find(|row| row["id"] == id));
                let mut row = previous
                    .and_then(Value::as_object)
                    .cloned()
                    .unwrap_or_default();
                for field in row_fields {
                    if let Some(raw) = sent_row.get(field.key) {
                        set(&mut row, field, raw, lang);
                    }
                }
                row.insert(
                    "id".into(),
                    Value::String(id.map_or_else(new_id, str::to_string)),
                );
                Value::Object(row)
            })
            .filter(|row| !is_blank(row, row_fields))
            .collect(),
        None => stored.to_vec(),
    };
    let mut next_meta = meta.as_object().cloned().unwrap_or_default();
    for field in meta_fields {
        if let Some(raw) = sent.get(field.key) {
            set(&mut next_meta, field, raw, lang);
        }
    }
    (rows, Value::Object(next_meta))
}

/// `stored` with `incoming` added, by the import rule.
pub fn import(
    stored: &[Value],
    incoming: &[Value],
    fields: &[FieldSpec],
    key: Option<&str>,
    max: Option<usize>,
    lang: &str,
) -> (Vec<Value>, ImportReport) {
    let mut rows = stored.to_vec();
    let mut report = ImportReport::default();
    for raw in incoming.iter().filter_map(Value::as_object) {
        let mut row = Map::new();
        for field in fields {
            if let Some(value) = raw.get(field.key).filter(|v| !v.is_null()) {
                set(&mut row, field, value, lang);
            }
        }
        let row = Value::Object(row);
        let duplicate = key.is_some_and(|key| {
            let wanted = keys_of(&row[key]);
            !wanted.is_empty()
                && rows
                    .iter()
                    .any(|existing| keys_of(&existing[key]).iter().any(|k| wanted.contains(k)))
        });
        if is_blank(&row, fields) || duplicate {
            report.skipped += 1;
            continue;
        }
        if max.is_some_and(|max| rows.len() >= max) {
            report.overflow += 1;
            continue;
        }
        let mut row = row;
        row["id"] = Value::String(new_id());
        rows.push(row);
        report.added += 1;
    }
    (rows, report)
}

/// `previous` with the declared fields of `raw` written over it, normalized to their kinds.
pub(crate) fn normalize(
    previous: &Map<String, Value>,
    raw: &Value,
    fields: &[FieldSpec],
    lang: &str,
) -> Value {
    let mut row = previous.clone();
    for field in fields {
        if let Some(value) = raw.get(field.key).filter(|v| !v.is_null()) {
            set(&mut row, field, value, lang);
        }
    }
    Value::Object(row)
}

/// Writes `raw` (what a form or an import sends) into `row[field.key]`, normalized to the kind.
fn set(row: &mut Map<String, Value>, field: &FieldSpec, raw: &Value, lang: &str) {
    let value = match field.kind {
        FieldKind::Localized | FieldKind::RichText => match raw {
            Value::Object(texts) => Value::Object(texts.clone()),
            other => {
                let mut texts = row
                    .get(field.key)
                    .and_then(Value::as_object)
                    .cloned()
                    .unwrap_or_default();
                let text = scalar(other);
                if text.is_empty() {
                    texts.remove(lang);
                } else {
                    texts.insert(lang.to_string(), Value::String(text));
                }
                Value::Object(texts)
            }
        },
        FieldKind::Text
        | FieldKind::TimeRange
        | FieldKind::Photo
        | FieldKind::Url
        | FieldKind::File => Value::String(scalar(raw)),
        FieldKind::Labels => Value::Array(
            array_of(raw)
                .iter()
                .map(|label| match label {
                    Value::Object(row) => scalar(row.get("label").unwrap_or(&Value::Null)),
                    other => scalar(other),
                })
                .filter(|label| !label.is_empty())
                .map(Value::String)
                .collect(),
        ),
        FieldKind::Rows => {
            let fields = field.rows.map(|rows| rows()).unwrap_or_default();
            let stored = row
                .get(field.key)
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            Value::Array(nested(&stored, &array_of(raw), fields, lang))
        }
        FieldKind::Choice => {
            let wire = scalar(raw);
            let known = field
                .options
                .map_or(true, |options| options().iter().any(|o| o.value == wire));
            if !known {
                // A value the enum does not have would make the whole collection unreadable.
                return;
            }
            Value::String(wire)
        }
        FieldKind::Toggle => Value::Bool(match raw {
            Value::Bool(b) => *b,
            other => scalar(other) == "true",
        }),
        FieldKind::Number => match raw {
            Value::Number(n) => integral(n.as_f64().unwrap_or_default()),
            other => match scalar(other).replace(',', ".").parse::<f64>() {
                Ok(n) if n.is_finite() => integral(n),
                _ => Value::Null,
            },
        },
    };
    if value.is_null() {
        row.remove(field.key);
    } else {
        row.insert(field.key.to_string(), value);
    }
}

/// A whole number as an integer, so an integer field deserializes it.
fn integral(n: f64) -> Value {
    if n.fract() == 0.0 && n.abs() < 9.0e15 {
        Value::from(n as i64)
    } else {
        Value::from(n)
    }
}

fn scalar(value: &Value) -> String {
    match value {
        Value::String(text) => text.trim().to_string(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        _ => String::new(),
    }
}

/// A row where the host wrote nothing: every text, photo, file, link and nested row empty.
fn is_blank(row: &Value, fields: &[FieldSpec]) -> bool {
    fields.iter().any(|f| f.kind.is_content())
        && fields
            .iter()
            .filter(|f| f.kind.is_content())
            .all(|f| f.is_empty(&row[f.key]))
}

/// The rows of a nested field after a save of `sent`: a row with an `id` is merged over the
/// stored row with that id, a row without over the stored row at its position (a form that
/// sends rows by position); blank rows go. The list sent fixes which rows exist and their order.
fn nested(stored: &[Value], sent: &[Value], fields: &[FieldSpec], lang: &str) -> Vec<Value> {
    sent.iter()
        .enumerate()
        .filter_map(|(index, raw)| {
            let raw = match raw {
                Value::Object(_) => raw.clone(),
                // A bare string is the row's first text, the way a list of steps is often typed.
                other => {
                    let key = fields.iter().find(|f| f.kind.is_text())?.key;
                    serde_json::json!({ key: scalar(other) })
                }
            };
            let id = raw["id"]
                .as_str()
                .map(str::trim)
                .filter(|id| !id.is_empty());
            let previous = match id {
                Some(id) => stored.iter().find(|row| row["id"] == id),
                None => stored.get(index),
            };
            let previous = previous
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            let mut row = normalize(&previous, &raw, fields, lang);
            let id = id
                .map(str::to_string)
                .or_else(|| {
                    previous
                        .get("id")
                        .and_then(Value::as_str)
                        .map(str::to_string)
                })
                .unwrap_or_else(new_id);
            row["id"] = Value::String(id);
            (!is_blank(&row, fields)).then_some(row)
        })
        .collect()
}

/// A list as a form sends it: an array, or the same array as a JSON string.
fn array_of(raw: &Value) -> Vec<Value> {
    match raw {
        Value::Array(items) => items.clone(),
        Value::String(json) => serde_json::from_str::<Value>(json)
            .ok()
            .and_then(|value| value.as_array().cloned())
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

/// The natural key's texts, normalized: lower case, single spaces.
fn keys_of(value: &Value) -> Vec<String> {
    super::texts_of(value)
        .into_iter()
        .map(|(_, text)| {
            text.split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .to_lowercase()
        })
        .filter(|text| !text.is_empty())
        .collect()
}

fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const FIELDS: &[FieldSpec] = &[
        FieldSpec {
            key: "title",
            kind: FieldKind::Localized,
            label: "t",
            description: None,
            placeholder: None,
            required: true,
            max_chars: Some(60),
            min: None,
            max: None,
            options: None,
            ..FieldSpec::BLANK
        },
        FieldSpec {
            key: "status",
            kind: FieldKind::Choice,
            label: "s",
            description: None,
            placeholder: None,
            required: false,
            max_chars: None,
            min: None,
            max: None,
            options: Some(|| {
                vec![
                    crate::sdui::common::ChoiceOption::new("neutral", "n"),
                    crate::sdui::common::ChoiceOption::new("important", "i"),
                ]
            }),
            ..FieldSpec::BLANK
        },
    ];
    const META: &[FieldSpec] = &[FieldSpec {
        key: "card_limit",
        kind: FieldKind::Number,
        label: "c",
        description: None,
        placeholder: None,
        required: false,
        max_chars: None,
        min: Some(2.0),
        max: Some(6.0),
        options: None,
        ..FieldSpec::BLANK
    }];

    fn stored() -> Vec<Value> {
        vec![
            json!({ "id": "a", "title": { "fr": "Silence", "en": "Quiet" }, "status": "important" }),
            json!({ "id": "b", "title": { "fr": "Pas de fête" }, "status": "neutral" }),
        ]
    }

    #[test]
    fn a_host_save_keeps_what_the_form_did_not_send_and_the_other_languages() {
        let sent = json!({ "rules": [ { "id": "b" }, { "id": "a", "title": "Calme" } ] });
        let (rows, _) = host_save(&stored(), &json!({}), &sent, "rules", FIELDS, META, "fr");
        assert_eq!(rows[0]["id"], "b");
        assert_eq!(
            rows[0]["title"]["fr"], "Pas de fête",
            "unopened row kept whole"
        );
        assert_eq!(rows[1]["title"], json!({ "fr": "Calme", "en": "Quiet" }));
        assert_eq!(rows[1]["status"], "important");
    }

    #[test]
    fn a_host_save_drops_rows_left_out_and_blank_rows_and_reads_meta() {
        let sent =
            json!({ "rules": [ { "id": "a" }, { "id": "new", "title": " " } ], "card_limit": "5" });
        let (rows, meta) = host_save(&stored(), &json!({}), &sent, "rules", FIELDS, META, "fr");
        assert_eq!(rows.len(), 1, "b left out, the blank new row dropped");
        assert_eq!(meta["card_limit"], 5);
    }

    #[test]
    fn an_absent_list_keeps_the_rows_and_an_empty_one_clears_them() {
        let (kept, _) = host_save(
            &stored(),
            &json!({}),
            &json!({}),
            "rules",
            FIELDS,
            META,
            "fr",
        );
        assert_eq!(kept.len(), 2);
        let (cleared, _) = host_save(
            &stored(),
            &json!({}),
            &json!({ "rules": [] }),
            "rules",
            FIELDS,
            META,
            "fr",
        );
        assert!(cleared.is_empty());
    }

    #[test]
    fn an_unknown_choice_keeps_the_stored_value() {
        let sent = json!({ "rules": [ { "id": "a", "status": "bogus" } ] });
        let (rows, _) = host_save(&stored(), &json!({}), &sent, "rules", FIELDS, META, "fr");
        assert_eq!(rows[0]["status"], "important");
    }

    #[test]
    fn an_import_only_adds_and_skips_what_is_already_there() {
        let incoming = vec![
            json!({ "title": "  silence " }),
            json!({ "title": "Animaux bienvenus", "status": "neutral" }),
            json!({ "title": "" }),
        ];
        let (rows, report) = import(&stored(), &incoming, FIELDS, Some("title"), None, "fr");
        assert_eq!(
            report,
            ImportReport {
                added: 1,
                skipped: 2,
                overflow: 0
            }
        );
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0], stored()[0], "the host's rows are untouched");
        assert_eq!(rows[2]["title"], json!({ "fr": "Animaux bienvenus" }));
        assert!(!rows[2]["id"].as_str().unwrap().is_empty());
    }

    #[test]
    fn an_import_stops_at_max() {
        let incoming = vec![json!({ "title": "Un" }), json!({ "title": "Deux" })];
        let (rows, report) = import(&stored(), &incoming, FIELDS, Some("title"), Some(3), "fr");
        assert_eq!(rows.len(), 3);
        assert_eq!(report.overflow, 1);
    }

    const STEP: &[FieldSpec] = &[
        FieldSpec {
            key: "text",
            kind: FieldKind::Localized,
            ..FieldSpec::BLANK
        },
        FieldSpec {
            key: "image",
            kind: FieldKind::Photo,
            ..FieldSpec::BLANK
        },
    ];
    const LIST: &[FieldSpec] = &[
        FieldSpec {
            key: "name",
            kind: FieldKind::Localized,
            ..FieldSpec::BLANK
        },
        FieldSpec {
            key: "steps",
            kind: FieldKind::Rows,
            rows: Some(|| STEP),
            max_rows: Some(2),
            ..FieldSpec::BLANK
        },
        FieldSpec {
            key: "rooms",
            kind: FieldKind::Labels,
            ..FieldSpec::BLANK
        },
    ];

    fn list() -> Vec<Value> {
        vec![json!({ "id": "l", "name": { "fr": "Départ" }, "steps": [
            { "id": "s1", "text": { "fr": "Fenêtres", "en": "Windows" } },
            { "id": "s2", "text": { "fr": "Poubelles" }, "image": "portaki-file:1" }
        ] })]
    }

    #[test]
    fn nested_rows_keep_their_ids_through_a_removal() {
        // The first step removed: the second keeps its id, its photo and its English.
        let sent = json!({ "lists": [ { "id": "l", "steps": [ { "id": "s2", "text": "Sortir les poubelles" } ] } ] });
        let (rows, _) = host_save(&list(), &json!({}), &sent, "lists", LIST, &[], "fr");
        let steps = rows[0]["steps"].as_array().unwrap();
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0]["id"], "s2");
        assert_eq!(steps[0]["image"], "portaki-file:1");
        assert_eq!(steps[0]["text"]["fr"], "Sortir les poubelles");
    }

    #[test]
    fn nested_rows_without_ids_merge_by_position_and_blank_ones_go() {
        let sent = json!({ "lists": [ { "id": "l", "steps": r#"[{"text":"Close the windows"},{"text":""},{"text":"New"},{"text":" "}]"# } ] });
        let (rows, _) = host_save(&list(), &json!({}), &sent, "lists", LIST, &[], "en");
        let steps = rows[0]["steps"].as_array().unwrap();
        assert_eq!(steps.len(), 3, "{steps:?}");
        assert_eq!(steps[0]["id"], "s1");
        assert_eq!(steps[0]["text"]["fr"], "Fenêtres", "French kept");
        assert_eq!(steps[0]["text"]["en"], "Close the windows");
        assert_eq!(
            steps[1]["id"], "s2",
            "emptied in English, still written in French"
        );
        assert_eq!(steps[2]["text"]["en"], "New");
        assert!(steps[2]["id"].as_str().is_some_and(|id| id.len() == 36));
    }

    #[test]
    fn too_many_nested_rows_is_a_problem_not_a_refusal() {
        let sent = json!({ "lists": [ { "id": "l", "steps": ["a", "b", "c"] } ] });
        let (rows, _) = host_save(&list(), &json!({}), &sent, "lists", LIST, &[], "fr");
        assert_eq!(rows[0]["steps"].as_array().unwrap().len(), 3);
        assert!(LIST[1].problem(&rows[0]).is_some());
    }

    #[test]
    fn labels_read_an_editable_list() {
        let sent = json!({ "lists": [ { "id": "l", "rooms": r#"[{"label":"Cuisine"},{"label":" "},{"label":"Salon"}]"# } ] });
        let (rows, _) = host_save(&list(), &json!({}), &sent, "lists", LIST, &[], "fr");
        assert_eq!(rows[0]["rooms"], json!(["Cuisine", "Salon"]));
    }
}
