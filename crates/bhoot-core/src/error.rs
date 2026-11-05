use thiserror::Error;

/// Core domain and accounting errors.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum CoreError {
    #[error("Arithmetic overflow or underflow occurred: {0}")]
    ArithmeticOverflow(String),

    #[error("Division by zero")]
    DivisionByZero,

    #[error("Invalid monetary amount: {0}")]
    InvalidAmount(String),

    #[error("Account not found: {0}")]
    AccountNotFound(String),

    #[error("Duplicate account ID: {0}")]
    DuplicateAccount(String),

    #[error("Duplicate transaction ID: {0}")]
    DuplicateTransaction(String),

    #[error("Transaction is unbalanced: total debits ({debits_paisa} paisa) != total credits ({credits_paisa} paisa)")]
    UnbalancedTransaction {
        debits_paisa: i64,
        credits_paisa: i64,
    },

    #[error("Transaction must contain at least 2 postings, got {0}")]
    InsufficientPostings(usize),

    #[error("Posting amount must be strictly positive, got {0} paisa")]
    NonPositivePosting(i64),

    #[error("Account {0} is frozen or inactive")]
    AccountInactive(String),

    #[error("Account type mismatch for operation: expected {expected}, found {found}")]
    AccountTypeMismatch {
        expected: &'static str,
        found: &'static str,
    },

    #[error("Invalid reversal transaction: {0}")]
    InvalidReversal(String),

    #[error("Invalid transfer posting: transfers can only involve Asset or Liability accounts, found {found} for account {account_id}")]
    InvalidTransferAccount { account_id: String, found: String },

    #[error("Invalid broker trade record: {0}")]
    InvalidTrade(String),

    #[error("Invalid broker payout record: {0}")]
    InvalidPayout(String),

    #[error("Global ledger equilibrium breach: sum(debit) != sum(credit)")]
    LedgerEquilibriumBreach,

    #[error("Accounting equation violation: Assets - Liabilities != Equity + (Income - Expense)")]
    AccountingEquationViolation,
}
