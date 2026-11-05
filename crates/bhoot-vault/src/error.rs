use thiserror::Error;

/// Document proof vault errors.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum VaultError {
    #[error("Document not found: {0}")]
    DocumentNotFound(String),

    #[error("Unsupported or invalid file format / magic bytes: {0}")]
    InvalidMimeType(String),

    #[error("Empty document bytes")]
    EmptyDocument,

    #[error("File size exceeds maximum allowed vault limit of {limit_bytes} bytes")]
    FileTooLarge { limit_bytes: usize },

    #[error("In-memory vault staging ceiling exceeded: {current_bytes} bytes stored exceeds limit of {ceiling_bytes} bytes")]
    VaultCapacityExceeded {
        current_bytes: usize,
        ceiling_bytes: usize,
    },

    #[error("Cryptographic operation failed: {0}")]
    CryptoError(String),

    #[error("Document integrity breach: decrypted content does not match recorded digest")]
    IntegrityBreach,
}

impl From<bhoot_crypto::CryptoError> for VaultError {
    fn from(err: bhoot_crypto::CryptoError) -> Self {
        Self::CryptoError(err.to_string())
    }
}
