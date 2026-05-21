//! Manifest generation from macro emissions.

pub mod generator;
pub mod validator;

pub use generator::{collect_emissions, find_emissions_dir, generate_manifest, write_manifest};
pub use validator::validate_manifest;
