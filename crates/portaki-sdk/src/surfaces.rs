//! Rules of the shared surfaces whose tree the platform reads under its own terms.
//!
//! ## `property.public`
//!
//! [`crate::ids::convention::PROPERTY_PUBLIC`] is a block of the property's public page, rendered
//! server-side for visitors without a stay. The tree is **static**: no action, no overlay, no
//! network call from the shell.
//!
//! - The root is a `Section`. Its `title` is the proposed title; its `subtitle` is read as the
//!   **eyebrow** (sur-titre) on this surface. Both may be `i18n:` keys; the host may override
//!   them. A root that is not a `Section` is wrapped by the platform in an untitled one.
//! - Only [`PROPERTY_PUBLIC_PRIMITIVES`] may appear, `Section` at the root only.
//! - No action-bearing prop ([`ACTION_PROPS`]) on any node.
//!
//! The platform filters what breaks these rules (the node and its subtree are dropped, action
//! props stripped); `conformance!()` fails on them first, through [`check_property_public_tree`].
//!
//! ```
//! use portaki_sdk::sdui::primitives::{Section, Text};
//! use portaki_sdk::surfaces::check_property_public_tree;
//! use portaki_sdk::Component;
//!
//! let root: Component = Section::new()
//!     .title("i18n:public.title")
//!     .subtitle("i18n:public.eyebrow")
//!     .child(Text::new().text("i18n:public.body"))
//!     .into();
//! let tree = serde_json::to_value(&root).unwrap();
//! assert!(check_property_public_tree(&tree).is_empty());
//! ```

use std::fmt;

use serde_json::Value;

/// The primitives a `property.public` tree may use — `Section` at the root only.
pub const PROPERTY_PUBLIC_PRIMITIVES: &[&str] = &[
    "Section",
    "Stack",
    "Grid",
    "Text",
    "RichText",
    "Image",
    "ListItem",
    "KeyValue",
    "Badge",
    "Icon",
    "Temperature",
];

/// Props that trigger something — a command, a navigation, a link. None may appear on a
/// `property.public` tree, at any depth (`Image.url` is not one: it is what the image shows).
pub const ACTION_PROPS: &[&str] = &[
    "action",
    "actions",
    "onPress",
    "href",
    "onSubmit",
    "onMarkerTap",
    "addAction",
    "cancelAction",
    "confirmAction",
    "retryAction",
];

/// One breach of the `property.public` rules, located by a JSON path from the root (`$`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Violation {
    /// A node whose type is not in [`PROPERTY_PUBLIC_PRIMITIVES`].
    ForbiddenPrimitive {
        /// Where the node sits.
        path: String,
        /// Its `type`.
        type_name: String,
    },
    /// A `Section` below the root.
    SectionNotAtRoot {
        /// Where the nested `Section` sits.
        path: String,
    },
    /// An action-bearing prop ([`ACTION_PROPS`]).
    ActionProp {
        /// Where the object carrying it sits.
        path: String,
        /// The prop.
        prop: String,
    },
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ForbiddenPrimitive { path, type_name } => write!(
                f,
                "`{type_name}` at {path} is not allowed on property.public — only {}",
                PROPERTY_PUBLIC_PRIMITIVES.join(", ")
            ),
            Self::SectionNotAtRoot { path } => write!(
                f,
                "`Section` at {path} — on property.public a Section is the root only"
            ),
            Self::ActionProp { path, prop } => write!(
                f,
                "`{prop}` at {path} — property.public is static, no action-bearing prop"
            ),
        }
    }
}

/// Every breach of the `property.public` rules in `root`, a serialized component (the `root` of
/// a surface). Empty when the tree conforms. A forbidden node is reported once, without walking
/// into it: the platform drops its whole subtree.
pub fn check_property_public_tree(root: &Value) -> Vec<Violation> {
    let mut violations = Vec::new();
    walk(root, "$".to_string(), true, &mut violations);
    violations
}

fn walk(value: &Value, path: String, at_root: bool, violations: &mut Vec<Violation>) {
    match value {
        Value::Object(fields) => {
            if let Some(type_name) = fields.get("type").and_then(Value::as_str) {
                if !PROPERTY_PUBLIC_PRIMITIVES.contains(&type_name) {
                    violations.push(Violation::ForbiddenPrimitive {
                        path,
                        type_name: type_name.to_string(),
                    });
                    return;
                }
                if type_name == "Section" && !at_root {
                    violations.push(Violation::SectionNotAtRoot { path: path.clone() });
                }
            }
            for (key, child) in fields {
                if ACTION_PROPS.contains(&key.as_str()) {
                    violations.push(Violation::ActionProp {
                        path: path.clone(),
                        prop: key.clone(),
                    });
                    continue;
                }
                walk(child, format!("{path}.{key}"), false, violations);
            }
        }
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                walk(item, format!("{path}[{index}]"), false, violations);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::Component;

    fn section(children: Value) -> Value {
        json!({ "type": "Section", "title": "t", "subtitle": "eyebrow", "children": children })
    }

    #[test]
    fn every_allowed_primitive_is_in_the_contract() {
        for name in PROPERTY_PUBLIC_PRIMITIVES {
            assert!(Component::TYPE_NAMES.contains(name), "{name}");
        }
    }

    #[test]
    fn a_static_tree_of_allowed_primitives_passes() {
        let tree = section(json!([
            { "type": "Stack", "children": [
                { "type": "Text", "text": "i18n:a" },
                { "type": "Image", "url": "https://img/x.jpg", "alt": "x" },
                { "type": "Temperature", "value": 21.5 }
            ]},
            { "type": "Grid", "children": [
                { "type": "ListItem", "title": "Plage", "leading": { "icon": "map-pin" },
                  "trailing": { "badge": { "label": "15 min" } } },
                { "type": "KeyValue", "key": "k", "value": "v" },
                { "type": "Badge", "label": "b" },
                { "type": "Icon", "name": "sun" },
                { "type": "RichText", "content": "i18n:b" }
            ]}
        ]));
        assert_eq!(check_property_public_tree(&tree), vec![]);
    }

    #[test]
    fn every_type_outside_the_list_is_reported() {
        for name in Component::TYPE_NAMES
            .iter()
            .filter(|name| !PROPERTY_PUBLIC_PRIMITIVES.contains(name))
        {
            let tree = section(json!([{ "type": name }]));
            assert_eq!(
                check_property_public_tree(&tree),
                vec![Violation::ForbiddenPrimitive {
                    path: "$.children[0]".into(),
                    type_name: name.to_string(),
                }],
                "{name}"
            );
        }
    }

    #[test]
    fn a_nested_forbidden_node_is_reported_once_without_its_subtree() {
        let tree = section(json!([{ "type": "Stack", "children": [
            { "type": "Card", "action": { "type": "navigate" }, "children": [{ "type": "Button" }] }
        ]}]));
        assert_eq!(
            check_property_public_tree(&tree),
            vec![Violation::ForbiddenPrimitive {
                path: "$.children[0].children[0]".into(),
                type_name: "Card".into(),
            }]
        );
    }

    #[test]
    fn an_action_prop_is_reported_wherever_it_sits() {
        let tree = section(json!([
            { "type": "ListItem", "title": "x", "action": { "type": "navigate" } },
            { "type": "Stack", "children": [{ "type": "Text", "text": "y", "href": "https://x" }] }
        ]));
        assert_eq!(
            check_property_public_tree(&tree),
            vec![
                Violation::ActionProp {
                    path: "$.children[0]".into(),
                    prop: "action".into()
                },
                Violation::ActionProp {
                    path: "$.children[1].children[0]".into(),
                    prop: "href".into()
                },
            ]
        );
    }

    #[test]
    fn a_section_below_the_root_is_reported_and_a_bare_root_is_not() {
        let tree = section(json!([section(json!([]))]));
        assert_eq!(
            check_property_public_tree(&tree),
            vec![Violation::SectionNotAtRoot {
                path: "$.children[0]".into()
            }]
        );
        // The platform wraps a root that is not a Section: not a breach.
        assert!(check_property_public_tree(&json!({ "type": "Stack", "children": [] })).is_empty());
    }
}
