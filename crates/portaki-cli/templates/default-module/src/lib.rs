//! Default Portaki module template.

use portaki_sdk::prelude::*;

mod config;
mod guest;
mod host;

// Re-exported so `tests/` can reach them: an integration test links the crate from outside.
pub use config::ModuleConfig;
pub use guest::render_guest_home_card;
pub use host::render_host_main;

portaki_sdk::portaki_module!(
    id = "{{MODULE_NAME}}",
    display_name_key = "module.displayName",
    description_key = "module.description",
    author = "{{AUTHOR_NAME}}",
    author_url = "https://github.com/TODO",
    module_type = ModuleType::Community,
    icon = IconName::Grid,
    maturity = Maturity::Beta,
);

#[portaki_sdk::capability(required, id = "core.storage")]
pub const STORAGE: CapabilityId = capability::core::STORAGE;

#[portaki_sdk::entity(schema_version = 1)]
pub struct SampleItem {
    pub id: uuid::Uuid,
    pub title: String,
}
