use crate::mime::DocumentMimeType;
use bhoot_core::TransactionId;
use bhoot_crypto::EncryptedEnvelope;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
