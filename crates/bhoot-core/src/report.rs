use crate::account::{AccountId, AccountSubtype};
use crate::error::CoreError;
use crate::money::Money;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Summary item representation of an individual account's balance in a report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountSummaryItem {
    pub account_id: AccountId,
    pub name: String,
    pub subtype: AccountSubtype,
    #[serde(rename = "balance_paisa", alias = "balance")]
    pub balance: Money,
}

/// Point-in-time Balance Sheet statement enforcing mathematical equilibrium.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BalanceSheet {
    pub as_of: DateTime<Utc>,
    pub assets: Vec<AccountSummaryItem>,
    #[serde(rename = "total_assets_paisa", alias = "total_assets")]
    pub total_assets: Money,
    pub liabilities: Vec<AccountSummaryItem>,
    #[serde(rename = "total_liabilities_paisa", alias = "total_liabilities")]
    pub total_liabilities: Money,
    pub equity: Vec<AccountSummaryItem>,
    #[serde(rename = "total_equity_paisa", alias = "total_equity")]
    pub total_equity: Money,
    #[serde(rename = "retained_earnings_paisa", alias = "retained_earnings")]
    pub retained_earnings: Money,
    #[serde(rename = "net_worth_paisa", alias = "net_worth")]
    pub net_worth: Money,
    pub is_balanced: bool,
}

impl BalanceSheet {
    /// Construct a verified Balance Sheet.
    ///
    /// # Errors
    /// Returns `CoreError::ArithmeticOverflow` on integer overflow.
    pub fn new(
        as_of: DateTime<Utc>,
        assets: Vec<AccountSummaryItem>,
        liabilities: Vec<AccountSummaryItem>,
        equity: Vec<AccountSummaryItem>,
        retained_earnings: Money,
    ) -> Result<Self, CoreError> {
        let mut total_assets = Money::ZERO;
        for a in &assets {
            total_assets = total_assets.checked_add(a.balance)?;
        }

        let mut total_liabilities = Money::ZERO;
        for l in &liabilities {
            total_liabilities = total_liabilities.checked_add(l.balance)?;
        }

        let mut total_equity = Money::ZERO;
        for e in &equity {
            total_equity = total_equity.checked_add(e.balance)?;
        }

        let net_worth = total_assets.checked_sub(total_liabilities)?;

        // Equilibrium: Assets = Liabilities + Equity + Retained Earnings
        let total_claims = total_liabilities
            .checked_add(total_equity)?
            .checked_add(retained_earnings)?;

        let is_balanced = total_assets == total_claims;

        Ok(Self {
            as_of,
            assets,
            total_assets,
            liabilities,
            total_liabilities,
            equity,
            total_equity,
            retained_earnings,
            net_worth,
            is_balanced,
        })
    }
}

/// Interval Profit & Loss (Income Statement) incorporating operating and trading records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IncomeStatement {
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
    pub operating_income: Vec<AccountSummaryItem>,
    #[serde(rename = "total_operating_income_paisa", alias = "total_operating_income")]
    pub total_operating_income: Money,
    #[serde(rename = "trading_gross_pnl_paisa", alias = "trading_gross_pnl")]
    pub trading_gross_pnl: Money,
    #[serde(rename = "trading_charges_paisa", alias = "trading_charges")]
    pub trading_charges: Money,
    #[serde(rename = "trading_net_pnl_paisa", alias = "trading_net_pnl")]
    pub trading_net_pnl: Money,
    #[serde(rename = "total_income_paisa", alias = "total_income")]
    pub total_income: Money,
    pub operating_expenses: Vec<AccountSummaryItem>,
    #[serde(rename = "total_operating_expenses_paisa", alias = "total_operating_expenses")]
    pub total_operating_expenses: Money,
    #[serde(rename = "net_profit_paisa", alias = "net_profit")]
    pub net_profit: Money,
}

impl IncomeStatement {
    /// Construct a verified Income Statement.
    ///
    /// # Errors
    /// Returns `CoreError::ArithmeticOverflow` on calculation overflow.
    pub fn new(
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        operating_income: Vec<AccountSummaryItem>,
        trading_gross_pnl: Money,
        trading_charges: Money,
        operating_expenses: Vec<AccountSummaryItem>,
    ) -> Result<Self, CoreError> {
        let mut total_operating_income = Money::ZERO;
        for inc in &operating_income {
            total_operating_income = total_operating_income.checked_add(inc.balance)?;
        }

        let trading_net_pnl = trading_gross_pnl.checked_sub(trading_charges)?;
        let total_income = total_operating_income.checked_add(trading_net_pnl)?;

        let mut total_operating_expenses = Money::ZERO;
        for exp in &operating_expenses {
            total_operating_expenses = total_operating_expenses.checked_add(exp.balance)?;
        }

        let net_profit = total_income.checked_sub(total_operating_expenses)?;

        Ok(Self {
            from,
            to,
            operating_income,
            total_operating_income,
            trading_gross_pnl,
            trading_charges,
            trading_net_pnl,
            total_income,
            operating_expenses,
            total_operating_expenses,
            net_profit,
        })
    }
}

/// Cash Flow Statement tracking movements in liquid cash and bank balances.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CashFlowStatement {
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
    #[serde(rename = "operating_cash_flow_paisa", alias = "operating_cash_flow")]
    pub operating_cash_flow: Money,
    #[serde(rename = "investing_cash_flow_paisa", alias = "investing_cash_flow")]
    pub investing_cash_flow: Money,
    #[serde(rename = "financing_cash_flow_paisa", alias = "financing_cash_flow")]
    pub financing_cash_flow: Money,
    #[serde(rename = "net_cash_flow_paisa", alias = "net_cash_flow")]
    pub net_cash_flow: Money,
    #[serde(rename = "beginning_cash_paisa", alias = "beginning_cash")]
    pub beginning_cash: Money,
    #[serde(rename = "ending_cash_paisa", alias = "ending_cash")]
    pub ending_cash: Money,
}

impl CashFlowStatement {
    /// Construct and validate a Cash Flow Statement.
    ///
    /// # Errors
    /// Returns `CoreError::ArithmeticOverflow` on arithmetic overflow.
    pub fn new(
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        operating_cash_flow: Money,
        investing_cash_flow: Money,
        financing_cash_flow: Money,
        beginning_cash: Money,
    ) -> Result<Self, CoreError> {
        let net_cash_flow = operating_cash_flow
            .checked_add(investing_cash_flow)?
            .checked_add(financing_cash_flow)?;

        let ending_cash = beginning_cash.checked_add(net_cash_flow)?;

        Ok(Self {
            from,
            to,
            operating_cash_flow,
            investing_cash_flow,
            financing_cash_flow,
            net_cash_flow,
            beginning_cash,
            ending_cash,
        })
    }
}
