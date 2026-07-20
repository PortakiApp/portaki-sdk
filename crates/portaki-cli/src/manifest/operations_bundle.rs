//! Builds schema-only `operations.bundle.json` v2 from `#[entity]` emissions.
//!
//! Minimum viable for typed-repo `createWithSchema` / upsert — empty `operations` map.
//! Full AS-style SQL steps are out of scope here.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use portaki_sdk::manifest::ManifestEntity;
use serde::Serialize;
use serde_json::Value;

/// OCI / runtime artifact consumed by `ModuleArtifactBundleLoader`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleOperationsBundle {
    pub bundle_version: u32,
    pub module_id: String,
    pub module_version: String,
    pub schema_version: String,
    pub schema: ModuleSchemaBundle,
    pub operations: serde_json::Map<String, Value>,
}

#[derive(Debug, Serialize)]
pub struct ModuleSchemaBundle {
    pub tables: Vec<TableDef>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TableDef {
    pub logical_name: String,
    pub table_name: String,
    pub columns: Vec<ColumnDef>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ColumnDef {
    pub name: String,
    pub sql_name: String,
    #[serde(rename = "type")]
    pub column_type: String,
    pub nullable: bool,
    pub primary_key: bool,
    pub unique: bool,
    pub default_sql: Option<String>,
}

/// Writes `operations.bundle.json` when the module declares entities.
pub fn write_operations_bundle(
    out_dir: &Path,
    module_id: &str,
    module_version: &str,
    schema_version: u32,
    entities: &[ManifestEntity],
) -> Result<Option<PathBuf>> {
    if entities.is_empty() {
        return Ok(None);
    }

    let tables: Vec<TableDef> = entities
        .iter()
        .map(|entity| entity_to_table(module_id, entity))
        .collect();

    let bundle = ModuleOperationsBundle {
        bundle_version: 2,
        module_id: module_id.to_string(),
        module_version: module_version.to_string(),
        schema_version: schema_version.to_string(),
        schema: ModuleSchemaBundle { tables },
        operations: serde_json::Map::new(),
    };

    let dest = out_dir.join("operations.bundle.json");
    let json = serde_json::to_string_pretty(&bundle).context("serialize operations.bundle.json")?;
    fs::write(&dest, format!("{json}\n")).context("write operations.bundle.json")?;
    Ok(Some(dest))
}

fn entity_to_table(module_id: &str, entity: &ManifestEntity) -> TableDef {
    let logical_name = pascal_to_snake(&entity.name);
    let schema = format!("module_{}", module_id.replace('-', "_"));
    let table_name = format!("{schema}.{logical_name}");

    let mut columns = Vec::new();
    let mut has_property_id = false;

    for field in &entity.fields {
        let field_name = field
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if field_name.is_empty() {
            continue;
        }
        let rust_type = field.get("type").and_then(Value::as_str).unwrap_or("String");
        let (column_type, nullable) = rust_type_to_sql(rust_type);
        let sql_name = to_snake_case(field_name);
        let name = snake_to_camel(&sql_name);
        if sql_name == "property_id" {
            has_property_id = true;
        }
        let primary_key = sql_name == "id";
        let default_sql = default_sql_for(&sql_name, &column_type);
        columns.push(ColumnDef {
            name,
            sql_name: sql_name.clone(),
            column_type,
            nullable: nullable && !primary_key,
            primary_key,
            unique: primary_key,
            default_sql,
        });
    }

    if !has_property_id {
        // Host injects property scope; always present on module entity tables.
        let insert_at = columns
            .iter()
            .position(|column| column.sql_name == "id")
            .map(|index| index + 1)
            .unwrap_or(0);
        columns.insert(
            insert_at,
            ColumnDef {
                name: "propertyId".to_string(),
                sql_name: "property_id".to_string(),
                column_type: "uuid".to_string(),
                nullable: false,
                primary_key: false,
                unique: true,
                default_sql: None,
            },
        );
    }

    TableDef {
        logical_name,
        table_name,
        columns,
    }
}

fn rust_type_to_sql(rust_type: &str) -> (String, bool) {
    let normalized: String = rust_type.chars().filter(|c| !c.is_whitespace()).collect();
    if let Some(inner) = normalized
        .strip_prefix("Option<")
        .and_then(|rest| rest.strip_suffix('>'))
    {
        let (inner_type, _) = rust_type_to_sql(inner);
        return (inner_type, true);
    }

    let column_type = if normalized == "Uuid" || normalized.ends_with("::Uuid") {
        "uuid"
    } else if normalized == "String" || normalized == "str" || normalized == "&str" {
        "text"
    } else if normalized == "bool" {
        "boolean"
    } else if matches!(
        normalized.as_str(),
        "i8" | "i16" | "i32" | "u8" | "u16" | "u32"
    ) {
        "int"
    } else if matches!(normalized.as_str(), "i64" | "u64" | "isize" | "usize") {
        "bigint"
    } else if matches!(normalized.as_str(), "f32" | "f64") {
        "float"
    } else if normalized.starts_with("DateTime") {
        "timestamptz"
    } else if normalized.contains("Value") || normalized.contains("JsonValue") {
        "jsonb"
    } else {
        // Enums / opaque types serialize as text JSON strings in practice.
        "text"
    };

    (column_type.to_string(), false)
}

fn default_sql_for(sql_name: &str, column_type: &str) -> Option<String> {
    if column_type == "timestamptz"
        && (sql_name == "created_at" || sql_name == "updated_at" || sql_name.ends_with("_at"))
    {
        return Some("now()".to_string());
    }
    None
}

fn pascal_to_snake(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 4);
    for (index, character) in name.chars().enumerate() {
        if character.is_uppercase() && index > 0 {
            out.push('_');
        }
        out.extend(character.to_lowercase());
    }
    out
}

fn to_snake_case(name: &str) -> String {
    if name.contains('_') || name.chars().all(|c| c.is_lowercase() || !c.is_alphabetic()) {
        return name.to_string();
    }
    pascal_to_snake(name)
}

fn snake_to_camel(snake: &str) -> String {
    let mut out = String::with_capacity(snake.len());
    let mut upper_next = false;
    for (index, character) in snake.chars().enumerate() {
        if character == '_' {
            upper_next = true;
            continue;
        }
        if upper_next {
            out.extend(character.to_uppercase());
            upper_next = false;
        } else if index == 0 {
            out.extend(character.to_lowercase());
        } else {
            out.push(character);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn when_entities_present_then_writes_v2_schema_bundle() {
        let out = tempdir().unwrap();
        let entities = vec![ManifestEntity {
            name: "AppliancesContent".to_string(),
            schema_version: 1,
            fields: vec![
                json!({"name": "id", "type": "Uuid"}),
                json!({"name": "content_fr", "type": "String"}),
                json!({"name": "content_en", "type": "String"}),
                json!({"name": "created_at", "type": "DateTime < Utc >"}),
                json!({"name": "updated_at", "type": "DateTime < Utc >"}),
            ],
        }];

        let path = write_operations_bundle(out.path(), "appliances", "0.2.3", 1, &entities)
            .unwrap()
            .expect("bundle path");
        assert!(path.ends_with("operations.bundle.json"));

        let raw = fs::read_to_string(path).unwrap();
        let value: Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(value["bundleVersion"], 2);
        assert_eq!(value["moduleId"], "appliances");
        assert_eq!(value["moduleVersion"], "0.2.3");
        assert_eq!(value["schemaVersion"], "1");
        assert!(value["operations"].as_object().unwrap().is_empty());

        let table = &value["schema"]["tables"][0];
        assert_eq!(table["logicalName"], "appliances_content");
        assert_eq!(table["tableName"], "module_appliances.appliances_content");

        let columns = table["columns"].as_array().unwrap();
        assert_eq!(columns[0]["name"], "id");
        assert_eq!(columns[0]["sqlName"], "id");
        assert_eq!(columns[0]["type"], "uuid");
        assert_eq!(columns[0]["primaryKey"], true);

        assert_eq!(columns[1]["name"], "propertyId");
        assert_eq!(columns[1]["sqlName"], "property_id");
        assert_eq!(columns[1]["type"], "uuid");
        assert_eq!(columns[1]["unique"], true);

        assert_eq!(columns[2]["name"], "contentFr");
        assert_eq!(columns[2]["sqlName"], "content_fr");
        assert_eq!(columns[2]["type"], "text");

        let created = columns
            .iter()
            .find(|column| column["sqlName"] == "created_at")
            .unwrap();
        assert_eq!(created["type"], "timestamptz");
        assert_eq!(created["defaultSql"], "now()");
    }

    #[test]
    fn when_no_entities_then_skips_bundle() {
        let out = tempdir().unwrap();
        let path = write_operations_bundle(out.path(), "empty", "1.0.0", 1, &[]).unwrap();
        assert!(path.is_none());
        assert!(!out.path().join("operations.bundle.json").exists());
    }

    #[test]
    fn pascal_to_snake_matches_runtime_resolver() {
        assert_eq!(pascal_to_snake("AppliancesContent"), "appliances_content");
        assert_eq!(pascal_to_snake("WeatherCache"), "weather_cache");
        assert_eq!(pascal_to_snake("PlaceEntry"), "place_entry");
    }
}
