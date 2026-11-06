use crate::error::VaultError;
use crate::mime::DocumentMimeType;
use bhoot_core::TransactionId;
use bhoot_crypto::{
    blake3_hash, constant_time_eq_32, decrypt_envelope, sha256_hash, EncryptedEnvelope,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use uuid::Uuid;

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

/// Cryptographically sealed document record stored in the vault.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VaultDocument {
    pub id: Uuid,
    pub filename: String,
    pub mime_type: DocumentMimeType,
    pub file_size_bytes: usize,
    pub blake3_hash: [u8; 32],
    pub sha256_hash: [u8; 32],
    pub created_at: DateTime<Utc>,
    pub linked_transaction_ids: Vec<TransactionId>,
    pub envelope: EncryptedEnvelope,
}

/// Audit report on integrity of stored vault documents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VaultIntegrityReport {
    pub total_documents: usize,
    pub verified_documents: usize,
    pub corrupted_documents: usize,
    pub corrupted_ids: Vec<Uuid>,
}

impl VaultDocument {
    /// Verify dual cryptographic hashes (BLAKE3 and SHA-256) after test decryption.
    ///
    /// # Errors
    /// Returns `VaultError` on AEAD decryption failure.
    pub fn verify_integrity(&self, vault_key: &[u8; 32]) -> Result<bool, VaultError> {
        let plaintext = decrypt_envelope(&self.envelope, vault_key)?;
        let b3 = blake3_hash(&plaintext);
        let s256 = sha256_hash(&plaintext);

        let b3_ok = constant_time_eq_32(&b3, &self.blake3_hash);
        let s256_ok = constant_time_eq_32(&s256, &self.sha256_hash);

        Ok(b3_ok && s256_ok)
    }

    /// Export the decrypted document to an unencrypted file on disk with strict permissions (0o600).
    ///
    /// # Errors
    /// Returns `VaultError` if decryption fails, integrity check fails, or I/O fails.
    pub fn export_to_file(&self, vault_key: &[u8; 32], destination: &Path) -> Result<usize, VaultError> {
        let plaintext = decrypt_envelope(&self.envelope, vault_key)?;
        let b3 = blake3_hash(&plaintext);

        if !constant_time_eq_32(&b3, &self.blake3_hash) {
            return Err(VaultError::IntegrityBreach);
        }

        let mut options = OpenOptions::new();
        options.write(true).create(true).truncate(true);

        #[cfg(unix)]
        options.mode(0o600); // Owner read/write only

        let mut file = options
            .open(destination)
            .map_err(|e| VaultError::CryptoError(format!("Failed to open export destination: {e}")))?;

        file.write_all(&plaintext)
            .map_err(|e| VaultError::CryptoError(format!("Failed to write export bytes: {e}")))?;

        file.sync_all()
            .map_err(|e| VaultError::CryptoError(format!("Failed to sync export file: {e}")))?;

        Ok(plaintext.len())
    }
}
