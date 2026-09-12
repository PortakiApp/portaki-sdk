//! Module commands — what the dashboard writes.

use portaki_sdk::prelude::*;
use serde::{Deserialize, Serialize};

use crate::config::{save_config, ModuleConfig};

/// The flat field map the modules sheet posts when the host presses Save.
///
/// Every name here matches an input in `host::render_host_main`: the shell merges field values
/// at click time, so a renamed input silently stops arriving.
#[portaki_sdk::params]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateConfigArgs {
    #[serde(default)]
    pub greeting: String,
}

#[portaki_sdk::command(name = "updateConfig")]
pub fn update_config(_ctx: Context, args: UpdateConfigArgs) -> Result<()> {
    save_config(&ModuleConfig {
        greeting: args.greeting.trim().to_string(),
    })
}
