use thiserror::Error;

/// Encrypted backup and disaster recovery errors.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum BackupError {
    #[error("Invalid backup archive header: {0}")]
    InvalidHeader(String),

    #[error("Backup archive corrupted: SHA-256 digest mismatch")]
    DigestMismatch,

    #[error("Cryptographic operation failed: {0}")]
    CryptoError(String),

    #[error("Decryption failed: incorrect passphrase or tampered ciphertext")]
    DecryptionFailed,

    #[error("I/O error during backup operation: {0}")]
    IoError(String),

    #[error("Disaster recovery test failed: {0}")]
    RecoveryTestFailed(String),

    #[error("Invalid destination path: target {0} is prohibited")]
    ProhibitedDestination(String),
}

impl From<bhoot_crypto::CryptoError> for BackupError {
    fn from(e: bhoot_crypto::CryptoError) -> Self {
        Self::CryptoError(e.to_string())
    }
}
