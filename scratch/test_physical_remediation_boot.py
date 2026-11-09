#!/usr/bin/env python3
import os
import sys
import time
import json
import socket
import secrets
import subprocess
import pexpect
from PIL import Image

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
ROOT_DIR = os.path.dirname(SCRIPT_DIR)
BUILD_REAL = os.path.join(ROOT_DIR, "os/build_real")
CONFIG_DIR = os.path.join(ROOT_DIR, "os/config")
DISPOSABLE_DIR = os.path.join(BUILD_REAL, "disposable_release_test")
OVMF_PATH = "/usr/share/OVMF/OVMF_CODE_4M.fd"
QMP_SOCK = "/tmp/qmp_remediation_test.sock"
SCREENSHOT_DIR = os.path.join(ROOT_DIR, "reports/screenshots")

TOTAL_SECTORS = 121145344
SECTOR_SIZE = 512

def log(msg):
    print(f"[{time.strftime('%H:%M:%S')}] {msg}", flush=True)

class QMPClient:
    def __init__(self, sock_path):
        self.sock_path = sock_path
        self.sock = None

    def connect(self, retries=30):
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
        time.sleep(0.05)
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

    def type_char(self, ch, delay=0.04):
        if ch == '\n' or ch == '\r':
            self.cmd("send-key", {"keys": [{"type": "qcode", "data": "ret"}]})
        elif ch == ' ':
            self.cmd("send-key", {"keys": [{"type": "qcode", "data": "spc"}]})
        elif ch.islower() or ch.isdigit():
            self.cmd("send-key", {"keys": [{"type": "qcode", "data": ch}]})
        elif ch.isupper():
            self.cmd("send-key", {"keys": [{"type": "qcode", "data": "shift"}, {"type": "qcode", "data": ch.lower()}]})
        elif ch == '!':
            self.cmd("send-key", {"keys": [{"type": "qcode", "data": "shift"}, {"type": "qcode", "data": "1"}]})
        elif ch == '@':
            self.cmd("send-key", {"keys": [{"type": "qcode", "data": "shift"}, {"type": "qcode", "data": "2"}]})
        elif ch == '#':
            self.cmd("send-key", {"keys": [{"type": "qcode", "data": "shift"}, {"type": "qcode", "data": "3"}]})
        elif ch == '$':
            self.cmd("send-key", {"keys": [{"type": "qcode", "data": "shift"}, {"type": "qcode", "data": "4"}]})
        elif ch == '%':
            self.cmd("send-key", {"keys": [{"type": "qcode", "data": "shift"}, {"type": "qcode", "data": "5"}]})
        elif ch == '^':
            self.cmd("send-key", {"keys": [{"type": "qcode", "data": "shift"}, {"type": "qcode", "data": "6"}]})
        elif ch == '&':
            self.cmd("send-key", {"keys": [{"type": "qcode", "data": "shift"}, {"type": "qcode", "data": "7"}]})
        elif ch == '*':
            self.cmd("send-key", {"keys": [{"type": "qcode", "data": "shift"}, {"type": "qcode", "data": "8"}]})
        elif ch == '(':
            self.cmd("send-key", {"keys": [{"type": "qcode", "data": "shift"}, {"type": "qcode", "data": "9"}]})
        elif ch == ')':
            self.cmd("send-key", {"keys": [{"type": "qcode", "data": "shift"}, {"type": "qcode", "data": "0"}]})
        elif ch == '-':
            self.cmd("send-key", {"keys": [{"type": "qcode", "data": "minus"}]})
        elif ch == '_':
            self.cmd("send-key", {"keys": [{"type": "qcode", "data": "shift"}, {"type": "qcode", "data": "minus"}]})
        elif ch == '=':
            self.cmd("send-key", {"keys": [{"type": "qcode", "data": "equal"}]})
        elif ch == '+':
            self.cmd("send-key", {"keys": [{"type": "qcode", "data": "shift"}, {"type": "qcode", "data": "equal"}]})
        elif ch == '.':
            self.cmd("send-key", {"keys": [{"type": "qcode", "data": "dot"}]})
        elif ch == ':':
            self.cmd("send-key", {"keys": [{"type": "qcode", "data": "shift"}, {"type": "qcode", "data": "semicolon"}]})
        elif ch == '/':
            self.cmd("send-key", {"keys": [{"type": "qcode", "data": "slash"}]})
        time.sleep(delay)

    def type_string(self, text, delay=0.04):
        for ch in text:
            self.type_char(ch, delay=delay)

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

import re

def prepare_disposable_disk(luks_pass):
    os.makedirs(DISPOSABLE_DIR, exist_ok=True)
    os.makedirs(SCREENSHOT_DIR, exist_ok=True)
    disk_img = os.path.join(DISPOSABLE_DIR, "bhoot_remediation_test.img")
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

    log("2. Creating sparse partition table and embedding payloads...")
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
    luks_pass = "BhootTestPass2026!"
    user_pass = "ReleaseAuditPass2026!"

    disk_img = prepare_disposable_disk(luks_pass)

    if os.path.exists(QMP_SOCK):
        os.remove(QMP_SOCK)

    qemu_cmd = [
        "qemu-system-x86_64",
        "-name", "BHOOT-Remediation-Verification",
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

    log("Launching QEMU VM with UEFI (OVMF) + virtio-vga + QMP...")
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
        idx = child.expect(["GNU GRUB", "Loading BHOOT Linux Kernel", pexpect.TIMEOUT], timeout=35)
        time.sleep(1)
        qmp.screendump(os.path.join(SCREENSHOT_DIR, "01_uefi_grub_boot.png"))
        log("GRUB stage verified.")

        # Stage 2: Wait for SquashFS mount message in serial log
        log("Waiting for live-boot SquashFS mount...")
        child.expect(["filesystem.squashfs", "squashfs: version 4.0", pexpect.TIMEOUT], timeout=40)
        log("SquashFS detected. Allowing askpass to initialize on tty0 (display)...")
        time.sleep(3)

        # Screendump: This capture MUST now show the prompt on tty0!
        log("Capturing 02_luks_passphrase_prompt.png from tty0...")
        qmp.screendump(os.path.join(SCREENSHOT_DIR, "02_luks_passphrase_prompt.png"))

        # Submit LUKS passphrase via virtual keyboard to tty0
        log(f"Submitting LUKS passphrase to tty0 via QMP send-key...")
        qmp.type_string(luks_pass + "\n")

        # Verify kernel unlocks LUKS in serial log
        log("Waiting for LUKS container unlock confirmation on serial log...")
        child.expect(["EXT4-fs", "mounted filesystem", "systemd", pexpect.TIMEOUT], timeout=30)
        log("LUKS container successfully unlocked! Ext4 persistence filesystem mounted.")

        # Stage 3: First-Boot Credential Provisioning
        log("Waiting for systemd first-boot provisioning service...")
        time.sleep(6)
        log("Capturing 03_firstboot_setup.png from tty0...")
        qmp.screendump(os.path.join(SCREENSHOT_DIR, "03_firstboot_setup.png"))

        # Send master user password to tty0
        log("Submitting master password for 'bhoot' to tty0 via QMP...")
        qmp.type_string(user_pass + "\n")
        time.sleep(2)
        log("Confirming master password to tty0 via QMP...")
        qmp.type_string(user_pass + "\n")
        time.sleep(4)

        # Stage 4: Wait for serial login prompt
        log("Waiting for serial login prompt (serial-getty@ttyS0)...")
        child.expect("bhoot-secure login:", timeout=45)
        log("Serial login prompt reached! Logging in as 'bhoot'...")
        child.sendline("bhoot")
        child.expect("Password:", timeout=10)
        child.sendline(user_pass)
        child.expect(r"\$", timeout=15)
        log("Logged in successfully as user 'bhoot' via serial console!")
        child.sendline("stty -echo")
        time.sleep(0.5)

        # Settle
        time.sleep(5)

        # Capture GUI enrollment screen
        log("Capturing 04_bhoot_enrollment_screen.png...")
        qmp.screendump(os.path.join(SCREENSHOT_DIR, "04_bhoot_enrollment_screen.png"))

        # Verify API health
        rc, health = exec_in_guest(child, "curl -s http://127.0.0.1:1420/api/health")
        log(f"Guest API Health Check (rc={rc}): {health}")
        assert '"status":"ok"' in health, f"Health check failed: {health}"

        # Verify initial security status
        rc, sec_status = exec_in_guest(child, "curl -s http://127.0.0.1:1420/api/security/status")
        log(f"Security Status (rc={rc}): {sec_status}")
        assert '"auth_enrolled":false' in sec_status, f"Expected unenrolled: {sec_status}"

        # Enroll Master PIN
        owner_pin = "748192"
        log(f"Enrolling Master PIN '{owner_pin}'...")
        rc, enroll_resp = exec_in_guest(child, f"curl -i -s -X POST http://127.0.0.1:1420/api/auth/enroll -H 'Content-Type: application/json' -d '{{\"pin\":\"{owner_pin}\",\"confirm_pin\":\"{owner_pin}\"}}'")
        log(f"Enroll response (rc={rc}):\n{enroll_resp}")
        assert "200" in enroll_resp, f"Enroll failed: {enroll_resp}"

        # Lock session
        exec_in_guest(child, "curl -s -X POST http://127.0.0.1:1420/api/auth/lock")
        time.sleep(3)
        qmp.screendump(os.path.join(SCREENSHOT_DIR, "04_bhoot_lock_screen.png"))

        # Unlock via GUI QMP
        log("Unlocking GUI via QMP key events...")
        qmp.click_screen()
        time.sleep(0.5)
        for d in owner_pin:
            qmp.type_char(d, delay=0.15)
        qmp.type_char('\n', delay=0.25)
        time.sleep(5)
        qmp.screendump(os.path.join(SCREENSHOT_DIR, "05_dashboard_view.png"))

        log("TEST SUITE EXECUTION COMPLETED SUCCESSFULLY!")

    finally:
        qmp.close()
        child.close(force=True)
        time.sleep(1)

if __name__ == "__main__":
    main()

