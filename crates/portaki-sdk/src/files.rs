//! Files a guest attaches from the booklet.
//!
//! An `ImageUpload` field in a guest form uploads the image to the platform (permission
//! [`crate::permission::GUEST_FILES`], limits in [`crate::limits`]) and submits a
//! [`FileRef`] in the command args — never the bytes. Store the reference; to show the image
//! on a host surface, put [`FileRef::image_url`] in an `Image` url. The platform swaps it for a
//! short-lived signed URL when the file belongs to the rendering property and module, and
//! drops it otherwise.
//!
//! ```
//! use portaki_sdk::files::FileRef;
//!
//! let photo = FileRef::parse("portaki-file:6f1c1d2e-3a4b-4c5d-8e9f-0a1b2c3d4e5f").unwrap();
//! assert_eq!(photo.image_url(), "portaki-file:6f1c1d2e-3a4b-4c5d-8e9f-0a1b2c3d4e5f");
//! assert!(FileRef::parse("https://example.com/cat.png").is_none());
//! ```

use std::fmt;

use uuid::Uuid;

/// Scheme of a guest file reference on the wire.
pub const SCHEME: &str = "portaki-file:";

/// Reference to a guest file stored by the platform: `portaki-file:<uuid>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileRef(Uuid);

impl FileRef {
    /// Parses a submitted value. Anything but `portaki-file:<uuid>` — an external URL
    /// included — is `None`: a guest must not make the host dashboard load an arbitrary URL.
    pub fn parse(raw: &str) -> Option<Self> {
        raw.trim()
            .strip_prefix(SCHEME)
            .and_then(|id| Uuid::parse_str(id).ok())
            .map(Self)
    }

    /// The file id.
    pub fn id(&self) -> Uuid {
        self.0
    }

    /// Value for an `Image` url on a host surface, resolved by the platform at render time.
    pub fn image_url(&self) -> String {
        self.to_string()
    }
}

impl fmt::Display for FileRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{SCHEME}{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_platform_references_parse() {
        let id = Uuid::new_v4();
        assert_eq!(
            FileRef::parse(&format!(" portaki-file:{id} ")).map(|r| r.id()),
            Some(id)
        );
        assert!(FileRef::parse("portaki-file:not-a-uuid").is_none());
        assert!(FileRef::parse(&id.to_string()).is_none());
        assert!(FileRef::parse("javascript:alert(1)").is_none());
    }
}
