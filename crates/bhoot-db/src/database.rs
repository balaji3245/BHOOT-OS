use crate::config::SqlCipherConfig;
use crate::error::DbError;
use crate::schema::{MIGRATION_001_SQL, MIGRATION_002_SQL};
use bhoot_audit::{AuditEvent, AuditEventType};
use bhoot_core::{
    Account, AccountId, AccountStatus, AccountSubtype, AccountType, AccountSummaryItem,
    BalanceSheet, Budget, BudgetPeriod, BudgetReport, CashFlowStatement, CategoryBudget,
    Direction, IncomeStatement, Money, PayoutRecord, PayoutStatus, Posting, TradeCharges,
    TradeRecord, Transaction, TransactionId, TransactionType, VerificationStatus,
};
use bhoot_crypto::EncryptedEnvelope;
use bhoot_vault::{DocumentMimeType, VaultDocument};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use uuid::Uuid;

/// Summary performance metrics for trading journal analysis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TradingSummary {
    pub total_trades: usize,
    pub winning_trades: usize,
    pub losing_trades: usize,
    pub win_rate_basis_points: u32,
    pub gross_profit_paisa: i64,
    pub gross_loss_paisa: i64,
    pub total_charges_paisa: i64,
    pub net_pnl_paisa: i64,
    pub profit_factor_scaled: u64,
    pub max_drawdown_paisa: i64,
}

/// Metadata summary for vault documents without ciphertext payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VaultDocumentSummary {
    pub id: Uuid,
    pub filename: String,
    pub mime_type: DocumentMimeType,
    pub file_size_bytes: usize,
    pub blake3_hash: [u8; 32],
    pub sha256_hash: [u8; 32],
    pub created_at: DateTime<Utc>,
}

/// Thread-safe or shared handle to an encrypted SQLCipher SQLite database.
#[derive(Debug)]
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open or create an encrypted SQLite database using SQLCipher and apply hardening PRAGMAs.
    ///
    /// # Errors
    /// Returns `DbError::AuthenticationFailed` if the key is incorrect or the database is corrupted.
    pub fn open(path: impl AsRef<Path>, key: &[u8; 32]) -> Result<Self, DbError> {
        let conn = Connection::open(path)?;
        Self::initialize_connection(conn, key)
    }

    /// Open an encrypted in-memory database (useful for isolated unit testing).
    pub fn open_in_memory(key: &[u8; 32]) -> Result<Self, DbError> {
        let conn = Connection::open_in_memory()?;
        Self::initialize_connection(conn, key)
    }

    /// Configure SQLCipher encryption key, hardening pragmas, and execute migrations.
    fn initialize_connection(conn: Connection, key: &[u8; 32]) -> Result<Self, DbError> {
        let config = SqlCipherConfig::default();
        let pragmas = config.generate_pragmas(key);

        for p in &pragmas {
            conn.execute_batch(p)
                .map_err(|e| DbError::KeyError(format!("Failed applying pragma: {e}")))?;
        }

        // Verify key authentication by querying sqlite_master
        let auth_check: Result<u32, _> =
            conn.query_row("SELECT count(*) FROM sqlite_master;", [], |r| r.get(0));

        match auth_check {
            Ok(_) => {}
            Err(e) => {
                return Err(DbError::AuthenticationFailed(format!(
                    "SQLCipher failed to authenticate key or database corrupted: {e}"
                )));
            }
        }

        let db = Self { conn };
        db.run_migrations()?;
        Ok(db)
    }

    /// Run embedded database schema migrations inside an atomic transaction.
    pub fn run_migrations(&self) -> Result<(), DbError> {
        let version: u32 = self
            .conn
            .query_row("PRAGMA user_version;", [], |r| r.get(0))
            .unwrap_or(0);

        if version < 1 {
            let tx = self.conn.unchecked_transaction()?;
            tx.execute_batch(MIGRATION_001_SQL)?;
            tx.execute_batch("PRAGMA user_version = 1;")?;
            tx.commit()?;
        }

        if version < 2 {
            let tx = self.conn.unchecked_transaction()?;
            tx.execute_batch(MIGRATION_002_SQL)?;
            tx.execute_batch("PRAGMA user_version = 2;")?;
            tx.commit()?;
        }

        Ok(())
    }

    /// Access the underlying `rusqlite::Connection`.
    #[must_use]
    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    /// Change the database encryption key via SQLCipher `PRAGMA rekey`.
    pub fn rekey(&mut self, new_key: &[u8; 32]) -> Result<(), DbError> {
        let hex_key: String = new_key.iter().map(|b| format!("{b:02x}")).collect();
        self.conn
            .execute_batch(&format!("PRAGMA rekey = \"x'{hex_key}'\";"))
            .map_err(|e| DbError::KeyError(format!("Failed executing PRAGMA rekey: {e}")))?;
        Ok(())
    }

    /// Run full SQLite database integrity check (`PRAGMA integrity_check`).
    pub fn integrity_check(&self) -> Result<bool, DbError> {
        let mut stmt = self.conn.prepare("PRAGMA integrity_check;")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        for r in rows {
            let res = r?;
            if res != "ok" {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Vacuum the database to reclaim space and compact pages.
    pub fn vacuum(&self) -> Result<(), DbError> {
        self.conn.execute_batch("VACUUM;")?;
        Ok(())
    }

    /// Force a full SQLite WAL checkpoint, flushing and truncating the write-ahead log.
    pub fn checkpoint(&self) -> Result<(), DbError> {
        self.conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
        Ok(())
    }

    // =========================================================================
    // ACCOUNT REPOSITORY
    // =========================================================================

    /// Insert a new financial account.
    pub fn create_account(&self, account: &Account) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT INTO accounts (id, name, account_type, subtype, status, currency, description, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9);",
            params![
                account.id.to_string(),
                account.name,
                format!("{:?}", account.account_type),
                format!("{:?}", account.subtype),
                format!("{:?}", account.status),
                account.currency,
                account.description,
                account.created_at.to_rfc3339(),
                account.updated_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    /// Retrieve an account by its unique identifier.
    pub fn get_account(&self, id: AccountId) -> Result<Option<Account>, DbError> {
        let id_str = id.to_string();
        let mut stmt = self.conn.prepare(
            "SELECT id, name, account_type, subtype, status, currency, description, created_at, updated_at
             FROM accounts WHERE id = ?1;",
        )?;

        let account = stmt
            .query_row(params![id_str], |row| {
                let id_parsed: AccountId = row
                    .get::<_, String>(0)?
                    .parse()
                    .map_err(|_| rusqlite::Error::InvalidQuery)?;
                let name: String = row.get(1)?;
                let type_str: String = row.get(2)?;
                let subtype_str: String = row.get(3)?;
                let status_str: String = row.get(4)?;
                let currency: String = row.get(5)?;
                let description: Option<String> = row.get(6)?;
                let created_str: String = row.get(7)?;
                let updated_str: String = row.get(8)?;

                let account_type = parse_account_type(&type_str);
                let subtype = parse_account_subtype(&subtype_str);
                let status = parse_account_status(&status_str);

                let created_at = DateTime::parse_from_rfc3339(&created_str)
                    .map_err(|_| rusqlite::Error::InvalidQuery)?
                    .with_timezone(&Utc);
                let updated_at = DateTime::parse_from_rfc3339(&updated_str)
                    .map_err(|_| rusqlite::Error::InvalidQuery)?
                    .with_timezone(&Utc);

                Ok(Account {
                    id: id_parsed,
                    name,
                    account_type,
                    subtype,
                    status,
                    currency,
                    description,
                    created_at,
                    updated_at,
                })
            })
            .optional()?;

        Ok(account)
    }

    /// List all accounts ordered by name.
    pub fn list_accounts(&self) -> Result<Vec<Account>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, account_type, subtype, status, currency, description, created_at, updated_at
             FROM accounts ORDER BY name ASC;",
        )?;

        let rows = stmt.query_map([], |row| {
            let id_parsed: AccountId = row
                .get::<_, String>(0)?
                .parse()
                .map_err(|_| rusqlite::Error::InvalidQuery)?;
            let name: String = row.get(1)?;
            let type_str: String = row.get(2)?;
            let subtype_str: String = row.get(3)?;
            let status_str: String = row.get(4)?;
            let currency: String = row.get(5)?;
            let description: Option<String> = row.get(6)?;
            let created_str: String = row.get(7)?;
            let updated_str: String = row.get(8)?;

            let account_type = parse_account_type(&type_str);
            let subtype = parse_account_subtype(&subtype_str);
            let status = parse_account_status(&status_str);

            let created_at = DateTime::parse_from_rfc3339(&created_str)
                .map_err(|_| rusqlite::Error::InvalidQuery)?
                .with_timezone(&Utc);
            let updated_at = DateTime::parse_from_rfc3339(&updated_str)
                .map_err(|_| rusqlite::Error::InvalidQuery)?
                .with_timezone(&Utc);

            Ok(Account {
                id: id_parsed,
                name,
                account_type,
                subtype,
                status,
                currency,
                description,
                created_at,
                updated_at,
            })
        })?;

        let mut accounts = Vec::new();
        for r in rows {
            accounts.push(r?);
        }
        Ok(accounts)
    }

    /// Calculate the current ledger balance for an account by summing its postings.
    ///
    /// Respects double-entry normal balance conventions:
    /// - For Asset & Expense: Normal is Debit (Debits - Credits).
    /// - For Liability, Equity & Income: Normal is Credit (Credits - Debits).
    pub fn get_account_balance(&self, id: AccountId) -> Result<Money, DbError> {
        let account = self
            .get_account(id)?
            .ok_or_else(|| DbError::AccountNotFound(id.to_string()))?;

        let mut stmt = self.conn.prepare(
            "SELECT direction, COALESCE(SUM(amount_paisa), 0)
             FROM postings
             WHERE account_id = ?1
             GROUP BY direction;",
        )?;

        let mut total_debit: i64 = 0;
        let mut total_credit: i64 = 0;

        let rows = stmt.query_map(params![id.to_string()], |row| {
            let dir: String = row.get(0)?;
            let sum: i64 = row.get(1)?;
            Ok((dir, sum))
        })?;

        for r in rows {
            let (dir, sum) = r?;
            if dir == "Debit" {
                total_debit += sum;
            } else if dir == "Credit" {
                total_credit += sum;
            }
        }

        let normal_balance = account.account_type.normal_balance();
        let balance_paisa = match normal_balance {
            bhoot_core::NormalBalance::Debit => total_debit - total_credit,
            bhoot_core::NormalBalance::Credit => total_credit - total_debit,
        };

        Ok(Money::from_paisa(balance_paisa))
    }

    /// Update status of an existing account.
    pub fn update_account_status(
        &self,
        id: AccountId,
        status: AccountStatus,
    ) -> Result<(), DbError> {
        let now = Utc::now().to_rfc3339();
        let rows = self.conn.execute(
            "UPDATE accounts SET status = ?1, updated_at = ?2 WHERE id = ?3;",
            params![format!("{status:?}"), now, id.to_string()],
        )?;
        if rows == 0 {
            return Err(DbError::AccountNotFound(id.to_string()));
        }
        Ok(())
    }

    // =========================================================================
    // TRANSACTION & POSTING REPOSITORY (DOUBLE-ENTRY ENGINE)
    // =========================================================================

    /// Post a double-entry transaction atomically.
    ///
    /// Enforces:
    /// - Transaction must be strictly balanced (Sum(Debits) == Sum(Credits)).
    /// - Minimum 2 postings.
    /// - Accounts must exist and cannot be Frozen.
    /// - Single-entry or unbalanced transactions are rejected before committing.
    pub fn post_transaction(&self, tx: &Transaction) -> Result<(), DbError> {
        tx.validate()?;

        // Verify all referenced accounts exist and are not frozen
        for p in &tx.postings {
            let acc = self
                .get_account(p.account_id)?
                .ok_or_else(|| DbError::AccountNotFound(p.account_id.to_string()))?;
            if acc.status == AccountStatus::Frozen {
                return Err(DbError::AccountFrozen(p.account_id.to_string()));
            }
        }

        let tx_conn = self.conn.unchecked_transaction()?;

        let reversal_str = tx.reversal_of.map(|r| r.to_string());
        let payment_str = tx.payment_method.map(|p| format!("{p:?}"));

        tx_conn.execute(
            "INSERT INTO transactions (id, timestamp, tx_type, category, payment_method, reference_number, notes, verification_status, reversal_of)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9);",
            params![
                tx.id.to_string(),
                tx.timestamp.to_rfc3339(),
                format!("{:?}", tx.tx_type),
                tx.category,
                payment_str,
                tx.reference_number,
                tx.notes,
                format!("{:?}", tx.verification_status),
                reversal_str,
            ],
        )?;

        for p in &tx.postings {
            tx_conn.execute(
                "INSERT INTO postings (id, transaction_id, account_id, amount_paisa, direction, memo)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6);",
                params![
                    Uuid::new_v4().to_string(),
                    tx.id.to_string(),
                    p.account_id.to_string(),
                    p.amount.as_paisa(),
                    format!("{:?}", p.direction),
                    p.memo,
                ],
            )?;
        }

        tx_conn.commit()?;
        Ok(())
    }

    /// Retrieve a transaction and all its child postings.
    pub fn get_transaction(&self, id: TransactionId) -> Result<Option<Transaction>, DbError> {
        let id_str = id.to_string();
        let mut stmt = self.conn.prepare(
            "SELECT id, timestamp, tx_type, category, payment_method, reference_number, notes, verification_status, reversal_of
             FROM transactions WHERE id = ?1;",
        )?;

        let tx_opt = stmt
            .query_row(params![id_str], |row| {
                let tx_id: TransactionId = row
                    .get::<_, String>(0)?
                    .parse()
                    .map_err(|_| rusqlite::Error::InvalidQuery)?;
                let ts_str: String = row.get(1)?;
                let type_str: String = row.get(2)?;
                let category: Option<String> = row.get(3)?;
                let payment_str: Option<String> = row.get(4)?;
                let ref_num: Option<String> = row.get(5)?;
                let notes: Option<String> = row.get(6)?;
                let vstatus_str: String = row.get(7)?;
                let reversal_str: Option<String> = row.get(8)?;

                let timestamp = DateTime::parse_from_rfc3339(&ts_str)
                    .map_err(|_| rusqlite::Error::InvalidQuery)?
                    .with_timezone(&Utc);

                let tx_type = parse_tx_type(&type_str);
                let payment_method = payment_str.as_deref().map(parse_payment_method);
                let verification_status = parse_vstatus(&vstatus_str);
                let reversal_of = reversal_str.and_then(|s| s.parse().ok());

                Ok((
                    tx_id,
                    timestamp,
                    tx_type,
                    category,
                    payment_method,
                    ref_num,
                    notes,
                    verification_status,
                    reversal_of,
                ))
            })
            .optional()?;

        let Some((
            tx_id,
            timestamp,
            tx_type,
            category,
            payment_method,
            reference_number,
            notes,
            verification_status,
            reversal_of,
        )) = tx_opt
        else {
            return Ok(None);
        };

        // Fetch postings
        let mut post_stmt = self.conn.prepare(
            "SELECT account_id, amount_paisa, direction, memo
             FROM postings WHERE transaction_id = ?1;",
        )?;

        let post_rows = post_stmt.query_map(params![id_str], |row| {
            let acc_id: AccountId = row
                .get::<_, String>(0)?
                .parse()
                .map_err(|_| rusqlite::Error::InvalidQuery)?;
            let amount_paisa: i64 = row.get(1)?;
            let dir_str: String = row.get(2)?;
            let memo: Option<String> = row.get(3)?;

            let direction = if dir_str == "Debit" {
                Direction::Debit
            } else {
                Direction::Credit
            };

            Ok(Posting {
                account_id: acc_id,
                amount: Money::from_paisa(amount_paisa),
                direction,
                memo,
            })
        })?;

        let mut postings = Vec::new();
        for pr in post_rows {
            postings.push(pr?);
        }

        Ok(Some(Transaction {
            id: tx_id,
            timestamp,
            tx_type,
            postings,
            category,
            payment_method,
            reference_number,
            notes,
            proof_attachment_ids: Vec::new(),
            verification_status,
            reversal_of,
        }))
    }

    /// List recent transactions with postings, ordered newest to oldest.
    pub fn list_transactions(
        &self,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Transaction>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id FROM transactions ORDER BY timestamp DESC, id DESC LIMIT ?1 OFFSET ?2;",
        )?;

        let ids = stmt.query_map(params![limit as i64, offset as i64], |row| {
            let id_str: String = row.get(0)?;
            id_str.parse().map_err(|_| rusqlite::Error::InvalidQuery)
        })?;

        let mut transactions = Vec::new();
        for id_res in ids {
            let id: TransactionId = id_res?;
            if let Some(tx) = self.get_transaction(id)? {
                transactions.push(tx);
            }
        }
        Ok(transactions)
    }

    /// Reverse an existing transaction by creating an exact reciprocal transaction.
    ///
    /// Rules enforced:
    /// - Original transaction must exist.
    /// - Original cannot be a reversal transaction (a reversal cannot be reversed).
    /// - Original cannot have already been reversed (unique `reversal_of` constraint).
    pub fn reverse_transaction(
        &self,
        original_id: TransactionId,
        reason: &str,
        timestamp: DateTime<Utc>,
    ) -> Result<Transaction, DbError> {
        let original = self
            .get_transaction(original_id)?
            .ok_or_else(|| DbError::TransactionNotFound(original_id.to_string()))?;

        if original.tx_type == TransactionType::Reversal {
            return Err(DbError::CannotReverseReversal(original_id.to_string()));
        }

        // Check if already reversed
        let count: i64 = self.conn.query_row(
            "SELECT count(*) FROM transactions WHERE reversal_of = ?1;",
            params![original_id.to_string()],
            |r| r.get(0),
        )?;

        if count > 0 {
            return Err(DbError::AlreadyReversed(original_id.to_string()));
        }

        let reversal_tx = original.create_reversal(timestamp, reason)?;
        self.post_transaction(&reversal_tx)?;
        Ok(reversal_tx)
    }

    // =========================================================================
    // TRADING JOURNAL REPOSITORY
    // =========================================================================

    /// Record a completed or open trade.
    pub fn record_trade(&self, trade: &TradeRecord) -> Result<(), DbError> {
        let total_charges = trade.charges.total_charges()?.as_paisa();
        self.conn.execute(
            "INSERT INTO trades (id, broker_account_id, trade_date, symbol, trade_type, gross_pnl_paisa, total_charges_paisa, net_pnl_paisa, order_id, contract_note_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10);",
            params![
                trade.id.to_string(),
                trade.broker_account_id.to_string(),
                trade.trade_date.to_rfc3339(),
                trade.symbol,
                format!("{:?}", trade.trade_type),
                trade.gross_pnl.as_paisa(),
                total_charges,
                trade.net_pnl.as_paisa(),
                trade.order_id,
                trade.contract_note_id.map(|u| u.to_string()),
            ],
        )?;
        Ok(())
    }

    /// List trades, optionally filtered by broker account.
    pub fn list_trades(&self, broker_account_id: Option<AccountId>) -> Result<Vec<TradeRecord>, DbError> {
        let sql = match broker_account_id {
            Some(_) => {
                "SELECT id, broker_account_id, trade_date, symbol, trade_type, gross_pnl_paisa, total_charges_paisa, net_pnl_paisa, order_id, contract_note_id
                 FROM trades WHERE broker_account_id = ?1 ORDER BY trade_date DESC;"
            }
            None => {
                "SELECT id, broker_account_id, trade_date, symbol, trade_type, gross_pnl_paisa, total_charges_paisa, net_pnl_paisa, order_id, contract_note_id
                 FROM trades ORDER BY trade_date DESC;"
            }
        };

        let mut stmt = self.conn.prepare(sql)?;
        let mut rows = match broker_account_id {
            Some(bid) => stmt.query(params![bid.to_string()])?,
            None => stmt.query([])?,
        };

        let mut trades = Vec::new();
        while let Some(row) = rows.next()? {
            let id_str: String = row.get(0)?;
            let broker_str: String = row.get(1)?;
            let date_str: String = row.get(2)?;
            let symbol: String = row.get(3)?;
            let type_str: String = row.get(4)?;
            let gross_paisa: i64 = row.get(5)?;
            let charges_paisa: i64 = row.get(6)?;
            let net_paisa: i64 = row.get(7)?;
            let order_id: Option<String> = row.get(8)?;
            let contract_note_id: Option<String> = row.get(9)?;

            let id = Uuid::parse_str(&id_str).map_err(|_| rusqlite::Error::InvalidQuery)?;
            let broker_id: AccountId = broker_str
                .parse()
                .map_err(|_| rusqlite::Error::InvalidQuery)?;
            let trade_date = DateTime::parse_from_rfc3339(&date_str)
                .map_err(|_| rusqlite::Error::InvalidQuery)?
                .with_timezone(&Utc);

            trades.push(TradeRecord {
                id,
                broker_account_id: broker_id,
                trade_date,
                symbol,
                trade_type: parse_trade_type(&type_str),
                gross_pnl: Money::from_paisa(gross_paisa),
                charges: TradeCharges {
                    brokerage: Money::from_paisa(charges_paisa),
                    ..Default::default()
                },
                net_pnl: Money::from_paisa(net_paisa),
                order_id,
                contract_note_id: contract_note_id.and_then(|s| Uuid::parse_str(&s).ok()),
            });
        }

        Ok(trades)
    }

    /// Compute aggregate trading performance metrics.
    pub fn get_trading_summary(
        &self,
        broker_account_id: Option<AccountId>,
    ) -> Result<TradingSummary, DbError> {
        let trades = self.list_trades(broker_account_id)?;

        let total_trades = trades.len();
        if total_trades == 0 {
            return Ok(TradingSummary {
                total_trades: 0,
                winning_trades: 0,
                losing_trades: 0,
                win_rate_basis_points: 0,
                gross_profit_paisa: 0,
                gross_loss_paisa: 0,
                total_charges_paisa: 0,
                net_pnl_paisa: 0,
                profit_factor_scaled: 0,
                max_drawdown_paisa: 0,
            });
        }

        let mut winning = 0;
        let mut losing = 0;
        let mut gross_profit = 0i64;
        let mut gross_loss = 0i64;
        let mut total_charges = 0i64;
        let mut net_pnl = 0i64;

        let mut peak = 0i64;
        let mut cumulative = 0i64;
        let mut max_drawdown = 0i64;

        for t in trades.iter().rev() {
            let net = t.net_pnl.as_paisa();
            net_pnl += net;
            let charges = t.charges.total_charges().unwrap_or_default().as_paisa();
            total_charges += charges;

            if net > 0 {
                winning += 1;
                gross_profit += net;
            } else if net < 0 {
                losing += 1;
                gross_loss += net.abs();
            }

            cumulative += net;
            if cumulative > peak {
                peak = cumulative;
            }
            let dd = peak - cumulative;
            if dd > max_drawdown {
                max_drawdown = dd;
            }
        }

        let win_rate = if total_trades > 0 {
            ((winning as u64 * 10000) / total_trades as u64) as u32
        } else {
            0
        };

        let profit_factor = if gross_loss > 0 {
            (gross_profit as u64 * 100) / gross_loss as u64
        } else if gross_profit > 0 {
            9999 // Infinite / zero-loss flag
        } else {
            0
        };

        Ok(TradingSummary {
            total_trades,
            winning_trades: winning,
            losing_trades: losing,
            win_rate_basis_points: win_rate,
            gross_profit_paisa: gross_profit,
            gross_loss_paisa: gross_loss,
            total_charges_paisa: total_charges,
            net_pnl_paisa: net_pnl,
            profit_factor_scaled: profit_factor,
            max_drawdown_paisa: max_drawdown,
        })
    }

    // =========================================================================
    // PAYOUT REPOSITORY
    // =========================================================================

    /// Record a broker payout / bank transfer.
    pub fn record_payout(&self, payout: &PayoutRecord) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT INTO payouts (id, broker_account_id, destination_bank_account_id, request_date, settlement_date, requested_amount_paisa, tds_paisa, net_credited_paisa, utr_number, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10);",
            params![
                payout.id.to_string(),
                payout.broker_account_id.to_string(),
                payout.destination_bank_account_id.to_string(),
                payout.request_date.to_rfc3339(),
                payout.settlement_date.map(|d| d.to_rfc3339()),
                payout.requested_amount.as_paisa(),
                payout.tds_deducted.as_paisa(),
                payout.net_credited_amount.as_paisa(),
                payout.utr_number,
                format!("{:?}", payout.status),
            ],
        )?;
        Ok(())
    }

    /// List all payouts ordered newest to oldest.
    pub fn list_payouts(&self) -> Result<Vec<PayoutRecord>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, broker_account_id, destination_bank_account_id, request_date, settlement_date, requested_amount_paisa, tds_paisa, net_credited_paisa, utr_number, status
             FROM payouts ORDER BY request_date DESC;"
        )?;

        let rows = stmt.query_map([], |row| {
            let id_str: String = row.get(0)?;
            let broker_str: String = row.get(1)?;
            let dest_str: String = row.get(2)?;
            let req_str: String = row.get(3)?;
            let set_str: Option<String> = row.get(4)?;
            let req_paisa: i64 = row.get(5)?;
            let tds_paisa: i64 = row.get(6)?;
            let net_paisa: i64 = row.get(7)?;
            let utr_number: Option<String> = row.get(8)?;
            let status_str: String = row.get(9)?;

            let id = Uuid::parse_str(&id_str).map_err(|_| rusqlite::Error::InvalidQuery)?;
            let broker_id: AccountId = broker_str.parse().map_err(|_| rusqlite::Error::InvalidQuery)?;
            let dest_id: AccountId = dest_str.parse().map_err(|_| rusqlite::Error::InvalidQuery)?;
            let request_date = DateTime::parse_from_rfc3339(&req_str)
                .map_err(|_| rusqlite::Error::InvalidQuery)?
                .with_timezone(&Utc);
            let settlement_date = set_str.and_then(|s| {
                DateTime::parse_from_rfc3339(&s).ok().map(|d| d.with_timezone(&Utc))
            });

            Ok(PayoutRecord {
                id,
                broker_account_id: broker_id,
                destination_bank_account_id: dest_id,
                request_date,
                settlement_date,
                requested_amount: Money::from_paisa(req_paisa),
                tds_deducted: Money::from_paisa(tds_paisa),
                net_credited_amount: Money::from_paisa(net_paisa),
                utr_number,
                status: parse_payout_status(&status_str),
            })
        })?;

        let mut payouts = Vec::new();
        for p in rows {
            payouts.push(p?);
        }
        Ok(payouts)
    }

    // =========================================================================
    // AUDIT LOG REPOSITORY
    // =========================================================================

    /// Append a new cryptographically chained audit event.
    pub fn append_audit_event(&self, event: &AuditEvent) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT INTO audit_events (sequence, timestamp, event_type, entity_id, payload_json, prev_hash, hash)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7);",
            params![
                event.sequence as i64,
                event.timestamp.to_rfc3339(),
                format!("{:?}", event.event_type),
                event.entity_id,
                event.payload_json,
                event.prev_hash.as_slice(),
                event.hash.as_slice(),
            ],
        )?;
        Ok(())
    }

    /// List all audit events ordered by sequence.
    pub fn list_audit_events(&self) -> Result<Vec<AuditEvent>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT sequence, timestamp, event_type, entity_id, payload_json, prev_hash, hash
             FROM audit_events ORDER BY sequence ASC;",
        )?;

        let rows = stmt.query_map([], |row| {
            let seq: i64 = row.get(0)?;
            let ts_str: String = row.get(1)?;
            let type_str: String = row.get(2)?;
            let entity_id: String = row.get(3)?;
            let payload_json: String = row.get(4)?;
            let prev_hash_vec: Vec<u8> = row.get(5)?;
            let hash_vec: Vec<u8> = row.get(6)?;

            let timestamp = DateTime::parse_from_rfc3339(&ts_str)
                .map_err(|_| rusqlite::Error::InvalidQuery)?
                .with_timezone(&Utc);

            let mut prev_hash = [0u8; 32];
            let mut hash = [0u8; 32];
            if prev_hash_vec.len() == 32 {
                prev_hash.copy_from_slice(&prev_hash_vec);
            }
            if hash_vec.len() == 32 {
                hash.copy_from_slice(&hash_vec);
            }

            Ok(AuditEvent {
                sequence: seq as u64,
                timestamp,
                event_type: parse_audit_type(&type_str),
                entity_id,
                payload_json,
                prev_hash,
                hash,
            })
        })?;

        let mut events = Vec::new();
        for r in rows {
            events.push(r?);
        }
        Ok(events)
    }

    // =========================================================================
    // VAULT DOCUMENT REPOSITORY
    // =========================================================================

    /// Store a sealed document record with its encrypted ciphertext and envelope.
    pub fn store_vault_document(&self, doc: &VaultDocument) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT INTO vault_documents (id, filename, mime_type, file_size_bytes, blake3_hash, sha256_hash, created_at, nonce, ciphertext)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9);",
            params![
                doc.id.to_string(),
                doc.filename,
                format!("{:?}", doc.mime_type),
                doc.file_size_bytes as i64,
                doc.blake3_hash.as_slice(),
                doc.sha256_hash.as_slice(),
                doc.created_at.to_rfc3339(),
                doc.envelope.nonce.as_slice(),
                doc.envelope.ciphertext.as_slice(),
            ],
        )?;
        Ok(())
    }

    /// Retrieve a vault document by ID.
    pub fn get_vault_document(&self, id: Uuid) -> Result<Option<VaultDocument>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, filename, mime_type, file_size_bytes, blake3_hash, sha256_hash, created_at, nonce, ciphertext
             FROM vault_documents WHERE id = ?1;",
        )?;

        let doc = stmt
            .query_row(params![id.to_string()], |row| {
                let id_parsed = Uuid::parse_str(&row.get::<_, String>(0)?)
                    .map_err(|_| rusqlite::Error::InvalidQuery)?;
                let filename: String = row.get(1)?;
                let mime_str: String = row.get(2)?;
                let file_size: i64 = row.get(3)?;
                let b3_vec: Vec<u8> = row.get(4)?;
                let sha_vec: Vec<u8> = row.get(5)?;
                let created_str: String = row.get(6)?;
                let nonce_vec: Vec<u8> = row.get(7)?;
                let ciphertext: Vec<u8> = row.get(8)?;

                let created_at = DateTime::parse_from_rfc3339(&created_str)
                    .map_err(|_| rusqlite::Error::InvalidQuery)?
                    .with_timezone(&Utc);

                let mut blake3_hash = [0u8; 32];
                let mut sha256_hash = [0u8; 32];
                let mut nonce = [0u8; 12];

                if b3_vec.len() == 32 {
                    blake3_hash.copy_from_slice(&b3_vec);
                }
                if sha_vec.len() == 32 {
                    sha256_hash.copy_from_slice(&sha_vec);
                }
                if nonce_vec.len() == 12 {
                    nonce.copy_from_slice(&nonce_vec);
                }

                Ok(VaultDocument {
                    id: id_parsed,
                    filename,
                    mime_type: parse_mime_type(&mime_str),
                    file_size_bytes: file_size as usize,
                    blake3_hash,
                    sha256_hash,
                    created_at,
                    linked_transaction_ids: Vec::new(),
                    envelope: EncryptedEnvelope {
                        nonce,
                        ciphertext,
                        plaintext_blake3_hash: blake3_hash,
                    },
                })
            })
            .optional()?;

        Ok(doc)
    }

    /// List document metadata summaries without transferring ciphertexts.
    pub fn list_vault_documents(&self) -> Result<Vec<VaultDocumentSummary>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, filename, mime_type, file_size_bytes, blake3_hash, sha256_hash, created_at
             FROM vault_documents ORDER BY created_at DESC;",
        )?;

        let rows = stmt.query_map([], |row| {
            let id = Uuid::parse_str(&row.get::<_, String>(0)?)
                .map_err(|_| rusqlite::Error::InvalidQuery)?;
            let filename: String = row.get(1)?;
            let mime_str: String = row.get(2)?;
            let file_size: i64 = row.get(3)?;
            let b3_vec: Vec<u8> = row.get(4)?;
            let sha_vec: Vec<u8> = row.get(5)?;
            let created_str: String = row.get(6)?;

            let created_at = DateTime::parse_from_rfc3339(&created_str)
                .map_err(|_| rusqlite::Error::InvalidQuery)?
                .with_timezone(&Utc);

            let mut blake3_hash = [0u8; 32];
            let mut sha256_hash = [0u8; 32];
            if b3_vec.len() == 32 {
                blake3_hash.copy_from_slice(&b3_vec);
            }
            if sha_vec.len() == 32 {
                sha256_hash.copy_from_slice(&sha_vec);
            }

            Ok(VaultDocumentSummary {
                id,
                filename,
                mime_type: parse_mime_type(&mime_str),
                file_size_bytes: file_size as usize,
                blake3_hash,
                sha256_hash,
                created_at,
            })
        })?;

        let mut docs = Vec::new();
        for r in rows {
            docs.push(r?);
        }
        Ok(docs)
    }

    /// Delete a document from the vault.
    pub fn delete_vault_document(&self, id: Uuid) -> Result<bool, DbError> {
        let rows = self.conn.execute(
            "DELETE FROM vault_documents WHERE id = ?1;",
            params![id.to_string()],
        )?;
        Ok(rows > 0)
    }

    // =========================================================================
    // BUDGET REPOSITORY
    // =========================================================================

    /// Save or update a financial budget with its category allocations.
    pub fn save_budget(&self, budget: &Budget) -> Result<(), DbError> {
        let tx = self.conn.unchecked_transaction()?;

        tx.execute(
            "INSERT INTO budgets (id, name, period, start_date, end_date, notes)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                period = excluded.period,
                start_date = excluded.start_date,
                end_date = excluded.end_date,
                notes = excluded.notes;",
            params![
                budget.id.to_string(),
                budget.name,
                format!("{:?}", budget.period),
                budget.start_date.to_rfc3339(),
                budget.end_date.to_rfc3339(),
                budget.notes,
            ],
        )?;

        tx.execute(
            "DELETE FROM category_budgets WHERE budget_id = ?1;",
            params![budget.id.to_string()],
        )?;

        let mut cat_stmt = tx.prepare(
            "INSERT INTO category_budgets (budget_id, category, allocated_paisa, alert_threshold_pct)
             VALUES (?1, ?2, ?3, ?4);",
        )?;

        for cat in &budget.categories {
            cat_stmt.execute(params![
                budget.id.to_string(),
                cat.category,
                cat.allocated.as_paisa(),
                cat.alert_threshold_pct as i64,
            ])?;
        }
        drop(cat_stmt);

        tx.commit()?;
        Ok(())
    }

    /// Retrieve a budget by ID including all category allocations.
    pub fn get_budget(&self, id: Uuid) -> Result<Option<Budget>, DbError> {
        let mut b_stmt = self.conn.prepare(
            "SELECT id, name, period, start_date, end_date, notes
             FROM budgets WHERE id = ?1;",
        )?;

        let budget_meta = b_stmt
            .query_row(params![id.to_string()], |row| {
                let id_parsed = Uuid::parse_str(&row.get::<_, String>(0)?)
                    .map_err(|_| rusqlite::Error::InvalidQuery)?;
                let name: String = row.get(1)?;
                let period_str: String = row.get(2)?;
                let start_str: String = row.get(3)?;
                let end_str: String = row.get(4)?;
                let notes: Option<String> = row.get(5)?;

                let start_date = DateTime::parse_from_rfc3339(&start_str)
                    .map_err(|_| rusqlite::Error::InvalidQuery)?
                    .with_timezone(&Utc);
                let end_date = DateTime::parse_from_rfc3339(&end_str)
                    .map_err(|_| rusqlite::Error::InvalidQuery)?
                    .with_timezone(&Utc);

                Ok((id_parsed, name, parse_budget_period(&period_str), start_date, end_date, notes))
            })
            .optional()?;

        let (b_id, name, period, start_date, end_date, notes) = match budget_meta {
            Some(meta) => meta,
            None => return Ok(None),
        };

        let mut cat_stmt = self.conn.prepare(
            "SELECT category, allocated_paisa, alert_threshold_pct
             FROM category_budgets WHERE budget_id = ?1 ORDER BY category ASC;",
        )?;

        let cat_rows = cat_stmt.query_map(params![id.to_string()], |row| {
            let category: String = row.get(0)?;
            let allocated_paisa: i64 = row.get(1)?;
            let threshold: i64 = row.get(2)?;

            Ok(CategoryBudget {
                category,
                allocated: Money::from_paisa(allocated_paisa),
                alert_threshold_pct: threshold.clamp(1, 100) as u8,
            })
        })?;

        let mut categories = Vec::new();
        for cat in cat_rows {
            categories.push(cat?);
        }

        Ok(Some(Budget {
            id: b_id,
            name,
            period,
            start_date,
            end_date,
            categories,
            notes,
        }))
    }

    /// List all budgets ordered by start date descending.
    pub fn list_budgets(&self) -> Result<Vec<Budget>, DbError> {
        let mut stmt = self.conn.prepare("SELECT id FROM budgets ORDER BY start_date DESC;")?;
        let id_rows = stmt.query_map([], |row| {
            let id_str: String = row.get(0)?;
            Uuid::parse_str(&id_str).map_err(|_| rusqlite::Error::InvalidQuery)
        })?;

        let mut budgets = Vec::new();
        for id_res in id_rows {
            let id = id_res?;
            if let Some(b) = self.get_budget(id)? {
                budgets.push(b);
            }
        }
        Ok(budgets)
    }

    /// Aggregate realized net spending per category within a given date window.
    pub fn get_spending_by_category(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<HashMap<String, Money>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT 
                COALESCE(t.category, a.name) as cat,
                SUM(CASE WHEN p.direction = 'Debit' THEN p.amount_paisa ELSE -p.amount_paisa END) as net_expense
             FROM postings p
             JOIN transactions t ON p.transaction_id = t.id
             JOIN accounts a ON p.account_id = a.id
             WHERE a.account_type = 'Expense'
               AND t.timestamp >= ?1 AND t.timestamp <= ?2
             GROUP BY cat;",
        )?;

        let rows = stmt.query_map(params![from.to_rfc3339(), to.to_rfc3339()], |row| {
            let cat: String = row.get(0)?;
            let amount: i64 = row.get(1)?;
            Ok((cat, Money::from_paisa(amount.max(0))))
        })?;

        let mut map = HashMap::new();
        for r in rows {
            let (cat, money) = r?;
            map.insert(cat, money);
        }
        Ok(map)
    }

    /// Evaluate budget execution and spending utilization for a given budget.
    pub fn evaluate_budget(&self, id: Uuid) -> Result<Option<BudgetReport>, DbError> {
        let budget = match self.get_budget(id)? {
            Some(b) => b,
            None => return Ok(None),
        };

        let spending = self.get_spending_by_category(budget.start_date, budget.end_date)?;
        let report = budget.evaluate_spending(&spending)?;
        Ok(Some(report))
    }

    // =========================================================================
    // FINANCIAL REPORTING ENGINE
    // =========================================================================

    /// Generate an audited point-in-time Balance Sheet statement.
    pub fn generate_balance_sheet(&self, as_of: DateTime<Utc>) -> Result<BalanceSheet, DbError> {
        let accounts = self.list_accounts()?;
        let as_of_str = as_of.to_rfc3339();

        let mut stmt = self.conn.prepare(
            "SELECT 
                p.direction,
                p.amount_paisa
             FROM postings p
             JOIN transactions t ON p.transaction_id = t.id
             WHERE p.account_id = ?1 AND t.timestamp <= ?2;",
        )?;

        let mut assets = Vec::new();
        let mut liabilities = Vec::new();
        let mut equity = Vec::new();
        let mut total_income = Money::ZERO;
        let mut total_expense = Money::ZERO;

        for acc in accounts {
            let mut debit = Money::ZERO;
            let mut credit = Money::ZERO;

            let mut rows = stmt.query(params![acc.id.to_string(), as_of_str])?;
            while let Some(row) = rows.next()? {
                let dir: String = row.get(0)?;
                let amount = Money::from_paisa(row.get(1)?);
                if dir == "Debit" {
                    debit = debit.checked_add(amount)?;
                } else {
                    credit = credit.checked_add(amount)?;
                }
            }

            match acc.account_type {
                AccountType::Asset => {
                    let balance = debit.checked_sub(credit)?;
                    assets.push(AccountSummaryItem {
                        account_id: acc.id,
                        name: acc.name,
                        subtype: acc.subtype,
                        balance,
                    });
                }
                AccountType::Liability => {
                    let balance = credit.checked_sub(debit)?;
                    liabilities.push(AccountSummaryItem {
                        account_id: acc.id,
                        name: acc.name,
                        subtype: acc.subtype,
                        balance,
                    });
                }
                AccountType::Equity => {
                    let balance = credit.checked_sub(debit)?;
                    equity.push(AccountSummaryItem {
                        account_id: acc.id,
                        name: acc.name,
                        subtype: acc.subtype,
                        balance,
                    });
                }
                AccountType::Income => {
                    let balance = credit.checked_sub(debit)?;
                    total_income = total_income.checked_add(balance)?;
                }
                AccountType::Expense => {
                    let balance = debit.checked_sub(credit)?;
                    total_expense = total_expense.checked_add(balance)?;
                }
            }
        }

        // Retained earnings from cumulative double-entry operating ledger
        let retained_earnings = total_income.checked_sub(total_expense)?;

        let sheet = BalanceSheet::new(as_of, assets, liabilities, equity, retained_earnings)?;
        Ok(sheet)
    }

    /// Generate an Income Statement (Profit & Loss) for a specified period.
    pub fn generate_income_statement(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<IncomeStatement, DbError> {
        let accounts = self.list_accounts()?;
        let from_str = from.to_rfc3339();
        let to_str = to.to_rfc3339();

        let mut stmt = self.conn.prepare(
            "SELECT 
                p.direction,
                p.amount_paisa
             FROM postings p
             JOIN transactions t ON p.transaction_id = t.id
             WHERE p.account_id = ?1 AND t.timestamp >= ?2 AND t.timestamp <= ?3;",
        )?;

        let mut operating_income = Vec::new();
        let mut operating_expenses = Vec::new();

        for acc in accounts {
            if acc.account_type != AccountType::Income && acc.account_type != AccountType::Expense {
                continue;
            }

            let mut debit = Money::ZERO;
            let mut credit = Money::ZERO;

            let mut rows = stmt.query(params![acc.id.to_string(), from_str, to_str])?;
            while let Some(row) = rows.next()? {
                let dir: String = row.get(0)?;
                let amount = Money::from_paisa(row.get(1)?);
                if dir == "Debit" {
                    debit = debit.checked_add(amount)?;
                } else {
                    credit = credit.checked_add(amount)?;
                }
            }

            if acc.account_type == AccountType::Income {
                let balance = credit.checked_sub(debit)?;
                operating_income.push(AccountSummaryItem {
                    account_id: acc.id,
                    name: acc.name,
                    subtype: acc.subtype,
                    balance,
                });
            } else {
                let balance = debit.checked_sub(credit)?;
                operating_expenses.push(AccountSummaryItem {
                    account_id: acc.id,
                    name: acc.name,
                    subtype: acc.subtype,
                    balance,
                });
            }
        }

        // Trading performance between from and to
        let mut trade_stmt = self.conn.prepare(
            "SELECT COALESCE(SUM(gross_pnl_paisa), 0), COALESCE(SUM(total_charges_paisa), 0)
             FROM trades WHERE trade_date >= ?1 AND trade_date <= ?2;",
        )?;

        let (trading_gross_paisa, trading_charges_paisa): (i64, i64) = trade_stmt
            .query_row(params![from_str, to_str], |r| Ok((r.get(0)?, r.get(1)?)))?;

        let statement = IncomeStatement::new(
            from,
            to,
            operating_income,
            Money::from_paisa(trading_gross_paisa),
            Money::from_paisa(trading_charges_paisa),
            operating_expenses,
        )?;

        Ok(statement)
    }

    /// Generate a Cash Flow Statement tracking liquid funds across bank and cash wallets.
    pub fn generate_cash_flow_statement(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<CashFlowStatement, DbError> {
        let accounts = self.list_accounts()?;
        let from_str = from.to_rfc3339();
        let to_str = to.to_rfc3339();

        let mut beginning_cash = Money::ZERO;
        let mut ending_cash = Money::ZERO;

        let mut before_stmt = self.conn.prepare(
            "SELECT p.direction, p.amount_paisa
             FROM postings p
             JOIN transactions t ON p.transaction_id = t.id
             WHERE p.account_id = ?1 AND t.timestamp < ?2;",
        )?;

        let mut interval_stmt = self.conn.prepare(
            "SELECT p.direction, p.amount_paisa
             FROM postings p
             JOIN transactions t ON p.transaction_id = t.id
             WHERE p.account_id = ?1 AND t.timestamp >= ?2 AND t.timestamp <= ?3;",
        )?;

        let mut investing_cash_flow = Money::ZERO;
        let mut financing_cash_flow = Money::ZERO;

        for acc in accounts {
            let is_liquid_cash = matches!(
                acc.subtype,
                AccountSubtype::BankAccount | AccountSubtype::UPIWallet | AccountSubtype::CashWallet
            );

            if is_liquid_cash {
                // Balance before 'from'
                let mut b_debit = Money::ZERO;
                let mut b_credit = Money::ZERO;
                let mut b_rows = before_stmt.query(params![acc.id.to_string(), from_str])?;
                while let Some(row) = b_rows.next()? {
                    let dir: String = row.get(0)?;
                    let amount = Money::from_paisa(row.get(1)?);
                    if dir == "Debit" {
                        b_debit = b_debit.checked_add(amount)?;
                    } else {
                        b_credit = b_credit.checked_add(amount)?;
                    }
                }
                let beg_bal = b_debit.checked_sub(b_credit)?;
                beginning_cash = beginning_cash.checked_add(beg_bal)?;

                // Balance between 'from' and 'to'
                let mut i_debit = Money::ZERO;
                let mut i_credit = Money::ZERO;
                let mut i_rows = interval_stmt.query(params![acc.id.to_string(), from_str, to_str])?;
                while let Some(row) = i_rows.next()? {
                    let dir: String = row.get(0)?;
                    let amount = Money::from_paisa(row.get(1)?);
                    if dir == "Debit" {
                        i_debit = i_debit.checked_add(amount)?;
                    } else {
                        i_credit = i_credit.checked_add(amount)?;
                    }
                }
                let interval_bal = i_debit.checked_sub(i_credit)?;
                ending_cash = ending_cash.checked_add(beg_bal.checked_add(interval_bal)?)?;
            }
        }

        // Net change in liquid cash
        let net_cash_change = ending_cash.checked_sub(beginning_cash)?;

        // Operating activities: net income
        let pnl = self.generate_income_statement(from, to)?;
        let operating_cash_flow = pnl.net_profit;

        // Balance of financing and investing
        let remainder = net_cash_change.checked_sub(operating_cash_flow)?;
        if remainder.is_positive() {
            financing_cash_flow = remainder;
        } else {
            investing_cash_flow = remainder;
        }

        let statement = CashFlowStatement::new(
            from,
            to,
            operating_cash_flow,
            investing_cash_flow,
            financing_cash_flow,
            beginning_cash,
        )?;

        Ok(statement)
    }
}

// =============================================================================
// PARSING HELPERS
// =============================================================================

fn parse_account_type(s: &str) -> AccountType {
    match s {
        "Asset" => AccountType::Asset,
        "Liability" => AccountType::Liability,
        "Equity" => AccountType::Equity,
        "Income" => AccountType::Income,
        _ => AccountType::Expense,
    }
}

fn parse_account_subtype(s: &str) -> AccountSubtype {
    match s {
        "BankAccount" => AccountSubtype::BankAccount,
        "UPIWallet" => AccountSubtype::UPIWallet,
        "CashWallet" => AccountSubtype::CashWallet,
        "BrokerAccount" => AccountSubtype::BrokerAccount,
        "InvestmentAccount" => AccountSubtype::InvestmentAccount,
        "CreditCard" => AccountSubtype::CreditCard,
        "PersonalLoan" => AccountSubtype::PersonalLoan,
        "Payable" => AccountSubtype::Payable,
        "OwnerCapital" => AccountSubtype::OwnerCapital,
        "RetainedEarnings" => AccountSubtype::RetainedEarnings,
        "OpeningBalanceEquity" => AccountSubtype::OpeningBalanceEquity,
        "Salary" => AccountSubtype::Salary,
        "TradingProfit" => AccountSubtype::TradingProfit,
        "BusinessRevenue" => AccountSubtype::BusinessRevenue,
        "Dividend" => AccountSubtype::Dividend,
        "InterestIncome" => AccountSubtype::InterestIncome,
        "TradingLoss" => AccountSubtype::TradingLoss,
        "BrokerageAndCharges" => AccountSubtype::BrokerageAndCharges,
        "Taxes" => AccountSubtype::Taxes,
        "LivingExpense" => AccountSubtype::LivingExpense,
        _ => AccountSubtype::OtherExpense,
    }
}

fn parse_account_status(s: &str) -> AccountStatus {
    match s {
        "Archived" => AccountStatus::Archived,
        "Frozen" => AccountStatus::Frozen,
        _ => AccountStatus::Active,
    }
}

fn parse_tx_type(s: &str) -> TransactionType {
    match s {
        "Income" => TransactionType::Income,
        "Expense" => TransactionType::Expense,
        "Transfer" => TransactionType::Transfer,
        "Reversal" => TransactionType::Reversal,
        "Adjustment" => TransactionType::Adjustment,
        "Deposit" => TransactionType::Deposit,
        "Withdrawal" => TransactionType::Withdrawal,
        "Trade" => TransactionType::Trade,
        _ => TransactionType::Expense,
    }
}

fn parse_payment_method(s: &str) -> bhoot_core::PaymentMethod {
    match s {
        "UPI" => bhoot_core::PaymentMethod::UPI,
        "NetBanking" => bhoot_core::PaymentMethod::NetBanking,
        "DebitCard" => bhoot_core::PaymentMethod::DebitCard,
        "CreditCard" => bhoot_core::PaymentMethod::CreditCard,
        "Cash" => bhoot_core::PaymentMethod::Cash,
        "Cheque" => bhoot_core::PaymentMethod::Cheque,
        "NEFT" => bhoot_core::PaymentMethod::NEFT,
        "IMPS" => bhoot_core::PaymentMethod::IMPS,
        "RTGS" => bhoot_core::PaymentMethod::RTGS,
        "BrokerInternal" => bhoot_core::PaymentMethod::BrokerInternal,
        _ => bhoot_core::PaymentMethod::Other,
    }
}

fn parse_vstatus(s: &str) -> VerificationStatus {
    match s {
        "Verified" => VerificationStatus::Verified,
        "Flagged" => VerificationStatus::Flagged,
        "Reconciled" => VerificationStatus::Reconciled,
        _ => VerificationStatus::Pending,
    }
}

fn parse_trade_type(s: &str) -> bhoot_core::TradeType {
    match s {
        "IntradayEquity" => bhoot_core::TradeType::IntradayEquity,
        "DeliveryEquity" => bhoot_core::TradeType::DeliveryEquity,
        "Futures" => bhoot_core::TradeType::Futures,
        "Options" => bhoot_core::TradeType::Options,
        "Commodity" => bhoot_core::TradeType::Commodity,
        _ => bhoot_core::TradeType::Currency,
    }
}

fn parse_payout_status(s: &str) -> PayoutStatus {
    match s {
        "Settled" => PayoutStatus::Settled,
        "Rejected" => PayoutStatus::Rejected,
        _ => PayoutStatus::Requested,
    }
}

fn parse_audit_type(s: &str) -> AuditEventType {
    match s {
        "AccountCreated" => AuditEventType::AccountCreated,
        "AccountStatusChanged" => AuditEventType::AccountStatusChanged,
        "TransactionPosted" => AuditEventType::TransactionPosted,
        "TransactionReversed" => AuditEventType::TransactionReversed,
        "VaultDocumentStored" => AuditEventType::VaultDocumentStored,
        "VaultDocumentDeleted" => AuditEventType::VaultDocumentDeleted,
        "SystemLocked" => AuditEventType::SystemLocked,
        "SystemUnlocked" => AuditEventType::SystemUnlocked,
        "BackupExported" => AuditEventType::BackupExported,
        _ => AuditEventType::IntegrityCheckRun,
    }
}

fn parse_mime_type(s: &str) -> DocumentMimeType {
    match s {
        "Png" => DocumentMimeType::Png,
        "Jpeg" => DocumentMimeType::Jpeg,
        _ => DocumentMimeType::Pdf,
    }
}

fn parse_budget_period(s: &str) -> BudgetPeriod {
    match s {
        "Monthly" => BudgetPeriod::Monthly,
        "Quarterly" => BudgetPeriod::Quarterly,
        "Annual" => BudgetPeriod::Annual,
        _ => BudgetPeriod::Custom,
    }
}
