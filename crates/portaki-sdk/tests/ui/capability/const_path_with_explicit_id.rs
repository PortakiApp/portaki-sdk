use portaki_sdk::capability::{self, CapabilityId};

#[portaki_sdk::capability(required, id = "core.storage")]
pub const STORAGE: CapabilityId = capability::core::STORAGE;

fn main() {}
