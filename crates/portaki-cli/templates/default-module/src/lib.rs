//! Default Portaki module template.

use portaki_sdk::prelude::*;

mod guest;
mod host;
mod ids;

portaki_sdk::portaki_module!(
    id = "{{MODULE_NAME}}",
    display_name_key = "module.displayName",
    description_key = "module.description",
    author = "Portaki",
);

#[portaki_sdk::capability(required, id = "core.storage")]
pub const STORAGE: CapabilityId = capability::core::STORAGE;

#[portaki_sdk::entity(schema_version = 1)]
pub struct SampleItem {
    pub id: uuid::Uuid,
    pub title: String,
}
