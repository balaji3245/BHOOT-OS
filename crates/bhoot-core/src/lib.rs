pub mod account;
pub mod budget;
pub mod error;
pub mod export;
pub mod ledger;
pub mod money;
pub mod report;
pub mod trading;
pub mod transaction;

pub use account::{Account, AccountId, AccountStatus, AccountSubtype, AccountType, NormalBalance};
pub use budget::{
    Budget, BudgetPeriod, BudgetReport, BudgetStatus, CategoryBudget, CategoryBudgetReport,
};
pub use error::CoreError;
pub use export::{
    export_trades_csv, export_transactions_csv, sanitize_csv_cell, CSV_SECURITY_BANNER,
};
pub use ledger::Ledger;
pub use money::Money;
pub use report::{AccountSummaryItem, BalanceSheet, CashFlowStatement, IncomeStatement};
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

        #[test]
        fn prop_transaction_reversal_balance_neutrality(
            amount in 1i64..50_000_000,
        ) {
            let mut ledger = Ledger::new();
            let bank = Account::new("Bank", AccountType::Asset, AccountSubtype::BankAccount, None);
            let salary = Account::new("Salary", AccountType::Income, AccountSubtype::Salary, None);

            let bank_id = bank.id;
            let salary_id = salary.id;

            ledger.add_account(bank).unwrap();
            ledger.add_account(salary).unwrap();

            let m = Money::from_paisa(amount);
            let tx = Transaction::new(
                Utc::now(),
                TransactionType::Income,
                vec![
                    Posting::new(bank_id, m, Direction::Debit, None).unwrap(),
                    Posting::new(salary_id, m, Direction::Credit, None).unwrap(),
                ],
                None, None, None, None, Vec::new(), None,
            ).unwrap();

            ledger.post_transaction(tx.clone()).unwrap();
            prop_assert_eq!(ledger.get_balance(&bank_id).unwrap(), m);

            let rev = tx.create_reversal(Utc::now(), "Reversal").unwrap();
            ledger.post_transaction(rev).unwrap();

            prop_assert_eq!(ledger.get_balance(&bank_id).unwrap(), Money::ZERO);
            prop_assert_eq!(ledger.get_balance(&salary_id).unwrap(), Money::ZERO);
            prop_assert!(ledger.verify_all_invariants().is_ok());
        }
    }

    #[test]
    fn test_duplicate_reversal_rejected_high_02() {
        // REGRESSION TEST FOR HIGH-02:
        // A historical transaction can only be reversed once.
        // Duplicate reversal attempts must be rejected and leave ledger state completely unaltered.
        let mut ledger = Ledger::new();
        let bank = Account::new(
            "Bank",
            AccountType::Asset,
            AccountSubtype::BankAccount,
            None,
        );
        let expense = Account::new(
            "Expense",
            AccountType::Expense,
            AccountSubtype::LivingExpense,
            None,
        );

        let bank_id = bank.id;
        let expense_id = expense.id;

        ledger.add_account(bank).unwrap();
        ledger.add_account(expense).unwrap();

        let tx = Transaction::new(
            Utc::now(),
            TransactionType::Expense,
            vec![
                Posting::new(bank_id, Money::from_paisa(100_000), Direction::Credit, None).unwrap(),
                Posting::new(
                    expense_id,
                    Money::from_paisa(100_000),
                    Direction::Debit,
                    None,
                )
                .unwrap(),
            ],
            None,
            None,
            None,
            None,
            Vec::new(),
            None,
        )
        .unwrap();

        ledger.post_transaction(tx.clone()).unwrap();

        // First reversal must succeed
        let rev1 = tx.create_reversal(Utc::now(), "First reversal").unwrap();
        ledger
            .post_transaction(rev1)
            .expect("First reversal succeeds");
        assert!(ledger.is_reversed(&tx.id));
        assert_eq!(ledger.get_balance(&bank_id).unwrap(), Money::ZERO);

        // Second reversal of the same original transaction MUST fail
        let rev2 = tx.create_reversal(Utc::now(), "Second reversal").unwrap();
        let err = ledger.post_transaction(rev2);
        assert!(matches!(err, Err(CoreError::InvalidReversal(_))));

        // Ledger state must remain unchanged after failed duplicate reversal
        assert_eq!(ledger.get_balance(&bank_id).unwrap(), Money::ZERO);
        assert_eq!(ledger.transactions().len(), 2);
    }

    #[test]
    fn test_reversal_of_reversal_rejected_med_02() {
        // REGRESSION TEST FOR MED-02:
        // Cannot reverse a transaction that is itself a reversal.
        let mut ledger = Ledger::new();
        let bank = Account::new(
            "Bank",
            AccountType::Asset,
            AccountSubtype::BankAccount,
            None,
        );
        let salary = Account::new("Salary", AccountType::Income, AccountSubtype::Salary, None);

        let bank_id = bank.id;
        let salary_id = salary.id;

        ledger.add_account(bank).unwrap();
        ledger.add_account(salary).unwrap();

        let tx = Transaction::new(
            Utc::now(),
            TransactionType::Income,
            vec![
                Posting::new(bank_id, Money::from_paisa(500_000), Direction::Debit, None).unwrap(),
                Posting::new(
                    salary_id,
                    Money::from_paisa(500_000),
                    Direction::Credit,
                    None,
                )
                .unwrap(),
            ],
            None,
            None,
            None,
            None,
            Vec::new(),
            None,
        )
        .unwrap();

        ledger.post_transaction(tx.clone()).unwrap();
        let rev = tx.create_reversal(Utc::now(), "First reversal").unwrap();
        ledger.post_transaction(rev.clone()).unwrap();

        // Attempting to reverse the reversal must be rejected
        let rev_of_rev = rev
            .create_reversal(Utc::now(), "Reversing reversal")
            .unwrap();
        let err = ledger.post_transaction(rev_of_rev);
        assert!(matches!(err, Err(CoreError::InvalidReversal(_))));
    }

    #[test]
    fn test_transfer_semantic_validation_med_01() {
        // REGRESSION TEST FOR MED-01:
        // Transfer transactions represent reallocation across balance-sheet accounts (Asset/Liability)
        // and must strictly reject Income and Expense legs.
        let mut ledger = Ledger::new();
        let bank = Account::new(
            "Bank",
            AccountType::Asset,
            AccountSubtype::BankAccount,
            None,
        );
        let credit_card = Account::new(
            "Credit Card",
            AccountType::Liability,
            AccountSubtype::CreditCard,
            None,
        );
        let income = Account::new(
            "Income",
            AccountType::Income,
            AccountSubtype::OtherIncome,
            None,
        );

        let bank_id = bank.id;
        let cc_id = credit_card.id;
        let income_id = income.id;

        ledger.add_account(bank).unwrap();
        ledger.add_account(credit_card).unwrap();
        ledger.add_account(income).unwrap();

        // 1. Valid Transfer: Paying credit card liability from bank asset
        let valid_transfer = Transaction::new(
            Utc::now(),
            TransactionType::Transfer,
            vec![
                Posting::new(bank_id, Money::from_paisa(250_000), Direction::Credit, None).unwrap(),
                Posting::new(cc_id, Money::from_paisa(250_000), Direction::Debit, None).unwrap(),
            ],
            None,
            None,
            None,
            None,
            Vec::new(),
            None,
        )
        .unwrap();
        assert!(ledger.post_transaction(valid_transfer).is_ok());

        // 2. Invalid Transfer: Transfer attempting to include an Income account leg
        let invalid_transfer = Transaction::new(
            Utc::now(),
            TransactionType::Transfer,
            vec![
                Posting::new(bank_id, Money::from_paisa(100_000), Direction::Debit, None).unwrap(),
                Posting::new(
                    income_id,
                    Money::from_paisa(100_000),
                    Direction::Credit,
                    None,
                )
                .unwrap(),
            ],
            None,
            None,
            None,
            None,
            Vec::new(),
            None,
        )
        .unwrap();
        let err = ledger.post_transaction(invalid_transfer);
        assert!(matches!(err, Err(CoreError::InvalidTransferAccount { .. })));
    }

    #[test]
    fn test_negative_fractional_paisa_low_01() {
        // REGRESSION TEST FOR LOW-01:
        // Explicit signed constructor allows unambiguous creation of negative fractional amounts (e.g. -₹0.50).
        let neg_half_rupee = Money::from_rupees_and_paisa_signed(true, 0, 50).expect("valid");
        assert_eq!(neg_half_rupee.as_paisa(), -50);
        assert_eq!(neg_half_rupee.format_inr(), "-₹0.50");

        let pos_half_rupee = Money::from_rupees_and_paisa_signed(false, 0, 50).expect("valid");
        assert_eq!(pos_half_rupee.as_paisa(), 50);
        assert_eq!(pos_half_rupee.format_inr(), "₹0.50");
    }

    #[test]
    fn test_budget_allocation_and_utilization_evaluation() {
        use std::collections::HashMap;

        let start = Utc::now();
        let end = start + chrono::Duration::days(30);

        let groceries = CategoryBudget::new("Groceries", Money::from_paisa(20_000 * 100), 80).unwrap();
        let rent = CategoryBudget::new("Rent", Money::from_paisa(40_000 * 100), 90).unwrap();
        let dining = CategoryBudget::new("Dining", Money::from_paisa(10_000 * 100), 75).unwrap();

        let budget = Budget::new(
            "November 2026 Household",
            BudgetPeriod::Monthly,
            start,
            end,
            vec![groceries, rent, dining],
            Some("Essential & lifestyle expenses".into()),
        )
        .unwrap();

        assert_eq!(budget.total_allocated().unwrap().as_paisa(), 70_000 * 100);

        let mut spending = HashMap::new();
        // Groceries: 15,000 / 20,000 = 75% -> OnTrack (< 80%)
        spending.insert("Groceries".into(), Money::from_paisa(15_000 * 100));
        // Rent: 36,000 / 40,000 = 90% -> Warning (>= 90%)
        spending.insert("Rent".into(), Money::from_paisa(36_000 * 100));
        // Dining: 12,000 / 10,000 = 120% -> Exceeded (>= 100%)
        spending.insert("Dining".into(), Money::from_paisa(12_000 * 100));

        let report = budget.evaluate_spending(&spending).unwrap();
        assert_eq!(report.total_spent.as_paisa(), 63_000 * 100);
        assert_eq!(report.total_remaining.as_paisa(), 7_000 * 100);
        assert_eq!(report.overall_utilization_basis_points, 9000); // 90.00%

        let groc_rep = report.category_reports.iter().find(|r| r.category == "Groceries").unwrap();
        assert_eq!(groc_rep.status, BudgetStatus::OnTrack);
        assert_eq!(groc_rep.utilization_basis_points, 7500);

        let rent_rep = report.category_reports.iter().find(|r| r.category == "Rent").unwrap();
        assert_eq!(rent_rep.status, BudgetStatus::Warning);
        assert_eq!(rent_rep.utilization_basis_points, 9000);

        let din_rep = report.category_reports.iter().find(|r| r.category == "Dining").unwrap();
        assert_eq!(din_rep.status, BudgetStatus::Exceeded);
        assert_eq!(din_rep.utilization_basis_points, 12000);
    }

    #[test]
    fn test_financial_reporting_balance_sheet_and_pnl() {
        let now = Utc::now();
        let hdfc_id = AccountId::new_v4();
        let zerodha_id = AccountId::new_v4();
        let cc_id = AccountId::new_v4();
        let equity_id = AccountId::new_v4();

        let assets = vec![
            AccountSummaryItem {
                account_id: hdfc_id,
                name: "HDFC Bank".into(),
                subtype: AccountSubtype::BankAccount,
                balance: Money::from_paisa(500_000 * 100),
            },
            AccountSummaryItem {
                account_id: zerodha_id,
                name: "Zerodha Demat".into(),
                subtype: AccountSubtype::BrokerAccount,
                balance: Money::from_paisa(300_000 * 100),
            },
        ];

        let liabilities = vec![AccountSummaryItem {
            account_id: cc_id,
            name: "HDFC Credit Card".into(),
            subtype: AccountSubtype::CreditCard,
            balance: Money::from_paisa(100_000 * 100),
        }];

        let equity = vec![AccountSummaryItem {
            account_id: equity_id,
            name: "Owner Equity".into(),
            subtype: AccountSubtype::RetainedEarnings,
            balance: Money::from_paisa(500_000 * 100),
        }];

        // Total Assets = 800,000
        // Total Liabilities = 100,000
        // Net Worth = 700,000
        // Claims: Liabilities(100k) + Equity(500k) + RetainedEarnings(200k) = 800k -> Balanced!
        let bs = BalanceSheet::new(
            now,
            assets,
            liabilities,
            equity,
            Money::from_paisa(200_000 * 100),
        )
        .unwrap();

        assert_eq!(bs.total_assets.as_paisa(), 800_000 * 100);
        assert_eq!(bs.total_liabilities.as_paisa(), 100_000 * 100);
        assert_eq!(bs.net_worth.as_paisa(), 700_000 * 100);
        assert!(bs.is_balanced);

        // P&L
        let op_inc = vec![AccountSummaryItem {
            account_id: AccountId::new_v4(),
            name: "Consulting Salary".into(),
            subtype: AccountSubtype::Salary,
            balance: Money::from_paisa(200_000 * 100),
        }];
        let op_exp = vec![AccountSummaryItem {
            account_id: AccountId::new_v4(),
            name: "Office Rent".into(),
            subtype: AccountSubtype::LivingExpense,
            balance: Money::from_paisa(50_000 * 100),
        }];

        let pnl = IncomeStatement::new(
            now - chrono::Duration::days(30),
            now,
            op_inc,
            Money::from_paisa(50_000 * 100), // Trading gross
            Money::from_paisa(5_000 * 100),  // Trading charges
            op_exp,
        )
        .unwrap();

        assert_eq!(pnl.trading_net_pnl.as_paisa(), 45_000 * 100);
        assert_eq!(pnl.total_income.as_paisa(), 245_000 * 100);
        assert_eq!(pnl.net_profit.as_paisa(), 195_000 * 100); // 245k - 50k
    }

    #[test]
    fn test_csv_export_security_sanitization_and_checksum() {
        let malicious_cell_1 = "=cmd|' /C calc'!A0";
        let malicious_cell_2 = "+2+3";
        let safe_cell = "Safe Description";

        assert_eq!(sanitize_csv_cell(malicious_cell_1), "'=cmd|' /C calc'!A0");
        assert_eq!(sanitize_csv_cell(malicious_cell_2), "'+2+3");
        assert_eq!(sanitize_csv_cell(safe_cell), "Safe Description");

        // Verify CSV export content
        let tx = Transaction::new(
            Utc::now(),
            TransactionType::Income,
            vec![
                Posting::new(
                    AccountId::new_v4(),
                    Money::from_paisa(10_000 * 100),
                    Direction::Debit,
                    Some("=1+1".into()),
                )
                .unwrap(),
                Posting::new(
                    AccountId::new_v4(),
                    Money::from_paisa(10_000 * 100),
                    Direction::Credit,
                    None,
                )
                .unwrap(),
            ],
            Some("Bonus".into()),
            None,
            None,
            None,
            Vec::new(),
            None,
        )
        .unwrap();

        let csv = export_transactions_csv(&[tx], &std::collections::HashMap::new());
        assert!(csv.contains(CSV_SECURITY_BANNER));
        assert!(csv.contains("Data SHA-256 Checksum:"));
        assert!(csv.contains("Total Transactions: 1"));
        assert!(csv.contains("Category,PaymentMethod"));
        // Confirm '=1+1' was sanitized with leading quote
        assert!(csv.contains("'=1+1"));
    }
}
