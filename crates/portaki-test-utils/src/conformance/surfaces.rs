//! Every declared surface renders in its shell with an empty mock, into contract primitives —
//! and every guest surface still shows something when the module is off, incomplete or failing.

use portaki_sdk::host::module::ModuleStatus;
use portaki_sdk::sdui::component::Component;
use portaki_sdk::sdui::primitives::{RichText, SduiPrimitive, Select};
use portaki_sdk::sdui::surface::Surface;
use portaki_sdk::wasm::registry::{HandlerDeclaration, HandlerKind};
use serde_json::Value;

use super::invoke::{describe, empty_params, invoke, surface_mock, Invocation, Outcome};
use super::{Findings, Module, MANIFEST_FILE, NO_DECLARATIONS};
use crate::{MockContextBuilder, SurfaceAssertions};

pub(super) fn check(module: &Module) -> Result<(), Findings> {
    Findings::of("surfaces", problems(module))
}

/// Every declared surface, rendered once with an empty mock in its own shell.
pub(super) fn render_all(module: &Module) -> Vec<(&'static HandlerDeclaration, Invocation)> {
    let module_id = module_id(module);
    module
        .declarations()
        .into_iter()
        .filter(|declaration| declaration.kind == HandlerKind::Surface)
        .map(|declaration| {
            let mock = surface_mock(declaration, module_id.as_deref());
            (declaration, invoke(declaration, mock, empty_params()))
        })
        .collect()
}

/// The manifest's `id`, when it can be read.
pub(super) fn module_id(module: &Module) -> Option<String> {
    module
        .manifest()
        .ok()
        .flatten()
        .and_then(|manifest| manifest.get("id")?.as_str().map(str::to_string))
}

fn problems(module: &Module) -> Vec<String> {
    let declarations = module.declarations();
    if declarations.is_empty() {
        return vec![NO_DECLARATIONS.to_string()];
    }

    let mut problems = Vec::new();
    for (declaration, invocation) in render_all(module) {
        let what = describe(declaration);
        match invocation.outcome {
            Outcome::Panicked(message) => {
                problems.push(format!("{what} panicked with an empty mock: {message}"))
            }
            Outcome::Failed(error) => problems.push(format!(
                "{what} failed with an empty mock — a first install has no data either, render an \
                 empty state: {error}"
            )),
            Outcome::Answered(tree) => {
                // The SDK turned an `Err` into the guest error state: the guest would see
                // « temporarily unavailable » on a first install.
                if let Some(error) = render_failure(declaration, &invocation.logs) {
                    problems.push(format!(
                        "{what} failed with an empty mock — a first install has no data either, \
                         render an empty state: {error}"
                    ));
                }
                problems.extend(
                    contract_problems(&tree)
                        .into_iter()
                        .map(|problem| format!("{what} {problem}")),
                )
            }
        }
    }

    problems.extend(guest_state_problems(module));
    problems.extend(rendered_file_problems(module));
    problems.extend(undeclared_guest_routes(module, &declarations));
    problems
}

/// The module's committed renderings, held to the same rules as a live one.
///
/// The walk above renders with an empty mock, where a `content` built from data is simply not
/// there to look at: `appliances` put pre-rendered HTML in its how-to card, and its guest detail
/// answers an empty state until an appliance is saved — every check stayed green, and the tags
/// only showed on a page that had one. These files are the module's rendering **on** data,
/// committed next to the manifest and reviewed in a PR: the one place a test reads it.
///
/// A module that keeps neither passes — the files are a catalogue and a demonstration, not a duty.
fn rendered_file_problems(module: &Module) -> Vec<String> {
    RENDERED_FILES
        .iter()
        .flat_map(|file| {
            let path = module.root().join(file);
            let Ok(raw) = std::fs::read_to_string(&path) else {
                return Vec::new();
            };
            let Ok(document) = serde_json::from_str::<Value>(&raw) else {
                // Whatever wrote the file owns its syntax; this check only reads what parses.
                return Vec::new();
            };
            rich_text_problems(&document)
                .into_iter()
                .map(|problem| format!("{file} {problem}"))
                .collect()
        })
        .collect()
}

/// What a module commits of its own rendering: the catalogue previews, and the booklet's
/// demonstration (`demo.json`, which the modules repository writes beside them).
const RENDERED_FILES: [&str; 2] = [crate::previews::FILE, "demo.json"];

/// The error a guest surface logged through `portaki_sdk::guest_shell`, if it did.
fn render_failure(declaration: &HandlerDeclaration, logs: &[crate::LogLine]) -> Option<String> {
    logs.iter()
        .find(|line| {
            line.level == "error"
                && line.message.ends_with("_render_failed")
                && line.fields["surfaceId"] == declaration.name
        })
        .map(|line| line.fields["error"].as_str().unwrap_or("?").to_string())
}

/// `mock` in each platform state a guest surface meets: module off, config incomplete, host
/// failing.
fn guest_states(mock: MockContextBuilder) -> [(&'static str, MockContextBuilder); 3] {
    fn status(active: bool, incomplete: bool) -> ModuleStatus {
        ModuleStatus {
            active,
            workspace_enabled: true,
            incomplete,
            requires_config: incomplete,
            missing_required_keys: if incomplete {
                vec!["required".to_string()]
            } else {
                Vec::new()
            },
        }
    }
    [
        (
            "inactive",
            mock.clone().with_module_status(status(false, false)),
        ),
        (
            "incomplete",
            mock.clone().with_module_status(status(true, true)),
        ),
        (
            "error",
            mock.with_module_status_error("module_status_unavailable"),
        ),
    ]
}

/// Every guest surface, in each of [`guest_states`]: it answers, with something to read.
///
/// Through the SDK's guest shell this holds by construction; a surface with `gate = false`
/// answers for itself.
fn guest_state_problems(module: &Module) -> Vec<String> {
    let module_id = module_id(module);
    let mut problems = Vec::new();
    for declaration in module.declarations() {
        if declaration.kind != HandlerKind::Surface || declaration.context != "guest" {
            continue;
        }
        let what = describe(declaration);
        for (state, mock) in guest_states(surface_mock(declaration, module_id.as_deref())) {
            match invoke(declaration, mock, empty_params()).outcome {
                Outcome::Panicked(message) => problems.push(format!(
                    "{what} panicked with the module {state}: {message}"
                )),
                Outcome::Failed(error) => problems.push(format!(
                    "{what} failed with the module {state} — the guest sees nothing, render a \
                     state: {error}"
                )),
                Outcome::Answered(tree) if is_blank(&tree) => problems.push(format!(
                    "{what} rendered nothing to read with the module {state}"
                )),
                Outcome::Answered(tree) => problems.extend(
                    contract_problems(&tree)
                        .into_iter()
                        .map(|problem| format!("{what} with the module {state} {problem}")),
                ),
            }
        }
    }
    problems
}

/// No text anywhere in the tree — the node types, ids and icons aside.
fn is_blank(tree: &Value) -> bool {
    match tree {
        Value::String(text) => text.trim().is_empty(),
        Value::Array(items) => items.iter().all(is_blank),
        Value::Object(map) => map
            .iter()
            .filter(|(key, _)| !matches!(key.as_str(), "type" | "id" | "icon"))
            .all(|(_, value)| is_blank(value)),
        _ => true,
    }
}

/// What the shell would refuse in the JSON a surface sends.
pub(super) fn contract_problems(tree: &Value) -> Vec<String> {
    // On the raw tree, so a `content` is still reported when the rest does not parse.
    let mut problems = rich_text_problems(tree);
    let surface: Surface = match serde_json::from_value(tree.clone()) {
        Ok(surface) => surface,
        Err(error) => {
            problems.push(format!(
                "sent a tree that does not parse as SDUI primitives of the contract: {error}"
            ));
            return problems;
        }
    };

    // Typed Rust cannot build a node outside the contract; this guards the wire instead — a
    // tree that re-parsed into a variant the contract does not list would be a generator bug.
    let mut unknown: Vec<String> = Vec::new();
    for node in SurfaceAssertions::new(&surface).nodes() {
        let name = node.type_name();
        if !Component::TYPE_NAMES.contains(&name) {
            unknown.push(name.to_string());
        }
        if let Component::Select(select) = node {
            problems.extend(select_problems(select));
        }
    }
    if !unknown.is_empty() {
        problems.push(format!(
            "rendered nodes outside the SDUI contract: {}",
            unknown.join(", ")
        ));
    }
    problems
}

/// A `Select` the host cannot use: nothing to pick, or a value none of its options carry.
///
/// An empty `value` means « nothing picked yet » and passes.
fn select_problems(select: &Select) -> Vec<String> {
    let name = select.name.as_deref().unwrap_or("?");
    let options = select.options.as_deref().unwrap_or_default();
    if options.is_empty() {
        return vec![format!("rendered Select `{name}` without options")];
    }
    match select.value.as_deref() {
        Some(value) if !value.is_empty() && !options.iter().any(|o| o.value == value) => {
            vec![format!(
                "rendered Select `{name}` with value `{value}`, which none of its options carries"
            )]
        }
        _ => Vec::new(),
    }
}

/// A manifest route to a surface the module does not serve: the booklet link leads nowhere.
fn undeclared_guest_routes(module: &Module, declarations: &[&HandlerDeclaration]) -> Vec<String> {
    let Ok(Some(manifest)) = module.manifest() else {
        return Vec::new();
    };
    let routes = manifest
        .get("guestSurfaces")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    routes
        .iter()
        .filter_map(|route| route.get("surfaceId")?.as_str())
        .filter(|surface_id| {
            !declarations.iter().any(|declaration| {
                declaration.kind == HandlerKind::Surface
                    && declaration.context == "guest"
                    && declaration.name == *surface_id
            })
        })
        .map(|surface_id| {
            format!(
                "{MANIFEST_FILE} routes guestSurfaces `{surface_id}`, but no \
                 #[surface(guest, id = \"{surface_id}\")] is declared"
            )
        })
        .collect()
}

/// Pre-rendered markup sitting in a `RichText.content`, which the guest reads as text.
///
/// `content` is a TipTap document. The booklet parses it and paints the result; anything that does
/// not parse as `{"type":"doc",…}` it renders as literal text — deliberately, since `content` is a
/// declared field whose value the platform boundary does not inspect, and `<img src=x onerror=…>`
/// from a module used to execute in the booklet. A module that pre-renders HTML into it therefore
/// ships its own tags to the guest, visible: `<p>Appuyez 2 secondes…</p>`, angle brackets and all.
///
/// Plain text passes — the booklet shows it as written, and `local-guide` puts the host's tips
/// there. An `i18n:` reference passes too: the booklet translates before it looks for a document.
///
/// Read on the serialized tree rather than on the typed one: the same rule then covers a
/// `previews.json` rendering, where this is the shape that is actually kept.
pub(crate) fn rich_text_problems(tree: &Value) -> Vec<String> {
    rich_text_contents(tree)
        .into_iter()
        .filter_map(|content| {
            let content = content.trim();
            if content.is_empty()
                || content.starts_with(super::i18n::I18N_PREFIX)
                || is_tiptap_doc(content)
            {
                return None;
            }
            let tag = first_markup_tag(content)?;
            Some(format!(
                "carries a RichText whose `content` holds the markup `{tag}` outside a TipTap \
                 document — `content` is a TipTap field and the booklet shows anything else as \
                 literal text, tags included; send the document \
                 (`RichTextDoc::to_json_string`) or plain text: {}",
                excerpt(content)
            ))
        })
        .collect()
}

/// Every `RichText.content` of a serialized tree, wherever it sits.
fn rich_text_contents(tree: &Value) -> Vec<&str> {
    let mut found = Vec::new();
    let mut stack = vec![tree];
    while let Some(value) = stack.pop() {
        match value {
            Value::Object(fields) => {
                if fields.get("type").and_then(Value::as_str) == Some(RichText::TYPE_NAME) {
                    found.extend(fields.get("content").and_then(Value::as_str));
                }
                stack.extend(fields.values());
            }
            Value::Array(items) => stack.extend(items),
            _ => {}
        }
    }
    found
}

/// The booklet's own test: it parses as JSON, and its `type` is `doc`.
///
/// The document itself is not validated against a node vocabulary — the booklet's converter walks
/// what it knows and falls through to the text of the rest, and a module may legitimately carry a
/// node it renders itself (`appliances` puts an `image` in its how-to steps).
fn is_tiptap_doc(content: &str) -> bool {
    content.starts_with('{')
        && serde_json::from_str::<Value>(content)
            .is_ok_and(|doc| doc.get("type").and_then(Value::as_str) == Some("doc"))
}

/// The first `<tag …>` or `</tag>` of `text` — `</?[a-z][^>]*>`, without a regex crate.
///
/// A lone `<` is not markup: « 3 < 5 » and « <3 » are text a host may well have typed, and
/// reporting them would push modules to escape prose that renders fine.
fn first_markup_tag(text: &str) -> Option<&str> {
    let bytes = text.as_bytes();
    for open in text
        .char_indices()
        .filter(|(_, character)| *character == '<')
        .map(|(index, _)| index)
    {
        let name = if bytes.get(open + 1) == Some(&b'/') {
            open + 2
        } else {
            open + 1
        };
        if !bytes.get(name).is_some_and(u8::is_ascii_alphabetic) {
            continue;
        }
        if let Some(length) = text[name..].find('>') {
            return Some(&text[open..name + length + 1]);
        }
    }
    None
}

/// Enough of the value to recognise it in a report, on one line.
fn excerpt(content: &str) -> String {
    let flat = content.replace(['\n', '\r', '\t'], " ");
    match flat.char_indices().nth(120) {
        Some((cut, _)) => format!("{}…", &flat[..cut]),
        None => flat,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::rich_text_problems;

    fn rich_text(content: &str) -> serde_json::Value {
        json!({ "root": { "type": "Stack", "children": [{ "type": "RichText", "content": content }] } })
    }

    #[test]
    fn pre_rendered_html_is_reported_with_its_tag() {
        let problems = rich_text_problems(&rich_text("<p>Appuyez 2 secondes.</p>"));
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].contains("`<p>`"), "{problems:?}");
    }

    #[test]
    fn a_tiptap_document_plain_text_and_an_i18n_reference_pass() {
        for content in [
            r#"{"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"Appuyez."}]}]}"#,
            // What `local-guide` sends: a host's tip, shown as written.
            "Venez avant 9 h : les croissants partent vite le dimanche.",
            "i18n:explore.item.howto",
            "",
            // A lone `<` is prose, not a tag.
            "Ouvrez à 3 < 5 bars, <3",
            // An unterminated tag never closes: nothing to render as markup.
            "Appuyez <2 secondes",
        ] {
            let problems = rich_text_problems(&rich_text(content));
            assert!(problems.is_empty(), "{content:?}: {problems:?}");
        }
    }

    #[test]
    fn a_closing_tag_and_an_attribute_are_markup_too() {
        for content in ["</div> fin", "<img src=x onerror=alert(1)>", "a <br/> b"] {
            assert_eq!(
                rich_text_problems(&rich_text(content)).len(),
                1,
                "{content:?}"
            );
        }
    }

    /// A `content` the module nests deep in the tree is read like any other.
    #[test]
    fn a_nested_rich_text_is_read() {
        let tree = json!({
            "root": { "type": "Card", "children": [
                { "type": "Tabs", "items": [{ "content": { "type": "RichText", "content": "<ul><li>a</li></ul>" } }] }
            ]}
        });
        assert_eq!(rich_text_problems(&tree).len(), 1);
    }
}
