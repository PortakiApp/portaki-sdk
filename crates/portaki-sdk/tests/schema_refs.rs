//! The manifest schema has to be usable: every local reference points somewhere.
//!
//! A `$ref` to a definition that does not exist stays invisible for as long as no manifest
//! exercises the property carrying it. 6.1.0 published `queries` and `commands` with items
//! `#/$defs/operation` without defining `operation`: every existing manifest still validated, and
//! the first module bound to that release would have got « schéma inexploitable » in devapi. This
//! test walks the whole schema rather than the manifests, so that it leans on no example.

use serde_json::Value;

const SCHEMA: &str = include_str!("../../../schema/module.v1.json");

fn local_refs(node: &Value, found: &mut Vec<String>) {
    match node {
        Value::Object(map) => {
            for (key, value) in map {
                if key == "$ref" {
                    if let Value::String(reference) = value {
                        found.push(reference.clone());
                    }
                } else {
                    local_refs(value, found);
                }
            }
        }
        Value::Array(items) => items.iter().for_each(|item| local_refs(item, found)),
        _ => {}
    }
}

#[test]
fn every_local_reference_resolves() {
    let schema: Value = serde_json::from_str(SCHEMA).expect("schema/module.v1.json est du JSON");
    let mut references = Vec::new();
    local_refs(&schema, &mut references);

    assert!(
        !references.is_empty(),
        "le schéma ne porte aucune référence : parcours cassé ?"
    );

    let dangling: Vec<&String> = references
        .iter()
        .filter(|reference| reference.starts_with("#/"))
        .filter(|reference| schema.pointer(&reference[1..]).is_none())
        .collect();

    assert!(
        dangling.is_empty(),
        "références qui ne pointent nulle part dans schema/module.v1.json : {dangling:?}"
    );
}

#[test]
fn operations_are_described_the_way_the_build_stamps_them() {
    let schema: Value = serde_json::from_str(SCHEMA).expect("schema");
    let operation = schema
        .pointer("/$defs/operation")
        .expect("queries et commands référencent #/$defs/operation");

    // What `portaki build` emits for each #[portaki_sdk::query] / #[portaki_sdk::command].
    let properties = operation
        .pointer("/properties")
        .expect("propriétés d'une opération");
    for field in ["name", "fn", "args", "params"] {
        assert!(
            properties.get(field).is_some(),
            "champ `{field}` absent de $defs/operation"
        );
    }
    let required = operation
        .pointer("/required")
        .and_then(Value::as_array)
        .expect("required");
    assert!(
        required.iter().any(|field| field == "name"),
        "le nom est ce qu'on dispatche : il doit être requis"
    );
}
