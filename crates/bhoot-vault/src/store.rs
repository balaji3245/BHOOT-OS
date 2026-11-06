use crate::document::VaultDocument;
use crate::error::VaultError;
use crate::mime::DocumentMimeType;
use bhoot_core::TransactionId;
use bhoot_crypto::{
    blake3_hash, constant_time_eq_32, decrypt_envelope, encrypt_envelope, sha256_hash,
};
use chrono::Utc;
use std::collections::HashMap;
use uuid::Uuid;
use zeroize::Zeroizing;

/// Maximum single document limit: 50 MiB.
pub const MAX_DOCUMENT_SIZE_BYTES: usize = 50 * 1024 * 1024;

/// Maximum aggregate in-memory vault staging bound: 256 MiB (MED-05 remediation).
///
/// In Phase 7 (Proof Vault), ciphertext envelopes will stream directly to disk-backed
/// LUKS2 encrypted storage, keeping only 32-byte content hashes and metadata in RAM.
pub const MAX_IN_MEMORY_VAULT_BYTES: usize = 256 * 1024 * 1024;

/// Encrypted Proof Vault store with content deduplication and integrity hashing.
#[derive(Debug, Default, Clone)]
pub struct VaultStore {
    documents: HashMap<Uuid, VaultDocument>,
    hash_index: HashMap<[u8; 32], Uuid>,
    total_stored_bytes: usize,
}

impl VaultStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Total plaintext bytes currently managed in memory.
    #[must_use]
    pub fn total_stored_bytes(&self) -> usize {
        self.total_stored_bytes
    }

    /// Number of documents currently stored.
    #[must_use]
    pub fn len(&self) -> usize {
        self.documents.len()
    }

    /// Check if vault is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }

    /// Store and envelope-encrypt a document with automatic magic byte detection and deduplication.
    ///
    /// # Errors
    /// Returns `VaultError` if:
    /// - Data is empty or exceeds 50 MiB.
    /// - Header magic bytes do not match supported formats (PDF, PNG, JPEG).
    /// - Encryption fails.
    pub fn store_document(
        &mut self,
        filename: impl Into<String>,
        raw_bytes: &[u8],
        vault_key: &[u8; 32],
        linked_tx_id: Option<TransactionId>,
    ) -> Result<Uuid, VaultError> {
        if raw_bytes.is_empty() {
            return Err(VaultError::EmptyDocument);
        }

        if raw_bytes.len() > MAX_DOCUMENT_SIZE_BYTES {
            return Err(VaultError::FileTooLarge {
                limit_bytes: MAX_DOCUMENT_SIZE_BYTES,
            });
        }

        // Enforce in-memory vault capacity ceiling (MED-05 remediation)
        let projected_bytes = self.total_stored_bytes.saturating_add(raw_bytes.len());
        if projected_bytes > MAX_IN_MEMORY_VAULT_BYTES {
            return Err(VaultError::VaultCapacityExceeded {
                current_bytes: self.total_stored_bytes,
                ceiling_bytes: MAX_IN_MEMORY_VAULT_BYTES,
            });
        }

        let mime_type = DocumentMimeType::detect_from_magic_bytes(raw_bytes)?;
        let b3_hash = blake3_hash(raw_bytes);
        let s256_hash = sha256_hash(raw_bytes);

        // Deduplication: if an exact identical file is already stored, link tx and return existing ID
        if let Some(&existing_id) = self.hash_index.get(&b3_hash) {
            if let Some(doc) = self.documents.get_mut(&existing_id) {
                if let Some(tx_id) = linked_tx_id {
                    if !doc.linked_transaction_ids.contains(&tx_id) {
                        doc.linked_transaction_ids.push(tx_id);
                    }
                }
                return Ok(existing_id);
            }
        }

        let envelope = encrypt_envelope(raw_bytes, vault_key)?;
        let doc_id = Uuid::new_v4();

        let mut linked_txs = Vec::new();
        if let Some(tx_id) = linked_tx_id {
            linked_txs.push(tx_id);
        }

        let doc = VaultDocument {
            id: doc_id,
            filename: filename.into(),
            mime_type,
            file_size_bytes: raw_bytes.len(),
            blake3_hash: b3_hash,
            sha256_hash: s256_hash,
            created_at: Utc::now(),
            linked_transaction_ids: linked_txs,
            envelope,
        };

        self.hash_index.insert(b3_hash, doc_id);
        self.documents.insert(doc_id, doc);
        self.total_stored_bytes = projected_bytes;

        Ok(doc_id)
    }

    /// Retrieve and decrypt a document, returning zeroize-on-drop plaintext.
    ///
    /// # Errors
    /// Returns `VaultError::DocumentNotFound` or `VaultError::CryptoError`.
    pub fn retrieve_document(
        &self,
        document_id: &Uuid,
        vault_key: &[u8; 32],
    ) -> Result<Zeroizing<Vec<u8>>, VaultError> {
        let doc = self
            .documents
            .get(document_id)
            .ok_or_else(|| VaultError::DocumentNotFound(document_id.to_string()))?;

        let plaintext = decrypt_envelope(&doc.envelope, vault_key)?;
        let computed_b3 = blake3_hash(&plaintext);

        if !constant_time_eq_32(&computed_b3, &doc.blake3_hash) {
            return Err(VaultError::IntegrityBreach);
        }

        Ok(plaintext)
    }

    /// Retrieve document metadata by ID without decrypting the payload.
    #[must_use]
    pub fn get_metadata(&self, document_id: &Uuid) -> Option<&VaultDocument> {
        self.documents.get(document_id)
    }

    /// List all documents stored in the vault.
    #[must_use]
    pub fn list_documents(&self) -> Vec<&VaultDocument> {
        self.documents.values().collect()
    }

    /// Delete a document by ID and reclaim memory.
    pub fn delete_document(&mut self, document_id: &Uuid) -> bool {
        if let Some(doc) = self.documents.remove(document_id) {
            self.hash_index.remove(&doc.blake3_hash);
            self.total_stored_bytes = self.total_stored_bytes.saturating_sub(doc.file_size_bytes);
            true
        } else {
            false
        }
    }

    /// Run full integrity audit decrypting and verifying all stored documents.
    pub fn verify_all_documents(&self, vault_key: &[u8; 32]) -> crate::document::VaultIntegrityReport {
        let total = self.documents.len();
        let mut verified = 0;
        let mut corrupted = 0;
        let mut corrupted_ids = Vec::new();

        for (id, doc) in &self.documents {
            match doc.verify_integrity(vault_key) {
                Ok(true) => verified += 1,
                _ => {
                    corrupted += 1;
                    corrupted_ids.push(*id);
                }
            }
        }

        crate::document::VaultIntegrityReport {
            total_documents: total,
            verified_documents: verified,
            corrupted_documents: corrupted,
            corrupted_ids,
        }
    }
}
