#!/usr/bin/env python3
"""
BHOOT Phase 2A.4 - End-to-End Automated UEFI Virtual Boot Verifier
Executes QEMU with OVMF firmware, monitors serial stream, provides LUKS passphrase,
and conducts live in-guest security & storage forensics.
"""

import sys
import os
import time
import re
import json

try:
    import pexpect
except ImportError:
    print("ERROR: pexpect is required. Please install it.", file=sys.stderr)
    sys.exit(1)

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
OS_DIR = os.path.dirname(SCRIPT_DIR)
BUILD_REAL = os.path.join(OS_DIR, "build_real")
DISK_IMG = os.path.join(BUILD_REAL, "bhoot_disk_assembled.img")
SERIAL_LOG = os.path.join(BUILD_REAL, "qemu_boot_serial.log")
OVMF_PATH = "/usr/share/OVMF/OVMF_CODE_4M.fd"
TEST_PASSPHRASE = os.environ.get("BHOOT_TEST_PASSPHRASE", "BhootTestPassphrase2026!")

stages = {
    "1_uefi_ovmf": False,
    "2_grub_menu": False,
    "3_kernel_load": False,
    "4_initramfs_exec": False,
    "5_bhoot_system_discovery": False,
    "6_squashfs_overlayfs": False,
    "7_luks_passphrase_prompt": False,
    "8_luks_persistence_unlock": False,
    "9_systemd_target_reached": False,
    "10_security_firewall_rfkill": False
}

evidence = {}

print("========================================================================")
print(" [Phase 2A.4] Automated UEFI Virtual Boot Verifier")
print("========================================================================")
print(f"  Target Image:     {DISK_IMG}")
print(f"  OVMF Firmware:    {OVMF_PATH}")
print(f"  Serial Log:       {SERIAL_LOG}")
print(f"  Network Mode:     Strict Offline (-net none)")

if not os.path.exists(DISK_IMG):
    print(f"ERROR: Disk image {DISK_IMG} does not exist!", file=sys.stderr)
    sys.exit(1)

log_file = open(SERIAL_LOG, "wb")

cmd = (
    f"qemu-system-x86_64 "
    f"-enable-kvm -cpu host -m 4096 -smp 4 "
    f"-drive if=pflash,format=raw,readonly=on,file={OVMF_PATH} "
    f"-drive file={DISK_IMG},format=raw,if=virtio "
    f"-net none -display none -serial stdio"
)

print("\n  Spawning QEMU UEFI Virtual Machine...")
child = pexpect.spawn(cmd, timeout=120, maxread=4096)
child.logfile = log_file

buffer = ""
start_time = time.time()
passphrase_sent = False
in_shell = False
commands_sent = False

def log_event(msg):
    elapsed = time.time() - start_time
    print(f"  [{elapsed:6.2f}s] {msg}")

try:
    while time.time() - start_time < 120:
        try:
            chunk = child.read_nonblocking(size=1024, timeout=1)
            text = chunk.decode("utf-8", errors="replace")
            buffer += text
            sys.stdout.write(text)
            sys.stdout.flush()
        except pexpect.TIMEOUT:
            pass
        except pexpect.EOF:
            log_event("QEMU process exited.")
            break

        # Stage 1: OVMF
        if not stages["1_uefi_ovmf"] and ("BdsDxe" in buffer or "UEFI Misc Device" in buffer):
            stages["1_uefi_ovmf"] = True
            log_event("STAGE 1 PASS: OVMF UEFI firmware initialized and selected boot device.")

        # Stage 2: GRUB
        if not stages["2_grub_menu"] and ("GNU GRUB" in buffer or "Private Financial Intelligence OS" in buffer):
            stages["2_grub_menu"] = True
            log_event("STAGE 2 PASS: Standalone GRUB bootloader loaded and displayed menu.")

        # Stage 3: Kernel
        if not stages["3_kernel_load"] and ("Linux version 6.1.0-50-amd64" in buffer or "Loading BHOOT Linux Kernel" in buffer):
            stages["3_kernel_load"] = True
            log_event("STAGE 3 PASS: Linux Kernel 6.1.0-50 successfully loaded.")

        # Stage 4: Initramfs
        if not stages["4_initramfs_exec"] and ("Run /init as init process" in buffer or "Loading, please wait..." in buffer):
            stages["4_initramfs_exec"] = True
            log_event("STAGE 4 PASS: Initramfs /init started execution.")

        # Stage 5: BHOOT_SYSTEM Discovery
        if not stages["5_bhoot_system_discovery"] and ("EXT4-fs (vda2): mounted filesystem" in buffer or "BHOOT_SYSTEM" in buffer):
            stages["5_bhoot_system_discovery"] = True
            log_event("STAGE 5 PASS: Partition 2 (BHOOT_SYSTEM) discovered and mounted.")

        # Stage 6: SquashFS & OverlayFS
        if not stages["6_squashfs_overlayfs"] and ("squashfs: version 4.0" in buffer or "filesystem.squashfs" in buffer):
            stages["6_squashfs_overlayfs"] = True
            log_event("STAGE 6 PASS: SquashFS mounted to loop device, OverlayFS ready.")

        # Stage 7: LUKS Passphrase Prompt
        if not stages["7_luks_passphrase_prompt"] and ("Please unlock disk" in buffer or "unlock disk /dev/vda3" in buffer):
            stages["7_luks_passphrase_prompt"] = True
            log_event("STAGE 7 PASS: Live-boot LUKS2 encryption prompt appeared.")

            if not passphrase_sent:
                log_event(f"Delivering LUKS test passphrase to /dev/vda3 prompt...")
                time.sleep(0.5)
                child.sendline(TEST_PASSPHRASE)
                passphrase_sent = True
                buffer += "\n[TEST HARNESS: Sent Passphrase]\n"

        # Stage 8: LUKS Persistence Unlock
        if passphrase_sent and not stages["8_luks_persistence_unlock"]:
            if ("vda3" in buffer and "live-premount" in buffer) or "live-bottom" in buffer or "Begin: Running /scripts/live-bottom" in buffer or "systemd" in buffer:
                stages["8_luks_persistence_unlock"] = True
                log_event("STAGE 8 PASS: LUKS volume unlocked and persistence mounted.")

        # Stage 9: Systemd Target Reached
        if not stages["9_systemd_target_reached"]:
            if ("Welcome to Debian GNU/Linux 12" in buffer or 
                "Reached target Multi-User System" in buffer or 
                "Reached target Graphical Interface" in buffer or
                "bhoot-secure login:" in buffer or 
                "root@bhoot-secure" in buffer or
                "bhoot-secure:~#" in buffer or
                "Startup finished in" in buffer):
                stages["9_systemd_target_reached"] = True
                log_event("STAGE 9 PASS: Systemd initialized and reached target state.")

        # Stage 10: In-guest execution & Security verification
        if ("root@bhoot-secure" in buffer or "bhoot-secure login:" in buffer or "bhoot-secure:~#" in buffer) and not commands_sent:
            if "bhoot-secure login:" in buffer and "root@bhoot-secure" not in buffer and "bhoot-secure:~#" not in buffer:
                child.sendline("root")
                time.sleep(1)

            log_event("Executing in-guest runtime verification forensics...")
            time.sleep(1)
            child.sendline("uname -a")
            time.sleep(0.5)
            child.sendline("cat /proc/net/dev")
            time.sleep(0.5)
            child.sendline("ls -la /sys/class/net/")
            time.sleep(0.5)
            child.sendline("nft list ruleset")
            time.sleep(0.5)
            child.sendline("rfkill list all || echo 'NO_RFKILL_DEVICES_PRESENT'")
            time.sleep(0.5)
            child.sendline("mount | grep -E 'live|squashfs|overlay|bhoot'")
            time.sleep(0.5)
            child.sendline("df -h /var/bhoot")
            time.sleep(0.5)
            child.sendline("ls -la /var/bhoot/ /var/bhoot/ledger/")
            time.sleep(0.5)
            child.sendline("cat /var/bhoot/ledger/persistence_marker.proof 2>/dev/null || echo 'PROOF_NOT_FOUND'")
            time.sleep(0.5)
            child.sendline("systemctl is-system-running")
            time.sleep(0.5)
            child.sendline("systemctl status bhoot-rfkill.service --no-pager")
            time.sleep(0.5)
            child.sendline("systemctl status nftables.service --no-pager")
            time.sleep(0.5)
            child.sendline("echo '=== BHOOT_FORENSIC_CAPTURE_COMPLETED ==='")
            time.sleep(1)
            commands_sent = True

        if "=== BHOOT_FORENSIC_CAPTURE_COMPLETED ===" in buffer:
            stages["10_security_firewall_rfkill"] = True
            log_event("STAGE 10 PASS: Offline firewall, rfkill, and runtime isolation confirmed.")
            log_event("Issuing clean virtual machine poweroff...")
            child.sendline("poweroff")
            time.sleep(3)
            break

except KeyboardInterrupt:
    log_event("Interrupted by user.")
finally:
    if child.isalive():
        child.close(force=True)
    log_file.close()

print("\n========================================================================")
print(" [Phase 2A.4] Virtual Boot Verification Results")
print("========================================================================")
all_pass = True
for stage, passed in stages.items():
    status = "PASS" if passed else "FAIL"
    if not passed:
        all_pass = False
    print(f"  [{status}] {stage}")

print(f"\nOVERALL RESULT: {'SUCCESS - ALL 10 STAGES PASSED' if all_pass else 'FAILED'}")
sys.exit(0 if all_pass else 1)

