use bhoot_db::Database;
use bhoot_desktop::server::{handle_request, run_server, seed_starter_data, ServerState};
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

fn print_usage() {
    println!("BHOOT Desktop Application Server v1.0.0");
    println!("Usage: bhoot-desktop [OPTIONS]");
    println!();
    println!("Options:");
    println!("  --port <PORT>       Port to bind (default: 1420)");
    println!("  --db <PATH>         Path to SQLCipher database (default: /var/bhoot/data/bhoot.db)");
    println!("  --vault-dir <PATH>  Path to document vault storage (default: /var/bhoot/vault)");
    println!("  --test-run          Execute non-interactive self-test and exit");
    println!("  --help              Display this help message");
}

fn run_self_test() -> Result<(), Box<dyn std::error::Error>> {
    println!("[TEST] Starting BHOOT Desktop comprehensive self-test harness...");

    // 1. Initialize encrypted database & ServerState
    let test_key = [0x42u8; 32];
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
        test_key,
        "1234",
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

    // Verify rejection while session is locked
    let (lock_status, _) = http_request("GET", "/api/accounts", None)?;
    assert_eq!(lock_status, 401, "Protected route MUST be rejected when session is locked");

    // Authenticate and unlock session with PIN
    let (status, unlock_resp) = http_request("POST", "/api/auth/unlock", Some(r#"{"pin":"1234"}"#))?;
    assert_eq!(status, 200, "PIN unlock should return 200 OK");
    let resp_body = unlock_resp.lines().last().unwrap_or("{}");
    let unlock_json: serde_json::Value = serde_json::from_str(resp_body)?;
    let token = unlock_json["session_token"].as_str().ok_or("Missing token in unlock response")?;
    *active_token.lock().unwrap() = Some(token.to_string());

    // 7. Test /api/accounts (GET)
    println!("[TEST 5/15] Checking GET /api/accounts...");
    let (status, resp) = http_request("GET", "/api/accounts", None)?;
    assert_eq!(status, 200);
    assert!(resp.contains("HDFC Savings Bank"));
    assert!(resp.contains("Zerodha Demat & Trading"));
    println!("  -> PASS (GET /api/accounts verified)");

    // 8. Test /api/accounts (POST mutation)
    println!("[TEST 6/15] Checking POST /api/accounts (Create Account)...");
    let new_acc_json = r#"{"name":"SBI Emergency Fund","account_type":"Asset","subtype":"BankAccount","currency":"INR","description":"Liquid reserve fund"}"#;
    let (status, resp) = http_request("POST", "/api/accounts", Some(new_acc_json))?;
    assert_eq!(status, 201, "Account creation should return 201 Created");
    assert!(resp.contains("SBI Emergency Fund"));
    println!("  -> PASS (POST /api/accounts verified)");

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
    // Wrong PIN
    let (status, _) = http_request("POST", "/api/auth/unlock", Some(r#"{"pin":"wrong_pin"}"#))?;
    assert_eq!(status, 401, "Wrong PIN must return 401 Unauthorized");
    // Correct PIN
    let (status, resp) = http_request("POST", "/api/auth/unlock", Some(r#"{"pin":"1234"}"#))?;
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
    println!("[TEST 14/15] Checking POST /api/backup/create...");
    let backup_dest = temp_dir.path().join("backup.bhoot");
    let backup_req = format!(
        r#"{{"passphrase":"master-secure-backup-passphrase-2026","destination_path":"{}"}}"#,
        backup_dest.display()
    );
    let (status, resp) = http_request("POST", "/api/backup/create", Some(&backup_req))?;
    assert_eq!(status, 200, "Backup creation should return 200 OK");
    assert!(resp.contains("bytes_written"));
    println!("  -> PASS (POST /api/backup/create verified)");

    // 17. Test 404 Handling
    println!("[TEST 15/15] Checking 404 handling...");
    let (status, _) = http_request("GET", "/unknown/nonexistent/path", None)?;
    assert_eq!(status, 404);
    println!("  -> PASS (404 verified)");

    // Shut down server
    is_running.store(false, Ordering::SeqCst);
    let _ = server_handle.join();

    println!("========================================================================");
    println!(" [TEST SUMMARY] All 15 BHOOT Desktop HTTP endpoints passed verification!");
    println!("========================================================================");

    Ok(())
}

fn decode_hex_32(s: &str) -> Result<[u8; 32], String> {
    let s = s.trim();
    if s.len() != 64 {
        return Err("Hex string must be exactly 64 characters (32 bytes)".into());
    }
    let mut bytes = [0u8; 32];
    for (i, byte) in bytes.iter_mut().enumerate() {
        let chunk = &s[i * 2..i * 2 + 2];
        *byte = u8::from_str_radix(chunk, 16).map_err(|e| format!("Invalid hex: {e}"))?;
    }
    Ok(bytes)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let mut port: u16 = 1420;
    let mut db_path = PathBuf::from("/var/bhoot/data/bhoot.db");
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

    // Default or configured database key
    let key = if let Ok(hex_key) = env::var("BHOOT_DB_KEY") {
        decode_hex_32(&hex_key).map_err(|e| format!("Invalid BHOOT_DB_KEY: {e}"))?
    } else {
        // Fallback production default key
        [0x5Au8; 32]
    };

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
            eprintln!("[BHOOT WARNING] Could not open database at {}: {e}", db_path.display());
            eprintln!("[BHOOT WARNING] Falling back to isolated in-memory encrypted database.");
            Database::open_in_memory(&key)?
        }
    };

    seed_starter_data(&db)?;

    let initial_pin = env::var("BHOOT_MASTER_PIN").unwrap_or_else(|_| "1234".to_string());
    let state = Arc::new(ServerState::new(db, db_path, vault_dir, key, &initial_pin));

    run_server(port, state)?;

    Ok(())
}
