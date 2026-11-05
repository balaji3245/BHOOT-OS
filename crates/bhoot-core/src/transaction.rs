use crate::account::AccountId;
use crate::error::CoreError;
use crate::money::Money;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

/// Strongly-typed unique identifier for transactions.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TransactionId(Uuid);

impl TransactionId {
    /// Generate a fresh random TransactionId (UUIDv4).
    #[must_use]
    pub fn new_v4() -> Self {
        Self(Uuid::new_v4())
    }

    /// Construct from an existing UUID.
    #[must_use]
    pub const fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Return the inner UUID.
    #[must_use]
    pub const fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for TransactionId {
    fn default() -> Self {
        Self::new_v4()
    }
}

impl fmt::Display for TransactionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for TransactionId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(s).map(Self)
    }
}

/// Ledger posting entry direction.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Direction {
    Debit,
    Credit,
}

impl Direction {
    /// Invert direction for reversal entries.
    #[must_use]
    pub const fn invert(&self) -> Self {
        match self {
            Self::Debit => Self::Credit,
            Self::Credit => Self::Debit,
        }
    }
}

/// Individual posting leg of a double-entry transaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Posting {
    pub account_id: AccountId,
    pub amount: Money,
    pub direction: Direction,
    pub memo: Option<String>,
}

impl Posting {
    /// Create a new posting leg with strictly positive amount.
    ///
    /// # Errors
    /// Returns `CoreError::NonPositivePosting` if amount <= 0.
    pub fn new(
        account_id: AccountId,
        amount: Money,
        direction: Direction,
        memo: Option<String>,
    ) -> Result<Self, CoreError> {
        if !amount.is_positive() {
            return Err(CoreError::NonPositivePosting(amount.as_paisa()));
        }
        Ok(Self {
            account_id,
            amount,
            direction,
            memo,
        })
    }
}

/// Nature and category of the financial transaction.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TransactionType {
    /// External money entering user ownership.
    Income,
    /// Money leaving user ownership for goods/services.
    Expense,
    /// Movement between user-owned accounts (Zero net income/expense).
    Transfer,
    /// Funding a broker or trading account from bank.
    Deposit,
    /// Moving funds from broker account back to bank.
    Withdrawal,
    /// Realized profit or loss and charges from market execution.
    Trade,
    /// Audited correction of discrepancies.
    Adjustment,
    /// Strict reciprocal reversal of an erroneous historical entry.
    Reversal,
}

/// Payment channel utilized for execution.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PaymentMethod {
    UPI,
    NetBanking,
    NEFT,
    IMPS,
    RTGS,
    Cash,
    DebitCard,
    CreditCard,
    Cheque,
    BrokerInternal,
    Other,
}

/// Verification and audit reconciliation state.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VerificationStatus {
    Pending,
    Verified,
    Flagged,
    Reconciled,
}

/// Complete double-entry balanced transaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transaction {
    pub id: TransactionId,
    pub timestamp: DateTime<Utc>,
    pub tx_type: TransactionType,
    pub postings: Vec<Posting>,
    pub category: Option<String>,
    pub payment_method: Option<PaymentMethod>,
    pub reference_number: Option<String>,
    pub notes: Option<String>,
    pub proof_attachment_ids: Vec<Uuid>,
    pub verification_status: VerificationStatus,
    pub reversal_of: Option<TransactionId>,
}

impl Transaction {
    /// Construct and validate a transaction.
    ///
    /// # Errors
    /// Returns `CoreError` if:
    /// - Fewer than 2 postings are present.
    /// - Total debits != total credits.
    /// - Any posting amount is <= 0.
    /// - Reversal transaction is missing `reversal_of` pointer.
    pub fn new(
        timestamp: DateTime<Utc>,
        tx_type: TransactionType,
        postings: Vec<Posting>,
        category: Option<String>,
        payment_method: Option<PaymentMethod>,
        reference_number: Option<String>,
        notes: Option<String>,
        proof_attachment_ids: Vec<Uuid>,
        reversal_of: Option<TransactionId>,
    ) -> Result<Self, CoreError> {
        let tx = Self {
            id: TransactionId::new_v4(),
            timestamp,
            tx_type,
            postings,
            category,
            payment_method,
            reference_number,
            notes,
            proof_attachment_ids,
            verification_status: VerificationStatus::Pending,
            reversal_of,
        };

        tx.validate()?;
        Ok(tx)
    }

    /// Strict mathematical validation of double-entry conservation and invariants.
    pub fn validate(&self) -> Result<(), CoreError> {
        if self.postings.len() < 2 {
            return Err(CoreError::InsufficientPostings(self.postings.len()));
        }

        let mut total_debits = Money::ZERO;
        let mut total_credits = Money::ZERO;

        for p in &self.postings {
            if !p.amount.is_positive() {
                return Err(CoreError::NonPositivePosting(p.amount.as_paisa()));
            }

            match p.direction {
                Direction::Debit => {
                    total_debits = total_debits.checked_add(p.amount)?;
                }
                Direction::Credit => {
                    total_credits = total_credits.checked_add(p.amount)?;
                }
            }
        }

        if total_debits != total_credits {
            return Err(CoreError::UnbalancedTransaction {
                debits_paisa: total_debits.as_paisa(),
                credits_paisa: total_credits.as_paisa(),
            });
        }

        if self.tx_type == TransactionType::Reversal && self.reversal_of.is_none() {
            return Err(CoreError::InvalidReversal(
                "Reversal transaction must explicitly reference the original transaction ID".into(),
            ));
        }

        Ok(())
    }

    /// Generate an exact reciprocal reversal transaction.
    ///
    /// Inverts all posting directions (Debit <-> Credit) while preserving amounts and accounts.
    ///
    /// # Errors
    /// Returns `CoreError` if reversal validation fails.
    pub fn create_reversal(
        &self,
        timestamp: DateTime<Utc>,
        reason: impl Into<String>,
    ) -> Result<Self, CoreError> {
        let inverted_postings: Result<Vec<Posting>, CoreError> = self
            .postings
            .iter()
            .map(|p| {
                Posting::new(
                    p.account_id,
                    p.amount,
                    p.direction.invert(),
                    Some(format!(
                        "Reversal of tx {}: {}",
                        self.id,
                        p.memo.as_deref().unwrap_or("")
                    )),
                )
            })
            .collect();

        let postings = inverted_postings?;
        let reason_str = reason.into();
        let combined_notes = match &self.notes {
            Some(existing) => format!("REVERSAL REASON: {reason_str} | ORIGINAL NOTES: {existing}"),
            None => format!("REVERSAL REASON: {reason_str}"),
        };

        Self::new(
            timestamp,
            TransactionType::Reversal,
            postings,
            self.category.clone(),
            self.payment_method,
            self.reference_number.clone(),
            Some(combined_notes),
            Vec::new(),
            Some(self.id),
        )
    }
}
