//! `host::images` — presigned uploads and transforms via Scaleway Object Storage.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::Result;

/// Presigned upload handshake returned by the gateway.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImageUploadRequest {
    /// HTTP PUT target.
    pub upload_url: String,
    /// Public CDN URL after upload completes.
    pub final_url: String,
    /// Upload URL expiry.
    pub expires_at: DateTime<Utc>,
}

/// Image transformation options.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TransformOptions {
    /// Target width in pixels.
    pub width: Option<u32>,
    /// Target height in pixels.
    pub height: Option<u32>,
    /// Fit mode (`cover`, `contain`, …).
    pub fit: Option<String>,
}

/// Starts an image upload flow.
pub fn upload_request(filename: &str, content_type: &str) -> Result<ImageUploadRequest> {
    let _ = (filename, content_type);
    Ok(ImageUploadRequest {
        upload_url: "https://example.invalid/upload".to_string(),
        final_url: "https://example.invalid/asset.png".to_string(),
        expires_at: Utc::now(),
    })
}

/// Deletes an image by URL.
pub fn delete(url: &str) -> Result<()> {
    let _ = url;
    Ok(())
}

/// Returns a transformed CDN URL.
pub fn transform(url: &str, opts: &TransformOptions) -> Result<String> {
    let _ = opts;
    Ok(url.to_string())
}
