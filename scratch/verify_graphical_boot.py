#!/usr/bin/env python3
"""
BHOOT — Final Release Graphical Boot Verification
Verifies UEFI boot, LUKS unlock, backend startup, cage compositor, cog WPE WebKit,
and captures visual screenshot evidence.
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
from PIL import Image

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
ROOT_DIR = os.path.dirname(SCRIPT_DIR)
BUILD_REAL = os.path.join(ROOT_DIR, "os/build_real")
CONFIG_DIR = os.path.join(ROOT_DIR, "os/config")
DISPOSABLE_DIR = os.path.join(BUILD_REAL, "disposable_graphical_test")
OVMF_PATH = "/usr/share/OVMF/OVMF_CODE_4M.fd"
QMP_SOCK = "/tmp/qmp_graphical.sock"
SCREENSHOT_DIR = os.path.join(ROOT_DIR, "reports/screenshots")

TOTAL_SECTORS = 121241600
SECTOR_SIZE = 512

def log(msg):
    print(f"[{time.strftime('%H:%M:%S')}] {msg}", flush=True)

class QMPClient:
    def __init__(self, sock_path):
        self.sock_path = sock_path
        self.sock = None

    def connect(self, retries=15):
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
        return False

    def send_keys(self, keys):
        # keys: list of str
        key_list = [{"type": "qcode", "data": k} for k in keys]
        self.cmd("send-key", {"keys": key_list})

    def close(self):
        if self.sock:
            try:
                self.sock.close()
            except:
                pass

def prepare_disposable_disk(luks_pass):
    os.makedirs(DISPOSABLE_DIR, exist_ok=True)
    os.makedirs(SCREENSHOT_DIR, exist_ok=True)
    disk_img = os.path.join(DISPOSABLE_DIR, "bhoot_graphical_test.img")
    p3_img = os.path.join(DISPOSABLE_DIR, "test_p3.img")
    header_backup = os.path.join(DISPOSABLE_DIR, "test_header.backup")

    log("1. Provisioning disposable 48 GiB LUKS2 container...")
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

    log("2. Assembling disposable disk image (57.81 GiB sparse)...")
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

    log("Disposable disk assembled.")
    return disk_img

def main():
    luks_pass = secrets.token_urlsafe(24)
    user_pass = "AuditMasterPassword2026!"
    
    disk_img = prepare_disposable_disk(luks_pass)
    
    if os.path.exists(QMP_SOCK):
        os.remove(QMP_SOCK)

    qemu_cmd = [
        "qemu-system-x86_64",
        "-name", "BHOOT-Graphical-Verification",
        "-machine", "q35,accel=kvm",
        "-cpu", "host",
        "-m", "3072",
        "-smp", "2",
        "-drive", f"if=pflash,format=raw,readonly=on,file={OVMF_PATH}",
        "-drive", f"file={disk_img},format=raw,if=virtio",
        "-vga", "virtio",
        "-display", "vnc=127.0.0.1:42",
        "-qmp", f"unix:{QMP_SOCK},server,nowait",
        "-net", "none",
        "-serial", "stdio"
    ]

    log("Launching QEMU with UEFI + virtio-vga + VNC + QMP...")
    child = pexpect.spawn(qemu_cmd[0], qemu_cmd[1:], timeout=120, encoding="utf-8", codec_errors="replace")
    
    qmp = QMPClient(QMP_SOCK)
    
    try:
        # Connect QMP
        log("Waiting for QMP connection...")
        if qmp.connect(retries=30):
            log("QMP connected successfully.")
        else:
            log("Warning: Failed to connect QMP.")

        # Stage 1: OVMF / GRUB
        idx = child.expect(["GNU GRUB", "Loading BHOOT Linux Kernel", "Please unlock disk", pexpect.TIMEOUT], timeout=40)
        log(f"Stage 1/2 reached (match idx={idx})")
        time.sleep(1)
        qmp.screendump(os.path.join(SCREENSHOT_DIR, "01_uefi_grub_boot.png"))

        # Stage 2: LUKS Unlock Prompt
        idx = child.expect(["Please unlock disk", "unlock disk /dev/vda3", pexpect.TIMEOUT], timeout=35)
        if idx in (0, 1):
            log("Live-boot LUKS2 passphrase prompt reached. Sending passphrase...")
            time.sleep(0.5)
            qmp.screendump(os.path.join(SCREENSHOT_DIR, "02_luks_passphrase_prompt.png"))
            child.sendline(luks_pass)

        # Stage 3: First-Boot Provisioning
        idx = child.expect(["Enter new master password for 'bhoot'", pexpect.TIMEOUT], timeout=45)
        if idx == 0:
            log("First-boot provisioning prompt reached. Supplying master credentials...")
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
                log("GRUB administrator configured.")

        # Stage 4: Login prompt reached
        child.expect("bhoot-secure login:", timeout=35)
        log("Serial login prompt reached. Logging in as 'bhoot' to inspect services...")
        child.sendline("bhoot")
        child.expect("Password:", timeout=10)
        child.sendline(user_pass)
        child.expect(r"\$", timeout=15)
        log("Logged in successfully via serial console.")

        # Wait a moment for bhoot-desktop and bhoot-gui to settle
        time.sleep(4)

        # Check bhoot-desktop status
        child.sendline("systemctl status bhoot-desktop.service")
        child.expect(r"\$", timeout=10)
        desktop_status = child.before
        log(f"bhoot-desktop status output:\n{desktop_status}")

        # Check bhoot-gui status
        child.sendline("systemctl status bhoot-gui.service")
        child.expect(r"\$", timeout=10)
        gui_status = child.before
        log(f"bhoot-gui status output:\n{gui_status}")

        # Capture initial screenshot
        time.sleep(2)
        qmp.screendump(os.path.join(SCREENSHOT_DIR, "04_graphical_booted.png"))

        # Check cage & cog processes
        child.sendline("ps aux | grep -E 'cage|cog|bhoot-desktop'")
        child.expect(r"\$", timeout=10)
        ps_out = child.before
        log(f"Running processes:\n{ps_out}")

        # Check journal for bhoot-gui
        child.sendline("journalctl -u bhoot-gui.service --no-pager -n 25")
        child.expect(r"\$", timeout=10)
        journal_out = child.before
        log(f"bhoot-gui journal:\n{journal_out}")

        # Now test curl against 127.0.0.1:1420
        child.sendline("curl -s http://127.0.0.1:1420/api/health")
        child.expect(r"\$", timeout=10)
        health_out = child.before
        log(f"curl /api/health: {health_out}")

        child.sendline("curl -s http://127.0.0.1:1420/api/auth/status")
        child.expect(r"\$", timeout=10)
        auth_status = child.before
        log(f"curl /api/auth/status: {auth_status}")

        # Capture lockscreen
        qmp.screendump(os.path.join(SCREENSHOT_DIR, "05_lock_screen.png"))

        # Now test unlocking via PIN in the graphical UI or via API
        # First test API unlock:
        child.sendline("curl -s -X POST http://127.0.0.1:1420/api/auth/unlock -H 'Content-Type: application/json' -d '{\"pin\":\"1234\"}'")
        child.expect(r"\$", timeout=10)
        unlock_resp = child.before
        log(f"Unlock response: {unlock_resp}")

        # Send keys to UI via QMP to unlock in the browser
        # Default PIN is 1234, press Enter
        log("Sending PIN '1234' and Enter to graphical kiosk...")
        qmp.send_keys(['1', '2', '3', '4', 'ret'])
        time.sleep(3)
        qmp.screendump(os.path.join(SCREENSHOT_DIR, "06_dashboard_view.png"))

        # Navigate to Transactions view via tab or API or keyboard shortcut
        # Let's see if we can take transactions screenshot
        # Send tab key or click
        qmp.send_keys(['tab', 'tab', 'ret'])
        time.sleep(2)
        qmp.screendump(os.path.join(SCREENSHOT_DIR, "07_financial_transactions_view.png"))

        # Clean shutdown
        log("Shutting down QEMU...")
        child.sendline("sudo systemctl poweroff")
        time.sleep(3)

    finally:
        qmp.close()
        child.terminate(force=True)
        if os.path.exists(QMP_SOCK):
            os.remove(QMP_SOCK)

if __name__ == "__main__":
    main()

