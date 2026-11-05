use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

/// Strongly-typed unique identifier for financial accounts.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AccountId(Uuid);

impl AccountId {
    /// Generate a fresh random AccountId (UUIDv4).
    #[must_use]
    pub fn new_v4() -> Self {
        Self(Uuid::new_v4())
    }

    /// Construct AccountId from an existing UUID.
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

impl Default for AccountId {
    fn default() -> Self {
        Self::new_v4()
    }
}

impl fmt::Display for AccountId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for AccountId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(s).map(Self)
    }
}

/// Normal balance classification in double-entry bookkeeping.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NormalBalance {
    /// Assets and Expenses naturally carry a Debit balance.
    Debit,
    /// Liabilities, Equity, and Income naturally carry a Credit balance.
    Credit,
}

/// Fundamental double-entry accounting account classifications.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AccountType {
    /// Economic resources owned (e.g. Bank, Cash, Broker, Investments).
    Asset,
    /// Future economic obligations (e.g. Credit Cards, Loans, Payables).
    Liability,
    /// Residual interest in assets after deducting liabilities (Owner capital, opening balances).
    Equity,
    /// Inflows of economic benefit (Salary, Trading Profits, Interest, Dividends).
    Income,
    /// Outflows or depletion of economic value (Living costs, Brokerages, Taxes, Trading Losses).
    Expense,
}

impl AccountType {
    /// Determine normal balance side according to GAAP/Double-Entry conventions.
    #[must_use]
    pub const fn normal_balance(&self) -> NormalBalance {
        match self {
            Self::Asset | Self::Expense => NormalBalance::Debit,
            Self::Liability | Self::Equity | Self::Income => NormalBalance::Credit,
        }
    }
}

/// Granular subtype classification for reporting and balance sheets.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AccountSubtype {
    // Assets
    BankAccount,
    UPIWallet,
    CashWallet,
    BrokerAccount,
    InvestmentAccount,

    // Liabilities
    CreditCard,
    PersonalLoan,
    Payable,

    // Equity
    OwnerCapital,
    RetainedEarnings,
    OpeningBalanceEquity,

    // Income
    Salary,
    TradingProfit,
    BusinessRevenue,
    Dividend,
    InterestIncome,
    OtherIncome,

    // Expenses
    LivingExpense,
    TradingLoss,
    BrokerageAndCharges,
    Taxes,
    OtherExpense,
}

/// Operational state of a financial account.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AccountStatus {
    /// Active and accepting postings.
    Active,
    /// Archived, read-only for reporting.
    Archived,
    /// Frozen due to investigation or dispute; postings rejected.
    Frozen,
}

/// Financial account representing an entity in the ledger.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    pub id: AccountId,
    pub name: String,
    pub account_type: AccountType,
    pub subtype: AccountSubtype,
    pub status: AccountStatus,
    pub currency: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Account {
    /// Create a new active account.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        account_type: AccountType,
        subtype: AccountSubtype,
        description: Option<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: AccountId::new_v4(),
            name: name.into(),
            account_type,
            subtype,
            status: AccountStatus::Active,
            currency: "INR".to_string(),
            description,
            created_at: now,
            updated_at: now,
        }
    }

    /// Check if account can accept new ledger postings.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.status == AccountStatus::Active
    }

    /// Freeze the account.
    pub fn freeze(&mut self) {
        self.status = AccountStatus::Frozen;
        self.updated_at = Utc::now();
    }

    /// Archive the account.
    pub fn archive(&mut self) {
        self.status = AccountStatus::Archived;
        self.updated_at = Utc::now();
    }
}
