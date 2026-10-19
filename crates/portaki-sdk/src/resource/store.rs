//! Where a collection lives: one row of the module's `PortakiResource` entity per resource,
//! rows and settings as JSON. `#[collection]` declares the entity in the module's manifest; the
//! module never names it.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use super::{Collection, CollectionDecl, Item, Meta};
use crate::error::{PortakiError, Result};
use crate::host::repo::{self, eq, Query};

/// One stored resource.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortakiResource {
    /// Row id.
    pub id: Uuid,
    /// The resource name (`#[collection(name = …)]`).
    pub resource: String,
    /// The rows, a JSON array.
    pub rows_json: String,
    /// The settings (a collection) or the record's bookkeeping (a record), a JSON object.
    pub meta_json: String,
    /// The stay a record belongs to — a column, so purging a stay takes its records with it.
    #[serde(default)]
    pub stay_id: Option<Uuid>,
    /// Last write.
    pub updated_at: DateTime<Utc>,
}

/// The stored rows and settings of `name`, as JSON; empty when nothing is stored yet.
pub fn load_raw(name: &str) -> Result<(Vec<Value>, Value)> {
    Ok(match find(name)? {
        Some(stored) => (
            serde_json::from_str(&stored.rows_json).map_err(corrupt)?,
            serde_json::from_str(&stored.meta_json).map_err(corrupt)?,
        ),
        None => (Vec::new(), Value::Object(Default::default())),
    })
}

/// Stores `rows` and `meta` under `name`.
pub fn save_raw(name: &str, rows: &[Value], meta: &Value) -> Result<()> {
    let id = find(name)?.map_or_else(Uuid::new_v4, |stored| stored.id);
    let row = PortakiResource {
        id,
        resource: name.to_string(),
        rows_json: serde_json::to_string(rows)?,
        meta_json: serde_json::to_string(meta)?,
        stay_id: None,
        updated_at: crate::host::time::now()?,
    };
    let _: PortakiResource = repo::create::<PortakiResource, _, _>(row)?;
    Ok(())
}

/// The collection, typed. Stored rows that do not read are an error, never a silent empty list
/// the next save would write over.
pub fn load<T: Item, M: Meta>(decl: CollectionDecl) -> Result<Collection<T, M>> {
    let (rows, meta) = load_raw(decl.name)?;
    let rows = rows
        .into_iter()
        .map(serde_json::from_value)
        .collect::<std::result::Result<Vec<T>, _>>()
        .map_err(corrupt)?;
    let meta = serde_json::from_value(meta).map_err(corrupt)?;
    Ok(Collection::new(decl, rows, meta))
}

/// Stores the collection.
pub fn save<T: Item, M: Meta>(collection: &Collection<T, M>) -> Result<()> {
    let rows = collection
        .rows()
        .iter()
        .map(serde_json::to_value)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    save_raw(
        collection.decl().name,
        &rows,
        &serde_json::to_value(collection.meta())?,
    )
}

fn find(name: &str) -> Result<Option<PortakiResource>> {
    let page = repo::find::<PortakiResource, PortakiResource>(
        Query::new().r#where(eq("resource", name)).limit(1),
    )?;
    Ok(page.items.into_iter().next())
}

fn corrupt(error: serde_json::Error) -> PortakiError {
    PortakiError::Storage(format!("resource_unreadable: {error}"))
}
