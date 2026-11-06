use bhoot_db::Database;
use bhoot_desktop::server::{handle_request, run_server, seed_starter_data};
use std::env;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
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
    println!("  --test-run          Execute non-interactive self-test and exit");
    println!("  --help              Display this help message");
}

fn run_self_test() -> Result<(), Box<dyn std::error::Error>> {
    println!("[TEST] Starting BHOOT Desktop self-test harness...");

    // 1. Initialize encrypted in-memory database
    let test_key = [0x42u8; 32];
    let db = Database::open_in_memory(&test_key)?;
    seed_starter_data(&db)?;
    let db = Arc::new(Mutex::new(db));

    // 2. Bind server to random ephemeral port
    let server = Server::http("127.0.0.1:0")
        .map_err(|e| format!("Failed to bind test server: {e}"))?;
    let bound_addr = server.server_addr().to_ip().unwrap();
    let port = bound_addr.port();
    println!("[TEST] Ephemeral test server bound to 127.0.0.1:{port}");

    let is_running = Arc::new(AtomicBool::new(true));
    let is_running_clone = is_running.clone();
    let db_clone = db.clone();

    let server_handle = thread::spawn(move || {
        while is_running_clone.load(Ordering::SeqCst) {
            if let Ok(Some(request)) = server.recv_timeout(Duration::from_millis(100)) {
                if let Ok(db) = db_clone.lock() {
                    handle_request(request, &db);
                }
            }
        }
    });

    // Helper closure to make raw HTTP GET requests
    let http_get = |path: &str| -> Result<(u16, String), Box<dyn std::error::Error>> {
        let mut stream = TcpStream::connect(format!("127.0.0.1:{port}"))?;
        stream.set_read_timeout(Some(Duration::from_secs(3)))?;
        let req = format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n");
        stream.write_all(req.as_bytes())?;

        let mut buffer = String::new();
        stream.read_to_string(&mut buffer)?;

        let mut lines = buffer.lines();
        let status_line = lines.next().ok_or("Empty response")?;
        let status_code: u16 = status_line
            .split_whitespace()
            .nth(1)
            .ok_or("Invalid status line")?
            .parse()?;

        Ok((status_code, buffer))
    };

    // 3. Test Static HTML Delivery
    println!("[TEST] Checking /index.html...");
    let (status, resp) = http_get("/")?;
    assert_eq!(status, 200, "Root should return 200 OK");
    assert!(resp.contains("<!doctype html>"), "Should contain DOCTYPE html");
    assert!(resp.contains("Content-Security-Policy"), "Should contain CSP header");
    println!("  -> PASS (HTML + CSP verified)");

    // 4. Test Static CSS Delivery
    println!("[TEST] Checking CSS bundle...");
    let (status, resp) = http_get("/assets/index-DkODw17l.css")?;
    assert_eq!(status, 200, "CSS should return 200 OK");
    assert!(resp.contains("text/css"), "Should have text/css Content-Type");
    println!("  -> PASS (CSS verified)");

    // 5. Test Static JS Delivery
    println!("[TEST] Checking JS bundle...");
    let (status, resp) = http_get("/assets/index-Cdis-eNv.js")?;
    assert_eq!(status, 200, "JS should return 200 OK");
    assert!(resp.contains("application/javascript"), "Should have application/javascript Content-Type");
    println!("  -> PASS (JS bundle verified)");

    // 6. Test /api/health
    println!("[TEST] Checking /api/health...");
    let (status, resp) = http_get("/api/health")?;
    assert_eq!(status, 200);
    assert!(resp.contains(r#""airgap":true"#));
    println!("  -> PASS (/api/health verified)");

    // 7. Test /api/accounts
    println!("[TEST] Checking /api/accounts...");
    let (status, resp) = http_get("/api/accounts")?;
    assert_eq!(status, 200);
    assert!(resp.contains("HDFC Savings Bank"));
    assert!(resp.contains("Zerodha Demat & Trading"));
    println!("  -> PASS (/api/accounts verified)");

    // 8. Test /api/transactions
    println!("[TEST] Checking /api/transactions...");
    let (status, resp) = http_get("/api/transactions")?;
    assert_eq!(status, 200);
    assert!(resp.contains("October Retainer") || resp.contains("CMS987654321"));
    println!("  -> PASS (/api/transactions verified)");

    // 9. Test /api/trades
    println!("[TEST] Checking /api/trades...");
    let (status, resp) = http_get("/api/trades")?;
    assert_eq!(status, 200);
    assert!(resp.contains("NIFTY26NOV24500CE"));
    println!("  -> PASS (/api/trades verified)");

    // 10. Test /api/reports/balance-sheet
    println!("[TEST] Checking /api/reports/balance-sheet...");
    let (status, resp) = http_get("/api/reports/balance-sheet")?;
    assert_eq!(status, 200);
    assert!(resp.contains("is_balanced"));
    println!("  -> PASS (/api/reports/balance-sheet verified)");

    // 11. Test /api/security/status
    println!("[TEST] Checking /api/security/status...");
    let (status, resp) = http_get("/api/security/status")?;
    assert_eq!(status, 200);
    assert!(resp.contains(r#""secure_boot":true"#));
    assert!(resp.contains(r#""luks2_encrypted":true"#));
    println!("  -> PASS (/api/security/status verified)");

    // 12. Test 404
    println!("[TEST] Checking 404 handling...");
    let (status, _) = http_get("/unknown/path")?;
    assert_eq!(status, 404);
    println!("  -> PASS (404 verified)");

    // Shut down server
    is_running.store(false, Ordering::SeqCst);
    let _ = server_handle.join();

    println!("========================================================================");
    println!(" [TEST SUMMARY] All 10 BHOOT Desktop HTTP endpoints passed verification!");
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

    // Ensure database directory exists
    if let Some(parent) = db_path.parent() {
        if !parent.exists() {
            let _ = fs::create_dir_all(parent);
        }
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
    run_server(port, &db)?;

    Ok(())
}
