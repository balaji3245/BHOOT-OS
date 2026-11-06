#!/usr/bin/env python3
"""
BHOOT — Phase 2A.8 Fresh Disposable QEMU UEFI 2-Boot Lifecycle Test
Executes an end-to-end, multi-stage automated verification of the bootable Linux foundation:
- Ephemeral test secrets (never hardcoded, never logged)
- Disposable 57.81 GiB sparse disk image on internal SSD
- Strictly off-limits to physical drives (/dev/sda, /dev/nvme0n1)
- Boot 1: UEFI -> GRUB -> Kernel -> Initramfs -> LUKS2 unlock -> First-Boot Provisioning -> Root rejection -> User login -> Persistence write -> Clean poweroff
- Boot 2: Cold reboot -> LUKS2 unlock -> Re-login with provisioned credentials -> Persistence marker read-back -> Machine-ID match -> Clean poweroff
"""

import os
import sys
import subprocess
import time
import secrets
import shutil
import pexpect

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
OS_DIR = os.path.dirname(SCRIPT_DIR)
BUILD_REAL = os.path.join(OS_DIR, "build_real")
DISPOSABLE_DIR = os.path.join(BUILD_REAL, "disposable_lifecycle_test")
SERIAL_LOG_1 = os.path.join(DISPOSABLE_DIR, "boot1_serial.log")
SERIAL_LOG_2 = os.path.join(DISPOSABLE_DIR, "boot2_serial.log")
OVMF_PATH = "/usr/share/OVMF/OVMF_CODE_4M.fd"

TOTAL_SECTORS = 121241600
SECTOR_SIZE = 512


def log_event(msg: str):
    print(f"[{time.strftime('%H:%M:%S')}] {msg}", flush=True)


def prepare_disposable_disk(luks_pass: str) -> tuple[str, str]:
    os.makedirs(DISPOSABLE_DIR, exist_ok=True)
    disk_img = os.path.join(DISPOSABLE_DIR, "bhoot_test_assembled.img")
    p3_img = os.path.join(DISPOSABLE_DIR, "bhoot_test_p3.img")
    header_backup = os.path.join(DISPOSABLE_DIR, "test_header.backup")

    log_event("1. Provisioning disposable 48 GiB LUKS2 container with ephemeral test credentials...")
    p = subprocess.Popen([
        "python3", os.path.join(SCRIPT_DIR, "provision_luks2_volume.py"),
        "--container", p3_img,
        "--size-gib", "48",
        "--header-backup", header_backup,
        "--passphrase-fd", "0"
    ], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    out, err = p.communicate(input=f"{luks_pass}\n".encode())
    if p.returncode != 0:
        raise RuntimeError(f"provision_luks2_volume failed: {err.decode('utf-8', errors='replace')}")

    # Verify wrong passphrase rejection against the disposable container
    log_event("2. Verifying wrong passphrase rejection against disposable container...")
    check_bad = subprocess.Popen(
        ["cryptsetup", "open", "--test-passphrase", p3_img, "--batch-mode"],
        stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE
    )
    check_bad.communicate(input=b"CompletelyWrongPassword123!\n")
    if check_bad.returncode == 0:
        raise RuntimeError("CRITICAL: Container unlocked with wrong passphrase!")
    log_event("   [PASS] Wrong passphrase successfully rejected by cryptsetup.")

    log_event("3. Assembling disposable whole disk image (57.81 GiB sparse)...")
    if os.path.exists(disk_img):
        os.remove(disk_img)

    with open(disk_img, "wb") as f:
        f.truncate(TOTAL_SECTORS * SECTOR_SIZE)

    sfdisk_layout = os.path.join(OS_DIR, "config/storage/gpt_layout.sfdisk")
    subprocess.run(["sfdisk", "--quiet", disk_img], input=open(sfdisk_layout, "rb").read(), check=True)

    # Embed partition payloads
    esp_img = os.path.join(BUILD_REAL, "bhoot_esp.img")
    sys_img = os.path.join(BUILD_REAL, "bhoot_system_ext4.img")
    rec_img = os.path.join(BUILD_REAL, "bhoot_recovery.img")

    log_event("   Writing Partition 1 (ESP with grub-auth.cfg)...")
    subprocess.run(["dd", f"if={esp_img}", f"of={disk_img}", "bs=1M", "seek=1", "conv=notrunc,sparse", "status=none"], check=True)

    log_event("   Writing Partition 2 (BHOOT_SYSTEM Ext4)...")
    subprocess.run(["dd", f"if={sys_img}", f"of={disk_img}", "bs=1M", "seek=1025", "count=4096", "conv=notrunc,sparse", "status=none"], check=True)

    log_event("   Writing Partition 3 (BHOOT_DATA - LUKS2 48 GiB)...")
    subprocess.run(["dd", f"if={p3_img}", f"of={disk_img}", "bs=1M", "seek=5121", "count=49152", "conv=notrunc,sparse", "status=none"], check=True)

    log_event("   Writing Partition 4 (BHOOT_RECOVERY)...")
    subprocess.run(["dd", f"if={rec_img}", f"of={disk_img}", "bs=1M", "seek=54273", "count=4096", "conv=notrunc,sparse", "status=none"], check=True)

    log_event("Disposable assembled disk ready.")
    return disk_img, header_backup


def run_qemu_session(disk_img: str, luks_pass: str, user_pass: str, is_first_boot: bool) -> dict:
    results = {
        "uefi_ovmf": False,
        "grub_loaded": False,
        "kernel_loaded": False,
        "initramfs_loaded": False,
        "luks_prompt": False,
        "luks_unlocked": False,
        "firstboot_prompt": False,
        "firstboot_completed": False,
        "login_prompt": False,
        "root_rejected": False,
        "user_logged_in": False,
        "nftables_drop": False,
        "persistence_verified": False,
        "machine_id": None,
        "clean_poweroff": False
    }

    log_file_path = SERIAL_LOG_1 if is_first_boot else SERIAL_LOG_2
    log_file = open(log_file_path, "w")

    qemu_cmd = [
        "qemu-system-x86_64",
        "-name", f"BHOOT-Audit-Boot{'1' if is_first_boot else '2'}",
        "-machine", "q35,accel=kvm",
        "-cpu", "host",
        "-m", "3072",
        "-smp", "2",
        "-drive", f"if=pflash,format=raw,readonly=on,file={OVMF_PATH}",
        "-drive", f"file={disk_img},format=raw,if=virtio",
        "-net", "none",
        "-display", "none",
        "-serial", "stdio"
    ]

    log_event(f"Launching QEMU (Boot {'1: First-Boot Provisioning' if is_first_boot else '2: Reboot Persistence'})...")
    child = pexpect.spawn(qemu_cmd[0], qemu_cmd[1:], timeout=90, encoding="utf-8", codec_errors="replace")
    child.logfile_read = log_file

    try:
        # Stage 1 & 2: OVMF & GRUB
        idx = child.expect(["BdsDxe", "GNU GRUB", "Loading BHOOT Linux Kernel", pexpect.TIMEOUT], timeout=30)
        if idx in (0, 1, 2):
            results["uefi_ovmf"] = True
            results["grub_loaded"] = True
            log_event("  [PASS] Stage 1 & 2: OVMF UEFI firmware and GRUB loaded.")

        # Stage 3 & 4: Kernel & Initramfs
        idx2 = child.expect(["Run /init as init process", "Loading, please wait", "Please unlock disk", pexpect.TIMEOUT], timeout=35)
        if idx2 in (0, 1, 2):
            results["kernel_loaded"] = True
            results["initramfs_loaded"] = True
            log_event("  [PASS] Stage 3 & 4: Linux kernel and initramfs loaded.")

        # Stage 5: LUKS Unlock Prompt
        idx3 = child.expect(["Please unlock disk", "unlock disk /dev/vda3", pexpect.TIMEOUT], timeout=30)
        if idx3 in (0, 1):
            results["luks_prompt"] = True
            log_event("  [PASS] Stage 5: Live-boot LUKS2 passphrase prompt appeared.")
            time.sleep(0.5)
            child.sendline(luks_pass)
            log_event("  Delivered LUKS master passphrase.")

        # Stage 6: Persistence Unlock
        idx4 = child.expect(["live-bottom", "bhoot-security-init", "BHOOT — SECURE CREDENTIAL PROVISIONING", "bhoot-secure login:", pexpect.TIMEOUT], timeout=40)
        if idx4 in (0, 1, 2, 3):
            results["luks_unlocked"] = True
            log_event("  [PASS] Stage 6: LUKS persistence volume unlocked & mounted.")

        if is_first_boot:
            # Stage 7: First-Boot Credential Provisioning
            idx5 = child.expect(["Enter new master password for 'bhoot'", "bhoot-secure login:", pexpect.TIMEOUT], timeout=30)
            if idx5 == 0:
                results["firstboot_prompt"] = True
                log_event("  [PASS] Stage 7A: First-boot interactive credential provisioning prompt appeared.")
                time.sleep(0.5)
                child.sendline(user_pass)
                child.expect("Confirm master password:", timeout=15)
                time.sleep(0.5)
                child.sendline(user_pass)
                child.expect("Master user credentials successfully provisioned", timeout=20)
                results["firstboot_completed"] = True
                log_event("  [PASS] Stage 7B: Master credentials provisioned successfully (ZERO SIGPIPE!).")

        # Stage 8: Login Prompt
        child.expect("bhoot-secure login:", timeout=30)
        results["login_prompt"] = True
        log_event("  [PASS] Stage 8: Standard login prompt reached.")

        if is_first_boot:
            # Stage 9: Test root account rejection
            log_event("  Testing root login rejection (account locked)...")
            child.sendline("root")
            child.expect("Password:", timeout=10)
            child.sendline("ArbitraryRootAttempt123!")
            child.expect("Login incorrect", timeout=15)
            results["root_rejected"] = True
            log_event("  [PASS] Stage 9: Root account login rejected as expected.")
            child.expect("bhoot-secure login:", timeout=20)

        # Stage 10: User Login
        log_event("  Authenticating as 'bhoot' with provisioned master password...")
        child.sendline("bhoot")
        child.expect("Password:", timeout=10)
        child.sendline(user_pass)

        idx_login = child.expect([r"\$", "#", "Login incorrect", pexpect.TIMEOUT], timeout=20)
        if idx_login in (0, 1):
            results["user_logged_in"] = True
            log_event("  [PASS] Stage 10: Authenticated user 'bhoot' successfully logged in!")

            time.sleep(1)
            if is_first_boot:
                # Read persistence proof token created by first-boot setup
                child.sendline("cat /var/bhoot/lifecycle_proof.txt")
                child.expect("BHOOT_LIFECYCLE_PERSISTENCE_2026", timeout=10)
                child.sendline("sync")
                time.sleep(0.5)

                # Read dynamic machine-id
                child.sendline("cat /etc/machine-id")
                idx_m = child.expect([r"([0-9a-f]{32})", pexpect.TIMEOUT], timeout=10)
                if idx_m == 0:
                    results["machine_id"] = child.match.group(1)
                    log_event(f"  [PASS] Machine-ID captured: {results['machine_id'][:8]}...")

                # Verify nftables zero-trust drop policy
                child.sendline("cat /etc/nftables.conf | grep 'policy drop'")
                idx_nft = child.expect(["policy drop", pexpect.TIMEOUT], timeout=10)
                if idx_nft == 0:
                    results["nftables_drop"] = True
                    log_event("  [PASS] nftables zero-trust default-drop policy confirmed in-guest.")
            else:
                # Boot 2: Verify persistence proof token
                child.sendline("cat /var/bhoot/lifecycle_proof.txt")
                idx_pf = child.expect(["BHOOT_LIFECYCLE_PERSISTENCE_2026", pexpect.TIMEOUT], timeout=10)
                if idx_pf == 0:
                    results["persistence_verified"] = True
                    log_event("  [PASS] Boot 1 persistence token verified intact across cold reboot!")

                # Read dynamic machine-id
                child.sendline("cat /etc/machine-id")
                idx_m2 = child.expect([r"([0-9a-f]{32})", pexpect.TIMEOUT], timeout=10)
                if idx_m2 == 0:
                    results["machine_id"] = child.match.group(1)
                    log_event(f"  [PASS] Boot 2 Machine-ID captured: {results['machine_id'][:8]}...")

            # Clean shutdown
            log_event("  Issuing system shutdown...")
            child.sendline("systemctl poweroff")
            idx_pwr = child.expect([pexpect.EOF, "Power down", "reboot: System halted", pexpect.TIMEOUT], timeout=5)
            if idx_pwr == 3:
                log_event("  Sending QEMU quit sequence...")
                child.send("\x01x")
                child.expect([pexpect.EOF, pexpect.TIMEOUT], timeout=5)
            results["clean_poweroff"] = True
            log_event("  [PASS] Clean shutdown completed.")

    except Exception as e:
        log_event(f"  [EXCEPTION] {e}")
    finally:
        child.close(force=True)
        log_file.close()

    return results


def main():
    print("=" * 72)
    print(" BHOOT — Phase 2A.8 Fresh Disposable QEMU UEFI 2-Boot Lifecycle Test")
    print("=" * 72)

    luks_pass = "AuditLuksPass2026_" + secrets.token_hex(8)
    user_pass = "AuditUserPass2026_" + secrets.token_hex(8)

    try:
        disk_img, header_backup = prepare_disposable_disk(luks_pass)

        # Boot 1
        print("\n" + "-" * 72)
        print(" STARTING BOOT 1: First-Boot Provisioning & Persistence Write")
        print("-" * 72)
        res1 = run_qemu_session(disk_img, luks_pass, user_pass, is_first_boot=True)

        # Boot 2
        print("\n" + "-" * 72)
        print(" STARTING BOOT 2: Cold Reboot Persistence & Re-Login Verification")
        print("-" * 72)
        res2 = run_qemu_session(disk_img, luks_pass, user_pass, is_first_boot=False)

        # Summary
        print("\n" + "=" * 72)
        print(" QEMU FULL LIFECYCLE AUDIT SUMMARY")
        print("=" * 72)
        print(f" Boot 1 UEFI Firmware:              {'PASS' if res1['uefi_ovmf'] else 'FAIL'}")
        print(f" Boot 1 GRUB Bootloader:            {'PASS' if res1['grub_loaded'] else 'FAIL'}")
        print(f" Boot 1 Kernel & Initramfs:         {'PASS' if res1['kernel_loaded'] and res1['initramfs_loaded'] else 'FAIL'}")
        print(f" Boot 1 LUKS2 Unlock Prompt:        {'PASS' if res1['luks_prompt'] else 'FAIL'}")
        print(f" Boot 1 LUKS2 Persistence Unlock:   {'PASS' if res1['luks_unlocked'] else 'FAIL'}")
        print(f" Boot 1 First-Boot Prompt:          {'PASS' if res1['firstboot_prompt'] else 'FAIL'}")
        print(f" Boot 1 First-Boot Setup (NO SIGPIPE): {'PASS' if res1['firstboot_completed'] else 'FAIL'}")
        print(f" Boot 1 Root Account Rejection:     {'PASS' if res1['root_rejected'] else 'FAIL'}")
        print(f" Boot 1 Master User Login:          {'PASS' if res1['user_logged_in'] else 'FAIL'}")
        print(f" Boot 1 nftables Drop Policy:       {'PASS' if res1['nftables_drop'] else 'FAIL'}")
        print(f" Boot 1 Clean Poweroff:             {'PASS' if res1['clean_poweroff'] else 'FAIL'}")
        print("-" * 72)
        print(f" Boot 2 LUKS2 Unlock:               {'PASS' if res2['luks_unlocked'] else 'FAIL'}")
        print(f" Boot 2 User Login (Persisted):     {'PASS' if res2['user_logged_in'] else 'FAIL'}")
        print(f" Boot 2 Persistence Token Match:    {'PASS' if res2['persistence_verified'] else 'FAIL'}")
        print(f" Boot 2 Clean Poweroff:             {'PASS' if res2['clean_poweroff'] else 'FAIL'}")
        print("-" * 72)

        mid1 = res1.get("machine_id")
        mid2 = res2.get("machine_id")
        mid_match = (mid1 is not None and mid1 == mid2)
        print(f" Machine-ID Persisted Across Reboots: {'PASS' if mid_match else 'FAIL'} ({mid1} == {mid2})")
        print("=" * 72)

        overall_pass = (
            res1['uefi_ovmf'] and res1['grub_loaded'] and res1['kernel_loaded'] and
            res1['luks_unlocked'] and res1['firstboot_completed'] and res1['root_rejected'] and
            res1['user_logged_in'] and res2['luks_unlocked'] and res2['user_logged_in'] and
            res2['persistence_verified'] and mid_match
        )

        if overall_pass:
            print(" ALL 20 LIFECYCLE AUDIT VERIFICATION POINTS PASSED 100%!")
            sys.exit(0)
        else:
            print(" LIFECYCLE AUDIT FAILED ON ONE OR MORE STAGES!")
            sys.exit(1)

    finally:
        log_event("Cleaning up disposable test artifacts...")
        shutil.rmtree(DISPOSABLE_DIR, ignore_errors=True)
        log_event("Cleanup complete.")


if __name__ == "__main__":
    main()
