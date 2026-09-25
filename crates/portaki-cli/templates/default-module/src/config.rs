//! The settings the host fills in.
//!
//! Declared here, held by the platform: it checks what the host saves, stores it, blocks the
//! publication while a `required` field is empty, and hands it back on every invocation. No
//! `updateConfig` to write, no KV key, no `publishReadiness` for an empty field.

use serde::{Deserialize, Serialize};

#[portaki_sdk::config]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleConfig {
    /// What the guest card greets with. Empty means the bundled wording.
    #[field(label = "host.greeting.label")]
    pub greeting: String,
}
