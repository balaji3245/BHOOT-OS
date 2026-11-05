use thiserror::Error;

/// Database persistence errors.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum DbError {
    #[error("Database initialization error: {0}")]
    InitError(String),

    #[error("SQLCipher encryption key error: {0}")]
    KeyError(String),

    #[error("Migration failed: {0}")]
    MigrationError(String),

    #[error("Database query error: {0}")]
    QueryError(String),

    #[error("Foreign key constraint violation: {0}")]
    ConstraintViolation(String),
}
