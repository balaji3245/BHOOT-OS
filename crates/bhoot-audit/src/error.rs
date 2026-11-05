use thiserror::Error;

/// Audit log and hash-chain errors.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum AuditError {
    #[error("Audit chain integrity breach at block {sequence}: expected hash {expected}, calculated {actual}")]
    HashMismatch {
        sequence: u64,
        expected: String,
        actual: String,
    },

    #[error("Audit chain broken link at block {sequence}: prev_hash does not match previous block's hash")]
    BrokenLink { sequence: u64 },

    #[error("Invalid sequence number: expected {expected}, got {actual}")]
    SequenceGap { expected: u64, actual: u64 },

    #[error("Timestamp monotonicity violation at block {sequence}")]
    TimestampRegression { sequence: u64 },

    #[error("Genesis block corruption")]
    CorruptedGenesis,
}
