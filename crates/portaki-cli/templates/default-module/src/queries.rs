//! Module queries — what the dashboard reads.

use portaki_sdk::prelude::*;

use crate::config::{load_config, ModuleConfig};

#[portaki_sdk::query(name = "getConfig")]
pub fn get_config(_ctx: Context) -> Result<ModuleConfig> {
    load_config()
}
