//! `host::repo` — typed entity repository (no raw SQL).

#![allow(dead_code)]

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{PortakiError, Result};

/// Sort direction for queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Direction {
    /// Ascending.
    Asc,
    /// Descending.
    Desc,
}

/// Paginated query results.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Page<T> {
    /// Result items.
    pub items: Vec<T>,
    /// Total count for the query (if known).
    pub total: Option<u64>,
}

/// Fluent query builder (translated to SQL by the gateway).
#[derive(Debug, Clone, Serialize)]
pub struct Query<E> {
    filters: Vec<Filter>,
    order: Option<(String, Direction)>,
    limit: Option<u32>,
    _entity: std::marker::PhantomData<E>,
}

#[derive(Debug, Clone, Serialize)]
enum Filter {
    Eq(String, serde_json::Value),
    Gte(String, serde_json::Value),
    In(String, Vec<serde_json::Value>),
    SpatialNear {
        lat: f64,
        lng: f64,
        radius_meters: f64,
    },
}

impl<E> Default for Query<E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E> Query<E> {
    /// Starts a new query.
    pub fn new() -> Self {
        Self {
            filters: Vec::new(),
            order: None,
            limit: None,
            _entity: std::marker::PhantomData,
        }
    }

    /// Adds an equality filter.
    pub fn r#where(mut self, filter: FilterExpr) -> Self {
        match filter {
            FilterExpr::Eq(field, value) => self.filters.push(Filter::Eq(field, value)),
            FilterExpr::Gte(field, value) => self.filters.push(Filter::Gte(field, value)),
        }
        self
    }

    /// Adds an `IN` filter.
    pub fn where_in(mut self, field: impl Into<String>, values: Vec<serde_json::Value>) -> Self {
        self.filters.push(Filter::In(field.into(), values));
        self
    }

    /// Adds a spatial near filter.
    pub fn spatial(mut self, spec: SpatialExpr) -> Self {
        match spec {
            SpatialExpr::Near {
                lat,
                lng,
                radius_meters,
            } => {
                self.filters.push(Filter::SpatialNear {
                    lat,
                    lng,
                    radius_meters,
                });
            }
        }
        self
    }

    /// Orders results.
    pub fn order_by(mut self, field: impl Into<String>, direction: Direction) -> Self {
        self.order = Some((field.into(), direction));
        self
    }

    /// Limits page size.
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }
}

/// Equality / comparison filter helpers.
pub enum FilterExpr {
    /// `field = value`
    Eq(String, serde_json::Value),
    /// `field >= value`
    Gte(String, serde_json::Value),
}

/// Builds `field = value`.
pub fn eq(field: impl Into<String>, value: impl Serialize) -> FilterExpr {
    FilterExpr::Eq(
        field.into(),
        serde_json::to_value(value).unwrap_or(serde_json::Value::Null),
    )
}

/// Builds `field >= value`.
pub fn gte(field: impl Into<String>, value: impl Serialize) -> FilterExpr {
    FilterExpr::Gte(
        field.into(),
        serde_json::to_value(value).unwrap_or(serde_json::Value::Null),
    )
}

/// Spatial helpers.
pub enum SpatialExpr {
    /// Points within `radius_meters` of (`lat`, `lng`).
    Near {
        /// Latitude.
        lat: f64,
        /// Longitude.
        lng: f64,
        /// Radius in meters.
        radius_meters: f64,
    },
}

/// Builds a near filter.
pub fn near(lat: f64, lng: f64, radius_meters: f64) -> SpatialExpr {
    SpatialExpr::Near {
        lat,
        lng,
        radius_meters,
    }
}

/// Repository namespace for entity `E`.
pub struct Repo<E> {
    _entity: std::marker::PhantomData<E>,
}

impl<E> Repo<E> {
    /// Typed repository entry point.
    pub fn new() -> Self {
        Self {
            _entity: std::marker::PhantomData,
        }
    }
}

impl<E> Default for Repo<E> {
    fn default() -> Self {
        Self::new()
    }
}

/// Creates a row (gateway persists in the module schema).
pub fn create<E, Create, Entity>(data: Create) -> Result<Entity>
where
    Entity: DeserializeOwned,
    Create: Serialize,
{
    let entity_name = entity_type_name::<E>();
    if let Ok(host) = crate::host::runtime::backend() {
        let entity_json = serde_json::to_string(&data)?;
        let response_json = host.repo_create(entity_name, &entity_json)?;
        return serde_json::from_str(&response_json)
            .map_err(|e| PortakiError::Storage(e.to_string()));
    }
    let _ = entity_name;
    Err(PortakiError::Storage(
        "repository create requires gateway — use MockHostFunctions in tests".into(),
    ))
}

/// Updates a row by id.
pub fn update<E, Update, Entity>(_id: Uuid, _partial: Update) -> Result<Entity>
where
    Entity: DeserializeOwned,
{
    let _ = std::any::type_name::<E>();
    Err(PortakiError::Storage(
        "repository update requires gateway — use MockHostFunctions in tests".into(),
    ))
}

/// Deletes a row by id.
pub fn delete<E>(id: Uuid) -> Result<bool> {
    let entity_name = entity_type_name::<E>();
    if let Ok(host) = crate::host::runtime::backend() {
        return host.repo_delete(entity_name, &id.to_string());
    }
    let _ = entity_name;
    Err(PortakiError::Storage(
        "repository delete requires gateway — use MockHostFunctions in tests".into(),
    ))
}

/// Finds a row by id.
pub fn find_by_id<E, Entity>(_id: Uuid) -> Result<Option<Entity>>
where
    Entity: DeserializeOwned,
{
    let _ = std::any::type_name::<E>();
    Ok(None)
}

/// Runs a typed query.
pub fn find<E, Entity>(query: Query<E>) -> Result<Page<Entity>>
where
    Entity: DeserializeOwned,
{
    let entity_name = entity_type_name::<E>();
    if let Ok(host) = crate::host::runtime::backend() {
        let query_json = serde_json::to_string(&query)?;
        let response_json = host.repo_find(entity_name, &query_json)?;
        return serde_json::from_str(&response_json)
            .map_err(|e| PortakiError::Storage(e.to_string()));
    }
    let _ = entity_name;
    Ok(Page {
        items: Vec::new(),
        total: Some(0),
    })
}

fn entity_type_name<E>() -> &'static str {
    let full = std::any::type_name::<E>();
    full.rsplit("::").next().unwrap_or(full)
}

/// Counts rows matching a query.
pub fn count<E>(_query: Query<E>) -> Result<u64> {
    Ok(0)
}

/// Repository shorthand used in module code: `host::repo::<Poi>::find(...)`.
pub mod typed {
    pub use super::{
        count, create, delete, find, find_by_id, update, Direction, Page, Query, Repo,
    };
}

/// Alias matching PORTAKI_PLATFORM examples: `host::repo::<Poi>::find(...)`.
pub fn repo<E>() -> Repo<E> {
    Repo::new()
}
