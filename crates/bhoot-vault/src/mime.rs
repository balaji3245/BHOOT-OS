use crate::error::VaultError;
use serde::{Deserialize, Serialize};

/// Supported proof attachment MIME types validated strictly via header magic bytes.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DocumentMimeType {
    Pdf,
    Png,
    Jpeg,
}

impl DocumentMimeType {
    /// Detect MIME type strictly from file content magic bytes (header inspection).
    ///
    /// # Errors
    /// Returns `VaultError::InvalidMimeType` if content header does not match known secure formats.
    pub fn detect_from_magic_bytes(bytes: &[u8]) -> Result<Self, VaultError> {
        if bytes.len() < 4 {
            return Err(VaultError::InvalidMimeType("File header too short".into()));
        }

        // PDF magic bytes: %PDF- (0x25, 0x50, 0x44, 0x46, 0x2D)
        if bytes.starts_with(b"%PDF-") {
            return Ok(Self::Pdf);
        }

        // PNG magic bytes: 0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A
        if bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
            return Ok(Self::Png);
        }

        // JPEG magic bytes: 0xFF, 0xD8, 0xFF
        if bytes.len() >= 3 && bytes[0] == 0xFF && bytes[1] == 0xD8 && bytes[2] == 0xFF {
            return Ok(Self::Jpeg);
        }

        Err(VaultError::InvalidMimeType(
            "File signature does not match PDF, PNG, or JPEG".into(),
        ))
    }
}
