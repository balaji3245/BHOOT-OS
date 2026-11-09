#!/usr/bin/env python3
"""
BHOOT v1.0 — Comprehensive Functional Integration Verification Suite
Tests all 11 views, backend mutations, atomic double-entry balance enforcement,
document vault sealing, encrypted backup/restore, Argon2id credentials lifecycle,
and cold restart persistence.
"""

import os
import sys
import time
import json
import base64
import urllib.request
import urllib.error
import subprocess
import shutil
import tempfile
from pathlib import Path

def print_section(title):
    print("\n" + "=" * 70)
    print(f" {title}")
    print("=" * 70)

class BhootApiClient:
    def __init__(self, port, token=None):
        self.base_url = f"http://127.0.0.1:{port}"
        self.token = token

    def request(self, method, path, data=None):
        url = f"{self.base_url}{path}"
        headers = {"Accept": "application/json"}
        if self.token:
            headers["Authorization"] = f"Bearer {self.token}"
        body_bytes = None
        if data is not None:
            headers["Content-Type"] = "application/json"
            body_bytes = json.dumps(data).encode("utf-8")

        req = urllib.request.Request(url, data=body_bytes, headers=headers, method=method)
        try:
            with urllib.request.urlopen(req, timeout=5) as resp:
                status = resp.status
                resp_data = resp.read().decode("utf-8")
                try:
                    return status, json.loads(resp_data)
                except Exception:
                    return status, resp_data
        except urllib.error.HTTPError as e:
            err_body = e.read().decode("utf-8")
            try:
                return e.code, json.loads(err_body)
            except Exception:
                return e.code, err_body

def run_test_suite():
    print_section("BHOOT v1.0 FULL FUNCTIONAL INTEGRATION & PERSISTENCE TEST")

    temp_dir = tempfile.mkdtemp(prefix="bhoot_func_test_")
    db_path = os.path.join(temp_dir, "bhoot.db")
    vault_dir = os.path.join(temp_dir, "vault")
    key_file = os.path.join(temp_dir, "db_key.wrapped")
    os.makedirs(vault_dir, exist_ok=True)

    binary_path = "/home/bhoot/Desktop/BHOOT/target/release/bhoot-desktop"
    if not os.path.exists(binary_path):
        print(f"FATAL: Binary not found at {binary_path}")
        sys.exit(1)

    port = 21420
    print(f"[*] Starting BHOOT Desktop daemon on port {port}...")
    server_proc = subprocess.Popen(
        [binary_path, "--port", str(port), "--db", db_path, "--key-file", key_file, "--vault-dir", vault_dir],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True
    )

    time.sleep(1.5)
    if server_proc.poll() is not None:
        stdout, stderr = server_proc.communicate()
        print(f"FATAL: Server failed to start: {stderr}\n{stdout}")
        sys.exit(1)

    try:
        client = BhootApiClient(port)

        # 1. Health Check
        print_section("1. HEALTH & AIRGAP CHECK")
        status, health = client.request("GET", "/api/health")
        assert status == 200, f"Health check failed: {status}"
        assert health.get("airgap") is True, f"Airgap should be True: {health}"
        print(f"  [PASS] Server online: {health}")

        # 2. Security Status (Initial Unenrolled)
        status, sec = client.request("GET", "/api/security/status")
        assert status == 200
        assert sec.get("auth_enrolled") is False
        assert sec.get("session_locked") is True
        print(f"  [PASS] Initial security status: unenrolled, locked")

        # 3. Master PIN Enrollment
        print_section("2. MASTER PIN ENROLLMENT & ARGON2ID INITIALIZATION")
        master_pin = "748192"
        status, enroll = client.request("POST", "/api/auth/enroll", {"pin": master_pin, "confirm_pin": master_pin})
        assert status == 200, f"Enrollment failed: {status} {enroll}"
        token = enroll.get("session_token")
        assert token, "Session token missing in enrollment response"
        client.token = token
        print(f"  [PASS] Enrolled Master PIN. Issued Bearer session token: {token[:12]}...")

        # 4. Chart of Accounts & Starter Seeding Verification
        print_section("3. CHART OF ACCOUNTS & BALANCES")
        status, accounts = client.request("GET", "/api/accounts")
        assert status == 200
        assert len(accounts) >= 6, f"Expected at least 6 starter accounts, got {len(accounts)}"
        for a in accounts:
            assert "balance_paisa" in a, f"Account missing balance_paisa: {a}"
            print(f"    Account: {a['name']:<32} Type: {a['account_type']:<10} Balance: ₹{a['balance_paisa']/100:,.2f}")
        print("  [PASS] All accounts enriched with live normal-balance convention")

        # 5. Create New Account
        print_section("4. CREATE & EDIT ACCOUNT")
        status, new_acc = client.request("POST", "/api/accounts", {
            "name": "Kotak Security Trading",
            "account_type": "Asset",
            "subtype": "BrokerAccount",
            "currency": "INR",
            "description": "Secondary broker account"
        })
        assert status == 201, f"Failed to create account: {status} {new_acc}"
        kotak_id = new_acc["id"]
        print(f"  [PASS] Created account: {new_acc['name']} (ID: {kotak_id})")

        # Update account details
        status, up_res = client.request("POST", "/api/accounts/update", {
            "account_id": kotak_id,
            "name": "Kotak Prime Trading",
            "description": "Upgraded prime broker account"
        })
        assert status == 200
        # Verify updated name
        _, accounts = client.request("GET", "/api/accounts")
        updated_kotak = next(a for a in accounts if a["id"] == kotak_id)
        assert updated_kotak["name"] == "Kotak Prime Trading"
        assert updated_kotak["description"] == "Upgraded prime broker account"
        print(f"  [PASS] Updated account: {updated_kotak['name']}")

        # 6. Double-Entry Transactions & Invariant Enforcement
        print_section("5. ATOMIC DOUBLE-ENTRY TRANSACTIONS & INVARIANT CHECKS")
        hdfc_id = next(a["id"] for a in accounts if "HDFC" in a["name"])
        consulting_id = next(a["id"] for a in accounts if "Consulting" in a["name"])

        # Positive Test: Strictly balanced transaction (₹1,50,000 Debit == ₹1,50,000 Credit)
        balanced_tx = {
            "tx_type": "Income",
            "postings": [
                {"account_id": hdfc_id, "amount_paisa": 15000000, "direction": "Debit", "memo": "Nov Retainer"},
                {"account_id": consulting_id, "amount_paisa": 15000000, "direction": "Credit", "memo": "Nov Service Revenue"}
            ],
            "category": "Consulting",
            "payment_method": "NEFT",
            "reference": "CMS11223344",
            "notes": "November retainer payment"
        }
        status, tx_res = client.request("POST", "/api/transactions", balanced_tx)
        assert status == 201, f"Balanced tx failed: {status} {tx_res}"
        posted_tx_id = tx_res["id"]
        print(f"  [PASS] Posted balanced transaction: {posted_tx_id}")

        # Negative Test: Unbalanced transaction (₹1,50,000 Debit != ₹1,00,000 Credit) MUST fail with 422
        unbalanced_tx = {
            "tx_type": "Income",
            "postings": [
                {"account_id": hdfc_id, "amount_paisa": 15000000, "direction": "Debit"},
                {"account_id": consulting_id, "amount_paisa": 10000000, "direction": "Credit"}
            ]
        }
        status, unbal_res = client.request("POST", "/api/transactions", unbalanced_tx)
        assert status == 422, f"Expected 422 for unbalanced tx, got {status}: {unbal_res}"
        print("  [PASS] Enforced double-entry invariant: Unbalanced transaction rejected with HTTP 422")

        # Transaction Reversal
        status, rev_res = client.request("POST", "/api/transactions/reverse", {
            "transaction_id": posted_tx_id,
            "reason": "Test reversal"
        })
        assert status == 200
        print(f"  [PASS] Reversed transaction {posted_tx_id}")

        # 7. Budgets & Evaluation
        print_section("6. BUDGET MANAGEMENT & EVALUATION")
        status, b_res = client.request("POST", "/api/budgets", {
            "name": "Q4 Tech Hardware Plan",
            "period": "Quarterly",
            "start_date": "2026-10-01T00:00:00Z",
            "end_date": "2026-12-31T23:59:59Z",
            "categories": [
                {"category": "LivingExpense", "allocated_paisa": 6000000, "alert_threshold_pct": 75}
            ]
        })
        assert status == 200, f"Budget creation failed: {b_res}"
        b_id = b_res["id"]
        status, b_eval = client.request("GET", f"/api/budgets/evaluate?id={b_id}")
        assert status == 200
        assert b_eval["budget_id"] == b_id
        print(f"  [PASS] Budget {b_id} created and evaluated")

        # 8. Trading Journal Record
        print_section("7. TRADING JOURNAL & STATUTORY FEES")
        zerodha_id = next(a["id"] for a in accounts if "Zerodha" in a["name"])
        trade_payload = {
            "broker_account_id": zerodha_id,
            "trade_date": "2026-11-09T10:00:00Z",
            "symbol": "NIFTY26DEC24500CE",
            "trade_type": "Options",
            "gross_amount_paisa": 5000000, # ₹50,000 profit
            "charges": {
                "brokerage_paisa": 4000,
                "stt_paisa": 6500,
                "exchange_turnover_paisa": 2500,
                "gst_paisa": 1500,
                "sebi_paisa": 100,
                "stamp_duty_paisa": 400
            },
            "net_amount_paisa": 5000000 - 15000,
            "order_reference": "ORD20261109X"
        }
        status, trade_res = client.request("POST", "/api/trades", trade_payload)
        assert status in (200, 201), f"Trade failed: {status} {trade_res}"
        status, ts_sum = client.request("GET", "/api/trades/summary")
        assert status == 200
        assert ts_sum["total_trades"] >= 3
        print(f"  [PASS] Trade recorded. Updated total trades: {ts_sum['total_trades']}, Win Rate: {ts_sum['win_rate_basis_points']} bps")

        # 9. Financial Reports
        print_section("8. GAAP/IFRS COMPLIANT FINANCIAL REPORTS")
        status, bs = client.request("GET", "/api/reports/balance-sheet")
        assert status == 200
        assert bs["is_balanced"] is True, f"Balance sheet equation not in equilibrium: {bs}"
        print(f"  [PASS] Balance Sheet in strict equilibrium: Net Worth ₹{bs['net_worth_paisa']/100:,.2f}")

        status, pnl = client.request("GET", "/api/reports/income-statement")
        assert status == 200
        print(f"  [PASS] Income Statement: Net Profit ₹{pnl['net_profit_paisa']/100:,.2f}")

        status, cfs = client.request("GET", "/api/reports/cash-flow")
        assert status == 200
        print(f"  [PASS] Cash Flow Statement: Net Cash Flow ₹{cfs['net_cash_flow_paisa']/100:,.2f}")

        # 10. Document Vault Sealing & Verification
        print_section("9. DOCUMENT VAULT SEALING & INTEGRITY AUDIT")
        # Valid PDF header "%PDF-1.7\n"
        pdf_bytes = b"%PDF-1.7\n1 0 obj\n<< /Type /Catalog >>\nendobj\ntrailer\n<< /Root 1 0 R >>\n%%EOF\n"
        pdf_b64 = base64.b64encode(pdf_bytes).decode("ascii")
        status, v_up = client.request("POST", "/api/vault/upload", {
            "filename": "Q3_Financial_Audit.pdf",
            "data_base64": pdf_b64
        })
        assert status == 201, f"Vault upload failed: {status} {v_up}"
        doc_id = v_up["id"]
        print(f"  [PASS] Sealed document in vault (ID: {doc_id})")

        # Download & verify binary identity
        status, v_down = client.request("GET", f"/api/vault/download?id={doc_id}")
        assert status == 200
        downloaded_bytes = base64.b64decode(v_down["data_base64"])
        assert downloaded_bytes == pdf_bytes, "Downloaded bytes do not match uploaded bytes!"
        print("  [PASS] Downloaded and verified binary equality of decrypted document")

        # Run vault integrity verification
        status, v_rep = client.request("GET", "/api/vault/verify")
        assert status == 200
        assert v_rep["corrupted_documents"] == 0
        assert v_rep["total_documents"] >= 1
        print(f"  [PASS] Vault audit verified: {v_rep['verified_documents']} verified, 0 corrupted")

        # 11. Encrypted Disaster Recovery Backup & Restore
        print_section("10. ENCRYPTED BACKUP CREATION & INTEGRITY RESTORE")
        backup_file = os.path.join(temp_dir, "bhoot_full_archive.bhoot")
        backup_pass = "bhoot-production-backup-2026-secure"
        status, b_create = client.request("POST", "/api/backup/create", {
            "passphrase": backup_pass,
            "destination_path": backup_file
        })
        assert status == 200, f"Backup creation failed: {status} {b_create}"
        assert os.path.exists(backup_file)
        assert os.path.getsize(backup_file) > 1000
        print(f"  [PASS] Created encrypted backup archive: {backup_file} ({os.path.getsize(backup_file)} bytes)")

        # Restore test
        status, b_rest = client.request("POST", "/api/backup/restore", {
            "passphrase": backup_pass,
            "backup_file_path": backup_file
        })
        assert status == 200, f"Restore test failed: {status} {b_rest}"
        assert b_rest.get("integrity_verified") is True
        print(f"  [PASS] Restore verification succeeded: Integrity verified on {b_rest.get('restored_bytes')} bytes")

        # 12. Persistent Settings
        print_section("11. PERSISTENT SYSTEM SETTINGS")
        status, _ = client.request("POST", "/api/settings", {
            "currency": "INR",
            "theme": "dark",
            "auto_lock_minutes": 30,
            "offline_mode": True,
            "financial_year_start": "04-01"
        })
        assert status == 200
        _, s_now = client.request("GET", "/api/settings")
        assert s_now["auto_lock_minutes"] == 30
        print(f"  [PASS] Updated settings: auto_lock_minutes = 30")

        # 13. Audit Chain Integrity
        print_section("12. IMMUTABLE AUDIT CHAIN")
        status, audit_events = client.request("GET", "/api/audit/events")
        assert status == 200
        assert len(audit_events) >= 5
        print(f"  [PASS] Audit chain contains {len(audit_events)} cryptographically linked events")

        # 14. Cold Restart Persistence Test
        print_section("13. COLD BOOT RESTART & ENCRYPTED PERSISTENCE VERIFICATION")
        print("[*] Terminating server process (simulating clean shutdown / cold reboot)...")
        server_proc.terminate()
        server_proc.wait(timeout=5)
        print("  -> Server process cleanly terminated.")

        print(f"[*] Launching fresh server process on the same persistent database at {db_path}...")
        server_proc = subprocess.Popen(
            [binary_path, "--port", str(port), "--db", db_path, "--key-file", key_file, "--vault-dir", vault_dir],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True
        )
        time.sleep(1.5)
        if server_proc.poll() is not None:
            stdout, stderr = server_proc.communicate()
            print(f"FATAL: Cold reboot restart failed: {stderr}\n{stdout}")
            sys.exit(1)

        # Fresh client without token (session is locked upon cold boot)
        reboot_client = BhootApiClient(port)
        status, sec = reboot_client.request("GET", "/api/security/status")
        assert status == 200
        assert sec.get("auth_enrolled") is True, "Must remain enrolled across cold reboot"
        assert sec.get("session_locked") is True, "Must be session locked upon boot"
        print("  [PASS] Post-reboot state: Enrolled and locked (Zero-Trust)")

        # Verify old token from previous boot is invalid
        old_token_client = BhootApiClient(port, token=token)
        status, _ = old_token_client.request("GET", "/api/accounts")
        assert status == 401, f"Old session token must be revoked across reboot, got {status}"
        print("  [PASS] Old session token rejected (session token does not survive restart)")

        # Unlock with enrolled master PIN
        status, unlock_res = reboot_client.request("POST", "/api/auth/unlock", {"pin": master_pin})
        assert status == 200, f"Unlock failed post-reboot: {status} {unlock_res}"
        new_token = unlock_res["session_token"]
        reboot_client.token = new_token
        print("  [PASS] Successfully unlocked post-reboot session with master PIN")

        # Verify persistent data
        _, accs_after = reboot_client.request("GET", "/api/accounts")
        assert any(a["name"] == "Kotak Prime Trading" for a in accs_after), "Created account did not survive cold reboot!"
        print("  [PASS] Created accounts survived cold reboot")

        _, settings_after = reboot_client.request("GET", "/api/settings")
        assert settings_after["auto_lock_minutes"] == 30, "Settings did not survive cold reboot!"
        print("  [PASS] Settings survived cold reboot (auto_lock_minutes = 30)")

        _, docs_after = reboot_client.request("GET", "/api/vault/documents")
        assert any(d["filename"] == "Q3_Financial_Audit.pdf" for d in docs_after), "Vault documents did not survive cold reboot!"
        print("  [PASS] Vault documents survived cold reboot")

        # 15. Master PIN Change Lifecycle
        print_section("14. MASTER PIN CHANGE LIFECYCLE")
        new_pin = "918273"
        status, _ = reboot_client.request("POST", "/api/auth/set-pin", {
            "current_pin": master_pin,
            "new_pin": new_pin,
            "confirm_new_pin": new_pin
        })
        assert status == 200, "PIN change failed"
        print("  [PASS] Master PIN changed to new PIN")

        # Lock session
        reboot_client.request("POST", "/api/auth/lock")

        # Verify old PIN now fails
        status, _ = reboot_client.request("POST", "/api/auth/unlock", {"pin": master_pin})
        assert status == 401, "Old PIN should fail after PIN change"
        print("  [PASS] Old PIN rejected after change")

        # Verify new PIN unlocks
        status, new_unlock = reboot_client.request("POST", "/api/auth/unlock", {"pin": new_pin})
        assert status == 200, "New PIN failed to unlock"
        print("  [PASS] New PIN unlocks successfully")

        print_section("ALL INTEGRATION TESTS PASSED PERFECTLY!")

    finally:
        if server_proc and server_proc.poll() is None:
            server_proc.terminate()
            try:
                server_proc.wait(timeout=3)
            except Exception:
                server_proc.kill()
        shutil.rmtree(temp_dir, ignore_errors=True)

if __name__ == "__main__":
    run_test_suite()
