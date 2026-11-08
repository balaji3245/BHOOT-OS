#!/usr/bin/env python3
"""
Adversarial Verification Suite for BHOOT Authentication Remediation
Tests all 17 critical authentication, enrollment, persistence, and brute-force gates.
"""

import os
import sys
import time
import json
import shutil
import tempfile
import subprocess
import requests

def run_tests():
    print("========================================================================")
    print(" [ADVERSARIAL AUTH AUDIT] Testing BHOOT v1.0 Authentication Remediation")
    print("========================================================================")

    temp_dir = tempfile.mkdtemp(prefix="bhoot_auth_audit_")
    db_path = os.path.join(temp_dir, "bhoot.db")
    vault_dir = os.path.join(temp_dir, "vault")
    os.makedirs(vault_dir, exist_ok=True)

    port = 19420
    base_url = f"http://127.0.0.1:{port}"

    # Compile desktop binary in release or debug
    bin_path = "/home/bhoot/Desktop/BHOOT/target/debug/bhoot-desktop"
    if not os.path.exists(bin_path):
        print(f"FATAL: Binary not found at {bin_path}")
        sys.exit(1)

    proc = None
    try:
        # Phase 1: Launch server with NO BHOOT_MASTER_PIN environment variable
        env = os.environ.copy()
        env.pop("BHOOT_MASTER_PIN", None)
        env["BHOOT_DB_KEY"] = "5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a"

        print("\n--- TEST PHASE 1: Fresh Installation Fails Closed & Unenrolled ---")
        proc = subprocess.Popen(
            [bin_path, "--port", str(port), "--db", db_path, "--vault-dir", vault_dir],
            env=env,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True
        )

        time.sleep(1.5)

        # 1. Health check
        r = requests.get(f"{base_url}/api/health")
        assert r.status_code == 200, f"Health check failed: {r.status_code}"
        print(" [PASS] Test 1: Server started on 127.0.0.1 and health endpoint is active.")

        # 2. Security status reports auth_enrolled: false
        r = requests.get(f"{base_url}/api/security/status")
        assert r.status_code == 200
        sec_json = r.json()
        assert sec_json.get("auth_enrolled") is False, f"Expected auth_enrolled=False, got: {sec_json}"
        assert sec_json.get("session_locked") is True
        print(" [PASS] Test 2: Security status accurately reports auth_enrolled=false and session_locked=true.")

        # 3. Protected endpoint rejected (401)
        r = requests.get(f"{base_url}/api/accounts")
        assert r.status_code == 401, f"Expected 401, got: {r.status_code}"
        assert "First-owner enrollment required" in r.text
        print(" [PASS] Test 3: Unauthenticated access rejected with HTTP 401 (First-owner enrollment required).")

        # 4. Direct mutation endpoint rejected (401)
        r = requests.post(f"{base_url}/api/accounts", json={"name": "Hacked", "account_type": "Asset", "subtype": "BankAccount", "currency": "INR"})
        assert r.status_code == 401, f"Expected 401, got: {r.status_code}"
        print(" [PASS] Test 4: Unauthenticated mutation rejected with HTTP 401.")

        # 5. Unlock fails closed when unenrolled (400)
        r = requests.post(f"{base_url}/api/auth/unlock", json={"pin": "1234"})
        assert r.status_code == 400, f"Expected 400, got: {r.status_code}"
        assert "First-owner enrollment required" in r.text
        print(" [PASS] Test 5: Default PIN '1234' rejected with HTTP 400 (no default credentials exist).")

        print("\n--- TEST PHASE 2: PIN Policy & First-Owner Enrollment ---")

        # 6. Weak PIN rejections
        weak_pins = [
            ("short", "12345"),
            ("repetitive_digits", "111111"),
            ("repetitive_zeros", "000000"),
            ("sequential_asc", "123456"),
            ("sequential_desc", "654321"),
            ("trivial_word", "password"),
            ("whitespace", "748 192"),
        ]
        for name, pin in weak_pins:
            r = requests.post(f"{base_url}/api/auth/enroll", json={"pin": pin, "confirm_pin": pin})
            assert r.status_code == 400, f"Weak PIN '{pin}' ({name}) was not rejected! Status: {r.status_code}, Body: {r.text}"
            print(f" [PASS] Test 6 ({name}): Rejected weak PIN '{pin}' ({r.json().get('error')})")

        # 7. Mismatched confirmation rejected
        r = requests.post(f"{base_url}/api/auth/enroll", json={"pin": "748192", "confirm_pin": "999999"})
        assert r.status_code == 400
        assert "confirmation does not match" in r.text
        print(" [PASS] Test 7: Mismatched PIN confirmation rejected with HTTP 400.")

        # 8. Valid enrollment
        r = requests.post(f"{base_url}/api/auth/enroll", json={"pin": "748192", "confirm_pin": "748192"})
        assert r.status_code == 200, f"Enrollment failed: {r.status_code} {r.text}"
        enroll_data = r.json()
        assert enroll_data.get("success") is True
        token = enroll_data.get("session_token")
        assert token and len(token) > 20
        print(f" [PASS] Test 8: Valid first-owner enrollment succeeded. Token issued: {token}")

        # 9. Second enrollment rejected (409 Conflict)
        r = requests.post(f"{base_url}/api/auth/enroll", json={"pin": "839201", "confirm_pin": "839201"})
        assert r.status_code == 409, f"Expected 409 Conflict, got: {r.status_code}"
        print(" [PASS] Test 9: Duplicate enrollment strictly rejected with HTTP 409 Conflict.")

        # 10. Access protected API using session token
        r = requests.get(f"{base_url}/api/accounts", headers={"Authorization": f"Bearer {token}"})
        assert r.status_code == 200
        accounts = r.json()
        assert len(accounts) >= 2
        print(f" [PASS] Test 10: Authenticated API request succeeded with {len(accounts)} accounts returned.")

        # 11. Lock session and verify token invalidation
        r = requests.post(f"{base_url}/api/auth/lock", headers={"Authorization": f"Bearer {token}"})
        assert r.status_code == 200
        # Re-using old token must fail with 401
        r = requests.get(f"{base_url}/api/accounts", headers={"Authorization": f"Bearer {token}"})
        assert r.status_code == 401
        print(" [PASS] Test 11: Session locked and previous token strictly revoked.")

        print("\n--- TEST PHASE 3: Brute-Force Rate Limiting & Lockout ---")

        # 12. 4 failed attempts: HTTP 401
        for i in range(1, 5):
            r = requests.post(f"{base_url}/api/auth/unlock", json={"pin": "wrong_pin"})
            assert r.status_code == 401, f"Attempt {i} expected 401, got {r.status_code}"
        print(" [PASS] Test 12A: 4 consecutive wrong attempts rejected with HTTP 401.")

        # 13. 5th failed attempt: HTTP 429 Lockout
        r = requests.post(f"{base_url}/api/auth/unlock", json={"pin": "wrong_pin"})
        assert r.status_code == 429, f"Attempt 5 expected 429, got {r.status_code}"
        assert "Rate limit exceeded" in r.text
        print(" [PASS] Test 12B: 5th failed attempt triggered HTTP 429 rate limit lockout.")

        # 14. Restart cannot reset lockout
        print(" Simulating service restart during active lockout...")
        proc.terminate()
        proc.wait(timeout=5)
        proc = subprocess.Popen(
            [bin_path, "--port", str(port), "--db", db_path, "--vault-dir", vault_dir],
            env=env,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True
        )
        time.sleep(1.5)

        # Even with correct PIN, must remain locked out
        r = requests.post(f"{base_url}/api/auth/unlock", json={"pin": "748192"})
        assert r.status_code == 429, f"Lockout was bypassed by restarting service! Status: {r.status_code}"
        print(" [PASS] Test 13: Restarting service CANNOT bypass rate limit lockout (HTTP 429 preserved).")

        print("\n--- TEST PHASE 4: Cold Persistence & Credential Mutation ---")
        print(" Waiting for 61s lockout cooldown to expire...")
        time.sleep(61)

        # 15. Unlock post-cooldown with correct enrolled PIN
        r = requests.post(f"{base_url}/api/auth/unlock", json={"pin": "748192"})
        assert r.status_code == 200, f"Expected 200 post-cooldown, got: {r.status_code} {r.text}"
        token2 = r.json()["session_token"]
        print(f" [PASS] Test 14: Post-cooldown unlock succeeded with enrolled PIN. New token: {token2}")

        # 16. Update PIN to new secure value
        r = requests.post(f"{base_url}/api/auth/set-pin", json={
            "current_pin": "748192",
            "new_pin": "839201",
            "confirm_new_pin": "839201"
        }, headers={"Authorization": f"Bearer {token2}"})
        assert r.status_code == 200, f"PIN update failed: {r.status_code} {r.text}"
        print(" [PASS] Test 15: Owner PIN successfully changed to '839201'.")

        # 17. Restart service and verify new PIN survives restart while old PIN fails
        print(" Simulating complete system restart / cold reboot...")
        proc.terminate()
        proc.wait(timeout=5)
        proc = subprocess.Popen(
            [bin_path, "--port", str(port), "--db", db_path, "--vault-dir", vault_dir],
            env=env,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True
        )
        time.sleep(1.5)

        # Old PIN rejected
        r = requests.post(f"{base_url}/api/auth/unlock", json={"pin": "748192"})
        assert r.status_code == 401, f"Old PIN was still accepted after update! Status: {r.status_code}"
        print(" [PASS] Test 16A: Old PIN '748192' correctly rejected post-reboot.")

        # New PIN accepted
        r = requests.post(f"{base_url}/api/auth/unlock", json={"pin": "839201"})
        assert r.status_code == 200, f"New PIN rejected post-reboot! Status: {r.status_code} {r.text}"
        print(" [PASS] Test 16B: Updated PIN '839201' survived restart and unlocked system.")

        # 18. Ciphertext inspection of database file on disk
        with open(db_path, "rb") as f:
            header_bytes = f.read(16)
        assert b"SQLite format 3" not in header_bytes, "FATAL: Plaintext SQLite header exposed!"
        print(" [PASS] Test 17: Database on disk is pure SQLCipher ciphertext (zero SQLite magic bytes).")

        print("\n========================================================================")
        print(" [ALL 17/17 ADVERSARIAL AUTH TESTS PASSED 100%]")
        print("========================================================================")

    finally:
        if proc:
            proc.terminate()
            try:
                proc.wait(timeout=5)
            except Exception:
                proc.kill()
        shutil.rmtree(temp_dir, ignore_errors=True)

if __name__ == "__main__":
    run_tests()

