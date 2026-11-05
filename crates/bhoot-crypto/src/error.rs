use thiserror::Error;

/// Cryptographic subsystem errors.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum CryptoError {
    #[error("Key derivation failed: {0}")]
    KeyDerivationFailed(String),

    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),

    #[error("Decryption failed: invalid ciphertext or authentication tag mismatch")]
    DecryptionFailed,

    #[error("Document integrity verification failed: BLAKE3 plaintext hash does not match")]
    IntegrityBreach,

    #[error("Invalid key length: expected {expected} bytes, got {actual}")]
    InvalidKeyLength { expected: usize, actual: usize },

    #[error("Invalid nonce length: expected {expected} bytes, got {actual}")]
    InvalidNonceLength { expected: usize, actual: usize },

    #[error("RNG generation failure")]
    RngFailure,
}
