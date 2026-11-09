use bhoot_db::Database;
use bhoot_desktop::db_key::resolve_or_provision_database_key;
use bhoot_desktop::server::{handle_request, run_server, seed_starter_data, ServerState};
use rand::rngs::OsRng;
use rand::RngCore;
use std::env;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tiny_http::Server;
use zeroize::Zeroizing;

fn print_usage() {
    println!("BHOOT Desktop Application Server v1.0.0");
    println!("Usage: bhoot-desktop [OPTIONS]");
    println!();
    println!("Options:");
    println!("  --port <PORT>       Port to bind (default: 1420)");
    println!("  --db <PATH>         Path to SQLCipher database (default: /var/bhoot/data/bhoot.db)");
    println!("  --key-file <PATH>   Path to wrapped database key file (default: /var/bhoot/system/db_key.wrapped)");
    println!("  --vault-dir <PATH>  Path to document vault storage (default: /var/bhoot/vault)");
    println!("  --test-run          Execute non-interactive self-test and exit");
    println!("  --help              Display this help message");
}

fn run_self_test() -> Result<(), Box<dyn std::error::Error>> {
    println!("[TEST] Starting BHOOT Desktop comprehensive self-test harness...");

    // 1. Initialize encrypted database with ephemeral random test key & ServerState
    let mut test_key_raw = [0u8; 32];
    OsRng.fill_bytes(&mut test_key_raw);
    let test_key = Zeroizing::new(test_key_raw);
    let temp_dir = tempfile::tempdir()?;
    let temp_db_path = temp_dir.path().join("test.db");
    let temp_vault_dir = temp_dir.path().join("vault");
    fs::create_dir_all(&temp_vault_dir)?;

    let db = Database::open(&temp_db_path, &test_key)?;
    seed_starter_data(&db)?;

    let state = Arc::new(ServerState::new(
        db,
        temp_db_path,
        temp_vault_dir,
        *test_key,
    ));

    // 2. Bind server to random ephemeral port
    let server = Server::http("127.0.0.1:0")
        .map_err(|e| format!("Failed to bind test server: {e}"))?;
    let bound_addr = server.server_addr().to_ip().unwrap();
    let port = bound_addr.port();
    println!("[TEST] Ephemeral test server bound to 127.0.0.1:{port}");

    let is_running = Arc::new(AtomicBool::new(true));
    let is_running_clone = is_running.clone();
    let state_clone = state.clone();

    let server_handle = thread::spawn(move || {
        while is_running_clone.load(Ordering::SeqCst) {
            if let Ok(Some(request)) = server.recv_timeout(Duration::from_millis(100)) {
                handle_request(request, &state_clone);
            }
        }
    });

    // Helper closure to make raw HTTP requests with active session token
    let active_token = Arc::new(Mutex::new(None::<String>));
    let active_token_clone = active_token.clone();

    let http_request = move |method: &str, path: &str, body: Option<&str>| -> Result<(u16, String), Box<dyn std::error::Error>> {
        let mut stream = TcpStream::connect(format!("127.0.0.1:{port}"))?;
        stream.set_read_timeout(Some(Duration::from_secs(4)))?;

        let body_str = body.unwrap_or("");
        let auth_hdr = match active_token_clone.lock().unwrap().as_ref() {
            Some(tok) => format!("Authorization: Bearer {tok}\r\n"),
            None => String::new(),
        };

        let req = format!(
            "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n{auth_hdr}Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body_str}",
            body_str.len()
        );
        stream.write_all(req.as_bytes())?;

        let mut response = String::new();
        stream.read_to_string(&mut response)?;

        let mut lines = response.lines();
        let status_line = lines.next().ok_or("Empty response")?;
        let status_code: u16 = status_line
            .split_whitespace()
            .nth(1)
            .ok_or("Invalid status line")?
            .parse()?;

        Ok((status_code, response))
    };

    // 3. Test Static HTML Delivery
    println!("[TEST 1/15] Checking /index.html...");
    let (status, resp) = http_request("GET", "/", None)?;
    assert_eq!(status, 200, "Root should return 200 OK");
    assert!(resp.contains("<!doctype html>"), "Should contain DOCTYPE html");
    assert!(resp.contains("Content-Security-Policy"), "Should contain CSP header");
    println!("  -> PASS (HTML + CSP verified)");

    // 4. Test Static CSS Delivery
    println!("[TEST 2/15] Checking CSS bundle...");
    let (status, resp) = http_request("GET", "/assets/index.css", None)?;
    assert_eq!(status, 200, "CSS should return 200 OK");
    assert!(resp.contains("text/css"), "Should have text/css Content-Type");
    println!("  -> PASS (CSS verified)");

    // 5. Test Static JS Delivery
    println!("[TEST 3/15] Checking JS bundle...");
    let (status, resp) = http_request("GET", "/assets/index.js", None)?;
    assert_eq!(status, 200, "JS should return 200 OK");
    assert!(resp.contains("application/javascript"), "Should have application/javascript Content-Type");
    println!("  -> PASS (JS bundle verified)");

    // 6. Test /api/health
    println!("[TEST 4/15] Checking /api/health...");
    let (status, resp) = http_request("GET", "/api/health", None)?;
    assert_eq!(status, 200);
    assert!(resp.contains(r#""airgap":true"#));
    println!("  -> PASS (/api/health verified)");

    // Verify rejection while application is unenrolled
    let (lock_status, _) = http_request("GET", "/api/accounts", None)?;
    assert_eq!(lock_status, 401, "Protected route MUST be rejected when unenrolled");

    // Verify unlock rejected while unenrolled
    let (unlock_unenrolled_status, _) = http_request("POST", "/api/auth/unlock", Some(r#"{"pin":"748192"}"#))?;
    assert_eq!(unlock_unenrolled_status, 400, "Unlock must fail when unenrolled");

    // Verify weak PIN enrollment rejected
    let (weak_enroll_status, _) = http_request("POST", "/api/auth/enroll", Some(r#"{"pin":"111111","confirm_pin":"111111"}"#))?;
    assert_eq!(weak_enroll_status, 400, "Weak PIN enrollment must be rejected");

    // Verify confirmation mismatch enrollment rejected
    let (mismatch_enroll_status, _) = http_request("POST", "/api/auth/enroll", Some(r#"{"pin":"748192","confirm_pin":"999999"}"#))?;
    assert_eq!(mismatch_enroll_status, 400, "Mismatched PIN enrollment must be rejected");

    // First-owner enrollment with valid secure PIN
    let (enroll_status, enroll_resp) = http_request("POST", "/api/auth/enroll", Some(r#"{"pin":"748192","confirm_pin":"748192"}"#))?;
    assert_eq!(enroll_status, 200, "Valid enrollment should return 200 OK");
    let resp_body = enroll_resp.lines().last().unwrap_or("{}");
    let enroll_json: serde_json::Value = serde_json::from_str(resp_body)?;
    let token = enroll_json["session_token"].as_str().ok_or("Missing token in enroll response")?;
    *active_token.lock().unwrap() = Some(token.to_string());

    // Duplicate enrollment attempt must return 409 Conflict
    let (dup_status, _) = http_request("POST", "/api/auth/enroll", Some(r#"{"pin":"748192","confirm_pin":"748192"}"#))?;
    assert_eq!(dup_status, 409, "Duplicate enrollment must return 409 Conflict");

    // 7. Test /api/accounts (GET)
    println!("[TEST 5/18] Checking GET /api/accounts...");
    let (status, resp) = http_request("GET", "/api/accounts", None)?;
    assert_eq!(status, 200);
    assert!(resp.contains("HDFC Savings Bank"));
    assert!(resp.contains("Zerodha Demat & Trading"));
    assert!(resp.contains("balance_paisa"));
    println!("  -> PASS (GET /api/accounts verified with balance_paisa)");

    // 8. Test /api/accounts (POST mutation)
    println!("[TEST 6/18] Checking POST /api/accounts (Create Account)...");
    let new_acc_json = r#"{"name":"SBI Emergency Fund","account_type":"Asset","subtype":"BankAccount","currency":"INR","description":"Liquid reserve fund"}"#;
    let (status, resp) = http_request("POST", "/api/accounts", Some(new_acc_json))?;
    assert_eq!(status, 201, "Account creation should return 201 Created");
    assert!(resp.contains("SBI Emergency Fund"));
    let created_acc: serde_json::Value = serde_json::from_str(resp.lines().last().unwrap_or("{}"))?;
    let created_acc_id = created_acc["id"].as_str().unwrap();

    // Test POST /api/accounts/update
    let update_acc_json = format!(
        r#"{{"account_id":"{created_acc_id}","name":"SBI Reserve Fund","description":"Updated reserve account"}}"#
    );
    let (up_status, _) = http_request("POST", "/api/accounts/update", Some(&update_acc_json))?;
    assert_eq!(up_status, 200, "Account update should return 200 OK");
    println!("  -> PASS (POST /api/accounts & update verified)");

    // 9. Test /api/transactions (GET)
    println!("[TEST 7/15] Checking GET /api/transactions...");
    let (status, resp) = http_request("GET", "/api/transactions", None)?;
    assert_eq!(status, 200);
    assert!(resp.contains("October Retainer") || resp.contains("CMS987654321"));
    println!("  -> PASS (GET /api/transactions verified)");

    // 10. Test /api/transactions (POST balanced mutation)
    println!("[TEST 8/15] Checking POST /api/transactions (Create Balanced Transaction)...");
    // Extract account IDs from accounts list
    let accounts: Vec<serde_json::Value> = {
        let db = state.db.lock().unwrap();
        let accs = db.list_accounts().unwrap();
        accs.iter().map(|a| serde_json::json!({"id": a.id.to_string(), "name": a.name})).collect()
    };
    let acc1_id = accounts[0]["id"].as_str().unwrap();
    let acc2_id = accounts[3]["id"].as_str().unwrap(); // Salary income

    let new_tx_json = format!(
        r#"{{"tx_type":"Income","postings":[{{"account_id":"{acc1_id}","amount_paisa":5000000,"direction":"Debit","memo":"Freelance project"}},{{"account_id":"{acc2_id}","amount_paisa":5000000,"direction":"Credit","memo":"Service fee"}}],"category":"Consulting","payment_method":"UPI","reference":"UPI88776655"}}"#
    );
    let (status, resp) = http_request("POST", "/api/transactions", Some(&new_tx_json))?;
    assert_eq!(status, 201, "Balanced transaction should return 201 Created");
    assert!(resp.contains("UPI88776655"));
    println!("  -> PASS (POST /api/transactions verified)");

    // 11. Test /api/transactions (POST unbalanced mutation REJECTION)
    println!("[TEST 9/15] Checking POST /api/transactions (Reject Unbalanced Transaction)...");
    let unbal_tx_json = format!(
        r#"{{"tx_type":"Income","postings":[{{"account_id":"{acc1_id}","amount_paisa":5000000,"direction":"Debit"}},{{"account_id":"{acc2_id}","amount_paisa":4000000,"direction":"Credit"}}]}}"#
    );
    let (status, _) = http_request("POST", "/api/transactions", Some(&unbal_tx_json))?;
    assert_eq!(status, 422, "Unbalanced transaction MUST be rejected with 422");
    println!("  -> PASS (Unbalanced transaction rejected with 422)");

    // 12. Test /api/trades & /api/trades/summary
    println!("[TEST 10/15] Checking /api/trades and summary...");
    let (status, resp) = http_request("GET", "/api/trades", None)?;
    assert_eq!(status, 200);
    assert!(resp.contains("NIFTY26NOV24500CE"));
    let (status2, resp2) = http_request("GET", "/api/trades/summary", None)?;
    assert_eq!(status2, 200);
    assert!(resp2.contains("win_rate_basis_points"));
    println!("  -> PASS (/api/trades & summary verified)");

    // 13. Test /api/reports/balance-sheet
    println!("[TEST 11/15] Checking /api/reports/balance-sheet...");
    let (status, resp) = http_request("GET", "/api/reports/balance-sheet", None)?;
    assert_eq!(status, 200);
    assert!(resp.contains("is_balanced"));
    println!("  -> PASS (/api/reports/balance-sheet verified)");

    // 14. Test Authentication & PIN Lock
    println!("[TEST 12/15] Checking Authentication & Argon2id PIN verification...");
    // Lock session
    let (lock_status, _) = http_request("POST", "/api/auth/lock", None)?;
    assert_eq!(lock_status, 200);

    // Wrong PIN
    let (status, _) = http_request("POST", "/api/auth/unlock", Some(r#"{"pin":"wrong_pin"}"#))?;
    assert_eq!(status, 401, "Wrong PIN must return 401 Unauthorized");

    // Correct enrolled PIN
    let (status, resp) = http_request("POST", "/api/auth/unlock", Some(r#"{"pin":"748192"}"#))?;
    assert_eq!(status, 200, "Correct PIN must return 200 OK");
    assert!(resp.contains("session_token"));
    let resp_body = resp.lines().last().unwrap_or("{}");
    let unlock_json: serde_json::Value = serde_json::from_str(resp_body)?;
    let new_token = unlock_json["session_token"].as_str().ok_or("Missing token in unlock response")?;
    *active_token.lock().unwrap() = Some(new_token.to_string());
    println!("  -> PASS (Argon2id PIN authentication verified)");

    // 15. Test Vault Verification
    println!("[TEST 13/15] Checking /api/vault/verify...");
    let (status, resp) = http_request("GET", "/api/vault/verify", None)?;
    assert_eq!(status, 200);
    assert!(resp.contains("total_documents"));
    println!("  -> PASS (/api/vault/verify verified)");

    // 16. Test Backup Creation API
    println!("[TEST 14/18] Checking POST /api/backup/create...");
    let backup_dest = temp_dir.path().join("backup.bhoot");
    let backup_req = format!(
        r#"{{"passphrase":"master-secure-backup-passphrase-2026","destination_path":"{}"}}"#,
        backup_dest.display()
    );
    let (status, resp) = http_request("POST", "/api/backup/create", Some(&backup_req))?;
    assert_eq!(status, 200, "Backup creation should return 200 OK");
    assert!(resp.contains("bytes_written"));
    println!("  -> PASS (POST /api/backup/create verified)");

    // 17. Test Persistent Settings (GET and POST)
    println!("[TEST 15/18] Checking Settings GET & POST persistence...");
    let new_settings = r#"{"currency":"INR","theme":"dark","auto_lock_minutes":30,"offline_mode":true,"financial_year_start":"04-01"}"#;
    let (s_post_status, s_post_resp) = http_request("POST", "/api/settings", Some(new_settings))?;
    assert_eq!(s_post_status, 200);
    assert!(s_post_resp.contains(r#""auto_lock_minutes":30"#));

    let (s_get_status, s_get_resp) = http_request("GET", "/api/settings", None)?;
    assert_eq!(s_get_status, 200);
    assert!(s_get_resp.contains(r#""auto_lock_minutes":30"#));
    println!("  -> PASS (Settings GET & POST persistence verified)");

    // 18. Test Budget Creation & Deletion
    println!("[TEST 16/18] Checking Budget Creation & Deletion...");
    let budget_json = r#"{"name":"Q4 Expense Plan","period":"Quarterly","start_date":"2026-10-01T00:00:00Z","end_date":"2026-12-31T23:59:59Z","categories":[{"category":"LivingExpense","allocated_paisa":5000000,"alert_threshold_pct":80}]}"#;
    let (b_status, b_resp) = http_request("POST", "/api/budgets", Some(budget_json))?;
    assert_eq!(b_status, 200);
    let b_val: serde_json::Value = serde_json::from_str(b_resp.lines().last().unwrap_or("{}"))?;
    let b_id = b_val["id"].as_str().unwrap();

    let del_json = format!(r#"{{"id":"{b_id}"}}"#);
    let (del_status, del_resp) = http_request("POST", "/api/budgets/delete", Some(&del_json))?;
    assert_eq!(del_status, 200);
    assert!(del_resp.contains(r#""success":true"#));
    println!("  -> PASS (Budget Creation & Deletion verified)");

    // 19. Test 404 Handling
    println!("[TEST 17/18] Checking 404 handling...");
    let (status, _) = http_request("GET", "/unknown/nonexistent/path", None)?;
    assert_eq!(status, 404);
    println!("  -> PASS (404 verified)");

    // Shut down server
    is_running.store(false, Ordering::SeqCst);
    let _ = server_handle.join();

    println!("========================================================================");
    println!(" [TEST SUMMARY] All 18 BHOOT Desktop HTTP endpoints passed verification!");
    println!("========================================================================");

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let mut port: u16 = 1420;
    let mut db_path = PathBuf::from("/var/bhoot/data/bhoot.db");
    let mut key_file: Option<PathBuf> = None;
    let mut vault_dir = PathBuf::from("/var/bhoot/vault");
    let mut test_run = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--port" => {
                if i + 1 < args.len() {
                    port = args[i + 1].parse().map_err(|_| "Invalid port number")?;
                    i += 1;
                }
            }
            "--db" => {
                if i + 1 < args.len() {
                    db_path = PathBuf::from(&args[i + 1]);
                    i += 1;
                }
            }
            "--key-file" => {
                if i + 1 < args.len() {
                    key_file = Some(PathBuf::from(&args[i + 1]));
                    i += 1;
                }
            }
            "--vault-dir" => {
                if i + 1 < args.len() {
                    vault_dir = PathBuf::from(&args[i + 1]);
                    i += 1;
                }
            }
            "--test-run" => {
                test_run = true;
            }
            "--help" | "-h" => {
                print_usage();
                return Ok(());
            }
            _ => {
                eprintln!("Unknown option: {}", args[i]);
                print_usage();
                std::process::exit(1);
            }
        }
        i += 1;
    }

    if test_run {
        return run_self_test();
    }

    // Resolve or provision persistent 256-bit database encryption key
    let key = resolve_or_provision_database_key(&db_path, key_file.as_deref())
        .map_err(|e| format!("Database key resolution failed: {e}"))?;

    // Ensure database and vault directories exist
    if let Some(parent) = db_path.parent() {
        if !parent.exists() {
            let _ = fs::create_dir_all(parent);
        }
    }
    if !vault_dir.exists() {
        let _ = fs::create_dir_all(&vault_dir);
    }

    println!("[BHOOT] Opening encrypted database at {}", db_path.display());
    let db = match Database::open(&db_path, &key) {
        Ok(db) => db,
        Err(e) => {
            eprintln!("[BHOOT FATAL] Failed to authenticate or open encrypted database at {}: {e}", db_path.display());
            eprintln!("[BHOOT FATAL] Missing, incorrect, or corrupted key material. Refusing to proceed. Failing closed.");
            return Err(format!("Database authentication failed: {e}").into());
        }
    };

    seed_starter_data(&db)?;

    let state = Arc::new(ServerState::new(db, db_path, vault_dir, *key));

    if !state.is_enrolled.load(Ordering::SeqCst) {
        if let Ok(preconfigured_pin) = env::var("BHOOT_MASTER_PIN") {
            if !preconfigured_pin.is_empty() {
                let _ = state.enroll_owner_pin(&preconfigured_pin, &preconfigured_pin);
            }
        }
    }

    run_server(port, state)?;

    Ok(())
}
