pub mod config;
pub mod database;
pub mod error;
pub mod schema;

pub use config::SqlCipherConfig;
pub use database::{Database, TradingSummary, VaultDocumentSummary};
pub use error::DbError;
pub use schema::MIGRATION_001_SQL;

#[cfg(test)]
mod tests {
    use super::*;
    use bhoot_core::{
        Account, AccountSubtype, AccountType, Direction, Money, Posting, TradeCharges,
        TradeRecord, TradeType, Transaction, TransactionId, TransactionType,
    };
    use chrono::Utc;
    use std::fs;
    use tempfile::NamedTempFile;

    fn test_key_1() -> [u8; 32] {
        [0x42u8; 32]
    }

    fn test_key_2() -> [u8; 32] {
        [0x99u8; 32]
    }

    #[test]
    fn test_sqlcipher_encryption_and_wrong_key_rejection() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path().to_path_buf();

        // 1. Create and populate encrypted database with key 1
        {
            let db = Database::open(&db_path, &test_key_1()).expect("Must open with key 1");
            let acc = Account::new(
                "Test Bank HDFC",
                AccountType::Asset,
                AccountSubtype::BankAccount,
                Some("Primary checking".into()),
            );
            db.create_account(&acc).expect("Must insert account");

            let fetched = db
                .get_account(acc.id)
                .expect("Query succeeds")
                .expect("Account found");
            assert_eq!(fetched.name, "Test Bank HDFC");
        }

        // 2. Attempt to open with incorrect key 2 -> MUST FAIL AUTHENTICATION
        {
            let res = Database::open(&db_path, &test_key_2());
            assert!(
                res.is_err(),
                "Opening encrypted database with wrong key must fail!"
            );
            match res {
                Err(DbError::AuthenticationFailed(_)) | Err(DbError::KeyError(_)) => {}
                other => panic!("Expected authentication/key failure, got {other:?}"),
            }
        }

        // 3. Reopen with correct key 1 -> MUST SUCCEED AND RETAIN DATA
        {
            let db = Database::open(&db_path, &test_key_1()).expect("Reopen with key 1 must work");
            let accounts = db.list_accounts().expect("Must list accounts");
            assert_eq!(accounts.len(), 1);
            assert_eq!(accounts[0].name, "Test Bank HDFC");
            assert!(db.integrity_check().expect("Integrity check succeeds"));
        }
    }

    #[test]
    fn test_ondisk_ciphertext_inspection_zero_plaintext() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path().to_path_buf();

        let secret_account_name = "CONFIDENTIAL_SWISS_VAULT_777";
        let secret_note = "PRIVATE_SECRET_INVESTMENT_MEMO_999";

        {
            let db = Database::open(&db_path, &test_key_1()).unwrap();
            let acc = Account::new(
                secret_account_name,
                AccountType::Asset,
                AccountSubtype::InvestmentAccount,
                Some(secret_note.into()),
            );
            db.create_account(&acc).unwrap();
            db.vacuum().unwrap();
        }

        // Read raw bytes directly from physical disk file
        let raw_bytes = fs::read(&db_path).unwrap();
        assert!(raw_bytes.len() >= 4096, "Database file must have allocated pages");

        let raw_str = String::from_utf8_lossy(&raw_bytes);

        // Verify standard SQLite plaintext magic header is NOT present
        assert!(
            !raw_bytes.starts_with(b"SQLite format 3\0"),
            "SQLCipher file must NOT have plaintext SQLite magic header!"
        );

        // Verify zero plaintext leakage of data or schema strings
        assert!(
            !raw_str.contains(secret_account_name),
            "Plaintext account name found in database file!"
        );
        assert!(
            !raw_str.contains(secret_note),
            "Plaintext note found in database file!"
        );
        assert!(
            !raw_str.contains("CREATE TABLE"),
            "Plaintext schema DDL found in encrypted database file!"
        );
        assert!(
            !raw_str.contains("accounts"),
            "Plaintext table name found in encrypted database file!"
        );

        // Verify high Shannon entropy (close to 8.0 for ciphertext)
        let mut freq = [0usize; 256];
        for &b in &raw_bytes {
            freq[b as usize] += 1;
        }
        let total = raw_bytes.len() as f64;
        let mut entropy = 0.0f64;
        for &c in &freq {
            if c > 0 {
                let p = c as f64 / total;
                entropy -= p * p.log2();
            }
        }

        assert!(
            entropy > 7.90,
            "Ciphertext entropy must be high (got {entropy:.3} bits/byte)"
        );
    }

    #[test]
    fn test_double_entry_transaction_posting_and_balance() {
        let db = Database::open_in_memory(&test_key_1()).unwrap();

        let bank = Account::new("Checking", AccountType::Asset, AccountSubtype::BankAccount, None);
        let grocery = Account::new(
            "Supermarket",
            AccountType::Expense,
            AccountSubtype::LivingExpense,
            None,
        );

        db.create_account(&bank).unwrap();
        db.create_account(&grocery).unwrap();

        // 1. Post valid balanced transaction (500 INR = 50,000 paisa)
        let postings = vec![
            Posting::new(
                grocery.id,
                Money::from_paisa(50_000),
                Direction::Debit,
                Some("Weekly groceries".into()),
            )
            .unwrap(),
            Posting::new(
                bank.id,
                Money::from_paisa(50_000),
                Direction::Credit,
                Some("Card payment".into()),
            )
            .unwrap(),
        ];

        let tx = Transaction::new(
            Utc::now(),
            TransactionType::Expense,
            postings,
            Some("Food".into()),
            None,
            None,
            Some("Paid via debit card".into()),
            vec![],
            None,
        )
        .unwrap();

        db.post_transaction(&tx).expect("Must post balanced transaction");

        // 2. Verify account balances
        // Bank (Asset, Normal=Debit): 0 Debit - 50,000 Credit = -50,000 paisa
        let bank_bal = db.get_account_balance(bank.id).unwrap();
        assert_eq!(bank_bal.as_paisa(), -50_000);

        // Grocery (Expense, Normal=Debit): 50,000 Debit - 0 Credit = +50,000 paisa
        let grocery_bal = db.get_account_balance(grocery.id).unwrap();
        assert_eq!(grocery_bal.as_paisa(), 50_000);

        // 3. Reject unbalanced transaction (Debits 50,000 != Credits 40,000)
        let unbalanced_postings = vec![
            Posting::new(grocery.id, Money::from_paisa(50_000), Direction::Debit, None).unwrap(),
            Posting::new(bank.id, Money::from_paisa(40_000), Direction::Credit, None).unwrap(),
        ];

        let unbalanced_tx = Transaction {
            id: TransactionId::new_v4(),
            timestamp: Utc::now(),
            tx_type: TransactionType::Expense,
            postings: unbalanced_postings,
            category: None,
            payment_method: None,
            reference_number: None,
            notes: None,
            proof_attachment_ids: vec![],
            verification_status: bhoot_core::VerificationStatus::Pending,
            reversal_of: None,
        };

        let post_res = db.post_transaction(&unbalanced_tx);
        assert!(post_res.is_err(), "Unbalanced transaction must be rejected!");
    }

    #[test]
    fn test_traceable_reversal_workflow() {
        let db = Database::open_in_memory(&test_key_1()).unwrap();

        let salary_income = Account::new(
            "Employer",
            AccountType::Income,
            AccountSubtype::Salary,
            None,
        );
        let bank = Account::new("Checking", AccountType::Asset, AccountSubtype::BankAccount, None);

        db.create_account(&salary_income).unwrap();
        db.create_account(&bank).unwrap();

        // Income: Debit Bank 100,000 INR, Credit Salary 100,000 INR
        let amount = Money::from_paisa(100_000 * 100);
        let postings = vec![
            Posting::new(bank.id, amount, Direction::Debit, Some("Salary payout".into())).unwrap(),
            Posting::new(salary_income.id, amount, Direction::Credit, Some("Salary accrual".into())).unwrap(),
        ];

        let tx = Transaction::new(
            Utc::now(),
            TransactionType::Income,
            postings,
            Some("Payroll".into()),
            None,
            None,
            None,
            vec![],
            None,
        )
        .unwrap();

        db.post_transaction(&tx).unwrap();

        assert_eq!(db.get_account_balance(bank.id).unwrap().as_paisa(), 100_000_00);
        assert_eq!(db.get_account_balance(salary_income.id).unwrap().as_paisa(), 100_000_00);

        // Execute reversal
        let rev_tx = db
            .reverse_transaction(tx.id, "Erroneous duplicate transfer", Utc::now())
            .expect("Reversal succeeds");

        assert_eq!(rev_tx.reversal_of, Some(tx.id));
        assert_eq!(rev_tx.tx_type, TransactionType::Reversal);

        // Balances must return to exactly zero
        assert_eq!(db.get_account_balance(bank.id).unwrap().as_paisa(), 0);
        assert_eq!(db.get_account_balance(salary_income.id).unwrap().as_paisa(), 0);

        // Attempting to reverse the transaction again must fail
        let duplicate_rev = db.reverse_transaction(tx.id, "Second attempt", Utc::now());
        assert!(duplicate_rev.is_err(), "Cannot reverse already reversed transaction");

        // Attempting to reverse the reversal itself must fail
        let rev_of_rev = db.reverse_transaction(rev_tx.id, "Reverse reversal", Utc::now());
        assert!(rev_of_rev.is_err(), "Cannot reverse a reversal transaction");
    }

    #[test]
    fn test_trading_journal_metrics() {
        let db = Database::open_in_memory(&test_key_1()).unwrap();

        let broker = Account::new("Zerodha", AccountType::Asset, AccountSubtype::BrokerAccount, None);
        db.create_account(&broker).unwrap();

        // Trade 1: +10,000 INR gross, 500 charges -> +9,500 net
        let t1 = TradeRecord::new(
            broker.id,
            Utc::now(),
            "NIFTY26NOV24500CE",
            TradeType::Options,
            Money::from_paisa(10_000 * 100),
            TradeCharges {
                brokerage: Money::from_paisa(500 * 100),
                ..Default::default()
            },
            Money::from_paisa(9_500 * 100),
            Some("ORD1001".into()),
            None,
        )
        .expect("Valid trade record 1");

        // Trade 2: -4,000 INR gross, 300 charges -> -4,300 net
        let t2 = TradeRecord::new(
            broker.id,
            Utc::now(),
            "BANKNIFTY26NOV52000PE",
            TradeType::Options,
            Money::from_paisa(-4_000 * 100),
            TradeCharges {
                brokerage: Money::from_paisa(300 * 100),
                ..Default::default()
            },
            Money::from_paisa(-4_300 * 100),
            Some("ORD1002".into()),
            None,
        )
        .expect("Valid trade record 2");

        db.record_trade(&t1).unwrap();
        db.record_trade(&t2).unwrap();

        let summary = db.get_trading_summary(Some(broker.id)).unwrap();
        assert_eq!(summary.total_trades, 2);
        assert_eq!(summary.winning_trades, 1);
        assert_eq!(summary.losing_trades, 1);
        assert_eq!(summary.win_rate_basis_points, 5000); // 50.00%
        assert_eq!(summary.net_pnl_paisa, 5_200_00); // +5,200 INR
        assert_eq!(summary.total_charges_paisa, 800_00); // 800 INR
    }
}
