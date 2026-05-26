//! Manifest generation from macro emissions.

pub mod generator;
pub mod migration_bundle;
pub mod validator;

pub use generator::{collect_emissions, find_emissions_dir, generate_manifest, write_manifest};
pub use migration_bundle::write_migration_bundle;
pub use validator::validate_manifest;
