use bhoot_core::CoreError;
use thiserror::Error;

/// Database persistence errors.
#[derive(Debug, Error)]
pub enum DbError {
    #[error("Database initialization error: {0}")]
    InitError(String),

    #[error("SQLCipher encryption key error: {0}")]
    KeyError(String),

    #[error("SQLCipher authentication failed (invalid key or corrupted database): {0}")]
    AuthenticationFailed(String),

    #[error("Migration failed: {0}")]
    MigrationError(String),

    #[error("Database query error: {0}")]
    QueryError(String),

    #[error("Foreign key constraint violation: {0}")]
    ConstraintViolation(String),

    #[error("Unbalanced transaction: Debits={debits_paisa} paisa, Credits={credits_paisa} paisa")]
    UnbalancedTransaction {
        debits_paisa: i64,
        credits_paisa: i64,
    },

    #[error("Account not found: {0}")]
    AccountNotFound(String),

    #[error("Transaction not found: {0}")]
    TransactionNotFound(String),

    #[error("Transaction already reversed: {0}")]
    AlreadyReversed(String),

    #[error("Cannot reverse a reversal transaction: {0}")]
    CannotReverseReversal(String),

    #[error("Account is frozen: {0}")]
    AccountFrozen(String),

    #[error("Core ledger error: {0}")]
    Core(#[from] CoreError),

    #[error("SQLite internal error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}
