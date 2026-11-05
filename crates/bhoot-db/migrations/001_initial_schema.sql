-- BHOOT SQLCipher Schema Migration 001: Initial Core Ledger Schema
-- Enforces strict foreign key integrity, non-null constraints, and integer Paisa representation.

PRAGMA foreign_keys = ON;

-- Accounts Table
CREATE TABLE IF NOT EXISTS accounts (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    account_type TEXT NOT NULL,
    subtype TEXT NOT NULL,
    status TEXT NOT NULL,
    currency TEXT NOT NULL DEFAULT 'INR',
    description TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_accounts_type ON accounts(account_type);
CREATE INDEX IF NOT EXISTS idx_accounts_status ON accounts(status);

-- Transactions Table
CREATE TABLE IF NOT EXISTS transactions (
    id TEXT PRIMARY KEY NOT NULL,
    timestamp TEXT NOT NULL,
    tx_type TEXT NOT NULL,
    category TEXT,
    payment_method TEXT,
    reference_number TEXT,
    notes TEXT,
    verification_status TEXT NOT NULL DEFAULT 'Pending',
    reversal_of TEXT UNIQUE,
    FOREIGN KEY (reversal_of) REFERENCES transactions(id) ON DELETE RESTRICT
);

CREATE INDEX IF NOT EXISTS idx_transactions_timestamp ON transactions(timestamp);
CREATE INDEX IF NOT EXISTS idx_transactions_type ON transactions(tx_type);
CREATE INDEX IF NOT EXISTS idx_transactions_reversal ON transactions(reversal_of);

-- Double-Entry Postings Table
CREATE TABLE IF NOT EXISTS postings (
    id TEXT PRIMARY KEY NOT NULL,
    transaction_id TEXT NOT NULL,
    account_id TEXT NOT NULL,
    amount_paisa INTEGER NOT NULL CHECK (amount_paisa > 0),
    direction TEXT NOT NULL CHECK (direction IN ('Debit', 'Credit')),
    memo TEXT,
    FOREIGN KEY (transaction_id) REFERENCES transactions(id) ON DELETE CASCADE,
    FOREIGN KEY (account_id) REFERENCES accounts(id) ON DELETE RESTRICT
);

CREATE INDEX IF NOT EXISTS idx_postings_tx ON postings(transaction_id);
CREATE INDEX IF NOT EXISTS idx_postings_acc ON postings(account_id);

-- Broker Trades Table
CREATE TABLE IF NOT EXISTS trades (
    id TEXT PRIMARY KEY NOT NULL,
    broker_account_id TEXT NOT NULL,
    trade_date TEXT NOT NULL,
    symbol TEXT NOT NULL,
    trade_type TEXT NOT NULL,
    gross_pnl_paisa INTEGER NOT NULL,
    total_charges_paisa INTEGER NOT NULL CHECK (total_charges_paisa >= 0),
    net_pnl_paisa INTEGER NOT NULL,
    order_id TEXT,
    contract_note_id TEXT,
    FOREIGN KEY (broker_account_id) REFERENCES accounts(id) ON DELETE RESTRICT
);

CREATE INDEX IF NOT EXISTS idx_trades_broker ON trades(broker_account_id);
CREATE INDEX IF NOT EXISTS idx_trades_date ON trades(trade_date);

-- Broker Payouts Table
CREATE TABLE IF NOT EXISTS payouts (
    id TEXT PRIMARY KEY NOT NULL,
    broker_account_id TEXT NOT NULL,
    destination_bank_account_id TEXT NOT NULL,
    request_date TEXT NOT NULL,
    settlement_date TEXT,
    requested_amount_paisa INTEGER NOT NULL CHECK (requested_amount_paisa > 0),
    tds_paisa INTEGER NOT NULL CHECK (tds_paisa >= 0),
    net_credited_paisa INTEGER NOT NULL,
    utr_number TEXT,
    status TEXT NOT NULL,
    FOREIGN KEY (broker_account_id) REFERENCES accounts(id) ON DELETE RESTRICT,
    FOREIGN KEY (destination_bank_account_id) REFERENCES accounts(id) ON DELETE RESTRICT
);

-- Append-Only Audit Trail Table
CREATE TABLE IF NOT EXISTS audit_events (
    sequence INTEGER PRIMARY KEY NOT NULL,
    timestamp TEXT NOT NULL,
    event_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    prev_hash BLOB NOT NULL,
    hash BLOB NOT NULL
);

-- Encrypted Document Vault Metadata Table
CREATE TABLE IF NOT EXISTS vault_documents (
    id TEXT PRIMARY KEY NOT NULL,
    filename TEXT NOT NULL,
    mime_type TEXT NOT NULL,
    file_size_bytes INTEGER NOT NULL,
    blake3_hash BLOB NOT NULL,
    sha256_hash BLOB NOT NULL,
    created_at TEXT NOT NULL,
    nonce BLOB NOT NULL,
    ciphertext BLOB NOT NULL
);

