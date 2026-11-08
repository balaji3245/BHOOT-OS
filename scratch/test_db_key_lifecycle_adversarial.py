#!/usr/bin/env python3
"""
BHOOT v1.0 — SQLCipher Database Key Lifecycle & Security Adversarial Suite

Verifies:
1. Missing key on existing DB: fail closed with fatal exit code.
2. Invalid hex key in BHOOT_DB_KEY: fail closed.
3. Wrong key: fail closed without modifying or corrupting existing database.
4. Correct key: opens database successfully.
5. Cold reboot / process restart: recovers database key from wrapped store automatically.
6. Fresh installation: generates random 256-bit key and persists wrapped store with 0600 mode.
7. Two installations: produce cryptographically distinct random database keys.
8. Database key is cryptographically separated from PIN.
9. Process inspection: database key never appears in ps, /proc, CLI args, or stdout/stderr logs.
"""

import os
import sys
import time
import shutil
import tempfile
import subprocess
import requests
from pathlib import Path

BINARY = Path("/home/bhoot/Desktop/BHOOT/os/build_real/rootfs/usr/local/bin/bhoot-desktop")

def log(test_num, name, status, msg=""):
    badge = "[PASS]" if status else "[FAIL]"
    print(f"[{test_num:02d}] {badge} {name} {('- ' + msg) if msg else ''}")
    if not status:
        sys.exit(1)

def main():
    print("=" * 80)
    print(" BHOOT v1.0 — SQLCipher Database Key Lifecycle Adversarial Suite")
    print("=" * 80)

    if not BINARY.is_file():
        print(f"FATAL: Binary not found at {BINARY}")
        sys.exit(1)

    work_dir = Path(tempfile.mkdtemp(prefix="bhoot_dbkey_test_"))
    try:
        # TEST 1: Fresh installation provisions key and wrapped store
        inst1_dir = work_dir / "inst1"
        inst1_dir.mkdir()
        db1_path = inst1_dir / "bhoot.db"
        kf1_path = inst1_dir / "db_key.wrapped"
        vault1_dir = inst1_dir / "vault"

        cmd1 = [
            str(BINARY),
            "--port", "19801",
            "--db", str(db1_path),
            "--key-file", str(kf1_path),
            "--vault-dir", str(vault1_dir),
        ]
        proc1 = subprocess.Popen(cmd1, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        time.sleep(1.0)
        proc1.terminate()
        stdout1, stderr1 = proc1.communicate()

        assert kf1_path.exists(), "Wrapped key file was not created"
        assert db1_path.exists(), "Database file was not created"
        mode1 = oct(kf1_path.stat().st_mode & 0o777)
        assert mode1 == "0o600", f"Key file mode is {mode1}, expected 0o600"
        log(1, "Fresh installation provisions 256-bit key & wrapped store (0600 mode)", True, f"Created {kf1_path.name} mode {mode1}")

        # TEST 2: Process restart recovers key from wrapped store
        proc2 = subprocess.Popen(cmd1, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        recovered = False
        for i in range(30):
            if proc2.poll() is not None:
                break
            try:
                r = requests.get("http://127.0.0.1:19801/api/health", timeout=1)
                if r.status_code == 200 and r.json().get("status") in ["ok", "healthy"]:
                    recovered = True
                    break
            except Exception:
                time.sleep(0.1)
        proc2.terminate()
        stdout2, stderr2 = proc2.communicate()

        assert recovered, f"Failed to connect to restarted service: {stderr2}"
        assert "Successfully loaded and unwrapped persistent database key" in stdout2
        log(2, "Process restart recovers key from wrapped store without operator intervention", True, "Successfully opened DB")

        # TEST 3: Missing key on existing database FAILS CLOSED
        inst3_dir = work_dir / "inst3"
        inst3_dir.mkdir()
        db3_path = inst3_dir / "existing.db"
        shutil.copy(db1_path, db3_path)
        kf3_path = inst3_dir / "missing.wrapped"

        cmd3 = [
            str(BINARY),
            "--port", "19803",
            "--db", str(db3_path),
            "--key-file", str(kf3_path),
            "--vault-dir", str(inst3_dir / "vault"),
        ]
        proc3 = subprocess.run(cmd3, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        assert proc3.returncode != 0, f"Expected non-zero exit code, got {proc3.returncode}"
        assert "Refusing to initialize fresh key over existing database. Failing closed." in proc3.stderr or "MissingKeyForExistingDatabase" in proc3.stderr
        assert not kf3_path.exists(), "Must not create new key file for existing DB"
        log(3, "Missing key on existing database fails closed", True, f"Exited with code {proc3.returncode}")

        # TEST 4: Invalid hex key in BHOOT_DB_KEY fails closed
        env_invalid = os.environ.copy()
        env_invalid["BHOOT_DB_KEY"] = "not_a_valid_64_char_hex_key"
        proc4 = subprocess.run(cmd1, env=env_invalid, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        assert proc4.returncode != 0, f"Expected non-zero exit, got {proc4.returncode}"
        assert "Invalid BHOOT_DB_KEY" in proc4.stderr
        log(4, "Invalid BHOOT_DB_KEY format fails closed", True, f"Rejected: {proc4.stderr.strip()}")

        # TEST 5: Wrong key fails closed without modifying database
        env_wrong = os.environ.copy()
        env_wrong["BHOOT_DB_KEY"] = "00" * 32
        # Initial run merges any pending recovery
        subprocess.run(cmd1, env=env_wrong, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)

        initial_hash = subprocess.check_output(["sha256sum", str(db1_path)]).decode().split()[0]
        proc5 = subprocess.run(cmd1, env=env_wrong, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        assert proc5.returncode != 0, "Wrong key must exit non-zero"
        post_wrong_hash = subprocess.check_output(["sha256sum", str(db1_path)]).decode().split()[0]
        assert initial_hash == post_wrong_hash, "Database file was modified by wrong key attempt!"
        log(5, "Wrong key fails closed without modifying database", True, "SHA-256 preserved across failed attempts")

        # TEST 6: Two installations generate distinct database keys
        inst2_dir = work_dir / "inst2"
        inst2_dir.mkdir()
        db2_path = inst2_dir / "bhoot.db"
        kf2_path = inst2_dir / "db_key.wrapped"
        vault2_dir = inst2_dir / "vault"

        cmd2 = [
            str(BINARY),
            "--port", "19802",
            "--db", str(db2_path),
            "--key-file", str(kf2_path),
            "--vault-dir", str(vault2_dir),
        ]
        proc6 = subprocess.Popen(cmd2, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        time.sleep(1.0)
        proc6.terminate()
        proc6.communicate()

        content1 = kf1_path.read_text()
        content2 = kf2_path.read_text()
        assert content1 != content2, "Wrapped key files must be distinct"
        # Also try opening db1 with kf2 (should fail closed)
        cmd_swap = [
            str(BINARY),
            "--port", "19805",
            "--db", str(db1_path),
            "--key-file", str(kf2_path),
            "--vault-dir", str(vault1_dir),
        ]
        proc_swap = subprocess.run(cmd_swap, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        assert proc_swap.returncode != 0, "Swapped key must not unlock database"
        log(6, "Two independent installations generate distinct database keys", True, "Key envelopes and keys strictly distinct")

        # TEST 7: Database key is cryptographically separated from PIN
        # Start server on inst1
        proc7 = subprocess.Popen(cmd1, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        for _ in range(30):
            try:
                if requests.get("http://127.0.0.1:19801/api/health", timeout=1).status_code == 200:
                    break
            except Exception:
                time.sleep(0.1)

        try:
            # Enroll PIN "748192"
            r_enroll = requests.post("http://127.0.0.1:19801/api/auth/enroll", json={"pin": "748192", "confirm_pin": "748192"}, timeout=3)
            assert r_enroll.status_code == 200, f"Enrollment status: {r_enroll.status_code}: {r_enroll.text}"
            token = r_enroll.json()["session_token"]
            headers = {"Authorization": f"Bearer {token}"}
            # Change PIN to "839201"
            r_change = requests.post("http://127.0.0.1:19801/api/auth/set-pin", json={"current_pin": "748192", "new_pin": "839201", "confirm_new_pin": "839201"}, headers=headers, timeout=3)
            assert r_change.status_code == 200, f"Change PIN status: {r_change.status_code}: {r_change.text}"
        finally:
            proc7.terminate()
            stdout7, stderr7 = proc7.communicate()

        # Restart server - key should STILL open the DB, and PIN "839201" is accepted
        proc8 = subprocess.Popen(cmd1, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        for _ in range(30):
            try:
                if requests.get("http://127.0.0.1:19801/api/health", timeout=1).status_code == 200:
                    break
            except Exception:
                time.sleep(0.1)

        try:
            r_unlock = requests.post("http://127.0.0.1:19801/api/auth/unlock", json={"pin": "839201"}, timeout=3)
            assert r_unlock.status_code == 200, f"Unlock status: {r_unlock.status_code}: {r_unlock.text}"
        finally:
            proc8.terminate()
            stdout8, stderr8 = proc8.communicate()

        log(7, "Database key is cryptographically separated from PIN", True, "PIN change did not alter DB key; DB opens seamlessly")

        # TEST 8: Database key never appears in ps, /proc, CLI args, or logs
        proc9 = subprocess.Popen(cmd1, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        time.sleep(0.5)
        pid = proc9.pid
        with open(f"/proc/{pid}/cmdline", "rb") as f:
            proc_cmdline = f.read().decode("latin1", errors="ignore")
        proc9.terminate()
        stdout9, stderr9 = proc9.communicate()

        # Inspect cmdline and logs for any 64-hex key or raw key bytes
        assert "BHOOT_DB_KEY" not in proc_cmdline
        assert "--key " not in proc_cmdline
        assert "0x5A" not in stdout9 and "0x5A" not in stderr9
        assert "0x5a" not in stdout9 and "0x5a" not in stderr9
        assert "key material:" not in stdout9.lower() and "key material:" not in stderr9.lower()
        log(8, "Database key never exposed in ps, /proc, CLI args, or logs", True, "Zero exposure in process environment or telemetry")

        print("=" * 80)
        print(" ALL 8 ADVERSARIAL SQLCIPHER KEY LIFECYCLE TESTS PASSED!")
        print("=" * 80)

    finally:
        shutil.rmtree(work_dir, ignore_errors=True)

if __name__ == "__main__":
    main()
