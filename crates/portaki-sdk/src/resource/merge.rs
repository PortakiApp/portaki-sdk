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
        FieldKind::Localized => match raw {
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
        FieldKind::Text | FieldKind::TimeRange | FieldKind::Photo => Value::String(scalar(raw)),
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

/// A row whose declared texts are all blank, in every language.
fn is_blank(row: &Value, fields: &[FieldSpec]) -> bool {
    fields.iter().any(|f| f.kind.is_text())
        && fields
            .iter()
            .filter(|f| f.kind.is_text())
            .all(|f| super::text_of(&row[f.key]).trim().is_empty())
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
}
