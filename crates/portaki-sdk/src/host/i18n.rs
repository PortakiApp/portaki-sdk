//! `host::i18n` — bundle resolution helpers.

use std::collections::BTreeMap;

use crate::error::Result;
use crate::host::runtime::backend;

/// Variables passed to translation lookups.
#[derive(Debug, Clone, Default)]
pub struct Vars {
    values: BTreeMap<String, String>,
}

impl Vars {
    /// Creates an empty variable map.
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a variable (stringified via `Display`).
    pub fn set(&mut self, key: impl Into<String>, value: impl std::fmt::Display) {
        self.values.insert(key.into(), value.to_string());
    }
}

/// Resolves `key` using the invocation locale.
pub fn translate(key: &str, vars: &Vars) -> Result<String> {
    let vars_json = serde_json::to_string(&vars.values)?;
    backend()?.i18n_translate(key, &vars_json)
}
