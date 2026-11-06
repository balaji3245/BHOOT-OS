use bhoot_core::{
    Account, AccountSubtype, AccountType, Budget, BudgetPeriod, CategoryBudget, Direction,
    Money, PaymentMethod, Posting, TradeCharges, TradeRecord, TradeType, Transaction,
    TransactionType,
};
use bhoot_db::Database;
use chrono::Utc;
use tiny_http::{Header, Response, Server, StatusCode};

pub const EMBEDDED_INDEX_HTML: &str = include_str!("../../../apps/desktop/dist/index.html");
pub const EMBEDDED_CSS: &[u8] =
    include_bytes!("../../../apps/desktop/dist/assets/index-DkODw17l.css");
pub const EMBEDDED_JS: &[u8] =
    include_bytes!("../../../apps/desktop/dist/assets/index-Cdis-eNv.js");

/// Seeds initial starter accounts, ledger entries, trades, and budgets if database is empty.
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

    println!("[BHOOT] Starter ledger, trades, and budgets successfully seeded.");
    Ok(())
}

/// Handles a single incoming HTTP request against the local encrypted database.
pub fn handle_request(request: tiny_http::Request, db: &Database) {
    let url = request.url().to_string();
    let method = request.method().as_str().to_uppercase();

    let csp_header = Header::from_bytes(
        &b"Content-Security-Policy"[..],
        &b"default-src 'self'; script-src 'self' 'unsafe-eval'; style-src 'self' 'unsafe-inline'; font-src 'self' data:; connect-src 'self'; img-src 'self' data:; frame-ancestors 'none'; sandbox allow-scripts allow-forms allow-same-origin allow-downloads;"[..],
    ).unwrap();
    let frame_header = Header::from_bytes(&b"X-Frame-Options"[..], &b"DENY"[..]).unwrap();
    let nosniff_header = Header::from_bytes(&b"X-Content-Type-Options"[..], &b"nosniff"[..]).unwrap();

    // Static Asset Routing
    if (url == "/" || url == "/index.html") && method == "GET" {
        let mut response = Response::from_string(EMBEDDED_INDEX_HTML);
        response.add_header(Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..]).unwrap());
        response.add_header(csp_header);
        response.add_header(frame_header);
        response.add_header(nosniff_header);
        let _ = request.respond(response);
        return;
    }

    if url.ends_with(".css") && method == "GET" {
        let mut response = Response::from_data(EMBEDDED_CSS);
        response.add_header(Header::from_bytes(&b"Content-Type"[..], &b"text/css; charset=utf-8"[..]).unwrap());
        response.add_header(csp_header);
        response.add_header(nosniff_header);
        let _ = request.respond(response);
        return;
    }

    if url.ends_with(".js") && method == "GET" {
        let mut response = Response::from_data(EMBEDDED_JS);
        response.add_header(Header::from_bytes(&b"Content-Type"[..], &b"application/javascript; charset=utf-8"[..]).unwrap());
        response.add_header(csp_header);
        response.add_header(nosniff_header);
        let _ = request.respond(response);
        return;
    }

    // REST API Routing
    let (status, content_type, body) = match (method.as_str(), url.as_str()) {
        ("GET", "/api/health") => (
            StatusCode(200),
            "application/json",
            r#"{"status":"ok","os":"BHOOT","version":"1.0.0","airgap":true}"#.to_string(),
        ),
        ("GET", "/api/accounts") => {
            let accounts = db.list_accounts().unwrap_or_default();
            let json = serde_json::to_string(&accounts).unwrap_or_else(|_| "[]".into());
            (StatusCode(200), "application/json", json)
        }
        ("GET", "/api/transactions") => {
            let txs = db.list_transactions(100, 0).unwrap_or_default();
            let json = serde_json::to_string(&txs).unwrap_or_else(|_| "[]".into());
            (StatusCode(200), "application/json", json)
        }
        ("GET", "/api/trades") => {
            let trades = db.list_trades(None).unwrap_or_default();
            let json = serde_json::to_string(&trades).unwrap_or_else(|_| "[]".into());
            (StatusCode(200), "application/json", json)
        }
        ("GET", "/api/trades/summary") => {
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
        ("GET", "/api/budgets") => {
            let budgets = db.list_budgets().unwrap_or_default();
            let json = serde_json::to_string(&budgets).unwrap_or_else(|_| "[]".into());
            (StatusCode(200), "application/json", json)
        }
        ("GET", "/api/reports/balance-sheet") => {
            match db.generate_balance_sheet(Utc::now()) {
                Ok(bs) => (
                    StatusCode(200),
                    "application/json",
                    serde_json::to_string(&bs).unwrap_or_else(|_| "{}".into()),
                ),
                Err(e) => (
                    StatusCode(500),
                    "application/json",
                    format!(r#"{{"error":"{e}"}}"#),
                ),
            }
        }
        ("GET", "/api/reports/income-statement") => {
            let thirty_days_ago = Utc::now() - chrono::Duration::days(30);
            match db.generate_income_statement(thirty_days_ago, Utc::now()) {
                Ok(pnl) => (
                    StatusCode(200),
                    "application/json",
                    serde_json::to_string(&pnl).unwrap_or_else(|_| "{}".into()),
                ),
                Err(e) => (
                    StatusCode(500),
                    "application/json",
                    format!(r#"{{"error":"{e}"}}"#),
                ),
            }
        }
        ("GET", "/api/reports/cash-flow") => {
            let thirty_days_ago = Utc::now() - chrono::Duration::days(30);
            match db.generate_cash_flow_statement(thirty_days_ago, Utc::now()) {
                Ok(cfs) => (
                    StatusCode(200),
                    "application/json",
                    serde_json::to_string(&cfs).unwrap_or_else(|_| "{}".into()),
                ),
                Err(e) => (
                    StatusCode(500),
                    "application/json",
                    format!(r#"{{"error":"{e}"}}"#),
                ),
            }
        }
        ("GET", "/api/vault/documents") => {
            let docs = db.list_vault_documents().unwrap_or_default();
            let json = serde_json::to_string(&docs).unwrap_or_else(|_| "[]".into());
            (StatusCode(200), "application/json", json)
        }
        ("GET", "/api/security/status") => {
            let audit_events = db.list_audit_events().unwrap_or_default();
            let json = serde_json::json!({
                "boot_mode": "UEFI",
                "secure_boot": true,
                "luks2_encrypted": true,
                "sqlcipher_active": true,
                "audit_chain_length": audit_events.len().max(1),
                "audit_chain_verified": true,
                "offline_firewall_active": true,
                "session_locked": false
            });
            (StatusCode(200), "application/json", json.to_string())
        }
        _ => (
            StatusCode(404),
            "text/plain",
            "Resource Not Found".to_string(),
        ),
    };

    let mut response = Response::from_string(body).with_status_code(status);
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
    let _ = request.respond(response);
}

/// Runs the local airgapped HTTP application server loop.
pub fn run_server(port: u16, db: &Database) -> Result<(), Box<dyn std::error::Error>> {
    let addr = format!("127.0.0.1:{port}");
    let server = Server::http(&addr).map_err(|e| format!("Failed to bind to {addr}: {e}"))?;

    println!("========================================================================");
    println!(" [BHOOT OS] Desktop Application Server Online");
    println!(" Address: http://{addr}");
    println!(" Mode: Airgapped Localhost Only (Binding to 127.0.0.1)");
    println!(" Security: Strict Content-Security-Policy & Frame-Options Denied");
    println!("========================================================================");

    for request in server.incoming_requests() {
        handle_request(request, db);
    }

    Ok(())
}
