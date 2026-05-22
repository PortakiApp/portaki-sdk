use portaki_sdk::capability;

const CAP_CORE_STORAGE: &str = "core.storage";

#[capability(required)]
pub const STORAGE: &str = CAP_CORE_STORAGE;

fn main() {}
