use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Categories of auditable system mutations.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AuditEventType {
    AccountCreated,
    AccountStatusChanged,
    TransactionPosted,
    TransactionReversed,
    VaultDocumentStored,
    VaultDocumentDeleted,
    SystemLocked,
    SystemUnlocked,
    BackupExported,
    IntegrityCheckRun,
}

/// Cryptographically chained append-only audit record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Monotonically increasing sequence index starting at 0.
    pub sequence: u64,
    /// UTC timestamp of event creation.
    pub timestamp: DateTime<Utc>,
    /// Type of recorded mutation.
    pub event_type: AuditEventType,
    /// Identifier of the affected domain entity (account, tx, or document).
    pub entity_id: String,
    /// Canonical JSON payload of the event details.
    pub payload_json: String,
    /// 256-bit hash of the immediately preceding audit event block.
    pub prev_hash: [u8; 32],
    /// 256-bit SHA-256 digest of this block.
    pub hash: [u8; 32],
}

impl AuditEvent {
    /// Compute the SHA-256 digest for an audit event block.
    #[must_use]
    pub fn compute_hash(
        sequence: u64,
        timestamp: &DateTime<Utc>,
        event_type: AuditEventType,
        entity_id: &str,
        payload_json: &str,
        prev_hash: &[u8; 32],
    ) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(sequence.to_be_bytes());
        hasher.update(timestamp.to_rfc3339().as_bytes());
        hasher.update(format!("{event_type:?}").as_bytes());
        hasher.update(entity_id.as_bytes());
        hasher.update(payload_json.as_bytes());
        hasher.update(prev_hash);

        let result = hasher.finalize();
        let mut out = [0u8; 32];
        out.copy_from_slice(&result);
        out
    }
}
