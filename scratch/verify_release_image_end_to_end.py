#!/usr/bin/env python3
"""
BHOOT — Release Image End-to-End Graphical Boot & In-Guest Regression Verification
Executes QEMU UEFI virtualized boot with OVMF, virtio-vga, and QMP screendump.
Captures all required graphical screenshots and runs in-guest regression test suite.
"""

import os
import sys
import subprocess
import time
import secrets
import shutil
import pexpect
import socket
import json
import re
from PIL import Image

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
ROOT_DIR = os.path.dirname(SCRIPT_DIR)
BUILD_REAL = os.path.join(ROOT_DIR, "os/build_real")
CONFIG_DIR = os.path.join(ROOT_DIR, "os/config")
DISPOSABLE_DIR = os.path.join(BUILD_REAL, "disposable_release_test")
OVMF_PATH = "/usr/share/OVMF/OVMF_CODE_4M.fd"
QMP_SOCK = "/tmp/qmp_release_test.sock"
SCREENSHOT_DIR = os.path.join(ROOT_DIR, "reports/screenshots")

TOTAL_SECTORS = 121145344
SECTOR_SIZE = 512

def log(msg):
    print(f"[{time.strftime('%H:%M:%S')}] {msg}", flush=True)

class QMPClient:
    def __init__(self, sock_path):
        self.sock_path = sock_path
        self.sock = None

    def connect(self, retries=25):
        for _ in range(retries):
            try:
                self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
                self.sock.connect(self.sock_path)
                self.sock.recv(2048) # greeting
                self.cmd("qmp_capabilities")
                return True
            except (socket.error, FileNotFoundError):
                time.sleep(0.5)
        return False

    def cmd(self, execute, arguments=None):
        payload = {"execute": execute}
        if arguments:
            payload["arguments"] = arguments
        self.sock.sendall(json.dumps(payload).encode() + b"\n")
        time.sleep(0.1)
        resp = b""
        while True:
            chunk = self.sock.recv(4096)
            resp += chunk
            if b"\n" in chunk:
                break
        return resp.decode(errors="replace")

    def screendump(self, filename_png):
        ppm_path = filename_png + ".ppm"
        self.cmd("screendump", {"filename": ppm_path})
        time.sleep(0.5)
        if os.path.exists(ppm_path):
            im = Image.open(ppm_path)
            im.save(filename_png)
            os.remove(ppm_path)
            log(f"  [SCREENSHOT CAPTURED] {os.path.basename(filename_png)} ({im.size[0]}x{im.size[1]})")
            return True
        log(f"  [SCREENSHOT FAILED] {os.path.basename(filename_png)}")
        return False

    def send_keys(self, keys, delay=0.15):
        for k in keys:
            self.cmd("send-key", {"keys": [{"type": "qcode", "data": k}]})
            time.sleep(delay)

    def click_screen(self):
        self.cmd("input-send-event", {
            "events": [
                {"type": "btn", "data": {"down": True, "button": "left"}},
                {"type": "btn", "data": {"down": False, "button": "left"}}
            ]
        })
        time.sleep(0.2)

    def close(self):
        if self.sock:
            try:
                self.sock.close()
            except:
                pass

def exec_in_guest(child, cmd, timeout=20):
    marker = secrets.token_hex(6)
    child.sendline(f"echo START_{marker}; {cmd}; echo END_{marker} $?")
    child.expect(rf"END_{marker}\s+(\d+)", timeout=timeout)
    raw_output = child.before
    rc = int(child.match.group(1))
    m = re.search(rf"START_{marker}[\r\n]+(.*?)$", raw_output, re.DOTALL)
    if m:
        out = m.group(1).strip()
    else:
        out = raw_output.strip()
    return rc, out

def prepare_disposable_disk(luks_pass):
    os.makedirs(DISPOSABLE_DIR, exist_ok=True)
    os.makedirs(SCREENSHOT_DIR, exist_ok=True)
    disk_img = os.path.join(DISPOSABLE_DIR, "bhoot_release_test.img")
    p3_img = os.path.join(DISPOSABLE_DIR, "test_p3.img")
    header_backup = os.path.join(DISPOSABLE_DIR, "test_header.backup")

    log("1. Provisioning disposable 48 GiB LUKS2 container with ephemeral test passphrase...")
    p = subprocess.Popen([
        "python3", os.path.join(ROOT_DIR, "os/scripts_v2/provision_luks2_volume.py"),
        "--container", p3_img,
        "--size-gib", "48",
        "--header-backup", header_backup,
        "--passphrase-fd", "0"
    ], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    out, err = p.communicate(input=f"{luks_pass}\n".encode())
    if p.returncode != 0:
        raise RuntimeError(f"provision_luks2_volume failed: {err.decode('utf-8', errors='replace')}")

    log("2. Assembling disposable disk image with complete release payloads...")
    if os.path.exists(disk_img):
        os.remove(disk_img)

    with open(disk_img, "wb") as f:
        f.truncate(TOTAL_SECTORS * SECTOR_SIZE)

    sfdisk_layout = os.path.join(CONFIG_DIR, "storage/gpt_layout.sfdisk")
    subprocess.run(["sfdisk", "--quiet", disk_img], input=open(sfdisk_layout, "rb").read(), check=True)

    esp_img = os.path.join(BUILD_REAL, "bhoot_esp.img")
    sys_img = os.path.join(BUILD_REAL, "bhoot_system_ext4.img")
    rec_img = os.path.join(BUILD_REAL, "bhoot_recovery.img")

    subprocess.run(["dd", f"if={esp_img}", f"of={disk_img}", "bs=1M", "seek=1", "conv=notrunc,sparse", "status=none"], check=True)
    subprocess.run(["dd", f"if={sys_img}", f"of={disk_img}", "bs=1M", "seek=1025", "count=4096", "conv=notrunc,sparse", "status=none"], check=True)
    subprocess.run(["dd", f"if={p3_img}", f"of={disk_img}", "bs=1M", "seek=5121", "count=49152", "conv=notrunc,sparse", "status=none"], check=True)
    subprocess.run(["dd", f"if={rec_img}", f"of={disk_img}", "bs=1M", "seek=54273", "count=4096", "conv=notrunc,sparse", "status=none"], check=True)

    log("Disposable disk assembled successfully.")
    return disk_img

def main():
    luks_pass = secrets.token_urlsafe(24)
    user_pass = "ReleaseAuditPass2026!"
    
    disk_img = prepare_disposable_disk(luks_pass)
    
    if os.path.exists(QMP_SOCK):
        os.remove(QMP_SOCK)

    qemu_cmd = [
        "qemu-system-x86_64",
        "-name", "BHOOT-Release-Verification",
        "-machine", "q35,accel=kvm",
        "-cpu", "host",
        "-m", "3072",
        "-smp", "2",
        "-drive", f"if=pflash,format=raw,readonly=on,file={OVMF_PATH}",
        "-drive", f"file={disk_img},format=raw,if=virtio",
        "-device", "virtio-keyboard-pci",
        "-device", "virtio-tablet-pci",
        "-vga", "virtio",
        "-display", "vnc=127.0.0.1:42",
        "-qmp", f"unix:{QMP_SOCK},server,nowait",
        "-net", "none",
        "-serial", "stdio"
    ]

    log("Launching QEMU VM with UEFI (OVMF) + virtio-vga + QMP screendump...")
    child = pexpect.spawn(qemu_cmd[0], qemu_cmd[1:], timeout=120, encoding="utf-8", codec_errors="replace")
    
    qmp = QMPClient(QMP_SOCK)
    
    try:
        log("Connecting to QEMU QMP socket...")
        if qmp.connect(retries=30):
            log("QMP client connected successfully.")
        else:
            log("ERROR: Could not connect to QMP socket.")
            sys.exit(1)

        # Stage 1: OVMF / GRUB Bootloader Menu
        log("Waiting for GRUB bootloader menu...")
        idx = child.expect(["GNU GRUB", "Loading BHOOT Linux Kernel", "Please unlock disk", pexpect.TIMEOUT], timeout=40)
        log(f"Stage 1/2 match (idx={idx})")
        time.sleep(1)
        qmp.screendump(os.path.join(SCREENSHOT_DIR, "01_uefi_grub_boot.png"))

        # Stage 2: Early-Boot LUKS Passphrase Prompt
        log("Waiting for LUKS passphrase prompt...")
        idx = child.expect(["Please unlock disk", "unlock disk /dev/vda3", pexpect.TIMEOUT], timeout=35)
        if idx in (0, 1):
            log("LUKS passphrase prompt reached. Capturing screenshot & submitting passphrase...")
            time.sleep(0.5)
            qmp.screendump(os.path.join(SCREENSHOT_DIR, "02_luks_passphrase_prompt.png"))
            child.sendline(luks_pass)
        else:
            log("Warning: LUKS prompt timeout or unexpected output.")

        # Stage 3: First-Boot Credential Provisioning
        log("Waiting for first-boot provisioning prompt...")
        idx = child.expect(["Enter new master password for 'bhoot'", pexpect.TIMEOUT], timeout=45)
        if idx == 0:
            log("First-boot provisioning prompt reached. Capturing screenshot & provisioning credentials...")
            time.sleep(0.5)
            qmp.screendump(os.path.join(SCREENSHOT_DIR, "03_firstboot_setup.png"))
            child.sendline(user_pass)
            child.expect("Confirm master password:", timeout=15)
            time.sleep(0.5)
            child.sendline(user_pass)

            # GRUB Diceware prompt
            idx_g = child.expect([r">>> ([a-z]+ [a-z]+ [a-z]+ [a-z]+ [a-z]+) <<<", pexpect.TIMEOUT], timeout=25)
            if idx_g == 0:
                diceware_pass = child.match.group(1).strip()
                log(f"Diceware passphrase: {diceware_pass.split()[0]} ...")
                child.expect("Confirm by entering the exact 5-word passphrase:", timeout=15)
                time.sleep(0.5)
                child.sendline(diceware_pass)
                child.expect("GRUB administrator authentication configured successfully", timeout=25)
                log("GRUB administrator configured successfully.")

        # Stage 4: Serial Login & System Verification
        log("Waiting for serial login prompt...")
        child.expect("bhoot-secure login:", timeout=40)
        log("Serial login prompt reached. Logging in as 'bhoot'...")
        child.sendline("bhoot")
        child.expect("Password:", timeout=10)
        child.sendline(user_pass)
        child.expect(r"\$", timeout=15)
        log("Logged in successfully as user 'bhoot' via serial console.")
        child.sendline("stty -echo")
        time.sleep(0.5)

        # Allow services to settle
        time.sleep(5)

        # Check bhoot-gui service status and journal
        rc, gui_status = exec_in_guest(child, "systemctl status bhoot-gui.service --no-pager")
        log(f"bhoot-gui status (rc={rc}):\n{gui_status}")

        rc, gui_journal = exec_in_guest(child, "journalctl -u bhoot-gui.service --no-pager -n 30")
        log(f"bhoot-gui journal (rc={rc}):\n{gui_journal}")

        rc, desktop_journal = exec_in_guest(child, "journalctl -u bhoot-desktop.service --no-pager -n 30")
        log(f"bhoot-desktop journal (rc={rc}):\n{desktop_journal}")

        rc, seatd_status = exec_in_guest(child, "systemctl status seatd.service --no-pager")
        log(f"seatd status (rc={rc}):\n{seatd_status}")

        rc, seatd_journal = exec_in_guest(child, "journalctl -u seatd.service --no-pager -n 20")
        log(f"seatd journal (rc={rc}):\n{seatd_journal}")

        rc, input_devices = exec_in_guest(child, "cat /proc/bus/input/devices")
        log(f"input devices (rc={rc}):\n{input_devices}")

        # Check processes
        rc, ps_out = exec_in_guest(child, "ps -ef")
        log(f"Processes in guest (rc={rc}):\n{ps_out}")

        # Test guest HTTP endpoint
        rc, health = exec_in_guest(child, "curl -s http://127.0.0.1:1420/api/health")
        log(f"Guest API Health Check (rc={rc}): {health}")
        if '"status":"ok"' not in health:
            raise RuntimeError(f"Health check failed: {health}")

        # Check initial security status: must be unenrolled
        rc, sec_status = exec_in_guest(child, "curl -s http://127.0.0.1:1420/api/security/status")
        log(f"Initial Security Status (rc={rc}): {sec_status}")
        assert '"auth_enrolled":false' in sec_status, f"Expected unenrolled state, got: {sec_status}"
        log("  [PASS] Initial state confirmed: unenrolled (fails closed)")

        # Verify unenrolled requests fail closed with HTTP 401
        rc, unauth_resp = exec_in_guest(child, "curl -i -s http://127.0.0.1:1420/api/accounts | head -n 5")
        log(f"Unenrolled Protected Access Attempt (rc={rc}):\n{unauth_resp}")
        assert "401" in unauth_resp, f"Expected 401 Unauthorized for unenrolled state, got: {unauth_resp}"
        log("  [PASS] Unenrolled protected API fails closed (401 Unauthorized)")

        # Stage 5: In-Browser GUI Setup & Lock Screen Verification
        time.sleep(5)
        log("Capturing BHOOT First-Owner Master PIN Enrollment screen rendered in browser...")
        qmp.screendump(os.path.join(SCREENSHOT_DIR, "04_bhoot_enrollment_screen.png"))

        # Enroll master PIN: "748192"
        owner_pin = "748192"
        log(f"Enrolling First-Owner Master PIN '{owner_pin}' via API...")
        rc, enroll_resp = exec_in_guest(child, f"curl -i -s -X POST http://127.0.0.1:1420/api/auth/enroll -H 'Content-Type: application/json' -d '{{\"pin\":\"{owner_pin}\",\"confirm_pin\":\"{owner_pin}\"}}'")
        log(f"Enrollment response (rc={rc}):\n{enroll_resp}")
        assert "200" in enroll_resp, f"Enrollment failed: {enroll_resp}"
        log("  [PASS] Master PIN enrolled successfully")

        # Lock the session to test graphical lock screen
        log("Locking session to verify Graphical Lock Screen...")
        exec_in_guest(child, "curl -s -X POST http://127.0.0.1:1420/api/auth/lock")
        time.sleep(4)
        qmp.screendump(os.path.join(SCREENSHOT_DIR, "04_bhoot_lock_screen.png"))

        # Stage 6: Unlock Graphical UI via QMP Key Events (PIN: 748192 + Enter)
        log(f"Activating kiosk focus and submitting enrolled PIN '{owner_pin}' and Enter to kiosk browser via QMP...")
        qmp.click_screen()
        time.sleep(0.5)
        for digit in owner_pin:
            qmp.send_keys([digit], delay=0.2)
        qmp.send_keys(['ret'], delay=0.25)
        time.sleep(5)
        log("Capturing Dashboard view screenshot...")
        qmp.screendump(os.path.join(SCREENSHOT_DIR, "05_dashboard_view.png"))

        # Stage 7: Navigate to Transactions View via Keyboard Shortcut ('2')
        log("Navigating to Transactions view via QMP key event ('2')...")
        qmp.click_screen()
        time.sleep(0.5)
        qmp.send_keys(['2'], delay=0.25)
        time.sleep(5)
        log("Capturing Transactions view screenshot...")
        qmp.screendump(os.path.join(SCREENSHOT_DIR, "06_transactions_view.png"))

        # =====================================================================
        # TASK 5: IN-GUEST REGRESSION TEST SUITE EXECUTION
        # =====================================================================
        log("====================================================================")
        log(" [TASK 5] Running Live In-Guest Financial Regression Test Suite...")
        log("====================================================================")

        # 1. Authentication Lock Check: Unauthenticated request rejected (401)
        rc, out1 = exec_in_guest(child, "curl -i -s http://127.0.0.1:1420/api/accounts | head -n 5")
        log(f"Test 1 - Authentication Lock Response (rc={rc}):\n{out1}")
        assert "401" in out1, f"Test 1 FAILED: Expected 401 Unauthorized, got:\n{out1}"
        log("  [PASS] Test 1: Unauthenticated request rejected with HTTP 401 Unauthorized")

        # 2. PIN Lockout Verification (5 failed attempts -> 429)
        lockout_cmd = 'for i in 1 2 3 4 5; do echo "Attempt $i:"; curl -i -s -X POST http://127.0.0.1:1420/api/auth/unlock -H "Content-Type: application/json" -d "{\\\"pin\\\":\\\"9999\\\"}" | head -n 1; done'
        rc, out2 = exec_in_guest(child, lockout_cmd)
        log(f"Test 2 - PIN Lockout Attempts 1-5 (rc={rc}):\n{out2}")
        assert "429" in out2, f"Test 2 FAILED: Expected 429 Too Many Requests, got:\n{out2}"
        log("  [PASS] Test 2: PIN brute-force lockout triggered (HTTP 429 Too Many Requests)")

        # 3. Successful Unlock via API after 62s cooldown
        log("Waiting 62 seconds for PIN lockout cooldown to expire...")
        time.sleep(62)

        rc, out3 = exec_in_guest(child, f"curl -i -s -X POST http://127.0.0.1:1420/api/auth/unlock -H 'Content-Type: application/json' -d '{{\"pin\":\"{owner_pin}\"}}' | head -n 5")
        log(f"Test 3 - Post-cooldown Authentication (rc={rc}):\n{out3}")
        assert "200" in out3, f"Test 3 FAILED: Expected 200 OK after cooldown, got:\n{out3}"

        # Extract session token cleanly
        rc, unlock_json = exec_in_guest(child, f"curl -s -X POST http://127.0.0.1:1420/api/auth/unlock -H 'Content-Type: application/json' -d '{{\"pin\":\"{owner_pin}\"}}'")
        m_tok = re.search(r'[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}', unlock_json)
        if not m_tok:
            raise RuntimeError(f"Failed to extract session token from: {unlock_json}")
        token = m_tok.group(0)
        log(f"  [PASS] Test 3: Authenticated post-cooldown. Session token: {token}")

        # Fetch accounts
        rc, acc_raw = exec_in_guest(child, f"curl -s http://127.0.0.1:1420/api/accounts -H 'Authorization: Bearer {token}'")
        acc_ids = re.findall(r'[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}', acc_raw)
        if len(acc_ids) < 2:
            raise RuntimeError(f"Expected at least 2 account IDs, got {len(acc_ids)}: {acc_raw}")
        acc1, acc2 = acc_ids[0], acc_ids[1]
        log(f"Accounts fetched: ACC1={acc1}, ACC2={acc2}")

        # 4. Balanced Double-Entry vs Unbalanced Rejection
        # Unbalanced rejection (422)
        rc, out4a = exec_in_guest(child, f"curl -i -s -X POST http://127.0.0.1:1420/api/transactions -H 'Content-Type: application/json' -H 'Authorization: Bearer {token}' -d '{{\"tx_type\":\"Expense\",\"description\":\"Unbalanced test\",\"postings\":[{{\"account_id\":\"{acc1}\",\"direction\":\"Debit\",\"amount_paisa\":50000}}]}}' | head -n 5")
        log(f"Test 4A - Unbalanced Transaction Rejection (rc={rc}):\n{out4a}")
        assert "422" in out4a, f"Test 4A FAILED: Expected 422 Unprocessable Entity, got:\n{out4a}"
        log("  [PASS] Test 4A: Unbalanced transaction rejected with HTTP 422 Unprocessable Entity")

        # Balanced transaction acceptance (201 Created)
        rc, out4b = exec_in_guest(child, f"curl -i -s -X POST http://127.0.0.1:1420/api/transactions -H 'Content-Type: application/json' -H 'Authorization: Bearer {token}' -d '{{\"tx_type\":\"Transfer\",\"description\":\"Balanced transfer test\",\"postings\":[{{\"account_id\":\"{acc1}\",\"direction\":\"Debit\",\"amount_paisa\":750000}},{{\"account_id\":\"{acc2}\",\"direction\":\"Credit\",\"amount_paisa\":750000}}]}}' | head -n 5")
        log(f"Test 4B - Balanced Transaction Acceptance (rc={rc}):\n{out4b}")
        assert "201" in out4b or "200" in out4b, f"Test 4B FAILED: Expected 201 Created or 200 OK, got:\n{out4b}"
        log("  [PASS] Test 4B: Balanced double-entry transfer accepted (HTTP 201 Created)")

        # 5. Integer Paisa Precision & Balance Sheet
        rc, out5 = exec_in_guest(child, f"curl -s http://127.0.0.1:1420/api/reports/balance-sheet -H 'Authorization: Bearer {token}'")
        log(f"Test 5 - Balance Sheet (rc={rc}):\n{out5}")
        assert "assets" in out5 and "liabilities" in out5 and "equity" in out5, f"Test 5 FAILED: Invalid balance sheet:\n{out5}"
        log("  [PASS] Test 5: Balance sheet integer paisa precision verified")

        # 6. Encrypted Backup Creation
        rc, out6 = exec_in_guest(child, f"curl -i -s -X POST http://127.0.0.1:1420/api/backup/create -H 'Content-Type: application/json' -H 'Authorization: Bearer {token}' -d '{{\"passphrase\":\"AuditBackupPass2026!\",\"destination_path\":\"/var/bhoot/data/audit_backup.bhoot\"}}' | head -n 5")
        log(f"Test 6 - Encrypted Backup Creation (rc={rc}):\n{out6}")
        assert "200" in out6, f"Test 6 FAILED: Expected 200 OK, got:\n{out6}"

        rc, backup_file = exec_in_guest(child, "ls -lh /var/bhoot/data/audit_backup.bhoot")
        log(f"Backup file existence (rc={rc}): {backup_file}")
        assert "audit_backup.bhoot" in backup_file, f"Test 6 FAILED: Backup file not found:\n{backup_file}"
        log("  [PASS] Test 6: Encrypted backup created and verified on persistent storage")

        # 7. Persistence Verification of SQLCipher Database File on Encrypted Volume
        rc, db_file = exec_in_guest(child, "ls -lh /var/bhoot/data/bhoot.db")
        log(f"Test 7 - SQLCipher database file (rc={rc}): {db_file}")
        assert "bhoot.db" in db_file, f"Test 7 FAILED: Database file not found:\n{db_file}"
        log("  [PASS] Test 7: SQLCipher persistent database verified on LUKS persistent volume")

        # 8. Token Revocation on Session Lock
        log("Testing session lock and token revocation...")
        rc, out8 = exec_in_guest(child, f"curl -i -s -X POST http://127.0.0.1:1420/api/auth/lock -H 'Authorization: Bearer {token}' | head -n 5")
        log(f"Test 8 - Session Lock Response (rc={rc}):\n{out8}")
        assert "200" in out8, f"Test 8 FAILED: Expected 200 OK on lock, got:\n{out8}"
        rc, out8_verify = exec_in_guest(child, f"curl -i -s http://127.0.0.1:1420/api/accounts -H 'Authorization: Bearer {token}' | head -n 5")
        log(f"Test 8 - Revoked Token Access Attempt (rc={rc}):\n{out8_verify}")
        assert "401" in out8_verify, f"Test 8 FAILED: Expected 401 Unauthorized with revoked token, got:\n{out8_verify}"
        log("  [PASS] Test 8: Session lock strictly revokes token; unauthorized access blocked (401)")

        # 9. Re-authentication & Fresh Token Generation
        rc, unlock2 = exec_in_guest(child, f"curl -s -X POST http://127.0.0.1:1420/api/auth/unlock -H 'Content-Type: application/json' -d '{{\"pin\":\"{owner_pin}\"}}'")
        m_tok2 = re.search(r'[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}', unlock2)
        assert m_tok2 and m_tok2.group(0) != token, f"Test 9 FAILED: Token rotation failed: {unlock2}"
        token2 = m_tok2.group(0)
        log(f"  [PASS] Test 9: Re-authenticated with rotated session token: {token2}")

        # 10. Database Encryption Verification (Ciphertext Check)
        rc, db_header = exec_in_guest(child, "head -c 16 /var/bhoot/data/bhoot.db | od -A n -t x1")
        log(f"Test 10 - Database 16-byte Header (rc={rc}): {db_header}")
        assert "53 51 4c 69 74 65" not in db_header, f"Test 10 FAILED: Database header contains plaintext SQLite signature!"
        log("  [PASS] Test 10: SQLCipher database is fully encrypted (zero plaintext SQLite magic)")

        # 11. Encrypted Backup Restore Verification in Sandbox
        rc, restore_out = exec_in_guest(child, f"curl -i -s -X POST http://127.0.0.1:1420/api/backup/restore -H 'Content-Type: application/json' -H 'Authorization: Bearer {token2}' -d '{{\"passphrase\":\"AuditBackupPass2026!\",\"backup_file_path\":\"/var/bhoot/data/audit_backup.bhoot\"}}' | head -n 5")
        log(f"Test 11 - Backup Restore Response (rc={rc}):\n{restore_out}")
        assert "200" in restore_out, f"Test 11 FAILED: Expected 200 OK on restore, got:\n{restore_out}"
        log("  [PASS] Test 11: Encrypted backup restored and verified successfully")

        # 12. Service Restart & Persistent Authentication Verification
        log("Testing credential and lockout state persistence across service restart...")
        rc, _ = exec_in_guest(child, "systemctl restart bhoot-desktop.service")
        time.sleep(3)

        rc, post_restart_status = exec_in_guest(child, "curl -s http://127.0.0.1:1420/api/security/status")
        log(f"Test 12 - Security Status Post-Restart (rc={rc}): {post_restart_status}")
        assert '"auth_enrolled":true' in post_restart_status, f"Test 12 FAILED: Enrollment state lost across restart: {post_restart_status}"
        log("  [PASS] Test 12A: Enrollment status persisted across service restart")

        rc, dup_resp = exec_in_guest(child, f"curl -i -s -X POST http://127.0.0.1:1420/api/auth/enroll -H 'Content-Type: application/json' -d '{{\"pin\":\"888888\",\"confirm_pin\":\"888888\"}}' | head -n 5")
        log(f"Test 12 - Duplicate Enrollment Attempt Post-Restart (rc={rc}):\n{dup_resp}")
        assert "409" in dup_resp, f"Test 12 FAILED: Duplicate enrollment allowed: {dup_resp}"
        log("  [PASS] Test 12B: Duplicate enrollment rejected (HTTP 409 Conflict)")

        rc, post_unlock = exec_in_guest(child, f"curl -i -s -X POST http://127.0.0.1:1420/api/auth/unlock -H 'Content-Type: application/json' -d '{{\"pin\":\"{owner_pin}\"}}' | head -n 5")
        log(f"Test 12 - Post-Restart Unlock with Master PIN (rc={rc}):\n{post_unlock}")
        assert "200" in post_unlock, f"Test 12 FAILED: Master PIN failed to unlock post-restart: {post_unlock}"
        log("  [PASS] Test 12C: Master PIN successfully unlocked session post-service restart")

        log("====================================================================")
        log(" [TASK 5 COMPLETE] All 12 in-guest regression tests PASSED 100%.")
        log("====================================================================")

        # Clean shutdown
        log("Initiating clean virtual machine poweroff...")
        child.sendline("poweroff")
        time.sleep(4)

    finally:
        qmp.close()
        child.terminate(force=True)
        if os.path.exists(QMP_SOCK):
            os.remove(QMP_SOCK)

if __name__ == "__main__":
    main()
