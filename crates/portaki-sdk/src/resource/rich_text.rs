//! A `kind = "rich_text"` field: the host writes in the dashboard's editor (a TipTap document),
//! an import writes plain text or Markdown. Both are stored as is; these read either.

use serde_json::Value;

use crate::sdui::common::RichTextDoc;
use crate::sdui::component::Component;
use crate::sdui::primitives::{Markdown, RichText};

/// Whether `raw` is a TipTap document (`{"type":"doc",…}`).
pub fn is_doc(raw: &str) -> bool {
    parse(raw).is_some()
}

/// The words of `raw`, one block per line — what a cell shows and what "empty" is judged on.
pub fn plain_text(raw: &str) -> String {
    match parse(raw) {
        Some(doc) => {
            let mut out = Vec::new();
            collect(&doc, &mut out);
            out.join("\n")
        }
        None => raw.trim().to_string(),
    }
}

/// What the dashboard's editor opens: the document, or the text wrapped in one.
pub fn editor_value(raw: &str) -> String {
    let raw = raw.trim();
    if is_doc(raw) {
        return raw.to_string();
    }
    RichTextDoc::new()
        .paragraph(raw)
        .ensure_non_empty()
        .to_json_string()
}

/// What the booklet shows: the document as `RichText`, other text as `Markdown`; `None` when
/// there are no words.
pub fn component(raw: &str) -> Option<Component> {
    let raw = raw.trim();
    if plain_text(raw).trim().is_empty() {
        return None;
    }
    Some(if is_doc(raw) {
        RichText::new().content(raw).into()
    } else {
        Markdown::new().content(raw).into()
    })
}

fn parse(raw: &str) -> Option<Value> {
    let raw = raw.trim();
    if !raw.starts_with('{') {
        return None;
    }
    serde_json::from_str::<Value>(raw)
        .ok()
        .filter(|value| value["type"] == "doc")
}

fn collect(node: &Value, out: &mut Vec<String>) {
    if let Some(text) = node["text"].as_str().filter(|text| !text.is_empty()) {
        out.push(text.to_string());
    }
    for child in node["content"].as_array().into_iter().flatten() {
        collect(child, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_document_reads_as_its_words() {
        let doc = r#"{"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"Bienvenue"}]}]}"#;
        assert!(is_doc(doc));
        assert_eq!(plain_text(doc), "Bienvenue");
        assert_eq!(editor_value(doc), doc);
        assert!(matches!(component(doc), Some(Component::RichText(_))));
    }

    #[test]
    fn plain_text_is_markdown_and_opens_wrapped() {
        assert_eq!(plain_text("  **Bonjour** "), "**Bonjour**");
        assert!(is_doc(&editor_value("Bonjour")));
        assert!(matches!(component("Bonjour"), Some(Component::Markdown(_))));
    }

    #[test]
    fn an_empty_document_has_no_words() {
        let empty = r#"{"type":"doc","content":[{"type":"paragraph"}]}"#;
        assert_eq!(plain_text(empty), "");
        assert!(component(empty).is_none());
        assert!(is_doc(&editor_value("")));
    }
}
