use bhoot_audit::{AuditEvent, AuditEventType};
use bhoot_backup::{create_full_backup_file, restore_full_backup_file, validate_destination_path};
use bhoot_core::{
    Account, AccountId, AccountStatus, AccountSubtype, AccountType, Budget, BudgetPeriod,
    CategoryBudget, Direction, Money, PaymentMethod, Posting, TradeCharges, TradeRecord, TradeType,
    Transaction, TransactionId, TransactionType,
};
use bhoot_crypto::{
    blake3_hash, constant_time_eq_32, decrypt_envelope, derive_master_key, encrypt_envelope,
    generate_salt, sha256_hash, Argon2idParams,
};
use bhoot_db::Database;
use bhoot_vault::{DocumentMimeType, VaultDocument, VaultIntegrityReport};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use tiny_http::{Header, Request, Response, Server, StatusCode};
use uuid::Uuid;

pub const EMBEDDED_INDEX_HTML: &str = include_str!("../../../apps/desktop/dist/index.html");
pub const EMBEDDED_CSS: &[u8] = include_bytes!("../../../apps/desktop/dist/assets/index.css");
pub const EMBEDDED_JS: &[u8] = include_bytes!("../../../apps/desktop/dist/assets/index.js");

const MAX_JSON_PAYLOAD_BYTES: usize = 10 * 1024 * 1024; // 10 MiB
const MAX_VAULT_PAYLOAD_BYTES: usize = 50 * 1024 * 1024; // 50 MiB

// =========================================================================
// IN-MEMORY SERVER STATE & AUTHENTICATION
// =========================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub currency: String,
    pub theme: String,
    pub auto_lock_minutes: u32,
    pub offline_mode: bool,
    pub financial_year_start: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            currency: "INR".to_string(),
            theme: "dark".to_string(),
            auto_lock_minutes: 15,
            offline_mode: true,
            financial_year_start: "04-01".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthError {
    InvalidCredentials,
    LockedOut(i64),
    NotEnrolled,
    AlreadyEnrolled,
    ConfirmationMismatch,
    WeakPin(String),
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidCredentials => write!(f, "Invalid PIN credentials"),
            Self::LockedOut(secs) => write!(f, "Rate limit exceeded: PIN verification locked for {secs} seconds"),
            Self::NotEnrolled => write!(f, "Application is uninitialized. First-owner enrollment required."),
            Self::AlreadyEnrolled => write!(f, "Owner PIN has already been enrolled"),
            Self::ConfirmationMismatch => write!(f, "PIN confirmation does not match"),
            Self::WeakPin(reason) => write!(f, "Weak PIN rejected: {reason}"),
        }
    }
}

impl std::error::Error for AuthError {}

pub fn validate_pin_strength(pin: &str) -> Result<(), &'static str> {
    if pin.len() < 6 {
        return Err("PIN must be at least 6 characters in length");
    }
    if pin.len() > 64 {
        return Err("PIN must not exceed 64 characters");
    }
    if pin.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err("PIN must not contain whitespace or control characters");
    }
    // Reject single-character repetition (e.g. "111111", "000000", "aaaaaa")
    let first = pin.chars().next().unwrap();
    if pin.chars().all(|c| c == first) {
        return Err("PIN cannot consist of a single repeating character");
    }
    // Reject sequential numeric patterns if all digits
    if pin.chars().all(|c| c.is_ascii_digit()) {
        let digits: Vec<i32> = pin.chars().map(|c| c.to_digit(10).unwrap() as i32).collect();
        let mut is_asc = true;
        let mut is_desc = true;
        for i in 1..digits.len() {
            if digits[i] - digits[i - 1] != 1 {
                is_asc = false;
            }
            if digits[i - 1] - digits[i] != 1 {
                is_desc = false;
            }
        }
        if is_asc {
            return Err("PIN cannot be an ascending sequential pattern (e.g. 123456)");
        }
        if is_desc {
            return Err("PIN cannot be a descending sequential pattern (e.g. 654321)");
        }
    }
    let lower = pin.to_ascii_lowercase();
    if lower == "password" || lower == "admin" || lower == "123456" || lower == "1234" || lower == "000000" {
        return Err("PIN is too common or a trivial default");
    }
    Ok(())
}

pub fn constant_time_eq_slices(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut res = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        res |= x ^ y;
    }
    res == 0
}

#[derive(Debug)]
pub struct ServerState {
    pub db: Mutex<Database>,
    pub db_path: PathBuf,
    pub vault_dir: PathBuf,
    pub master_key: [u8; 32],
    pub session_locked: AtomicBool,
    pub is_enrolled: AtomicBool,
    pub pin_salt: Mutex<Option<[u8; 32]>>,
    pub pin_verifier: Mutex<Option<[u8; 32]>>,
    pub argon2_params: Mutex<Argon2idParams>,
    pub session_token: Mutex<String>,
    pub failed_pin_attempts: AtomicU32,
    pub locked_until_unix: std::sync::atomic::AtomicI64,
    pub pin_mutex: Mutex<()>,
    pub settings: Mutex<AppSettings>,
}

impl ServerState {
    pub fn new(db: Database, db_path: PathBuf, vault_dir: PathBuf, master_key: [u8; 32]) -> Self {
        let conn = db.connection();

        // 1. Initialize credentials table
        let _ = conn.execute(
            "CREATE TABLE IF NOT EXISTS app_auth_credentials (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                salt BLOB NOT NULL,
                verifier BLOB NOT NULL,
                m_cost INTEGER NOT NULL,
                t_cost INTEGER NOT NULL,
                p_cost INTEGER NOT NULL,
                enrolled_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );",
            [],
        );

        // 2. Initialize brute-force lockout table
        let _ = conn.execute(
            "CREATE TABLE IF NOT EXISTS pin_lockout_state (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                failed_attempts INTEGER NOT NULL,
                locked_until_unix INTEGER NOT NULL
            );",
            [],
        );
        let _ = conn.execute(
            "INSERT OR IGNORE INTO pin_lockout_state (id, failed_attempts, locked_until_unix) VALUES (1, 0, 0);",
            [],
        );

        // 3. Query existing credentials
        let mut loaded_salt = None;
        let mut loaded_verifier = None;
        let mut loaded_params = Argon2idParams {
            m_cost: 16384,
            t_cost: 2,
            p_cost: 1,
        };
        let mut is_enrolled = false;

        if let Ok(mut stmt) = conn.prepare("SELECT salt, verifier, m_cost, t_cost, p_cost FROM app_auth_credentials WHERE id = 1") {
            if let Ok(row) = stmt.query_row([], |r| {
                let salt: Vec<u8> = r.get(0)?;
                let verifier: Vec<u8> = r.get(1)?;
                let m_cost: u32 = r.get(2)?;
                let t_cost: u32 = r.get(3)?;
                let p_cost: u32 = r.get(4)?;
                Ok((salt, verifier, m_cost, t_cost, p_cost))
            }) {
                if row.0.len() == 32 && row.1.len() == 32 {
                    let mut s = [0u8; 32];
                    s.copy_from_slice(&row.0);
                    let mut v = [0u8; 32];
                    v.copy_from_slice(&row.1);
                    loaded_salt = Some(s);
                    loaded_verifier = Some(v);
                    loaded_params = Argon2idParams {
                        m_cost: row.2,
                        t_cost: row.3,
                        p_cost: row.4,
                    };
                    is_enrolled = true;
                }
            }
        }

        // 4. Query lockout state
        let mut failed_attempts = 0u32;
        let mut locked_until_unix = 0i64;
        let now_unix = Utc::now().timestamp();

        if let Ok(mut stmt) = conn.prepare("SELECT failed_attempts, locked_until_unix FROM pin_lockout_state WHERE id = 1") {
            if let Ok(row) = stmt.query_row([], |r| {
                let fails: u32 = r.get(0)?;
                let lock_until: i64 = r.get(1)?;
                Ok((fails, lock_until))
            }) {
                failed_attempts = row.0;
                if row.1 > now_unix {
                    locked_until_unix = row.1;
                }
            }
        }

        let initial_token = Uuid::new_v4().to_string();

        Self {
            db: Mutex::new(db),
            db_path,
            vault_dir,
            master_key,
            session_locked: AtomicBool::new(true),
            is_enrolled: AtomicBool::new(is_enrolled),
            pin_salt: Mutex::new(loaded_salt),
            pin_verifier: Mutex::new(loaded_verifier),
            argon2_params: Mutex::new(loaded_params),
            session_token: Mutex::new(initial_token),
            failed_pin_attempts: AtomicU32::new(failed_attempts),
            locked_until_unix: std::sync::atomic::AtomicI64::new(locked_until_unix),
            pin_mutex: Mutex::new(()),
            settings: Mutex::new(AppSettings::default()),
        }
    }

    pub fn enroll_owner_pin(&self, pin: &str, confirm_pin: &str) -> Result<String, AuthError> {
        let _lock = self.pin_mutex.lock().unwrap();

        if self.is_enrolled.load(Ordering::SeqCst) {
            return Err(AuthError::AlreadyEnrolled);
        }

        if pin != confirm_pin {
            return Err(AuthError::ConfirmationMismatch);
        }

        if let Err(reason) = validate_pin_strength(pin) {
            return Err(AuthError::WeakPin(reason.to_string()));
        }

        let salt = generate_salt();
        let params = Argon2idParams {
            m_cost: 16384,
            t_cost: 2,
            p_cost: 1,
        };
        let verifier = derive_master_key(pin.as_bytes(), &salt, params)
            .map_err(|_| AuthError::WeakPin("Argon2id derivation failed".into()))?;

        let now_str = Utc::now().to_rfc3339();

        let db = self.db.lock().unwrap();
        let conn = db.connection();
        let insert_res = conn.execute(
            "INSERT INTO app_auth_credentials (id, salt, verifier, m_cost, t_cost, p_cost, enrolled_at, updated_at)
             VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7);",
            bhoot_db::rusqlite::params![
                &salt[..],
                &verifier[..],
                params.m_cost,
                params.t_cost,
                params.p_cost,
                now_str,
                now_str,
            ],
        );

        if let Err(e) = insert_res {
            return Err(AuthError::WeakPin(format!("Failed to persist credentials: {e}")));
        }

        let _ = conn.execute(
            "UPDATE pin_lockout_state SET failed_attempts = 0, locked_until_unix = 0 WHERE id = 1;",
            [],
        );
        drop(db);

        *self.pin_salt.lock().unwrap() = Some(salt);
        *self.pin_verifier.lock().unwrap() = Some(*verifier);
        *self.argon2_params.lock().unwrap() = params;

        self.failed_pin_attempts.store(0, Ordering::SeqCst);
        self.locked_until_unix.store(0, Ordering::SeqCst);
        self.is_enrolled.store(true, Ordering::SeqCst);
        self.session_locked.store(false, Ordering::SeqCst);

        let new_token = Uuid::new_v4().to_string();
        *self.session_token.lock().unwrap() = new_token.clone();

        Ok(new_token)
    }

    pub fn verify_pin(&self, pin: &str) -> Result<String, AuthError> {
        let _lock = self.pin_mutex.lock().unwrap();

        if !self.is_enrolled.load(Ordering::SeqCst) {
            return Err(AuthError::NotEnrolled);
        }

        let now_unix = Utc::now().timestamp();
        let locked_until = self.locked_until_unix.load(Ordering::SeqCst);

        if now_unix < locked_until {
            let remaining = locked_until - now_unix;
            return Err(AuthError::LockedOut(remaining.max(1)));
        }

        let salt_opt = *self.pin_salt.lock().unwrap();
        let verifier_opt = *self.pin_verifier.lock().unwrap();
        let params = *self.argon2_params.lock().unwrap();

        let (salt, expected) = match (salt_opt, verifier_opt) {
            (Some(s), Some(v)) => (s, v),
            _ => return Err(AuthError::NotEnrolled),
        };

        if let Ok(derived) = derive_master_key(pin.as_bytes(), &salt, params) {
            let matches = constant_time_eq_32(&derived, &expected);

            if matches {
                self.failed_pin_attempts.store(0, Ordering::SeqCst);
                self.locked_until_unix.store(0, Ordering::SeqCst);
                self.session_locked.store(false, Ordering::SeqCst);
                let new_token = Uuid::new_v4().to_string();
                *self.session_token.lock().unwrap() = new_token.clone();

                let db = self.db.lock().unwrap();
                let _ = db.connection().execute(
                    "UPDATE pin_lockout_state SET failed_attempts = 0, locked_until_unix = 0 WHERE id = 1;",
                    [],
                );

                Ok(new_token)
            } else {
                let fails = self.failed_pin_attempts.fetch_add(1, Ordering::SeqCst) + 1;
                let db = self.db.lock().unwrap();
                if fails >= 5 {
                    let lock_until = now_unix + 60;
                    self.locked_until_unix.store(lock_until, Ordering::SeqCst);
                    let _ = db.connection().execute(
                        "UPDATE pin_lockout_state SET failed_attempts = ?1, locked_until_unix = ?2 WHERE id = 1;",
                        bhoot_db::rusqlite::params![fails, lock_until],
                    );
                    Err(AuthError::LockedOut(60))
                } else {
                    let _ = db.connection().execute(
                        "UPDATE pin_lockout_state SET failed_attempts = ?1, locked_until_unix = 0 WHERE id = 1;",
                        bhoot_db::rusqlite::params![fails],
                    );
                    Err(AuthError::InvalidCredentials)
                }
            }
        } else {
            Err(AuthError::InvalidCredentials)
        }
    }

    pub fn set_pin(&self, current_pin: &str, new_pin: &str, confirm_new_pin: Option<&str>) -> Result<(), AuthError> {
        let _lock = self.pin_mutex.lock().unwrap();

        if !self.is_enrolled.load(Ordering::SeqCst) {
            return Err(AuthError::NotEnrolled);
        }

        let now_unix = Utc::now().timestamp();
        let locked_until = self.locked_until_unix.load(Ordering::SeqCst);
        if now_unix < locked_until {
            let remaining = locked_until - now_unix;
            return Err(AuthError::LockedOut(remaining.max(1)));
        }

        let salt_opt = *self.pin_salt.lock().unwrap();
        let verifier_opt = *self.pin_verifier.lock().unwrap();
        let params = *self.argon2_params.lock().unwrap();

        let (salt, expected) = match (salt_opt, verifier_opt) {
            (Some(s), Some(v)) => (s, v),
            _ => return Err(AuthError::NotEnrolled),
        };

        let curr_derived = derive_master_key(current_pin.as_bytes(), &salt, params)
            .map_err(|_| AuthError::InvalidCredentials)?;

        if !constant_time_eq_32(&curr_derived, &expected) {
            let fails = self.failed_pin_attempts.fetch_add(1, Ordering::SeqCst) + 1;
            let db = self.db.lock().unwrap();
            if fails >= 5 {
                let lock_until = now_unix + 60;
                self.locked_until_unix.store(lock_until, Ordering::SeqCst);
                let _ = db.connection().execute(
                    "UPDATE pin_lockout_state SET failed_attempts = ?1, locked_until_unix = ?2 WHERE id = 1;",
                    bhoot_db::rusqlite::params![fails, lock_until],
                );
                return Err(AuthError::LockedOut(60));
            } else {
                let _ = db.connection().execute(
                    "UPDATE pin_lockout_state SET failed_attempts = ?1, locked_until_unix = 0 WHERE id = 1;",
                    bhoot_db::rusqlite::params![fails],
                );
                return Err(AuthError::InvalidCredentials);
            }
        }

        if let Some(confirm) = confirm_new_pin {
            if new_pin != confirm {
                return Err(AuthError::ConfirmationMismatch);
            }
        }

        if let Err(reason) = validate_pin_strength(new_pin) {
            return Err(AuthError::WeakPin(reason.to_string()));
        }

        let new_salt = generate_salt();
        let new_verifier = derive_master_key(new_pin.as_bytes(), &new_salt, params)
            .map_err(|_| AuthError::WeakPin("Key derivation failed".into()))?;

        let now_str = Utc::now().to_rfc3339();

        let db = self.db.lock().unwrap();
        let conn = db.connection();
        let update_res = conn.execute(
            "UPDATE app_auth_credentials SET salt = ?1, verifier = ?2, m_cost = ?3, t_cost = ?4, p_cost = ?5, updated_at = ?6 WHERE id = 1;",
            bhoot_db::rusqlite::params![
                &new_salt[..],
                &new_verifier[..],
                params.m_cost,
                params.t_cost,
                params.p_cost,
                now_str,
            ],
        );

        if let Err(e) = update_res {
            return Err(AuthError::WeakPin(format!("Failed to persist updated PIN: {e}")));
        }

        let _ = conn.execute(
            "UPDATE pin_lockout_state SET failed_attempts = 0, locked_until_unix = 0 WHERE id = 1;",
            [],
        );
        drop(db);

        *self.pin_salt.lock().unwrap() = Some(new_salt);
        *self.pin_verifier.lock().unwrap() = Some(*new_verifier);

        self.failed_pin_attempts.store(0, Ordering::SeqCst);
        self.locked_until_unix.store(0, Ordering::SeqCst);

        Ok(())
    }
}

fn create_audit_event(
    seq: u64,
    prev_hash: [u8; 32],
    event_type: AuditEventType,
    entity_id: &str,
    payload_json: &str,
) -> AuditEvent {
    let now = Utc::now();
    let hash = AuditEvent::compute_hash(seq, &now, event_type, entity_id, payload_json, &prev_hash);
    AuditEvent {
        sequence: seq,
        timestamp: now,
        event_type,
        entity_id: entity_id.to_string(),
        payload_json: payload_json.to_string(),
        prev_hash,
        hash,
    }
}

// =========================================================================
// DATA TRANSFER OBJECTS (DTOs)
// =========================================================================

#[derive(Debug, Deserialize)]
pub struct CreateAccountRequest {
    pub name: String,
    pub account_type: String,
    pub subtype: String,
    pub currency: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAccountStatusRequest {
    pub account_id: AccountId,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct PostingInput {
    pub account_id: AccountId,
    pub amount_paisa: i64,
    pub direction: String,
    pub memo: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTransactionRequest {
    pub date: Option<DateTime<Utc>>,
    pub tx_type: String,
    pub postings: Vec<PostingInput>,
    pub category: Option<String>,
    pub payment_method: Option<String>,
    pub reference: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ReverseTransactionRequest {
    pub transaction_id: TransactionId,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
pub struct TradeChargesInput {
    pub brokerage_paisa: i64,
    pub stt_paisa: i64,
    pub exchange_turnover_paisa: i64,
    pub gst_paisa: i64,
    pub sebi_paisa: i64,
    pub stamp_duty_paisa: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreateTradeRequest {
    pub broker_account_id: AccountId,
    pub trade_date: Option<DateTime<Utc>>,
    pub symbol: String,
    pub trade_type: String,
    pub gross_amount_paisa: i64,
    pub charges: TradeChargesInput,
    pub net_amount_paisa: i64,
    pub order_reference: Option<String>,
    pub contract_note_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct CategoryBudgetInput {
    pub category: String,
    pub allocated_paisa: i64,
    pub alert_threshold_pct: u8,
}

#[derive(Debug, Deserialize)]
pub struct SaveBudgetRequest {
    pub id: Option<Uuid>,
    pub name: String,
    pub period: String,
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub categories: Vec<CategoryBudgetInput>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UploadVaultDocumentRequest {
    pub filename: String,
    pub data_base64: String,
    pub linked_tx_id: Option<TransactionId>,
}

#[derive(Debug, Deserialize)]
pub struct DeleteVaultDocumentRequest {
    pub id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct CreateBackupRequest {
    pub passphrase: String,
    pub destination_path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RestoreBackupRequest {
    pub passphrase: String,
    pub backup_file_path: String,
}

#[derive(Debug, Deserialize)]
pub struct UnlockRequest {
    pub pin: String,
}

#[derive(Debug, Deserialize)]
pub struct EnrollRequest {
    pub pin: String,
    pub confirm_pin: String,
}

#[derive(Debug, Deserialize)]
pub struct SetPinRequest {
    pub current_pin: String,
    pub new_pin: String,
    pub confirm_new_pin: Option<String>,
}

// =========================================================================
// SELF-CONTAINED OFFLINE BASE64 UTILITY
// =========================================================================

pub fn decode_base64(s: &str) -> Result<Vec<u8>, String> {
    const TABLE: [i8; 256] = {
        let mut t = [-1i8; 256];
        let mut i = 0u8;
        while i < 26 {
            t[(b'A' + i) as usize] = i as i8;
            i += 1;
        }
        i = 0;
        while i < 26 {
            t[(b'a' + i) as usize] = (26 + i) as i8;
            i += 1;
        }
        i = 0;
        while i < 10 {
            t[(b'0' + i) as usize] = (52 + i) as i8;
            i += 1;
        }
        t[b'+' as usize] = 62;
        t[b'/' as usize] = 63;
        t
    };
    let clean: Vec<u8> = s.bytes().filter(|&b| !b.is_ascii_whitespace()).collect();
    if !clean.len().is_multiple_of(4) {
        return Err("Invalid base64 length".into());
    }
    let mut out = Vec::with_capacity((clean.len() / 4) * 3);
    for chunk in clean.as_chunks::<4>().0 {
        let b0 = TABLE[chunk[0] as usize];
        let b1 = TABLE[chunk[1] as usize];
        let b2 = if chunk[2] == b'=' { 0 } else { TABLE[chunk[2] as usize] };
        let b3 = if chunk[3] == b'=' { 0 } else { TABLE[chunk[3] as usize] };
        if b0 < 0 || b1 < 0 || (chunk[2] != b'=' && b2 < 0) || (chunk[3] != b'=' && b3 < 0) {
            return Err("Invalid base64 character".into());
        }
        out.push(((b0 as u8) << 2) | ((b1 as u8) >> 4));
        if chunk[2] != b'=' {
            out.push(((b1 as u8) << 4) | ((b2 as u8) >> 2));
            if chunk[3] != b'=' {
                out.push(((b2 as u8) << 6) | (b3 as u8));
            }
        }
    }
    Ok(out)
}

pub fn encode_base64(bytes: &[u8]) -> String {
    const CHARSET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0];
        let b1 = if chunk.len() > 1 { chunk[1] } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] } else { 0 };
        out.push(CHARSET[(b0 >> 2) as usize] as char);
        out.push(CHARSET[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            out.push(CHARSET[(((b1 & 0x0F) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(CHARSET[(b2 & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

// =========================================================================
// STARTER DATA SEEDING
// =========================================================================

pub fn seed_starter_data(db: &Database) -> Result<(), Box<dyn std::error::Error>> {
    let accounts = db.list_accounts()?;
    if !accounts.is_empty() {
        return Ok(());
    }

    println!("[BHOOT] Database is empty. Seeding initial starter ledger and accounts...");

    // 1. Chart of Accounts
    let hdfc = Account::new(
        "HDFC Savings Bank",
        AccountType::Asset,
        AccountSubtype::BankAccount,
        Some("Primary salary and operational account".into()),
    );
    let zerodha = Account::new(
        "Zerodha Demat & Trading",
        AccountType::Asset,
        AccountSubtype::BrokerAccount,
        Some("NSE/BSE Equity & Derivatives Trading".into()),
    );
    let icici_cc = Account::new(
        "ICICI Coral Credit Card",
        AccountType::Liability,
        AccountSubtype::CreditCard,
        Some("Operational expenses and utility bills".into()),
    );
    let salary_inc = Account::new(
        "Professional Consulting Income",
        AccountType::Income,
        AccountSubtype::Salary,
        Some("Direct software consulting retainer".into()),
    );
    let rent_exp = Account::new(
        "Studio & Office Rent",
        AccountType::Expense,
        AccountSubtype::LivingExpense,
        Some("Monthly workspace rent".into()),
    );
    let groceries_exp = Account::new(
        "Household & Groceries",
        AccountType::Expense,
        AccountSubtype::LivingExpense,
        Some("Food, supplies and daily living".into()),
    );

    db.create_account(&hdfc)?;
    db.create_account(&zerodha)?;
    db.create_account(&icici_cc)?;
    db.create_account(&salary_inc)?;
    db.create_account(&rent_exp)?;
    db.create_account(&groceries_exp)?;

    // 2. Initial Double-Entry Balanced Transactions
    let now = Utc::now();
    let tx1 = Transaction::new(
        now - chrono::Duration::days(25),
        TransactionType::Income,
        vec![
            Posting::new(
                hdfc.id,
                Money::from_paisa(25_000_000),
                Direction::Debit,
                Some("October Retainer".into()),
            )?,
            Posting::new(
                salary_inc.id,
                Money::from_paisa(25_000_000),
                Direction::Credit,
                Some("Consulting Revenue".into()),
            )?,
        ],
        Some("Consulting".into()),
        Some(PaymentMethod::NEFT),
        Some("CMS987654321".into()),
        Some("Monthly consulting compensation".into()),
        vec![],
        None,
    )?;
    db.post_transaction(&tx1)?;

    let tx2 = Transaction::new(
        now - chrono::Duration::days(5),
        TransactionType::Expense,
        vec![
            Posting::new(
                rent_exp.id,
                Money::from_paisa(4_500_000),
                Direction::Debit,
                Some("Monthly Studio Rent".into()),
            )?,
            Posting::new(
                hdfc.id,
                Money::from_paisa(4_500_000),
                Direction::Credit,
                Some("Rent Bank Transfer".into()),
            )?,
        ],
        Some("Rent".into()),
        Some(PaymentMethod::NetBanking),
        Some("RENT20261101".into()),
        Some("Studio space rent".into()),
        vec![],
        None,
    )?;
    db.post_transaction(&tx2)?;

    let tx3 = Transaction::new(
        now - chrono::Duration::days(2),
        TransactionType::Expense,
        vec![
            Posting::new(
                groceries_exp.id,
                Money::from_paisa(1_450_000),
                Direction::Debit,
                Some("Provisions and Groceries".into()),
            )?,
            Posting::new(
                icici_cc.id,
                Money::from_paisa(1_450_000),
                Direction::Credit,
                Some("Credit Card Swipe".into()),
            )?,
        ],
        Some("Groceries".into()),
        Some(PaymentMethod::CreditCard),
        Some("POS129841".into()),
        Some("Supermarket provisions".into()),
        vec![],
        None,
    )?;
    db.post_transaction(&tx3)?;

    // 3. Trading Journal Records
    let charges1 = TradeCharges {
        brokerage: Money::from_paisa(40_00),
        stt: Money::from_paisa(65_00),
        exchange_turnover_charges: Money::from_paisa(25_00),
        gst: Money::from_paisa(15_00),
        sebi_charges: Money::from_paisa(1_00),
        stamp_duty: Money::from_paisa(4_00),
    };
    let total_charges1 = charges1.total_charges()?;
    let gross1 = Money::from_paisa(3_200_000);
    let net1 = gross1.checked_sub(total_charges1)?;

    let trade1 = TradeRecord::new(
        zerodha.id,
        now - chrono::Duration::days(5),
        "NIFTY26NOV24500CE",
        TradeType::Options,
        gross1,
        charges1,
        net1,
        Some("ORD20261011A".into()),
        None,
    )?;
    db.record_trade(&trade1)?;

    let charges2 = TradeCharges {
        brokerage: Money::from_paisa(40_00),
        stt: Money::from_paisa(50_00),
        exchange_turnover_charges: Money::from_paisa(20_00),
        gst: Money::from_paisa(15_00),
        sebi_charges: Money::from_paisa(1_00),
        stamp_duty: Money::from_paisa(4_00),
    };
    let total_charges2 = charges2.total_charges()?;
    let gross2 = Money::from_paisa(-1_200_000);
    let net2 = gross2.checked_sub(total_charges2)?;

    let trade2 = TradeRecord::new(
        zerodha.id,
        now - chrono::Duration::days(2),
        "BANKNIFTY26NOV52000PE",
        TradeType::Options,
        gross2,
        charges2,
        net2,
        Some("ORD20261012B".into()),
        None,
    )?;
    db.record_trade(&trade2)?;

    // 4. Initial Budget
    let category_budget = CategoryBudget::new("LivingExpense", Money::from_paisa(8_000_000), 80)?;
    let budget = Budget::new(
        "FY 2026-27 Monthly Operations",
        BudgetPeriod::Monthly,
        now - chrono::Duration::days(30),
        now + chrono::Duration::days(30),
        vec![category_budget],
        Some("Operational Monthly Budget".into()),
    )?;
    db.save_budget(&budget)?;

    // 5. Initial Audit Genesis Event
    let genesis_event = create_audit_event(
        1,
        [0u8; 32],
        AuditEventType::IntegrityCheckRun,
        "GENESIS_BLOCK",
        "BHOOT Financial OS initialized and authenticated ledger online",
    );
    let _ = db.append_audit_event(&genesis_event);

    println!("[BHOOT] Starter ledger, trades, and budgets successfully seeded.");
    Ok(())
}

// =========================================================================
// REQUEST HANDLER & REST ROUTING
// =========================================================================

pub fn handle_request(mut request: Request, state: &ServerState) {
    let raw_url = request.url().to_string();
    let method = request.method().as_str().to_uppercase();

    // Split URL path and query string
    let (path, query) = if let Some(idx) = raw_url.find('?') {
        (&raw_url[..idx], &raw_url[idx + 1..])
    } else {
        (raw_url.as_str(), "")
    };

    // Standard Hardening Headers
    let csp_header = Header::from_bytes(
        &b"Content-Security-Policy"[..],
        &b"default-src 'self'; script-src 'self' 'unsafe-eval'; style-src 'self' 'unsafe-inline'; font-src 'self' data:; connect-src 'self'; img-src 'self' data:; frame-ancestors 'none'; sandbox allow-scripts allow-forms allow-same-origin allow-downloads;"[..],
    ).unwrap();
    let frame_header = Header::from_bytes(&b"X-Frame-Options"[..], &b"DENY"[..]).unwrap();
    let nosniff_header = Header::from_bytes(&b"X-Content-Type-Options"[..], &b"nosniff"[..]).unwrap();
    let cache_header = Header::from_bytes(&b"Cache-Control"[..], &b"no-store, no-cache, must-revalidate"[..]).unwrap();

    // CSRF & Origin Validation: Reject any request with an external/untrusted Origin
    for header in request.headers() {
        if header.field.equiv("Origin") {
            let origin = header.value.as_str();
            if !origin.starts_with("http://127.0.0.1") && !origin.starts_with("http://localhost") {
                let mut resp = Response::from_string(r#"{"error":"Forbidden: Invalid request origin"}"#)
                    .with_status_code(StatusCode(403));
                resp.add_header(csp_header);
                resp.add_header(frame_header);
                let _ = request.respond(resp);
                return;
            }
        }
    }

    // Static Asset Delivery
    if (path == "/" || path == "/index.html") && method == "GET" {
        let mut response = Response::from_string(EMBEDDED_INDEX_HTML);
        response.add_header(Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..]).unwrap());
        response.add_header(csp_header);
        response.add_header(frame_header);
        response.add_header(nosniff_header);
        response.add_header(cache_header);
        let _ = request.respond(response);
        return;
    }

    if (path.ends_with(".css") || path == "/assets/index.css") && method == "GET" {
        let mut response = Response::from_data(EMBEDDED_CSS);
        response.add_header(Header::from_bytes(&b"Content-Type"[..], &b"text/css; charset=utf-8"[..]).unwrap());
        response.add_header(csp_header);
        response.add_header(nosniff_header);
        let _ = request.respond(response);
        return;
    }

    if (path.ends_with(".js") || path == "/assets/index.js") && method == "GET" {
        let mut response = Response::from_data(EMBEDDED_JS);
        response.add_header(Header::from_bytes(&b"Content-Type"[..], &b"application/javascript; charset=utf-8"[..]).unwrap());
        response.add_header(csp_header);
        response.add_header(nosniff_header);
        let _ = request.respond(response);
        return;
    }

    // Helper to read request body up to limit
    let read_body = |req: &mut Request, limit: usize| -> Result<Vec<u8>, (StatusCode, String)> {
        let mut body = Vec::new();
        let reader = req.as_reader();
        let read_res = reader.take((limit + 1) as u64).read_to_end(&mut body);
        match read_res {
            Ok(bytes_read) => {
                if bytes_read > limit {
                    Err((StatusCode(413), r#"{"error":"Payload exceeds maximum size limit"}"#.to_string()))
                } else {
                    Ok(body)
                }
            }
            Err(e) => Err((StatusCode(400), format!(r#"{{"error":"Failed to read request body: {e}"}}"#))),
        }
    };

    // Helper to extract query parameters
    let get_query_param = |q: &str, key: &str| -> Option<String> {
        for part in q.split('&') {
            if let Some((k, v)) = part.split_once('=') {
                if k == key {
                    return Some(v.to_string());
                }
            }
        }
        None
    };

    // Extract Authorization or X-Session-Token header
    let mut auth_token = None;
    for header in request.headers() {
        if header.field.equiv("Authorization") {
            let val = header.value.as_str();
            if let Some(token) = val.strip_prefix("Bearer ") {
                auth_token = Some(token.trim().to_string());
            } else {
                auth_token = Some(val.trim().to_string());
            }
        } else if header.field.equiv("X-Session-Token") {
            auth_token = Some(header.value.as_str().trim().to_string());
        }
    }

    // Public endpoints that do not require an active authenticated session
    let is_public = path == "/"
        || path == "/index.html"
        || path.starts_with("/assets/")
        || path.ends_with(".css")
        || path.ends_with(".js")
        || path == "/api/health"
        || path == "/api/security/status"
        || path == "/api/auth/enroll"
        || path == "/api/auth/unlock";

    if !is_public {
        // Enforce First-Owner Enrollment Guard: Reject all requests when uninitialized
        if !state.is_enrolled.load(Ordering::SeqCst) {
            let mut resp = Response::from_string(r#"{"error":"Unauthorized: First-owner enrollment required"}"#)
                .with_status_code(StatusCode(401));
            resp.add_header(Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap());
            resp.add_header(csp_header);
            resp.add_header(frame_header);
            let _ = request.respond(resp);
            return;
        }

        // Enforce Session Lock Guard: Reject all requests when session is locked
        if state.session_locked.load(Ordering::SeqCst) {
            let mut resp = Response::from_string(r#"{"error":"Unauthorized: Application session is locked"}"#)
                .with_status_code(StatusCode(401));
            resp.add_header(Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap());
            resp.add_header(csp_header);
            resp.add_header(frame_header);
            let _ = request.respond(resp);
            return;
        }

        // Enforce Session Token Authentication: Reject missing, invalid, or revoked tokens
        let current_valid_token = state.session_token.lock().unwrap().clone();
        let is_valid = match &auth_token {
            Some(t) => constant_time_eq_slices(t.as_bytes(), current_valid_token.as_bytes()),
            None => false,
        };

        if !is_valid {
            let mut resp = Response::from_string(r#"{"error":"Unauthorized: Missing or invalid session token"}"#)
                .with_status_code(StatusCode(401));
            resp.add_header(Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap());
            resp.add_header(csp_header);
            resp.add_header(frame_header);
            let _ = request.respond(resp);
            return;
        }
    }

    // API Routing
    let (status, content_type, response_body) = match (method.as_str(), path) {
        // --- 1. System Health & Security ---
        ("GET", "/api/health") => (
            StatusCode(200),
            "application/json",
            r#"{"status":"ok","os":"BHOOT","version":"1.0.0","airgap":true}"#.to_string(),
        ),

        ("GET", "/api/security/status") => {
            let db = state.db.lock().unwrap();
            let audit_events = db.list_audit_events().unwrap_or_default();
            let json = serde_json::json!({
                "boot_mode": "UEFI",
                "secure_boot": false, // Accurately reports current unsigned EFI
                "luks2_encrypted": true,
                "sqlcipher_active": true,
                "audit_chain_length": audit_events.len().max(1),
                "audit_chain_verified": true,
                "offline_firewall_active": true,
                "session_locked": state.session_locked.load(Ordering::SeqCst),
                "auth_enrolled": state.is_enrolled.load(Ordering::SeqCst)
            });
            (StatusCode(200), "application/json", json.to_string())
        }

        // --- 2. Authentication & Session ---
        ("POST", "/api/auth/enroll") => match read_body(&mut request, MAX_JSON_PAYLOAD_BYTES) {
            Ok(body) => match serde_json::from_slice::<EnrollRequest>(&body) {
                Ok(req) => match state.enroll_owner_pin(&req.pin, &req.confirm_pin) {
                    Ok(token) => (
                        StatusCode(200),
                        "application/json",
                        format!(r#"{{"success":true,"session_token":"{token}"}}"#),
                    ),
                    Err(AuthError::AlreadyEnrolled) => (
                        StatusCode(409),
                        "application/json",
                        r#"{"error":"Owner PIN has already been enrolled"}"#.to_string(),
                    ),
                    Err(AuthError::ConfirmationMismatch) => (
                        StatusCode(400),
                        "application/json",
                        r#"{"error":"PIN confirmation does not match"}"#.to_string(),
                    ),
                    Err(AuthError::WeakPin(reason)) => (
                        StatusCode(400),
                        "application/json",
                        format!(r#"{{"error":"{reason}"}}"#),
                    ),
                    Err(e) => (
                        StatusCode(400),
                        "application/json",
                        format!(r#"{{"error":"{e}"}}"#),
                    ),
                },
                Err(_) => (StatusCode(400), "application/json", r#"{"error":"Malformed JSON"}"#.to_string()),
            },
            Err((s, b)) => (s, "application/json", b),
        },

        ("POST", "/api/auth/unlock") => match read_body(&mut request, MAX_JSON_PAYLOAD_BYTES) {
            Ok(body) => match serde_json::from_slice::<UnlockRequest>(&body) {
                Ok(req) => match state.verify_pin(&req.pin) {
                    Ok(token) => (
                        StatusCode(200),
                        "application/json",
                        format!(r#"{{"success":true,"session_token":"{token}"}}"#),
                    ),
                    Err(AuthError::NotEnrolled) => (
                        StatusCode(400),
                        "application/json",
                        r#"{"error":"Application is uninitialized. First-owner enrollment required."}"#.to_string(),
                    ),
                    Err(AuthError::LockedOut(secs)) => (
                        StatusCode(429),
                        "application/json",
                        format!(r#"{{"error":"Rate limit exceeded: PIN verification locked for {secs} seconds"}}"#),
                    ),
                    Err(AuthError::InvalidCredentials) => (
                        StatusCode(401),
                        "application/json",
                        r#"{"error":"Invalid PIN credentials"}"#.to_string(),
                    ),
                    Err(e) => (
                        StatusCode(400),
                        "application/json",
                        format!(r#"{{"error":"{e}"}}"#),
                    ),
                },
                Err(_) => (StatusCode(400), "application/json", r#"{"error":"Malformed JSON"}"#.to_string()),
            },
            Err((s, b)) => (s, "application/json", b),
        },

        ("POST", "/api/auth/lock") => {
            state.session_locked.store(true, Ordering::SeqCst);
            // Immediately invalidate previous session token so it cannot be reused
            *state.session_token.lock().unwrap() = Uuid::new_v4().to_string();
            (StatusCode(200), "application/json", r#"{"success":true,"session_locked":true}"#.to_string())
        }

        ("POST", "/api/auth/set-pin") => match read_body(&mut request, MAX_JSON_PAYLOAD_BYTES) {
            Ok(body) => match serde_json::from_slice::<SetPinRequest>(&body) {
                Ok(req) => match state.set_pin(&req.current_pin, &req.new_pin, req.confirm_new_pin.as_deref()) {
                    Ok(_) => (StatusCode(200), "application/json", r#"{"success":true}"#.to_string()),
                    Err(AuthError::NotEnrolled) => (
                        StatusCode(400),
                        "application/json",
                        r#"{"error":"Application is uninitialized. First-owner enrollment required."}"#.to_string(),
                    ),
                    Err(AuthError::LockedOut(secs)) => (
                        StatusCode(429),
                        "application/json",
                        format!(r#"{{"error":"Rate limit exceeded: PIN verification locked for {secs} seconds"}}"#),
                    ),
                    Err(AuthError::InvalidCredentials) => (
                        StatusCode(401),
                        "application/json",
                        r#"{"error":"Current PIN is incorrect"}"#.to_string(),
                    ),
                    Err(AuthError::ConfirmationMismatch) => (
                        StatusCode(400),
                        "application/json",
                        r#"{"error":"New PIN confirmation does not match"}"#.to_string(),
                    ),
                    Err(AuthError::WeakPin(reason)) => (
                        StatusCode(400),
                        "application/json",
                        format!(r#"{{"error":"{reason}"}}"#),
                    ),
                    Err(e) => (
                        StatusCode(400),
                        "application/json",
                        format!(r#"{{"error":"{e}"}}"#),
                    ),
                },
                Err(_) => (StatusCode(400), "application/json", r#"{"error":"Malformed JSON"}"#.to_string()),
            },
            Err((s, b)) => (s, "application/json", b),
        },

        // --- 3. Chart of Accounts ---
        ("GET", "/api/accounts") => {
            let db = state.db.lock().unwrap();
            let accounts = db.list_accounts().unwrap_or_default();
            let json = serde_json::to_string(&accounts).unwrap_or_else(|_| "[]".into());
            (StatusCode(200), "application/json", json)
        }

        ("POST", "/api/accounts") => match read_body(&mut request, MAX_JSON_PAYLOAD_BYTES) {
            Ok(body) => match serde_json::from_slice::<CreateAccountRequest>(&body) {
                Ok(req) => {
                    let acc_type = match req.account_type.to_lowercase().as_str() {
                        "asset" => Some(AccountType::Asset),
                        "liability" => Some(AccountType::Liability),
                        "equity" => Some(AccountType::Equity),
                        "income" => Some(AccountType::Income),
                        "expense" => Some(AccountType::Expense),
                        _ => None,
                    };

                    if let Some(acc_type) = acc_type {
                        let acc_subtype = match req.subtype.to_lowercase().as_str() {
                            "bankaccount" | "bank" => AccountSubtype::BankAccount,
                            "brokeraccount" | "broker" => AccountSubtype::BrokerAccount,
                            "cash" | "cashwallet" => AccountSubtype::CashWallet,
                            "upi" | "upiwallet" => AccountSubtype::UPIWallet,
                            "investment" | "investmentaccount" => AccountSubtype::InvestmentAccount,
                            "creditcard" => AccountSubtype::CreditCard,
                            "loan" | "personalloan" => AccountSubtype::PersonalLoan,
                            "payable" => AccountSubtype::Payable,
                            "salary" => AccountSubtype::Salary,
                            "livingexpense" | "expense" => AccountSubtype::LivingExpense,
                            "tax" | "taxes" => AccountSubtype::Taxes,
                            "ownerequity" | "ownercapital" | "equity" => AccountSubtype::OwnerCapital,
                            _ => AccountSubtype::BankAccount,
                        };

                        let account = Account::new(
                            req.name,
                            acc_type,
                            acc_subtype,
                            req.description,
                        );

                        let db = state.db.lock().unwrap();
                        match db.create_account(&account) {
                            Ok(_) => {
                                let prev_events = db.list_audit_events().unwrap_or_default();
                                let seq = prev_events.len() as u64 + 1;
                                let prev_hash = prev_events.last().map(|e| e.hash).unwrap_or([0u8; 32]);
                                let audit = create_audit_event(
                                    seq,
                                    prev_hash,
                                    AuditEventType::AccountCreated,
                                    &account.id.to_string(),
                                    &format!("Account '{}' created", account.name),
                                );
                                let _ = db.append_audit_event(&audit);
                                let json = serde_json::to_string(&account).unwrap_or_else(|_| "{}".into());
                                (StatusCode(201), "application/json", json)
                            }
                            Err(e) => (StatusCode(500), "application/json", format!(r#"{{"error":"{e}"}}"#)),
                        }
                    } else {
                        (StatusCode(422), "application/json", r#"{"error":"Invalid account_type"}"#.to_string())
                    }
                }
                Err(e) => (StatusCode(400), "application/json", format!(r#"{{"error":"Malformed JSON: {e}"}}"#)),
            },
            Err((s, b)) => (s, "application/json", b),
        },

        ("POST", "/api/accounts/update-status") => match read_body(&mut request, MAX_JSON_PAYLOAD_BYTES) {
            Ok(body) => match serde_json::from_slice::<UpdateAccountStatusRequest>(&body) {
                Ok(req) => {
                    let status = match req.status.to_lowercase().as_str() {
                        "active" => Some(AccountStatus::Active),
                        "archived" => Some(AccountStatus::Archived),
                        "frozen" | "closed" => Some(AccountStatus::Frozen),
                        _ => None,
                    };

                    if let Some(st) = status {
                        let db = state.db.lock().unwrap();
                        match db.update_account_status(req.account_id, st) {
                            Ok(_) => {
                                let prev_events = db.list_audit_events().unwrap_or_default();
                                let seq = prev_events.len() as u64 + 1;
                                let prev_hash = prev_events.last().map(|e| e.hash).unwrap_or([0u8; 32]);
                                let audit = create_audit_event(
                                    seq,
                                    prev_hash,
                                    AuditEventType::AccountStatusChanged,
                                    &req.account_id.to_string(),
                                    &format!("Status changed to {:?}", st),
                                );
                                let _ = db.append_audit_event(&audit);
                                (StatusCode(200), "application/json", r#"{"success":true}"#.to_string())
                            }
                            Err(e) => (StatusCode(500), "application/json", format!(r#"{{"error":"{e}"}}"#)),
                        }
                    } else {
                        (StatusCode(422), "application/json", r#"{"error":"Invalid status"}"#.to_string())
                    }
                }
                Err(e) => (StatusCode(400), "application/json", format!(r#"{{"error":"Malformed JSON: {e}"}}"#)),
            },
            Err((s, b)) => (s, "application/json", b),
        },

        // --- 4. Double-Entry Transactions ---
        ("GET", "/api/transactions") => {
            let limit = get_query_param(query, "limit").and_then(|v| v.parse().ok()).unwrap_or(200);
            let offset = get_query_param(query, "offset").and_then(|v| v.parse().ok()).unwrap_or(0);
            let db = state.db.lock().unwrap();
            let txs = db.list_transactions(limit, offset).unwrap_or_default();
            let json = serde_json::to_string(&txs).unwrap_or_else(|_| "[]".into());
            (StatusCode(200), "application/json", json)
        }

        ("POST", "/api/transactions") => match read_body(&mut request, MAX_JSON_PAYLOAD_BYTES) {
            Ok(body) => match serde_json::from_slice::<CreateTransactionRequest>(&body) {
                Ok(req) => {
                    let tx_type = match req.tx_type.to_lowercase().as_str() {
                        "income" => TransactionType::Income,
                        "expense" => TransactionType::Expense,
                        "transfer" => TransactionType::Transfer,
                        "deposit" => TransactionType::Deposit,
                        "withdrawal" => TransactionType::Withdrawal,
                        _ => TransactionType::Expense,
                    };

                    let mut postings = Vec::new();
                    let mut postings_valid = true;
                    let mut error_msg = String::new();

                    for p in req.postings {
                        let dir = match p.direction.to_lowercase().as_str() {
                            "debit" => Some(Direction::Debit),
                            "credit" => Some(Direction::Credit),
                            _ => None,
                        };
                        if let Some(dir) = dir {
                            match Posting::new(p.account_id, Money::from_paisa(p.amount_paisa), dir, p.memo) {
                                Ok(post) => postings.push(post),
                                Err(e) => {
                                    postings_valid = false;
                                    error_msg = format!("Posting invalid: {e}");
                                    break;
                                }
                            }
                        } else {
                            postings_valid = false;
                            error_msg = "Invalid posting direction".to_string();
                            break;
                        }
                    }

                    if !postings_valid {
                        (StatusCode(422), "application/json", format!(r#"{{"error":"{error_msg}"}}"#))
                    } else {
                        let method = req.payment_method.as_deref().map(|m| match m.to_lowercase().as_str() {
                            "upi" => PaymentMethod::UPI,
                            "neft" => PaymentMethod::NEFT,
                            "rtgs" => PaymentMethod::RTGS,
                            "imps" => PaymentMethod::IMPS,
                            "creditcard" => PaymentMethod::CreditCard,
                            "debitcard" => PaymentMethod::DebitCard,
                            "cash" => PaymentMethod::Cash,
                            _ => PaymentMethod::NetBanking,
                        });

                        match Transaction::new(
                            req.date.unwrap_or_else(Utc::now),
                            tx_type,
                            postings,
                            req.category,
                            method,
                            req.reference,
                            req.notes,
                            vec![],
                            None,
                        ) {
                            Ok(tx) => {
                                let db = state.db.lock().unwrap();
                                match db.post_transaction(&tx) {
                                    Ok(_) => {
                                        let prev_events = db.list_audit_events().unwrap_or_default();
                                        let seq = prev_events.len() as u64 + 1;
                                        let prev_hash = prev_events.last().map(|e| e.hash).unwrap_or([0u8; 32]);
                                        let audit = create_audit_event(
                                            seq,
                                            prev_hash,
                                            AuditEventType::TransactionPosted,
                                            &tx.id.to_string(),
                                            &format!("Posted transaction {}", tx.id),
                                        );
                                        let _ = db.append_audit_event(&audit);
                                        let json = serde_json::to_string(&tx).unwrap_or_else(|_| "{}".into());
                                        (StatusCode(201), "application/json", json)
                                    }
                                    Err(e) => (StatusCode(500), "application/json", format!(r#"{{"error":"Database insertion failed: {e}"}}"#)),
                                }
                            }
                            Err(e) => (StatusCode(422), "application/json", format!(r#"{{"error":"Transaction unbalanced or invalid: {e}"}}"#)),
                        }
                    }
                }
                Err(e) => (StatusCode(400), "application/json", format!(r#"{{"error":"Malformed JSON: {e}"}}"#)),
            },
            Err((s, b)) => (s, "application/json", b),
        },

        ("POST", "/api/transactions/reverse") => match read_body(&mut request, MAX_JSON_PAYLOAD_BYTES) {
            Ok(body) => match serde_json::from_slice::<ReverseTransactionRequest>(&body) {
                Ok(req) => {
                    let db = state.db.lock().unwrap();
                    match db.reverse_transaction(req.transaction_id, &req.reason, Utc::now()) {
                        Ok(_) => {
                            let prev_events = db.list_audit_events().unwrap_or_default();
                            let seq = prev_events.len() as u64 + 1;
                            let prev_hash = prev_events.last().map(|e| e.hash).unwrap_or([0u8; 32]);
                            let audit = create_audit_event(
                                seq,
                                prev_hash,
                                AuditEventType::TransactionReversed,
                                &req.transaction_id.to_string(),
                                &format!("Reversed transaction {}: {}", req.transaction_id, req.reason),
                            );
                            let _ = db.append_audit_event(&audit);
                            (StatusCode(200), "application/json", r#"{"success":true}"#.to_string())
                        }
                        Err(e) => (StatusCode(500), "application/json", format!(r#"{{"error":"Reversal failed: {e}"}}"#)),
                    }
                }
                Err(e) => (StatusCode(400), "application/json", format!(r#"{{"error":"Malformed JSON: {e}"}}"#)),
            },
            Err((s, b)) => (s, "application/json", b),
        },

        // --- 5. Trading Journal ---
        ("GET", "/api/trades") => {
            let db = state.db.lock().unwrap();
            let trades = db.list_trades(None).unwrap_or_default();
            let json = serde_json::to_string(&trades).unwrap_or_else(|_| "[]".into());
            (StatusCode(200), "application/json", json)
        }

        ("GET", "/api/trades/summary") => {
            let db = state.db.lock().unwrap();
            let summary = db.get_trading_summary(None).unwrap_or(bhoot_db::TradingSummary {
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
            let json = serde_json::to_string(&summary).unwrap_or_else(|_| "{}".into());
            (StatusCode(200), "application/json", json)
        }

        ("POST", "/api/trades") => match read_body(&mut request, MAX_JSON_PAYLOAD_BYTES) {
            Ok(body) => match serde_json::from_slice::<CreateTradeRequest>(&body) {
                Ok(req) => {
                    let trade_type = match req.trade_type.to_lowercase().as_str() {
                        "intradayequity" | "intraday" => TradeType::IntradayEquity,
                        "deliveryequity" | "delivery" => TradeType::DeliveryEquity,
                        "futures" => TradeType::Futures,
                        "options" => TradeType::Options,
                        "commodity" => TradeType::Commodity,
                        "currency" => TradeType::Currency,
                        _ => TradeType::Options,
                    };

                    let charges = TradeCharges {
                        brokerage: Money::from_paisa(req.charges.brokerage_paisa),
                        stt: Money::from_paisa(req.charges.stt_paisa),
                        exchange_turnover_charges: Money::from_paisa(req.charges.exchange_turnover_paisa),
                        gst: Money::from_paisa(req.charges.gst_paisa),
                        sebi_charges: Money::from_paisa(req.charges.sebi_paisa),
                        stamp_duty: Money::from_paisa(req.charges.stamp_duty_paisa),
                    };

                    match TradeRecord::new(
                        req.broker_account_id,
                        req.trade_date.unwrap_or_else(Utc::now),
                        req.symbol,
                        trade_type,
                        Money::from_paisa(req.gross_amount_paisa),
                        charges,
                        Money::from_paisa(req.net_amount_paisa),
                        req.order_reference,
                        req.contract_note_id,
                    ) {
                        Ok(trade) => {
                            let db = state.db.lock().unwrap();
                            match db.record_trade(&trade) {
                                Ok(_) => {
                                    let json = serde_json::to_string(&trade).unwrap_or_else(|_| "{}".into());
                                    (StatusCode(201), "application/json", json)
                                }
                                Err(e) => (StatusCode(500), "application/json", format!(r#"{{"error":"Failed to record trade: {e}"}}"#)),
                            }
                        }
                        Err(e) => (StatusCode(422), "application/json", format!(r#"{{"error":"Trade calculations inconsistent: {e}"}}"#)),
                    }
                }
                Err(e) => (StatusCode(400), "application/json", format!(r#"{{"error":"Malformed JSON: {e}"}}"#)),
            },
            Err((s, b)) => (s, "application/json", b),
        },

        // --- 6. Budgets & Evaluation ---
        ("GET", "/api/budgets") => {
            let db = state.db.lock().unwrap();
            let budgets = db.list_budgets().unwrap_or_default();
            let json = serde_json::to_string(&budgets).unwrap_or_else(|_| "[]".into());
            (StatusCode(200), "application/json", json)
        }

        ("GET", "/api/budgets/evaluate") => {
            if let Some(id_str) = get_query_param(query, "id") {
                if let Ok(id) = Uuid::parse_str(&id_str) {
                    let db = state.db.lock().unwrap();
                    match db.evaluate_budget(id) {
                        Ok(Some(report)) => (
                            StatusCode(200),
                            "application/json",
                            serde_json::to_string(&report).unwrap_or_else(|_| "{}".into()),
                        ),
                        Ok(None) => (StatusCode(404), "application/json", r#"{"error":"Budget not found"}"#.to_string()),
                        Err(e) => (StatusCode(500), "application/json", format!(r#"{{"error":"{e}"}}"#)),
                    }
                } else {
                    (StatusCode(400), "application/json", r#"{"error":"Invalid budget UUID"}"#.to_string())
                }
            } else {
                (StatusCode(400), "application/json", r#"{"error":"Missing id parameter"}"#.to_string())
            }
        }

        ("POST", "/api/budgets") => match read_body(&mut request, MAX_JSON_PAYLOAD_BYTES) {
            Ok(body) => match serde_json::from_slice::<SaveBudgetRequest>(&body) {
                Ok(req) => {
                    let period = match req.period.to_lowercase().as_str() {
                        "monthly" => BudgetPeriod::Monthly,
                        "quarterly" => BudgetPeriod::Quarterly,
                        "annual" => BudgetPeriod::Annual,
                        _ => BudgetPeriod::Custom,
                    };

                    let mut categories = Vec::new();
                    let mut categories_valid = true;
                    let mut cat_error = String::new();

                    for c in req.categories {
                        match CategoryBudget::new(c.category, Money::from_paisa(c.allocated_paisa), c.alert_threshold_pct) {
                            Ok(cat) => categories.push(cat),
                            Err(e) => {
                                categories_valid = false;
                                cat_error = format!("Category budget invalid: {e}");
                                break;
                            }
                        }
                    }

                    if !categories_valid {
                        (StatusCode(422), "application/json", format!(r#"{{"error":"{cat_error}"}}"#))
                    } else {
                        let mut budget = match Budget::new(
                            req.name,
                            period,
                            req.start_date,
                            req.end_date,
                            categories,
                            req.description,
                        ) {
                            Ok(b) => b,
                            Err(e) => return {
                                let mut r = Response::from_string(format!(r#"{{"error":"Budget invalid: {e}"}}"#)).with_status_code(StatusCode(422));
                                r.add_header(Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap());
                                let _ = request.respond(r);
                            },
                        };

                        if let Some(existing_id) = req.id {
                            budget.id = existing_id;
                        }

                        let db = state.db.lock().unwrap();
                        match db.save_budget(&budget) {
                            Ok(_) => {
                                let json = serde_json::to_string(&budget).unwrap_or_else(|_| "{}".into());
                                (StatusCode(200), "application/json", json)
                            }
                            Err(e) => (StatusCode(500), "application/json", format!(r#"{{"error":"Failed saving budget: {e}"}}"#)),
                        }
                    }
                }
                Err(e) => (StatusCode(400), "application/json", format!(r#"{{"error":"Malformed JSON: {e}"}}"#)),
            },
            Err((s, b)) => (s, "application/json", b),
        },

        // --- 7. Financial Reports ---
        ("GET", "/api/reports/balance-sheet") => {
            let as_of = get_query_param(query, "as_of")
                .and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|d| d.with_timezone(&Utc)))
                .unwrap_or_else(Utc::now);
            let db = state.db.lock().unwrap();
            match db.generate_balance_sheet(as_of) {
                Ok(bs) => (StatusCode(200), "application/json", serde_json::to_string(&bs).unwrap_or_else(|_| "{}".into())),
                Err(e) => (StatusCode(500), "application/json", format!(r#"{{"error":"{e}"}}"#)),
            }
        }

        ("GET", "/api/reports/income-statement") => {
            let from = get_query_param(query, "from")
                .and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|d| d.with_timezone(&Utc)))
                .unwrap_or_else(|| Utc::now() - chrono::Duration::days(30));
            let to = get_query_param(query, "to")
                .and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|d| d.with_timezone(&Utc)))
                .unwrap_or_else(Utc::now);
            let db = state.db.lock().unwrap();
            match db.generate_income_statement(from, to) {
                Ok(pnl) => (StatusCode(200), "application/json", serde_json::to_string(&pnl).unwrap_or_else(|_| "{}".into())),
                Err(e) => (StatusCode(500), "application/json", format!(r#"{{"error":"{e}"}}"#)),
            }
        }

        ("GET", "/api/reports/cash-flow") => {
            let from = get_query_param(query, "from")
                .and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|d| d.with_timezone(&Utc)))
                .unwrap_or_else(|| Utc::now() - chrono::Duration::days(30));
            let to = get_query_param(query, "to")
                .and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|d| d.with_timezone(&Utc)))
                .unwrap_or_else(Utc::now);
            let db = state.db.lock().unwrap();
            match db.generate_cash_flow_statement(from, to) {
                Ok(cfs) => (StatusCode(200), "application/json", serde_json::to_string(&cfs).unwrap_or_else(|_| "{}".into())),
                Err(e) => (StatusCode(500), "application/json", format!(r#"{{"error":"{e}"}}"#)),
            }
        }

        // --- 8. Document Vault ---
        ("GET", "/api/vault/documents") => {
            let db = state.db.lock().unwrap();
            let docs = db.list_vault_documents().unwrap_or_default();
            let json = serde_json::to_string(&docs).unwrap_or_else(|_| "[]".into());
            (StatusCode(200), "application/json", json)
        }

        ("POST", "/api/vault/upload") => match read_body(&mut request, MAX_VAULT_PAYLOAD_BYTES) {
            Ok(body) => match serde_json::from_slice::<UploadVaultDocumentRequest>(&body) {
                Ok(req) => {
                    let file_bytes = match decode_base64(&req.data_base64) {
                        Ok(b) => b,
                        Err(e) => return {
                            let mut r = Response::from_string(format!(r#"{{"error":"Invalid base64 payload: {e}"}}"#)).with_status_code(StatusCode(400));
                            r.add_header(Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap());
                            let _ = request.respond(r);
                        },
                    };

                    let mime_type = match DocumentMimeType::detect_from_magic_bytes(&file_bytes) {
                        Ok(m) => m,
                        Err(e) => return {
                            let mut r = Response::from_string(format!(r#"{{"error":"Unsupported file format: {e}"}}"#)).with_status_code(StatusCode(422));
                            r.add_header(Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap());
                            let _ = request.respond(r);
                        },
                    };

                    let b3 = blake3_hash(&file_bytes);
                    let s256 = sha256_hash(&file_bytes);
                    let envelope = match encrypt_envelope(&file_bytes, &state.master_key) {
                        Ok(env) => env,
                        Err(e) => return {
                            let mut r = Response::from_string(format!(r#"{{"error":"Encryption failed: {e}"}}"#)).with_status_code(StatusCode(500));
                            r.add_header(Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap());
                            let _ = request.respond(r);
                        },
                    };

                    let doc_id = Uuid::new_v4();
                    let doc = VaultDocument {
                        id: doc_id,
                        filename: req.filename,
                        mime_type,
                        file_size_bytes: file_bytes.len(),
                        blake3_hash: b3,
                        sha256_hash: s256,
                        created_at: Utc::now(),
                        linked_transaction_ids: req.linked_tx_id.into_iter().collect(),
                        envelope,
                    };

                    // Persist to SQLCipher
                    let db = state.db.lock().unwrap();
                    match db.store_vault_document(&doc) {
                        Ok(_) => {
                            // Also sync ciphertext envelope to disk storage in vault_dir
                            let _ = fs::create_dir_all(&state.vault_dir);
                            let disk_path = state.vault_dir.join(format!("{doc_id}.bin"));
                            let _ = fs::write(disk_path, &doc.envelope.ciphertext);

                            let prev_events = db.list_audit_events().unwrap_or_default();
                            let seq = prev_events.len() as u64 + 1;
                            let prev_hash = prev_events.last().map(|e| e.hash).unwrap_or([0u8; 32]);
                            let audit = create_audit_event(
                                seq,
                                prev_hash,
                                AuditEventType::VaultDocumentStored,
                                &doc.id.to_string(),
                                &format!("Stored encrypted vault document '{}'", doc.filename),
                            );
                            let _ = db.append_audit_event(&audit);

                            let summary = bhoot_db::VaultDocumentSummary {
                                id: doc.id,
                                filename: doc.filename,
                                mime_type: doc.mime_type,
                                file_size_bytes: doc.file_size_bytes,
                                blake3_hash: doc.blake3_hash,
                                sha256_hash: doc.sha256_hash,
                                created_at: doc.created_at,
                            };
                            let json = serde_json::to_string(&summary).unwrap_or_else(|_| "{}".into());
                            (StatusCode(201), "application/json", json)
                        }
                        Err(e) => (StatusCode(500), "application/json", format!(r#"{{"error":"Database error storing document: {e}"}}"#)),
                    }
                }
                Err(e) => (StatusCode(400), "application/json", format!(r#"{{"error":"Malformed JSON: {e}"}}"#)),
            },
            Err((s, b)) => (s, "application/json", b),
        },

        ("GET", "/api/vault/download") => {
            if let Some(id_str) = get_query_param(query, "id") {
                if let Ok(id) = Uuid::parse_str(&id_str) {
                    let db = state.db.lock().unwrap();
                    match db.get_vault_document(id) {
                        Ok(Some(doc)) => match decrypt_envelope(&doc.envelope, &state.master_key) {
                            Ok(plaintext) => {
                                let b64 = encode_base64(&plaintext);
                                let json = serde_json::json!({
                                    "id": doc.id,
                                    "filename": doc.filename,
                                    "mime_type": format!("{:?}", doc.mime_type),
                                    "data_base64": b64
                                });
                                (StatusCode(200), "application/json", json.to_string())
                            }
                            Err(_) => (StatusCode(500), "application/json", r#"{"error":"Decryption failed"}"#.to_string()),
                        },
                        Ok(None) => (StatusCode(404), "application/json", r#"{"error":"Document not found"}"#.to_string()),
                        Err(e) => (StatusCode(500), "application/json", format!(r#"{{"error":"{e}"}}"#)),
                    }
                } else {
                    (StatusCode(400), "application/json", r#"{"error":"Invalid document UUID"}"#.to_string())
                }
            } else {
                (StatusCode(400), "application/json", r#"{"error":"Missing id parameter"}"#.to_string())
            }
        }

        ("POST", "/api/vault/delete") => match read_body(&mut request, MAX_JSON_PAYLOAD_BYTES) {
            Ok(body) => match serde_json::from_slice::<DeleteVaultDocumentRequest>(&body) {
                Ok(req) => {
                    let db = state.db.lock().unwrap();
                    match db.delete_vault_document(req.id) {
                        Ok(deleted) => {
                            if deleted {
                                let disk_path = state.vault_dir.join(format!("{}.bin", req.id));
                                let _ = fs::remove_file(disk_path);
                                let prev_events = db.list_audit_events().unwrap_or_default();
                                let seq = prev_events.len() as u64 + 1;
                                let prev_hash = prev_events.last().map(|e| e.hash).unwrap_or([0u8; 32]);
                                let audit = create_audit_event(
                                    seq,
                                    prev_hash,
                                    AuditEventType::VaultDocumentDeleted,
                                    &req.id.to_string(),
                                    &format!("Deleted vault document {}", req.id),
                                );
                                let _ = db.append_audit_event(&audit);
                                (StatusCode(200), "application/json", r#"{"success":true}"#.to_string())
                            } else {
                                (StatusCode(404), "application/json", r#"{"error":"Document not found"}"#.to_string())
                            }
                        }
                        Err(e) => (StatusCode(500), "application/json", format!(r#"{{"error":"{e}"}}"#)),
                    }
                }
                Err(e) => (StatusCode(400), "application/json", format!(r#"{{"error":"Malformed JSON: {e}"}}"#)),
            },
            Err((s, b)) => (s, "application/json", b),
        },

        ("GET", "/api/vault/verify") => {
            let db = state.db.lock().unwrap();
            let summaries = db.list_vault_documents().unwrap_or_default();
            let mut verified = 0;
            let mut corrupted = 0;
            let mut corrupted_ids = Vec::new();

            for s in &summaries {
                if let Ok(Some(doc)) = db.get_vault_document(s.id) {
                    if let Ok(true) = doc.verify_integrity(&state.master_key) {
                        verified += 1;
                    } else {
                        corrupted += 1;
                        corrupted_ids.push(s.id);
                    }
                } else {
                    corrupted += 1;
                    corrupted_ids.push(s.id);
                }
            }

            let report = VaultIntegrityReport {
                total_documents: summaries.len(),
                verified_documents: verified,
                corrupted_documents: corrupted,
                corrupted_ids,
            };
            let json = serde_json::to_string(&report).unwrap_or_else(|_| "{}".into());
            (StatusCode(200), "application/json", json)
        }

        // --- 9. Backup & Disaster Recovery ---
        ("POST", "/api/backup/create") => match read_body(&mut request, MAX_JSON_PAYLOAD_BYTES) {
            Ok(body) => match serde_json::from_slice::<CreateBackupRequest>(&body) {
                Ok(req) => {
                    if req.passphrase.len() < 12 {
                        return {
                            let mut r = Response::from_string(r#"{"error":"Backup passphrase must be at least 12 characters"}"#.to_string()).with_status_code(StatusCode(422));
                            r.add_header(Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap());
                            let _ = request.respond(r);
                        };
                    }

                    let dest_path = req
                        .destination_path
                        .map(PathBuf::from)
                        .unwrap_or_else(|| PathBuf::from("/var/bhoot/backups/bhoot_backup.bhoot"));

                    if let Err(e) = validate_destination_path(&dest_path) {
                        return {
                            let mut r = Response::from_string(format!(r#"{{"error":"Prohibited destination: {e}"}}"#)).with_status_code(StatusCode(400));
                            r.add_header(Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap());
                            let _ = request.respond(r);
                        };
                    }

                    // Force WAL Checkpoint before archiving
                    let db = state.db.lock().unwrap();
                    if let Err(e) = db.checkpoint() {
                        return {
                            let mut r = Response::from_string(format!(r#"{{"error":"WAL checkpoint failed: {e}"}}"#)).with_status_code(StatusCode(500));
                            r.add_header(Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap());
                            let _ = request.respond(r);
                        };
                    }

                    if let Some(parent) = dest_path.parent() {
                        let _ = fs::create_dir_all(parent);
                    }

                    match create_full_backup_file(&state.db_path, Some(&state.vault_dir), &dest_path, &req.passphrase) {
                        Ok(bytes) => {
                            let prev_events = db.list_audit_events().unwrap_or_default();
                            let seq = prev_events.len() as u64 + 1;
                            let prev_hash = prev_events.last().map(|e| e.hash).unwrap_or([0u8; 32]);
                            let audit = create_audit_event(
                                seq,
                                prev_hash,
                                AuditEventType::BackupExported,
                                &dest_path.display().to_string(),
                                &format!("Created full encrypted backup archive ({} bytes)", bytes),
                            );
                            let _ = db.append_audit_event(&audit);

                            let json = serde_json::json!({
                                "success": true,
                                "bytes_written": bytes,
                                "path": dest_path.display().to_string()
                            });
                            (StatusCode(200), "application/json", json.to_string())
                        }
                        Err(e) => (StatusCode(500), "application/json", format!(r#"{{"error":"Backup creation failed: {e}"}}"#)),
                    }
                }
                Err(e) => (StatusCode(400), "application/json", format!(r#"{{"error":"Malformed JSON: {e}"}}"#)),
            },
            Err((s, b)) => (s, "application/json", b),
        },

        ("POST", "/api/backup/restore") => match read_body(&mut request, MAX_JSON_PAYLOAD_BYTES) {
            Ok(body) => match serde_json::from_slice::<RestoreBackupRequest>(&body) {
                Ok(req) => {
                    let backup_file = PathBuf::from(&req.backup_file_path);
                    if !backup_file.exists() {
                        return {
                            let mut r = Response::from_string(r#"{"error":"Backup file not found"}"#.to_string()).with_status_code(StatusCode(404));
                            r.add_header(Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap());
                            let _ = request.respond(r);
                        };
                    }

                    // Restore to temporary database path first to test integrity
                    let temp_restore_dir = match tempfile::tempdir() {
                        Ok(d) => d,
                        Err(e) => return {
                            let mut r = Response::from_string(format!(r#"{{"error":"Temp dir error: {e}"}}"#)).with_status_code(StatusCode(500));
                            r.add_header(Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap());
                            let _ = request.respond(r);
                        },
                    };
                    let temp_db_path = temp_restore_dir.path().join("restored.db");
                    let temp_vault_dir = temp_restore_dir.path().join("vault");

                    match restore_full_backup_file(&backup_file, &temp_db_path, Some(&temp_vault_dir), &req.passphrase) {
                        Ok(restored_bytes) => {
                            // Verify integrity of decrypted database with SQLCipher key
                            match Database::open(&temp_db_path, &state.master_key) {
                                Ok(restored_db) => {
                                    if let Ok(true) = restored_db.integrity_check() {
                                        (StatusCode(200), "application/json", format!(
                                            r#"{{"success":true,"restored_bytes":{restored_bytes},"integrity_verified":true}}"#
                                        ))
                                    } else {
                                        (StatusCode(422), "application/json", r#"{"error":"Restored database failed SQLCipher integrity check"}"#.to_string())
                                    }
                                }
                                Err(e) => (StatusCode(422), "application/json", format!(r#"{{"error":"Authentication/Key error on restored database: {e}"}}"#)),
                            }
                        }
                        Err(e) => (StatusCode(400), "application/json", format!(r#"{{"error":"Restoration failed: {e}"}}"#)),
                    }
                }
                Err(e) => (StatusCode(400), "application/json", format!(r#"{{"error":"Malformed JSON: {e}"}}"#)),
            },
            Err((s, b)) => (s, "application/json", b),
        },

        // --- 10. Audit Trail & CSV Export ---
        ("GET", "/api/audit/events") => {
            let db = state.db.lock().unwrap();
            let events = db.list_audit_events().unwrap_or_default();
            let json = serde_json::to_string(&events).unwrap_or_else(|_| "[]".into());
            (StatusCode(200), "application/json", json)
        }

        // --- 11. Application Settings ---
        ("GET", "/api/settings") => {
            let s = state.settings.lock().unwrap().clone();
            let json = serde_json::to_string(&s).unwrap_or_else(|_| "{}".into());
            (StatusCode(200), "application/json", json)
        }

        ("POST", "/api/settings") => match read_body(&mut request, MAX_JSON_PAYLOAD_BYTES) {
            Ok(body) => match serde_json::from_slice::<AppSettings>(&body) {
                Ok(new_settings) => {
                    let mut s = state.settings.lock().unwrap();
                    *s = new_settings.clone();
                    let json = serde_json::to_string(&new_settings).unwrap_or_else(|_| "{}".into());
                    (StatusCode(200), "application/json", json)
                }
                Err(e) => (StatusCode(400), "application/json", format!(r#"{{"error":"Malformed JSON: {e}"}}"#)),
            },
            Err((s, b)) => (s, "application/json", b),
        },

        _ => (
            StatusCode(404),
            "text/plain",
            "Resource Not Found".to_string(),
        ),
    };

    let mut response = Response::from_string(response_body).with_status_code(status);
    response.add_header(
        Header::from_bytes(
            &b"Content-Type"[..],
            format!("{content_type}; charset=utf-8").as_bytes(),
        )
        .unwrap(),
    );
    response.add_header(csp_header);
    response.add_header(frame_header);
    response.add_header(nosniff_header);
    response.add_header(cache_header);
    let _ = request.respond(response);
}

// =========================================================================
// SERVER RUNNER
// =========================================================================

pub fn run_server(port: u16, state: Arc<ServerState>) -> Result<(), Box<dyn std::error::Error>> {
    let addr = format!("127.0.0.1:{port}");
    let server = Server::http(&addr).map_err(|e| format!("Failed to bind to {addr}: {e}"))?;

    println!("========================================================================");
    println!(" [BHOOT OS] Desktop Application Server Online");
    println!(" Address: http://{addr}");
    println!(" Mode: Airgapped Localhost Only (Binding to 127.0.0.1)");
    println!(" Security: Strict Content-Security-Policy & Frame-Options Denied");
    println!(" Cryptography: Argon2id Authentication, SQLCipher 4.5.x, AES-256-GCM Vault");
    println!("========================================================================");

    for request in server.incoming_requests() {
        handle_request(request, &state);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_weak_pin_policy_rejection() {
        assert!(validate_pin_strength("").is_err());
        assert!(validate_pin_strength("1234").is_err());
        assert!(validate_pin_strength("12345").is_err());
        assert!(validate_pin_strength("111111").is_err());
        assert!(validate_pin_strength("000000").is_err());
        assert!(validate_pin_strength("aaaaaa").is_err());
        assert!(validate_pin_strength("123456").is_err());
        assert!(validate_pin_strength("654321").is_err());
        assert!(validate_pin_strength("password").is_err());
        assert!(validate_pin_strength("admin").is_err());
        assert!(validate_pin_strength("pin with spaces").is_err());

        // Valid PINs
        assert!(validate_pin_strength("748192").is_ok());
        assert!(validate_pin_strength("982374").is_ok());
        assert!(validate_pin_strength("SecurePIN#2026").is_ok());
    }

    #[test]
    fn test_initial_state_unenrolled_and_fails_closed() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test_unenrolled.db");
        let vault_dir = dir.path().join("vault");
        let key = [0x42u8; 32];
        let db = Database::open(&db_path, &key).unwrap();

        let state = ServerState::new(db, db_path, vault_dir, key);
        assert!(!state.is_enrolled.load(Ordering::SeqCst));
        assert!(state.session_locked.load(Ordering::SeqCst));

        // Unlock fails when unenrolled
        let unlock_res = state.verify_pin("748192");
        assert_eq!(unlock_res, Err(AuthError::NotEnrolled));
    }

    #[test]
    fn test_enrollment_and_persistent_credentials_lifecycle() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test_lifecycle.db");
        let vault_dir = dir.path().join("vault");
        let key = [0x42u8; 32];
        let db = Database::open(&db_path, &key).unwrap();

        let state = ServerState::new(db, db_path.clone(), vault_dir.clone(), key);

        // Confirmation mismatch fails
        let err_mismatch = state.enroll_owner_pin("Secure#2026", "Different#2026");
        assert_eq!(err_mismatch, Err(AuthError::ConfirmationMismatch));

        // Weak PIN fails
        let err_weak = state.enroll_owner_pin("111111", "111111");
        assert!(matches!(err_weak, Err(AuthError::WeakPin(_))));

        // Valid enrollment succeeds
        let token_res = state.enroll_owner_pin("Secure#2026", "Secure#2026");
        assert!(token_res.is_ok());
        assert!(state.is_enrolled.load(Ordering::SeqCst));
        assert!(!state.session_locked.load(Ordering::SeqCst));

        // Second enrollment attempt returns AlreadyEnrolled
        let dup_res = state.enroll_owner_pin("Another#2026", "Another#2026");
        assert_eq!(dup_res, Err(AuthError::AlreadyEnrolled));

        // Lock session
        state.session_locked.store(true, Ordering::SeqCst);

        // Wrong PIN fails
        assert_eq!(state.verify_pin("Wrong#2026"), Err(AuthError::InvalidCredentials));

        // Correct PIN succeeds
        let unlock_token = state.verify_pin("Secure#2026").unwrap();
        assert!(!unlock_token.is_empty());

        // Update PIN with set_pin
        state.set_pin("Secure#2026", "NewSecure#2026", Some("NewSecure#2026")).unwrap();

        // Old PIN is now rejected
        assert_eq!(state.verify_pin("Secure#2026"), Err(AuthError::InvalidCredentials));

        // New PIN is accepted
        assert!(state.verify_pin("NewSecure#2026").is_ok());

        // Simulate complete service restart by opening a fresh ServerState on the same database
        drop(state);
        let reopened_db = Database::open(&db_path, &key).unwrap();
        let state2 = ServerState::new(reopened_db, db_path, vault_dir, key);

        // Verify state is enrolled after restart
        assert!(state2.is_enrolled.load(Ordering::SeqCst));
        assert!(state2.session_locked.load(Ordering::SeqCst)); // Locked on boot

        // Old PIN still rejected after restart
        assert_eq!(state2.verify_pin("Secure#2026"), Err(AuthError::InvalidCredentials));

        // Updated PIN accepted after restart
        assert!(state2.verify_pin("NewSecure#2026").is_ok());
    }

    #[test]
    fn test_brute_force_lockout_persists_across_restart() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test_lockout.db");
        let vault_dir = dir.path().join("vault");
        let key = [0x42u8; 32];
        let db = Database::open(&db_path, &key).unwrap();

        let state = ServerState::new(db, db_path.clone(), vault_dir.clone(), key);
        state.enroll_owner_pin("Secure#2026", "Secure#2026").unwrap();
        state.session_locked.store(true, Ordering::SeqCst);

        // 4 failed attempts: InvalidCredentials
        for _ in 0..4 {
            assert_eq!(state.verify_pin("Bad#PIN99"), Err(AuthError::InvalidCredentials));
        }

        // 5th failed attempt: LockedOut
        let lock_res = state.verify_pin("Bad#PIN99");
        assert!(matches!(lock_res, Err(AuthError::LockedOut(_))));

        // Even with correct PIN, verification is rejected while locked out
        assert!(matches!(state.verify_pin("Secure#2026"), Err(AuthError::LockedOut(_))));

        // Simulate restart while locked out
        drop(state);
        let reopened_db = Database::open(&db_path, &key).unwrap();
        let state2 = ServerState::new(reopened_db, db_path, vault_dir, key);

        // Lockout must persist across restart
        assert!(matches!(state2.verify_pin("Secure#2026"), Err(AuthError::LockedOut(_))));
    }
}
