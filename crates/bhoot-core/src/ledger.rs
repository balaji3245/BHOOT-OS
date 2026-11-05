use crate::account::{Account, AccountId, AccountType, NormalBalance};
use crate::error::CoreError;
use crate::money::Money;
use crate::transaction::{Direction, Posting, Transaction, TransactionId, TransactionType};
use std::collections::HashMap;

/// Pure Rust in-memory double-entry accounting ledger engine.
///
/// Enforces conservation of value, mathematical equilibrium, and financial invariants.
#[derive(Debug, Default, Clone)]
pub struct Ledger {
    accounts: HashMap<AccountId, Account>,
    transactions: Vec<Transaction>,
    transaction_index: HashMap<TransactionId, usize>,
    account_postings: HashMap<AccountId, Vec<usize>>,
}

impl Ledger {
    /// Create a new, empty ledger.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new account into the ledger.
    ///
    /// # Errors
    /// Returns `CoreError::DuplicateAccount` if account ID already exists.
    pub fn add_account(&mut self, account: Account) -> Result<(), CoreError> {
        if self.accounts.contains_key(&account.id) {
            return Err(CoreError::DuplicateAccount(account.id.to_string()));
        }
        self.accounts.insert(account.id, account);
        Ok(())
    }

    /// Retrieve an account reference by ID.
    #[must_use]
    pub fn get_account(&self, id: &AccountId) -> Option<&Account> {
        self.accounts.get(id)
    }

    /// Retrieve all registered accounts.
    #[must_use]
    pub fn accounts(&self) -> &HashMap<AccountId, Account> {
        &self.accounts
    }

    /// Retrieve all recorded transactions.
    #[must_use]
    pub fn transactions(&self) -> &[Transaction] {
        &self.transactions
    }

    /// Post a balanced double-entry transaction into the ledger.
    ///
    /// # Errors
    /// Returns `CoreError` if:
    /// - Transaction is mathematically unbalanced.
    /// - Transaction ID already exists.
    /// - Any posting references an unknown or inactive account.
    /// - Arithmetic overflow occurs.
    pub fn post_transaction(&mut self, tx: Transaction) -> Result<(), CoreError> {
        tx.validate()?;

        if self.transaction_index.contains_key(&tx.id) {
            return Err(CoreError::DuplicateTransaction(tx.id.to_string()));
        }

        // Verify that all referenced accounts exist and are active
        for posting in &tx.postings {
            let account = self
                .accounts
                .get(&posting.account_id)
                .ok_or_else(|| CoreError::AccountNotFound(posting.account_id.to_string()))?;

            if !account.is_active() {
                return Err(CoreError::AccountInactive(account.id.to_string()));
            }
        }

        // Verify reversal constraints
        if tx.tx_type == TransactionType::Reversal {
            if let Some(orig_id) = tx.reversal_of {
                if !self.transaction_index.contains_key(&orig_id) {
                    return Err(CoreError::InvalidReversal(format!(
                        "Original transaction {orig_id} to be reversed does not exist in ledger"
                    )));
                }
            }
        }

        let tx_index = self.transactions.len();
        for posting in &tx.postings {
            self.account_postings
                .entry(posting.account_id)
                .or_default()
                .push(tx_index);
        }

        self.transaction_index.insert(tx.id, tx_index);
        self.transactions.push(tx);

        Ok(())
    }

    /// Calculate the current balance of an account according to its normal balance convention.
    ///
    /// - For Asset & Expense (Normal Debit): Balance = Debits - Credits
    /// - For Liability, Equity & Income (Normal Credit): Balance = Credits - Debits
    ///
    /// # Errors
    /// Returns `CoreError::AccountNotFound` or `CoreError::ArithmeticOverflow`.
    pub fn get_balance(&self, account_id: &AccountId) -> Result<Money, CoreError> {
        let account = self
            .accounts
            .get(account_id)
            .ok_or_else(|| CoreError::AccountNotFound(account_id.to_string()))?;

        let tx_indices = match self.account_postings.get(account_id) {
            Some(indices) => indices,
            None => return Ok(Money::ZERO),
        };

        let mut total_debit = Money::ZERO;
        let mut total_credit = Money::ZERO;

        for &idx in tx_indices {
            let tx = &self.transactions[idx];
            for posting in &tx.postings {
                if posting.account_id == *account_id {
                    match posting.direction {
                        Direction::Debit => {
                            total_debit = total_debit.checked_add(posting.amount)?;
                        }
                        Direction::Credit => {
                            total_credit = total_credit.checked_add(posting.amount)?;
                        }
                    }
                }
            }
        }

        match account.account_type.normal_balance() {
            NormalBalance::Debit => total_debit.checked_sub(total_credit),
            NormalBalance::Credit => total_credit.checked_sub(total_debit),
        }
    }

    /// Calculate Net Worth: Sum of all Asset balances - Sum of all Liability balances.
    ///
    /// # Errors
    /// Returns `CoreError::ArithmeticOverflow` on calculation overflow.
    pub fn get_net_worth(&self) -> Result<Money, CoreError> {
        let mut total_assets = Money::ZERO;
        let mut total_liabilities = Money::ZERO;

        for account in self.accounts.values() {
            let balance = self.get_balance(&account.id)?;
            match account.account_type {
                AccountType::Asset => {
                    total_assets = total_assets.checked_add(balance)?;
                }
                AccountType::Liability => {
                    total_liabilities = total_liabilities.checked_add(balance)?;
                }
                _ => {}
            }
        }

        total_assets.checked_sub(total_liabilities)
    }

    /// Calculate Total Income accumulated across all Income accounts.
    pub fn get_total_income(&self) -> Result<Money, CoreError> {
        let mut total = Money::ZERO;
        for account in self.accounts.values() {
            if account.account_type == AccountType::Income {
                let bal = self.get_balance(&account.id)?;
                total = total.checked_add(bal)?;
            }
        }
        Ok(total)
    }

    /// Calculate Total Expenses accumulated across all Expense accounts.
    pub fn get_total_expenses(&self) -> Result<Money, CoreError> {
        let mut total = Money::ZERO;
        for account in self.accounts.values() {
            if account.account_type == AccountType::Expense {
                let bal = self.get_balance(&account.id)?;
                total = total.checked_add(bal)?;
            }
        }
        Ok(total)
    }

    /// Compute Net Income (Total Income - Total Expenses).
    pub fn get_net_income(&self) -> Result<Money, CoreError> {
        let income = self.get_total_income()?;
        let expenses = self.get_total_expenses()?;
        income.checked_sub(expenses)
    }

    /// Verify the Transfer Neutrality Invariant:
    /// An internal transfer between two owned asset accounts must have zero net income or expense effect.
    pub fn verify_transfer_neutrality(
        &self,
        from_asset: AccountId,
        to_asset: AccountId,
        amount: Money,
        timestamp: chrono::DateTime<chrono::Utc>,
    ) -> Result<bool, CoreError> {
        let income_before = self.get_total_income()?;
        let expense_before = self.get_total_expenses()?;
        let net_worth_before = self.get_net_worth()?;

        // Create a simulated transfer transaction
        let mut simulated_ledger = self.clone();
        let postings = vec![
            Posting::new(
                from_asset,
                amount,
                Direction::Credit,
                Some("Transfer source".into()),
            )?,
            Posting::new(
                to_asset,
                amount,
                Direction::Debit,
                Some("Transfer dest".into()),
            )?,
        ];

        let tx = Transaction::new(
            timestamp,
            TransactionType::Transfer,
            postings,
            Some("Transfer".into()),
            None,
            None,
            None,
            Vec::new(),
            None,
        )?;

        simulated_ledger.post_transaction(tx)?;

        let income_after = simulated_ledger.get_total_income()?;
        let expense_after = simulated_ledger.get_total_expenses()?;
        let net_worth_after = simulated_ledger.get_net_worth()?;

        Ok(income_before == income_after
            && expense_before == expense_after
            && net_worth_before == net_worth_after)
    }

    /// Execute a comprehensive verification of all mathematical and accounting invariants.
    ///
    /// Verifies:
    /// 1. Global Conservation: sum(all debits) == sum(all credits) across all transactions.
    /// 2. Fundamental Accounting Equation:
    ///    Assets - Liabilities == Equity + (Income - Expenses)
    pub fn verify_all_invariants(&self) -> Result<(), CoreError> {
        let mut global_debits = Money::ZERO;
        let mut global_credits = Money::ZERO;

        for tx in &self.transactions {
            for posting in &tx.postings {
                match posting.direction {
                    Direction::Debit => {
                        global_debits = global_debits.checked_add(posting.amount)?;
                    }
                    Direction::Credit => {
                        global_credits = global_credits.checked_add(posting.amount)?;
                    }
                }
            }
        }

        if global_debits != global_credits {
            return Err(CoreError::LedgerEquilibriumBreach);
        }

        // Check Fundamental Accounting Equation
        let mut assets = Money::ZERO;
        let mut liabilities = Money::ZERO;
        let mut equity = Money::ZERO;
        let mut income = Money::ZERO;
        let mut expense = Money::ZERO;

        for account in self.accounts.values() {
            let bal = self.get_balance(&account.id)?;
            match account.account_type {
                AccountType::Asset => assets = assets.checked_add(bal)?,
                AccountType::Liability => liabilities = liabilities.checked_add(bal)?,
                AccountType::Equity => equity = equity.checked_add(bal)?,
                AccountType::Income => income = income.checked_add(bal)?,
                AccountType::Expense => expense = expense.checked_add(bal)?,
            }
        }

        let left_side = assets.checked_sub(liabilities)?;
        let net_income = income.checked_sub(expense)?;
        let right_side = equity.checked_add(net_income)?;

        if left_side != right_side {
            return Err(CoreError::AccountingEquationViolation);
        }

        Ok(())
    }
}
