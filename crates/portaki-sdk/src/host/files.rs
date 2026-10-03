//! Reading the bytes of a file the host attached to this module.
//!
//! A form field holds a [`crate::files::FileRef`] — never the content. That is enough to show a
//! photo: the platform swaps the reference for a signed URL at render time. It is not enough to
//! *derive* something from the file, and one case needs that: a GPS track, whose distance,
//! elevation and start point are read from the file itself (§2.23).
//!
//! ## Contract
//!
//! - Only a file of **this property and this module** can be read. The platform checks it; a
//!   reference borrowed from elsewhere answers `FileNotFound`, not someone else's bytes.
//! - The host refuses anything above [`crate::limits::HOST_FILE_READ_MAX_BYTES`]. A module must
//!   not assume it can hold an arbitrary file in memory.
//! - The bytes are what the host stored, unchanged. Whatever the platform checked at the door, a
//!   module parses defensively — it is the one that decides what the content means.
//!
//! Requires the `host-files` feature, which declares the permission.
//!
//! # Examples
//!
//! ```no_run
//! use portaki_sdk::files::FileRef;
//!
//! # fn main() -> portaki_sdk::error::Result<()> {
//! let track = FileRef::parse("portaki-file:6f1c1d2e-3a4b-4c5d-8e9f-0a1b2c3d4e5f").unwrap();
//! let bytes = portaki_sdk::host::files::read(track)?;
//! assert!(!bytes.is_empty());
//! # Ok(())
//! # }
//! ```

use crate::error::{PortakiError, Result};
use crate::files::FileRef;
use crate::host::runtime::backend;
use crate::limits::HOST_FILE_READ_MAX_BYTES;

/// Reads the bytes of a file attached to this module.
///
/// Fails when the reference belongs to another property or module, when the file is gone, or when
/// it is larger than [`HOST_FILE_READ_MAX_BYTES`] — the host refuses before sending, so a big
/// file costs nothing to the module beyond the refusal.
pub fn read(reference: FileRef) -> Result<Vec<u8>> {
    let bytes = backend()?.file_read(&reference.to_string())?;
    if bytes.len() > HOST_FILE_READ_MAX_BYTES {
        // Ceinture et bretelles : l'hôte refuse déjà, mais un hôte de test pourrait ne pas le faire
        // et un module ne doit jamais découvrir qu'il tient dix mégaoctets en mémoire.
        return Err(PortakiError::Host("file_too_large".into()));
    }
    Ok(bytes)
}
