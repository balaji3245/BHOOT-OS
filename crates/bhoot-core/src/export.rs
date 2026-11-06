use crate::money::Money;
use crate::trading::TradeRecord;
use crate::transaction::Transaction;
use chrono::Utc;
use sha2::{Digest, Sha256};
use std::fmt::Write;

/// Prominent security warning banner prepended to all unencrypted exports.
pub const CSV_SECURITY_BANNER: &str = "\
# ==============================================================================
# WARNING: CONFIDENTIAL FINANCIAL RECORD EXPORTED FROM BHOOT OS.
# THIS FILE IS STORED UNENCRYPTED IN PLAINTEXT ON DISK.
# CONTAINS HIGHLY SENSITIVE ASSET, ACCOUNT, AND TRANSACTION BALANCES.
# SECURELY DESTROY OR ENCRYPT THIS FILE IMMEDIATELY AFTER USE.
# ==============================================================================";

/// Sanitizes CSV fields against formula injection / CSV command execution vulnerabilities.
/// If a value begins with `=`, `+`, `-`, `@`, `\t`, or `\r`, it is prepended with a single quote.
pub fn sanitize_csv_cell(value: &str) -> String {
    let trimmed = value.trim();
    let sanitized = if trimmed.starts_with('=')
        || trimmed.starts_with('+')
        || trimmed.starts_with('-')
        || trimmed.starts_with('@')
        || trimmed.starts_with('\t')
        || trimmed.starts_with('\r')
    {
        format!("'{value}")
    } else {
        value.to_string()
    };

    // RFC 4180 escaping: wrap in quotes if contains comma, quote, or newline
    if sanitized.contains(',') || sanitized.contains('"') || sanitized.contains('\n') || sanitized.contains('\r') {
        format!("\"{}\"", sanitized.replace('"', "\"\""))
    } else {
        sanitized
    }
}

/// Generates a standardized, sanitized CSV document from double-entry transactions.
pub fn export_transactions_csv(
    transactions: &[Transaction],
    account_names: &std::collections::HashMap<crate::account::AccountId, String>,
) -> String {
    let mut body = String::new();
    writeln!(
        body,
        "TransactionID,Timestamp,Type,Category,PaymentMethod,ReferenceNumber,AccountID,AccountName,Direction,AmountINR,AmountPaisa,Memo"
    )
    .unwrap();

    for tx in transactions {
        let cat = tx.category.as_deref().unwrap_or("");
        let method = tx
            .payment_method
            .map(|m| format!("{m:?}"))
            .unwrap_or_default();
        let ref_num = tx.reference_number.as_deref().unwrap_or("");

        for p in &tx.postings {
            let acc_name = account_names
                .get(&p.account_id)
                .map(String::as_str)
                .unwrap_or("Unknown Account");
            let memo = p.memo.as_deref().unwrap_or("");

            writeln!(
                body,
                "{},{},{},{},{},{},{},{},{:?},{},{},{}",
                sanitize_csv_cell(&tx.id.to_string()),
                sanitize_csv_cell(&tx.timestamp.to_rfc3339()),
                sanitize_csv_cell(&format!("{:?}", tx.tx_type)),
                sanitize_csv_cell(cat),
                sanitize_csv_cell(&method),
                sanitize_csv_cell(ref_num),
                sanitize_csv_cell(&p.account_id.to_string()),
                sanitize_csv_cell(acc_name),
                p.direction,
                sanitize_csv_cell(&p.amount.format_inr()),
                p.amount.as_paisa(),
                sanitize_csv_cell(memo)
            )
            .unwrap();
        }
    }

    // Compute SHA-256 checksum of data body
    let mut hasher = Sha256::new();
    hasher.update(body.as_bytes());
    let checksum = hasher
        .finalize()
        .iter()
        .fold(String::new(), |mut acc, b| {
            let _ = write!(acc, "{b:02x}");
            acc
        });

    format!(
        "{}\n# Export Timestamp: {}\n# Data SHA-256 Checksum: {}\n# Total Transactions: {}\n# ==============================================================================\n{}",
        CSV_SECURITY_BANNER,
        Utc::now().to_rfc3339(),
        checksum,
        transactions.len(),
        body
    )
}

/// Generates a sanitized CSV document from broker trading records.
pub fn export_trades_csv(trades: &[TradeRecord]) -> String {
    let mut body = String::new();
    writeln!(
        body,
        "TradeID,BrokerAccountID,TradeDate,Symbol,TradeType,GrossPnL_INR,GrossPnL_Paisa,Brokerage_Paisa,STT_Paisa,ExchangeCharges_Paisa,GST_Paisa,SEBI_Paisa,StampDuty_Paisa,TotalCharges_Paisa,NetPnL_INR,NetPnL_Paisa,OrderID,ContractNoteID"
    )
    .unwrap();

    for t in trades {
        let order_id = t.order_id.as_deref().unwrap_or("");
        let contract_note = t
            .contract_note_id
            .map(|u| u.to_string())
            .unwrap_or_default();
        let total_charges = t.charges.total_charges().unwrap_or(Money::ZERO);

        writeln!(
            body,
            "{},{},{},{},{:?},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            sanitize_csv_cell(&t.id.to_string()),
            sanitize_csv_cell(&t.broker_account_id.to_string()),
            sanitize_csv_cell(&t.trade_date.to_rfc3339()),
            sanitize_csv_cell(&t.symbol),
            t.trade_type,
            sanitize_csv_cell(&t.gross_pnl.format_inr()),
            t.gross_pnl.as_paisa(),
            t.charges.brokerage.as_paisa(),
            t.charges.stt.as_paisa(),
            t.charges.exchange_turnover_charges.as_paisa(),
            t.charges.gst.as_paisa(),
            t.charges.sebi_charges.as_paisa(),
            t.charges.stamp_duty.as_paisa(),
            total_charges.as_paisa(),
            sanitize_csv_cell(&t.net_pnl.format_inr()),
            t.net_pnl.as_paisa(),
            sanitize_csv_cell(order_id),
            sanitize_csv_cell(&contract_note)
        )
        .unwrap();
    }

    let mut hasher = Sha256::new();
    hasher.update(body.as_bytes());
    let checksum = hasher
        .finalize()
        .iter()
        .fold(String::new(), |mut acc, b| {
            let _ = write!(acc, "{b:02x}");
            acc
        });

    format!(
        "{}\n# Export Timestamp: {}\n# Data SHA-256 Checksum: {}\n# Total Trades: {}\n# ==============================================================================\n{}",
        CSV_SECURITY_BANNER,
        Utc::now().to_rfc3339(),
        checksum,
        trades.len(),
        body
    )
}
