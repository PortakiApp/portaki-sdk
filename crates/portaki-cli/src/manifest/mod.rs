//! Manifest generation from macro emissions.

pub mod catalog;
pub mod generator;
pub mod loader;
pub mod migration_bundle;
pub mod operations_bundle;
pub mod source;
pub mod validator;

pub use generator::{
    collect_emissions, find_emissions_dir, find_emissions_dir_in, generate_manifest, imply_storage,
    write_manifest,
};
pub use loader::{load as load_manifest, Source as ManifestSource};
pub use migration_bundle::write_migration_bundle;
pub use operations_bundle::write_operations_bundle;
pub use validator::validate_manifest;

use std::path::Path;

/// A file of the module that is about to be published: neither a symbolic link nor, through a
/// linked directory, anything outside `module_root`.
///
/// What `publish` reads from `i18n/` or `db/migrations/` ends up in a public OCI layer. Followed
/// blindly, `i18n/fr.json -> ~/.aws/credentials` shipped the target to everyone.
pub fn ensure_inside(module_root: &Path, path: &Path) -> anyhow::Result<()> {
    use anyhow::Context;
    let meta =
        std::fs::symlink_metadata(path).with_context(|| format!("read {}", path.display()))?;
    if meta.file_type().is_symlink() {
        anyhow::bail!(
            "{} is a symbolic link — refusing to publish it",
            path.display()
        );
    }
    let root = module_root
        .canonicalize()
        .with_context(|| format!("resolve {}", module_root.display()))?;
    let resolved = path
        .canonicalize()
        .with_context(|| format!("resolve {}", path.display()))?;
    if !resolved.starts_with(&root) {
        anyhow::bail!(
            "{} resolves outside the module ({}) — refusing to publish it",
            path.display(),
            resolved.display()
        );
    }
    Ok(())
}
