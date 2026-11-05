use crate::account::AccountId;
use crate::error::CoreError;
use crate::money::Money;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Trading account configuration registered with a SEBI-registered broker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrokerAccount {
    pub account_id: AccountId,
    pub broker_name: String,
    pub client_id: String,
    pub clearing_bank_account_id: AccountId,
    pub is_active: bool,
}

impl BrokerAccount {
    #[must_use]
    pub fn new(
        account_id: AccountId,
        broker_name: impl Into<String>,
        client_id: impl Into<String>,
        clearing_bank_account_id: AccountId,
    ) -> Self {
        Self {
            account_id,
            broker_name: broker_name.into(),
            client_id: client_id.into(),
            clearing_bank_account_id,
            is_active: true,
        }
    }
}

/// Trading segment / instrument classification.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TradeType {
    IntradayEquity,
    DeliveryEquity,
    Futures,
    Options,
    Commodity,
    Currency,
}

/// Regulatory charges, brokerage, and statutory taxes on trade execution.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TradeCharges {
    pub brokerage: Money,
    pub stt: Money,
    pub exchange_turnover_charges: Money,
    pub gst: Money,
    pub sebi_charges: Money,
    pub stamp_duty: Money,
}

impl TradeCharges {
    /// Calculate total statutory fees and broker commissions.
    ///
    /// # Errors
    /// Returns `CoreError::ArithmeticOverflow` on integer overflow.
    pub fn total_charges(&self) -> Result<Money, CoreError> {
        self.brokerage
            .checked_add(self.stt)?
            .checked_add(self.exchange_turnover_charges)?
            .checked_add(self.gst)?
            .checked_add(self.sebi_charges)?
            .checked_add(self.stamp_duty)
    }

    /// Validate that no charge is negative.
    pub fn validate(&self) -> Result<(), CoreError> {
        if self.brokerage.is_negative()
            || self.stt.is_negative()
            || self.exchange_turnover_charges.is_negative()
            || self.gst.is_negative()
            || self.sebi_charges.is_negative()
            || self.stamp_duty.is_negative()
        {
            return Err(CoreError::InvalidTrade(
                "Trade charges and statutory fees cannot be negative".into(),
            ));
        }
        Ok(())
    }
}

/// Trade transaction execution record with audited P&L and fees.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TradeRecord {
    pub id: Uuid,
    pub broker_account_id: AccountId,
    pub trade_date: DateTime<Utc>,
    pub symbol: String,
    pub trade_type: TradeType,
    pub gross_pnl: Money,
    pub charges: TradeCharges,
    pub net_pnl: Money,
    pub order_id: Option<String>,
    pub contract_note_id: Option<Uuid>,
}

impl TradeRecord {
    /// Construct and mathematically validate a trade record.
    ///
    /// # Errors
    /// Returns `CoreError::InvalidTrade` if Net P&L != Gross P&L - Total Charges.
    pub fn new(
        broker_account_id: AccountId,
        trade_date: DateTime<Utc>,
        symbol: impl Into<String>,
        trade_type: TradeType,
        gross_pnl: Money,
        charges: TradeCharges,
        net_pnl: Money,
        order_id: Option<String>,
        contract_note_id: Option<Uuid>,
    ) -> Result<Self, CoreError> {
        charges.validate()?;
        let total_charges = charges.total_charges()?;
        let expected_net = gross_pnl.checked_sub(total_charges)?;

        if expected_net != net_pnl {
            return Err(CoreError::InvalidTrade(format!(
                "Trade record invariant breach: gross_pnl ({gross_pnl}) - charges ({total_charges}) = {expected_net}, but declared net_pnl = {net_pnl}"
            )));
        }

        Ok(Self {
            id: Uuid::new_v4(),
            broker_account_id,
            trade_date,
            symbol: symbol.into(),
            trade_type,
            gross_pnl,
            charges,
            net_pnl,
            order_id,
            contract_note_id,
        })
    }
}

/// Lifecycle status of broker funds withdrawal / payout.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PayoutStatus {
    Requested,
    Processed,
    Settled,
    Rejected,
}

/// Record of funds withdrawal from broker account to linked bank account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PayoutRecord {
    pub id: Uuid,
    pub broker_account_id: AccountId,
    pub destination_bank_account_id: AccountId,
    pub request_date: DateTime<Utc>,
    pub settlement_date: Option<DateTime<Utc>>,
    pub requested_amount: Money,
    pub tds_deducted: Money,
    pub net_credited_amount: Money,
    pub utr_number: Option<String>,
    pub status: PayoutStatus,
}

impl PayoutRecord {
    /// Construct and validate a broker payout record.
    ///
    /// # Errors
    /// Returns `CoreError::InvalidPayout` if:
    /// - requested_amount is not strictly positive.
    /// - tds_deducted is negative.
    /// - net_credited_amount != requested_amount - tds_deducted.
    pub fn new(
        broker_account_id: AccountId,
        destination_bank_account_id: AccountId,
        request_date: DateTime<Utc>,
        settlement_date: Option<DateTime<Utc>>,
        requested_amount: Money,
        tds_deducted: Money,
        net_credited_amount: Money,
        utr_number: Option<String>,
        status: PayoutStatus,
    ) -> Result<Self, CoreError> {
        if !requested_amount.is_positive() {
            return Err(CoreError::InvalidPayout(
                "Requested payout amount must be strictly positive".into(),
            ));
        }

        if tds_deducted.is_negative() {
            return Err(CoreError::InvalidPayout(
                "TDS deduction cannot be negative".into(),
            ));
        }

        let expected_net = requested_amount.checked_sub(tds_deducted)?;
        if expected_net != net_credited_amount {
            return Err(CoreError::InvalidPayout(format!(
                "Payout invariant breach: requested ({requested_amount}) - TDS ({tds_deducted}) = {expected_net}, but net_credited = {net_credited_amount}"
            )));
        }

        Ok(Self {
            id: Uuid::new_v4(),
            broker_account_id,
            destination_bank_account_id,
            request_date,
            settlement_date,
            requested_amount,
            tds_deducted,
            net_credited_amount,
            utr_number,
            status,
        })
    }
}
