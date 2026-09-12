//! Host configuration stored in KV (`config` key).
//!
//! The orchestrator owns whether a module is enabled; what it is set to is the module's own
//! business, and this is where it keeps it. One key, one JSON blob, loaded on every render.

use portaki_sdk::host;
use portaki_sdk::prelude::*;
use serde::{Deserialize, Serialize};

const CONFIG_KEY: &str = "config";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleConfig {
    /// What the guest card greets with. Empty means the bundled wording.
    #[serde(default)]
    pub greeting: String,
}

/// Reads the settings, or their defaults when the host has saved nothing yet.
pub fn load_config() -> Result<ModuleConfig> {
    let Some(bytes) = host::kv::get(CONFIG_KEY)? else {
        return Ok(ModuleConfig::default());
    };
    serde_json::from_slice(&bytes)
        .map_err(|error| PortakiError::Storage(format!("invalid config JSON: {error}")))
}

pub fn save_config(config: &ModuleConfig) -> Result<()> {
    let bytes = serde_json::to_vec(config)
        .map_err(|error| PortakiError::Storage(format!("config serialize: {error}")))?;
    host::kv::set(CONFIG_KEY, &bytes, None)
}
