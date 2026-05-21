//! `host::log` — structured logging forwarded to platform observability.

use serde::Serialize;
use std::collections::BTreeMap;

use crate::error::Result;
use crate::host::runtime::backend;

/// Structured log fields.
#[derive(Debug, Default)]
pub struct Fields {
    values: BTreeMap<String, serde_json::Value>,
}

impl Fields {
    /// Creates an empty field map.
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a serializable field.
    pub fn insert<T: Serialize>(&mut self, key: impl Into<String>, value: &T) {
        if let Ok(json) = serde_json::to_value(value) {
            self.values.insert(key.into(), json);
        }
    }
}

fn write(level: &str, message: &str, fields: &Fields) -> Result<()> {
    let fields_json = serde_json::to_string(&fields.values)?;
    backend()?.log(level, message, &fields_json)
}

/// Debug log.
pub fn debug(message: &str, fields: &Fields) -> Result<()> {
    write("debug", message, fields)
}

/// Info log.
pub fn info(message: &str, fields: &Fields) -> Result<()> {
    write("info", message, fields)
}

/// Warning log.
pub fn warn(message: &str, fields: &Fields) -> Result<()> {
    write("warn", message, fields)
}

/// Error log.
pub fn error(message: &str, fields: &Fields) -> Result<()> {
    write("error", message, fields)
}
