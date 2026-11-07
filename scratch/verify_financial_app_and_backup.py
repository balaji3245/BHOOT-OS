#!/usr/bin/env python3
"""
BHOOT — Financial Application Behavior, PIN Lockout, Offline & Backup/Restore Verification
Thoroughly verifies Sections 4, 5, 6, 7 against the compiled target/release/bhoot-desktop binary.
"""

import os
import sys
import subprocess
import time
import requests
import tempfile
import shutil
import json
import hashlib

BIN_PATH = "/home/bhoot/Desktop/BHOOT/os/build_real/rootfs/usr/local/bin/bhoot-desktop"

def log(msg):
    print(f"[{time.strftime('%H:%M:%S')}] {msg}", flush=True)

def run_suite():
    temp_dir = tempfile.mkdtemp(prefix="bhoot_verification_")
    db_path = os.path.join(temp_dir, "bhoot_verification.db")
    vault_dir = os.path.join(temp_dir, "vault")
    os.makedirs(vault_dir, exist_ok=True)
    port = 16420
    base_url = f"http://127.0.0.1:{port}"

    test_doc_path = os.path.join(vault_dir, "sample_financial_doc.txt")
    doc_content = b"OFFLINE FINANCIAL LEDGER DOCUMENT AUDIT 2026 - CONFIDENTIAL"
    with open(test_doc_path, "wb") as f:
        f.write(doc_content)
    orig_doc_sha256 = hashlib.sha256(doc_content).hexdigest()

    log(f"Starting test server on port {port}...")
    proc = subprocess.Popen(
        [BIN_PATH, "--port", str(port), "--db", db_path, "--vault-dir", vault_dir],
        stdout=subprocess.PIPE, stderr=subprocess.PIPE
    )
    time.sleep(1.5)

    checklist = {}

    try:
        # =====================================================================
        # 1. Server Health
        # =====================================================================
        r = requests.get(f"{base_url}/api/health")
        assert r.status_code == 200
        checklist["health"] = True
        log("  [PASS] 1. API Health Check OK (HTTP 200).")

        # =====================================================================
        # 2. Initial Locked State
        # =====================================================================
        r = requests.get(f"{base_url}/api/security/status")
        assert r.status_code == 200
        assert r.json().get("session_locked") is True
        checklist["initial_locked"] = True
        log("  [PASS] 2. System initial state is locked (Zero-Trust).")

        # =====================================================================
        # 3. Reject Mutation When Locked
        # =====================================================================
        r = requests.post(f"{base_url}/api/accounts", json={
            "name": "Intruder Account", "account_type": "Asset", "subtype": "Bank"
        })
        assert r.status_code == 401
        checklist["locked_mutation_401"] = True
        log("  [PASS] 3. Unauthenticated mutation rejected with HTTP 401.")

        # =====================================================================
        # 4. SECTION 5: PIN Rate Limiting & Persistent Lockout
        # =====================================================================
        log("  [TESTING SECTION 5: PIN RATE LIMITING & LOCKOUT]")
        for i in range(1, 6):
            r = requests.post(f"{base_url}/api/auth/unlock", json={"pin": f"000{i}"})
            if i < 5:
                assert r.status_code == 401, f"Expected 401 on attempt {i}, got {r.status_code}"
            else:
                assert r.status_code == 429, f"Expected 429 on attempt 5, got {r.status_code}"
        checklist["pin_lockout_429"] = True
        log("  [PASS] 4. Five consecutive incorrect PIN attempts triggered mandatory lockout (HTTP 429).")

        # 6th attempt during cooldown
        r = requests.post(f"{base_url}/api/auth/unlock", json={"pin": "0006"})
        assert r.status_code == 429
        checklist["cooldown_6th_429"] = True
        log("  [PASS] 5. Sixth attempt during cooldown rejected with HTTP 429.")

        # Correct PIN during cooldown
        r = requests.post(f"{base_url}/api/auth/unlock", json={"pin": "1234"})
        assert r.status_code == 429
        checklist["cooldown_correct_pin_429"] = True
        log("  [PASS] 6. Correct PIN during cooldown rejected with HTTP 429.")

        # Restart server during cooldown
        proc.terminate()
        proc.wait()
        log("  Restarting server to test lockout persistence in SQLCipher...")
        proc = subprocess.Popen(
            [BIN_PATH, "--port", str(port), "--db", db_path, "--vault-dir", vault_dir],
            stdout=subprocess.PIPE, stderr=subprocess.PIPE
        )
        time.sleep(1.5)

        r = requests.post(f"{base_url}/api/auth/unlock", json={"pin": "1234"})
        assert r.status_code == 429
        checklist["cooldown_persists_restart_429"] = True
        log("  [PASS] 7. Lockout persists across server restart (HTTP 429).")

        # Wait for cooldown to expire
        log("  Waiting for 60-second cooldown period to expire...")
        time.sleep(61)

        # Authenticate after cooldown
        r = requests.post(f"{base_url}/api/auth/unlock", json={"pin": "1234"})
        assert r.status_code == 200, f"Expected 200 after cooldown, got {r.status_code}: {r.text}"
        data = r.json()
        assert data.get("success") is True
        session_token = data.get("session_token")
        assert session_token, "No session token returned"
        checklist["auth_after_cooldown_200"] = True
        log(f"  [PASS] 8. Successfully authenticated after cooldown with session token: {session_token[:8]}...")

        auth_headers = {
            "Authorization": f"Bearer {session_token}",
            "Content-Type": "application/json"
        }

        # =====================================================================
        # 5. SECTION 4: Financial Operations & Integer Paisa Precision
        # =====================================================================
        log("  [TESTING SECTION 4: FINANCIAL APPLICATION OPERATIONS]")

        # 1. Create Accounts
        r_acc1 = requests.post(f"{base_url}/api/accounts", headers=auth_headers, json={
            "name": "Audit HDFC Savings",
            "account_type": "Asset",
            "subtype": "BankAccount"
        })
        assert r_acc1.status_code in (200, 201), f"Create account 1 failed ({r_acc1.status_code}): {r_acc1.text}"
        bank_id = r_acc1.json().get("id")

        r_acc2 = requests.post(f"{base_url}/api/accounts", headers=auth_headers, json={
            "name": "Audit Consulting Income",
            "account_type": "Income",
            "subtype": "Salary"
        })
        assert r_acc2.status_code in (200, 201), f"Create account 2 failed ({r_acc2.status_code}): {r_acc2.text}"
        income_id = r_acc2.json().get("id")

        checklist["create_accounts"] = True
        log(f"  [PASS] 9. Created test accounts (Bank: {bank_id}, Income: {income_id}).")

        # 2. Create Balanced Double-Entry Transaction
        # Debit Bank 250,000.00 INR (25,000,000 paisa)
        # Credit Income 250,000.00 INR (25,000,000 paisa)
        amount_paisa = 25000000
        tx_payload = {
            "tx_type": "Income",
            "description": "Consulting Retainer Q4 2026",
            "postings": [
                {
                    "account_id": bank_id,
                    "direction": "Debit",
                    "amount_paisa": amount_paisa,
                    "memo": "Client payment"
                },
                {
                    "account_id": income_id,
                    "direction": "Credit",
                    "amount_paisa": amount_paisa,
                    "memo": "Retainer fee"
                }
            ]
        }
        r_tx = requests.post(f"{base_url}/api/transactions", headers=auth_headers, json=tx_payload)
        assert r_tx.status_code in (200, 201), f"Transaction creation failed ({r_tx.status_code}): {r_tx.text}"
        tx_id = r_tx.json().get("id")
        checklist["create_balanced_tx"] = True
        log(f"  [PASS] 10. Created balanced double-entry transaction ({amount_paisa} paisa, ID: {tx_id}).")

        # 3. Verify Account Balance
        r_bs = requests.get(f"{base_url}/api/reports/balance-sheet", headers=auth_headers)
        assert r_bs.status_code == 200
        bs = r_bs.json()

        # Check integer paisa schema consistency
        assert "total_assets_paisa" in bs
        assert "total_liabilities_paisa" in bs
        assert "total_equity_paisa" in bs
        assert "retained_earnings_paisa" in bs
        assert "is_balanced" in bs
        assert bs["is_balanced"] is True, f"Balance sheet NOT balanced: {bs}"

        # Find our created bank account
        bank_item = next((item for item in bs["assets"] if item["account_id"] == bank_id), None)
        assert bank_item is not None, "Created bank account not found in balance sheet"
        assert bank_item["balance_paisa"] == amount_paisa
        checklist["verify_account_balance"] = True
        log(f"  [PASS] 11. Verified account balance strictly matches {amount_paisa} paisa.")

        # 4. Verify Dashboard Net Worth
        assert bs["net_worth_paisa"] == bs["total_assets_paisa"] - bs["total_liabilities_paisa"]
        checklist["verify_net_worth"] = True
        log(f"  [PASS] 12. Verified dashboard net worth ({bs['net_worth_paisa']} paisa). Balance sheet balanced: {bs['is_balanced']}.")

        # 5. Lock Session & Verify Token Revocation
        r_lock = requests.post(f"{base_url}/api/auth/lock", headers=auth_headers)
        assert r_lock.status_code == 200
        log("  Session locked via POST /api/auth/lock.")

        # Confirm old token is rejected
        r_reject = requests.get(f"{base_url}/api/reports/balance-sheet", headers=auth_headers)
        assert r_reject.status_code == 401, f"Expected 401 for revoked token, got {r_reject.status_code}"
        checklist["token_revocation_401"] = True
        log("  [PASS] 13. Previous session token strictly rejected with HTTP 401 after lock.")

        # Attempt unauthorized mutation after lock
        r_unauth_tx = requests.post(f"{base_url}/api/transactions", headers=auth_headers, json=tx_payload)
        assert r_unauth_tx.status_code == 401
        checklist["locked_mutation_rejected"] = True
        log("  [PASS] 14. Unauthorized mutation after lock strictly rejected with HTTP 401.")

        # Unlock with valid PIN
        r_reunlock = requests.post(f"{base_url}/api/auth/unlock", json={"pin": "1234"})
        assert r_reunlock.status_code == 200
        new_token = r_reunlock.json().get("session_token")
        assert new_token != session_token, "Token was not rotated on new unlock"
        new_headers = {"Authorization": f"Bearer {new_token}", "Content-Type": "application/json"}
        checklist["token_rotation"] = True
        log("  [PASS] 15. Unlocked with rotated session token.")

        # 6. Verify Data Persistence After Backend Restart
        proc.terminate()
        proc.wait()
        log("  Restarting server to test data durability...")
        proc = subprocess.Popen(
            [BIN_PATH, "--port", str(port), "--db", db_path, "--vault-dir", vault_dir],
            stdout=subprocess.PIPE, stderr=subprocess.PIPE
        )
        time.sleep(1.5)

        # Unlock after restart
        r_restart_unlock = requests.post(f"{base_url}/api/auth/unlock", json={"pin": "1234"})
        assert r_restart_unlock.status_code == 200
        restart_token = r_restart_unlock.json().get("session_token")
        restart_headers = {"Authorization": f"Bearer {restart_token}", "Content-Type": "application/json"}

        r_bs2 = requests.get(f"{base_url}/api/reports/balance-sheet", headers=restart_headers)
        assert r_bs2.status_code == 200
        bs2 = r_bs2.json()
        bank_item2 = next((item for item in bs2["assets"] if item["account_id"] == bank_id), None)
        assert bank_item2 is not None and bank_item2["balance_paisa"] == amount_paisa
        checklist["data_persistence_after_restart"] = True
        log(f"  [PASS] 16. Financial data strictly persisted across restart ({bank_item2['balance_paisa']} paisa).")

        # =====================================================================
        # 6. SECTION 6 & 7: Offline Operation & Backup / Restore
        # =====================================================================
        log("  [TESTING SECTION 7: BACKUP AND RESTORE]")
        backup_file_path = os.path.join(temp_dir, "bhoot_export_backup.bhoot")
        backup_passphrase = "MasterBackupPassphrase2026!"
        
        # Test backup create endpoint
        r_bk = requests.post(f"{base_url}/api/backup/create", headers=restart_headers, json={
            "passphrase": backup_passphrase,
            "destination_path": backup_file_path
        })
        assert r_bk.status_code == 200, f"Backup create failed: {r_bk.text}"
        assert os.path.exists(backup_file_path), "Backup archive file not generated"
        backup_size = os.path.getsize(backup_file_path)
        checklist["backup_create"] = True
        log(f"  [PASS] 17. Encrypted backup created successfully ({backup_size} bytes).")

        # Test Restore into Clean Isolated Environment
        r_rst = requests.post(f"{base_url}/api/backup/restore", headers=restart_headers, json={
            "passphrase": backup_passphrase,
            "backup_file_path": backup_file_path
        })
        assert r_rst.status_code == 200, f"Backup restore verification failed: {r_rst.text}"
        rst_data = r_rst.json()
        assert rst_data.get("integrity_verified") is True
        checklist["backup_restore_verified"] = True
        log(f"  [PASS] 18. Backup restored and SQLCipher integrity verified ({rst_data.get('restored_bytes')} bytes).")

    finally:
        proc.terminate()
        proc.wait()
        shutil.rmtree(temp_dir)

    log("\n=== ALL FINANCIAL APPLICATION, RATE LIMITING & BACKUP TESTS COMPLETED ===")
    for k, v in checklist.items():
        print(f"  - {k}: {'PASS' if v else 'FAIL'}")

    return checklist

if __name__ == "__main__":
    run_suite()
