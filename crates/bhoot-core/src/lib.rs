pub mod account;
pub mod error;
pub mod ledger;
pub mod money;
pub mod trading;
pub mod transaction;

pub use account::{Account, AccountId, AccountStatus, AccountSubtype, AccountType, NormalBalance};
pub use error::CoreError;
pub use ledger::Ledger;
pub use money::Money;
pub use trading::{
    BrokerAccount, PayoutRecord, PayoutStatus, TradeCharges, TradeRecord, TradeType,
};
pub use transaction::{
    Direction, PaymentMethod, Posting, Transaction, TransactionId, TransactionType,
    VerificationStatus,
};

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use proptest::prelude::*;

    #[test]
    fn test_double_entry_transfer_neutrality_and_reversal() {
        let mut ledger = Ledger::new();

        // 1. Setup accounts
        let bank_account = Account::new(
            "HDFC Bank",
            AccountType::Asset,
            AccountSubtype::BankAccount,
            None,
        );
        let broker_account = Account::new(
            "Zerodha Trading",
            AccountType::Asset,
            AccountSubtype::BrokerAccount,
            None,
        );
        let salary_account = Account::new(
            "Salary Income",
            AccountType::Income,
            AccountSubtype::Salary,
            None,
        );
        let rent_account = Account::new(
            "Office Rent",
            AccountType::Expense,
            AccountSubtype::LivingExpense,
            None,
        );

        let bank_id = bank_account.id;
        let broker_id = broker_account.id;
        let salary_id = salary_account.id;
        let rent_id = rent_account.id;

        ledger.add_account(bank_account).expect("added");
        ledger.add_account(broker_account).expect("added");
        ledger.add_account(salary_account).expect("added");
        ledger.add_account(rent_account).expect("added");

        // 2. Post Salary Income: ₹2,00,000
        // Debit: HDFC Bank (₹2,00,000)
        // Credit: Salary Income (₹2,00,000)
        let salary_tx = Transaction::new(
            Utc::now(),
            TransactionType::Income,
            vec![
                Posting::new(
                    bank_id,
                    Money::from_paisa(20_000_000),
                    Direction::Debit,
                    Some("Salary credit".into()),
                )
                .unwrap(),
                Posting::new(
                    salary_id,
                    Money::from_paisa(20_000_000),
                    Direction::Credit,
                    Some("Salary revenue".into()),
                )
                .unwrap(),
            ],
            Some("Salary".into()),
            Some(PaymentMethod::NEFT),
            Some("CMS123456789".into()),
            None,
            Vec::new(),
            None,
        )
        .expect("valid tx");

        ledger.post_transaction(salary_tx).expect("posted");

        assert_eq!(
            ledger.get_balance(&bank_id).unwrap(),
            Money::from_paisa(20_000_000)
        );
        assert_eq!(
            ledger.get_balance(&salary_id).unwrap(),
            Money::from_paisa(20_000_000)
        );
        assert_eq!(
            ledger.get_net_worth().unwrap(),
            Money::from_paisa(20_000_000)
        );
        assert_eq!(
            ledger.get_total_income().unwrap(),
            Money::from_paisa(20_000_000)
        );
        assert_eq!(ledger.get_total_expenses().unwrap(), Money::ZERO);

        // 3. Test Transfer Neutrality: Move ₹50,000 from Bank to Broker
        let is_neutral = ledger
            .verify_transfer_neutrality(
                bank_id,
                broker_id,
                Money::from_paisa(5_000_000),
                Utc::now(),
            )
            .expect("neutral check");
        assert!(
            is_neutral,
            "Transfer between assets must have zero net income/expense effect"
        );

        // Execute actual transfer
        let transfer_tx = Transaction::new(
            Utc::now(),
            TransactionType::Transfer,
            vec![
                Posting::new(
                    bank_id,
                    Money::from_paisa(5_000_000),
                    Direction::Credit,
                    Some("Fund broker".into()),
                )
                .unwrap(),
                Posting::new(
                    broker_id,
                    Money::from_paisa(5_000_000),
                    Direction::Debit,
                    Some("Receive funds".into()),
                )
                .unwrap(),
            ],
            Some("Transfer".into()),
            Some(PaymentMethod::NetBanking),
            Some("UTR987654321".into()),
            None,
            Vec::new(),
            None,
        )
        .expect("valid transfer");

        ledger
            .post_transaction(transfer_tx)
            .expect("posted transfer");

        // Balances updated:
        assert_eq!(
            ledger.get_balance(&bank_id).unwrap(),
            Money::from_paisa(15_000_000)
        );
        assert_eq!(
            ledger.get_balance(&broker_id).unwrap(),
            Money::from_paisa(5_000_000)
        );
        // Net worth and income unchanged:
        assert_eq!(
            ledger.get_net_worth().unwrap(),
            Money::from_paisa(20_000_000)
        );
        assert_eq!(
            ledger.get_total_income().unwrap(),
            Money::from_paisa(20_000_000)
        );
        assert_eq!(ledger.get_total_expenses().unwrap(), Money::ZERO);

        // 4. Post Rent Expense: ₹30,000
        let rent_tx = Transaction::new(
            Utc::now(),
            TransactionType::Expense,
            vec![
                Posting::new(
                    bank_id,
                    Money::from_paisa(3_000_000),
                    Direction::Credit,
                    Some("Rent payment".into()),
                )
                .unwrap(),
                Posting::new(
                    rent_id,
                    Money::from_paisa(3_000_000),
                    Direction::Debit,
                    Some("Rent expense".into()),
                )
                .unwrap(),
            ],
            Some("Rent".into()),
            Some(PaymentMethod::UPI),
            Some("UPI000111222".into()),
            None,
            Vec::new(),
            None,
        )
        .expect("valid rent tx");

        let rent_tx_id = rent_tx.id;
        ledger.post_transaction(rent_tx).expect("posted rent");

        assert_eq!(
            ledger.get_balance(&bank_id).unwrap(),
            Money::from_paisa(12_000_000)
        );
        assert_eq!(
            ledger.get_balance(&rent_id).unwrap(),
            Money::from_paisa(3_000_000)
        );
        assert_eq!(
            ledger.get_net_worth().unwrap(),
            Money::from_paisa(17_000_000)
        );
        assert_eq!(
            ledger.get_total_expenses().unwrap(),
            Money::from_paisa(3_000_000)
        );
        assert_eq!(
            ledger.get_net_income().unwrap(),
            Money::from_paisa(17_000_000)
        );

        // 5. Reversal of Rent Transaction (Erroneous payment refunded/reversed)
        let original_rent = ledger
            .transactions()
            .iter()
            .find(|t| t.id == rent_tx_id)
            .unwrap();
        let reversal_tx = original_rent
            .create_reversal(Utc::now(), "Wrong account specified")
            .expect("reversal created");
        ledger
            .post_transaction(reversal_tx)
            .expect("reversal posted");

        // Verify balances restored after reversal
        assert_eq!(
            ledger.get_balance(&bank_id).unwrap(),
            Money::from_paisa(15_000_000)
        );
        assert_eq!(ledger.get_balance(&rent_id).unwrap(), Money::ZERO);
        assert_eq!(ledger.get_total_expenses().unwrap(), Money::ZERO);
        assert_eq!(
            ledger.get_net_worth().unwrap(),
            Money::from_paisa(20_000_000)
        );

        // 6. Global Accounting Equation and Equilibrium verification
        ledger
            .verify_all_invariants()
            .expect("All invariants must hold");
    }

    #[test]
    fn test_trade_record_and_payout_invariants() {
        let broker_acc_id = AccountId::new_v4();
        let bank_acc_id = AccountId::new_v4();

        // 1. Valid Trade Record
        let charges = TradeCharges {
            brokerage: Money::from_paisa(4000),                // ₹40.00
            stt: Money::from_paisa(2500),                      // ₹25.00
            exchange_turnover_charges: Money::from_paisa(700), // ₹7.00
            gst: Money::from_paisa(846),                       // ₹8.46
            sebi_charges: Money::from_paisa(20),               // ₹0.20
            stamp_duty: Money::from_paisa(300),                // ₹3.00
        };
        let total_charges = charges.total_charges().unwrap(); // ₹83.66 (8366 paisa)
        assert_eq!(total_charges.as_paisa(), 8366);

        let gross_pnl = Money::from_paisa(100_000); // ₹1,000.00 profit
        let net_pnl = Money::from_paisa(100_000 - 8366); // ₹916.34

        let trade = TradeRecord::new(
            broker_acc_id,
            Utc::now(),
            "NIFTY26NOV25000CE",
            TradeType::Options,
            gross_pnl,
            charges,
            net_pnl,
            Some("ORD_1001".into()),
            None,
        )
        .expect("valid trade");

        assert_eq!(trade.net_pnl.as_paisa(), 91634);

        // Discrepant net P&L must fail validation
        let bad_trade = TradeRecord::new(
            broker_acc_id,
            Utc::now(),
            "NIFTY26NOV25000CE",
            TradeType::Options,
            gross_pnl,
            charges,
            Money::from_paisa(95000), // Bad net P&L
            None,
            None,
        );
        assert!(bad_trade.is_err());

        // 2. Valid Payout Record
        let requested = Money::from_paisa(5_000_000); // ₹50,000.00
        let tds = Money::from_paisa(500_000); // ₹5,000.00 (10% TDS)
        let net_credited = Money::from_paisa(4_500_000); // ₹45,000.00

        let payout = PayoutRecord::new(
            broker_acc_id,
            bank_acc_id,
            Utc::now(),
            Some(Utc::now()),
            requested,
            tds,
            net_credited,
            Some("HDFC_UTR_999".into()),
            PayoutStatus::Settled,
        )
        .expect("valid payout");

        assert_eq!(payout.net_credited_amount.as_paisa(), 4_500_000);

        // Discrepant Payout must fail validation
        let bad_payout = PayoutRecord::new(
            broker_acc_id,
            bank_acc_id,
            Utc::now(),
            None,
            requested,
            tds,
            Money::from_paisa(4_000_000), // Invariant violated
            None,
            PayoutStatus::Requested,
        );
        assert!(bad_payout.is_err());
    }

    // Property-based test: Any arbitrarily generated balanced transaction preserves equilibrium
    proptest! {
        #[test]
        fn prop_balanced_transaction_preserves_equilibrium(
            amount1 in 1i64..10_000_000,
            amount2 in 1i64..10_000_000,
        ) {
            let mut ledger = Ledger::new();
            let acc1 = Account::new("A1", AccountType::Asset, AccountSubtype::BankAccount, None);
            let acc2 = Account::new("A2", AccountType::Asset, AccountSubtype::CashWallet, None);
            let acc3 = Account::new("A3", AccountType::Income, AccountSubtype::BusinessRevenue, None);

            let id1 = acc1.id;
            let id2 = acc2.id;
            let id3 = acc3.id;

            ledger.add_account(acc1).unwrap();
            ledger.add_account(acc2).unwrap();
            ledger.add_account(acc3).unwrap();

            let m1 = Money::from_paisa(amount1);
            let m2 = Money::from_paisa(amount2);
            let total = m1.checked_add(m2).unwrap();

            // Transaction: Debit acc1 (m1), Debit acc2 (m2), Credit acc3 (m1 + m2)
            let tx = Transaction::new(
                Utc::now(),
                TransactionType::Income,
                vec![
                    Posting::new(id1, m1, Direction::Debit, None).unwrap(),
                    Posting::new(id2, m2, Direction::Debit, None).unwrap(),
                    Posting::new(id3, total, Direction::Credit, None).unwrap(),
                ],
                None,
                None,
                None,
                None,
                Vec::new(),
                None,
            ).unwrap();

            ledger.post_transaction(tx).unwrap();
            prop_assert!(ledger.verify_all_invariants().is_ok());
        }
    }
}
