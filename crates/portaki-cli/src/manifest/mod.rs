//! Manifest generation from macro emissions.

pub mod generator;
pub mod loader;
pub mod migration_bundle;
pub mod operations_bundle;
pub mod validator;

pub use generator::{collect_emissions, find_emissions_dir, generate_manifest, write_manifest};
pub use loader::{load as load_manifest, Source as ManifestSource};
pub use migration_bundle::write_migration_bundle;
pub use operations_bundle::write_operations_bundle;
pub use validator::validate_manifest;
