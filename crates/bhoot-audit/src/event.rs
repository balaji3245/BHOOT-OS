use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<Sha256>;

/// Unique cryptographic domain separator preventing cross-protocol collision.
pub const AUDIT_BLOCK_DOMAIN_V1: &[u8] = b"BHOOT-AUDIT-BLOCK-V1\0";

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
    /// 256-bit digest of this block (HMAC-SHA256 or SHA-256).
    pub hash: [u8; 32],
}

impl AuditEvent {
    /// Feeds all event fields into a consumer using unambiguous length-prefixed serialization.
    fn feed_canonical_fields<F>(
        sequence: u64,
        timestamp: &DateTime<Utc>,
        event_type: AuditEventType,
        entity_id: &str,
        payload_json: &str,
        prev_hash: &[u8; 32],
        mut update: F,
    ) where
        F: FnMut(&[u8]),
    {
        // 1. Domain Separation Header
        update(AUDIT_BLOCK_DOMAIN_V1);

        // 2. Sequence (8 bytes big-endian)
        update(&sequence.to_be_bytes());

        // 3. Timestamp (RFC 3339 string with 8-byte length prefix)
        let ts_str = timestamp.to_rfc3339();
        update(&(ts_str.len() as u64).to_be_bytes());
        update(ts_str.as_bytes());

        // 4. Event Type (Deterministic identifier with 8-byte length prefix)
        let type_str = format!("{event_type:?}");
        update(&(type_str.len() as u64).to_be_bytes());
        update(type_str.as_bytes());

        // 5. Entity ID (Variable-length with 8-byte length prefix)
        update(&(entity_id.len() as u64).to_be_bytes());
        update(entity_id.as_bytes());

        // 6. Payload JSON (Variable-length with 8-byte length prefix)
        update(&(payload_json.len() as u64).to_be_bytes());
        update(payload_json.as_bytes());

        // 7. Previous Hash (32 bytes with length prefix)
        update(&(prev_hash.len() as u64).to_be_bytes());
        update(prev_hash);
    }

    /// Compute the SHA-256 digest using unambiguous length-prefixed serialization (HIGH-01 remediation).
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
        Self::feed_canonical_fields(
            sequence,
            timestamp,
            event_type,
            entity_id,
            payload_json,
            prev_hash,
            |bytes| hasher.update(bytes),
        );

        let result = hasher.finalize();
        let mut out = [0u8; 32];
        out.copy_from_slice(&result);
        out
    }

    /// Compute a keyed HMAC-SHA256 digest using the audit subkey (MED-04 remediation).
    ///
    /// # Panics
    /// Will not panic as the 32-byte key is a valid HMAC-SHA256 key size.
    #[must_use]
    pub fn compute_hmac(
        audit_key: &[u8; 32],
        sequence: u64,
        timestamp: &DateTime<Utc>,
        event_type: AuditEventType,
        entity_id: &str,
        payload_json: &str,
        prev_hash: &[u8; 32],
    ) -> [u8; 32] {
        let mut mac =
            HmacSha256::new_from_slice(audit_key).expect("HMAC-SHA256 accepts 32-byte key");

        Self::feed_canonical_fields(
            sequence,
            timestamp,
            event_type,
            entity_id,
            payload_json,
            prev_hash,
            |bytes| mac.update(bytes),
        );

        let result = mac.finalize();
        let mut out = [0u8; 32];
        out.copy_from_slice(&result.into_bytes());
        out
    }

    /// Compute block digest using either keyed HMAC (recommended) or SHA-256.
    #[must_use]
    pub fn compute_block_digest(
        audit_key: Option<&[u8; 32]>,
        sequence: u64,
        timestamp: &DateTime<Utc>,
        event_type: AuditEventType,
        entity_id: &str,
        payload_json: &str,
        prev_hash: &[u8; 32],
    ) -> [u8; 32] {
        match audit_key {
            Some(key) => Self::compute_hmac(
                key,
                sequence,
                timestamp,
                event_type,
                entity_id,
                payload_json,
                prev_hash,
            ),
            None => Self::compute_hash(
                sequence,
                timestamp,
                event_type,
                entity_id,
                payload_json,
                prev_hash,
            ),
        }
    }
}
