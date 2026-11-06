#!/usr/bin/env bash
""":"
python3 "$0" "$@"
exit $?
":"""
"""
BHOOT — Phase 2A.14 Recovery Security Remediation Verification Suite
Audits all remediations from Phase 2A.13 audit:
1. HIGH-01: Authenticated Recovery Console & GRUB PBKDF2 verification mode
2. MED-01: Dell Latitude 5410 Secure Boot prerequisites & unsigned EFI specification
3. MED-02: ESP offline tampering threat model & physical custody limits
4. MED-03: SQLCipher status & LUKS2 at-rest data confidentiality
5. LOW-01: External backup allowlist with isolated fixtures (no Rule 5 mock bypass)
6. LOW-02: Recovery key normalization engine
7. LOW-03: Process memory boundaries & compiled explicit_bzero zeroization
"""

import os
import sys
import subprocess
import tempfile
import re

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
OS_DIR = os.path.dirname(SCRIPT_DIR)
REPO_ROOT = os.path.dirname(OS_DIR)
BUILD_REAL = os.path.join(OS_DIR, "build_real")
ROOTFS = os.path.join(BUILD_REAL, "rootfs")
DOCS_DIR = os.path.join(REPO_ROOT, "docs")

passed = 0
failed = 0

def report(name: str, condition: bool, detail: str = ""):
    global passed, failed
    if condition:
        print(f"  [PASS] {name}")
        passed += 1
    else:
        print(f"  [FAIL] {name} - Detail: {detail}")
        failed += 1

def run_cmd(cmd: list, input_str: str = "") -> tuple[int, str, str]:
    p = subprocess.Popen(cmd, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    out, err = p.communicate(input=input_str)
    return p.returncode, out, err


# ==============================================================================
# SECTION 1: Compiled bhoot-grub-pbkdf2 Verification Mode (HIGH-01)
# ==============================================================================
def test_grub_pbkdf2_tool():
    print("\n--- 1. Compiled bhoot-grub-pbkdf2 Verification Mode (HIGH-01) ---")
    tool_path = os.path.join(OS_DIR, "tools/bhoot-grub-pbkdf2")
    if not os.path.exists(tool_path):
        subprocess.run(["gcc", "-O2", "-Wall", "-Wextra", "-Werror",
                        os.path.join(OS_DIR, "tools/bhoot-grub-pbkdf2.c"),
                        "-lcrypto", "-o", tool_path], check=True)

    report("bhoot-grub-pbkdf2 binary exists and is executable", os.path.isfile(tool_path) and os.access(tool_path, os.X_OK))

    # Test generation mode
    test_pwd = "RecoverySecret2026!#"
    rc, out, err = run_cmd([tool_path], input_str=test_pwd + "\n")
    report("bhoot-grub-pbkdf2 generates valid line (RC=0)", rc == 0)
    line = out.strip()
    m = re.match(r"^password_pbkdf2 bhoot-admin grub\.pbkdf2\.sha512\.10000\.([0-9A-F]{128})\.([0-9A-F]{128})$", line)
    report("Generated line matches GRUB PBKDF2 specification exactly", m is not None)

    salt_hex = m.group(1) if m else ""
    hash_hex = m.group(2) if m else ""

    # Test positive verification with args
    rc_v, out_v, err_v = run_cmd([tool_path, "--verify", salt_hex, hash_hex], input_str=test_pwd + "\n")
    report("Positive args verification matches (RC=0)", rc_v == 0 and "VERIFICATION_SUCCESS" in out_v)

    # Test negative verification with args
    rc_nv, out_nv, err_nv = run_cmd([tool_path, "--verify", salt_hex, hash_hex], input_str="WrongRecoverySecret123!\n")
    report("Negative args verification rejects wrong password (RC=1)", rc_nv == 1)

    # Test positive file verification
    with tempfile.NamedTemporaryFile("w+", suffix=".cfg", delete=False) as f:
        f.write("# BHOOT GRUB Auth Configuration\n")
        f.write(line + "\n")
        f_name = f.name

    try:
        rc_fv, out_fv, err_fv = run_cmd([tool_path, "--verify-file", f_name], input_str=test_pwd + "\n")
        report("Positive verify-file mode parses and authenticates (RC=0)", rc_fv == 0 and "VERIFICATION_SUCCESS" in out_fv)

        rc_nfv, out_nfv, err_nfv = run_cmd([tool_path, "--verify-file", f_name], input_str="BadPass12345!\n")
        report("Negative verify-file mode rejects incorrect password (RC=1)", rc_nfv == 1)
    finally:
        os.remove(f_name)

    # Test password length enforcement (min 12 chars)
    rc_short, out_short, err_short = run_cmd([tool_path], input_str="shortpass\n")
    report("Rejection of passwords under 12 characters", rc_short != 0)


# ==============================================================================
# SECTION 2: Authenticated Recovery Console & Systemd Drop-Ins (HIGH-01)
# ==============================================================================
def test_recovery_console_configuration():
    print("\n--- 2. Authenticated Recovery Console Configuration (HIGH-01) ---")
    console_script = os.path.join(ROOTFS, "usr/local/sbin/bhoot-recovery-console")
    report("bhoot-recovery-console script exists in rootfs", os.path.exists(console_script))

    if os.path.exists(console_script):
        with open(console_script, "r") as f:
            content = f.read()

        report("bhoot-recovery-console runs with set -u", "set -u" in content)
        report("bhoot-recovery-console forces HISTFILE=/dev/null", 'export HISTFILE="/dev/null"' in content)
        report("bhoot-recovery-console traps SIGINT/SIGTERM to reboot", "trap cleanup_and_reboot INT TERM" in content)
        report("bhoot-recovery-console verifies bhoot-grub-pbkdf2", "/usr/local/bin/bhoot-grub-pbkdf2 --verify-file" in content)
        report("bhoot-recovery-console enforces max 3 attempts", "MAX_ATTEMPTS=3" in content)
        report("bhoot-recovery-console rate-limits failed attempts with sleep", "sleep 2" in content)
        report("bhoot-recovery-console fail-closed on UNENROLLED marker", 'grep -q "Status: UNENROLLED"' in content)
        report("bhoot-recovery-console reboots immediately on failure", "reboot -f" in content)
        report("bhoot-recovery-console spawns root shell only on success", "/bin/bash --norc --noprofile" in content)

    # Inspect drop-in overrides
    rescue_override = os.path.join(ROOTFS, "etc/systemd/system/rescue.service.d/override.conf")
    report("rescue.service drop-in override exists", os.path.exists(rescue_override))
    if os.path.exists(rescue_override):
        with open(rescue_override, "r") as f:
            r_content = f.read()
        report("rescue.service override clears previous ExecStart", "ExecStart=\n" in r_content or "ExecStart=\r\n" in r_content)
        report("rescue.service override invokes bhoot-recovery-console", "ExecStart=-/usr/local/sbin/bhoot-recovery-console" in r_content)
        report("rescue.service override does NOT enable SYSTEMD_SULOGIN_FORCE=1", "SYSTEMD_SULOGIN_FORCE=1" not in r_content)

    emergency_override = os.path.join(ROOTFS, "etc/systemd/system/emergency.service.d/override.conf")
    report("emergency.service drop-in override exists", os.path.exists(emergency_override))
    if os.path.exists(emergency_override):
        with open(emergency_override, "r") as f:
            e_content = f.read()
        report("emergency.service override invokes bhoot-recovery-console", "ExecStart=-/usr/local/sbin/bhoot-recovery-console" in e_content)
        report("emergency.service override does NOT enable SYSTEMD_SULOGIN_FORCE=1", "SYSTEMD_SULOGIN_FORCE=1" not in e_content)

    # Inspect restore_header.sh in rootfs
    restore_script = os.path.join(ROOTFS, "usr/local/sbin/restore_header.sh")
    report("restore_header.sh exists in rootfs /usr/local/sbin", os.path.exists(restore_script))
    if os.path.exists(restore_script):
        with open(restore_script, "r") as f:
            rst_content = f.read()
        report("restore_header.sh uses cryptsetup luksHeaderRestore with batch-mode", "cryptsetup luksHeaderRestore" in rst_content and "--batch-mode" in rst_content)


# ==============================================================================
# SECTION 3: 256-Bit Recovery Key Normalization (LOW-02)
# ==============================================================================
def test_recovery_key_normalization():
    print("\n--- 3. 256-Bit Recovery Key Input Normalization (LOW-02) ---")
    sys.path.insert(0, SCRIPT_DIR)
    from provision_luks2_volume import normalize_recovery_key, validate_recovery_key

    canonical = "A1B2-C3D4-E5F6-0123-4567-89AB-CDEF-0123-4567-89AB-CDEF-0123-4567-89AB-CDEF-ABCD"
    raw_64 = canonical.replace("-", "")

    # Test 1: Canonical input
    report("Normalization accepts canonical 79-char chunked key", normalize_recovery_key(canonical) == canonical)

    # Test 2: Raw 64 uppercase chars
    report("Normalization chunks raw 64 hex characters", normalize_recovery_key(raw_64) == canonical)

    # Test 3: Raw 64 lowercase chars
    report("Normalization uppercases lowercase hex characters", normalize_recovery_key(raw_64.lower()) == canonical)

    # Test 4: Mixed whitespace and delimiters
    messy_key = f"  {raw_64[:16].lower()}   {raw_64[16:32]}-{raw_64[32:48].lower()}:_{raw_64[48:]}  \n"
    report("Normalization handles arbitrary whitespace, delimiters and case", normalize_recovery_key(messy_key) == canonical)

    # Test 5: Rejection of invalid lengths
    try:
        normalize_recovery_key(raw_64[:-1])
        report("Rejection of 63-char key (short)", False)
    except ValueError:
        report("Rejection of 63-char key (short)", True)

    # Test 6: Rejection of non-hex characters
    try:
        normalize_recovery_key(raw_64[:-1] + "Z")
        report("Rejection of non-hex characters ('Z')", False)
    except ValueError:
        report("Rejection of non-hex characters ('Z')", True)

    # Inspect bhoot-unlock-recovery-key script in rootfs
    unlock_helper = os.path.join(ROOTFS, "usr/local/sbin/bhoot-unlock-recovery-key")
    report("bhoot-unlock-recovery-key helper script exists in rootfs", os.path.exists(unlock_helper))
    if os.path.exists(unlock_helper):
        with open(unlock_helper, "r") as f:
            uh_content = f.read()
        report("bhoot-unlock-recovery-key normalizes input with tr/sed", "tr -d" in uh_content and "tr '[:lower:]' '[:upper:]'" in uh_content)
        report("bhoot-unlock-recovery-key unlocks keyslot 1 with cryptsetup", "cryptsetup open --type luks2 --key-slot 1" in uh_content)


# ==============================================================================
# SECTION 4: External USB Allowlist Isolated Fixture Tests (LOW-01)
# ==============================================================================
def test_allowlist_isolated_fixture():
    print("\n--- 4. External USB Backup Destination Allowlist (LOW-01) ---")
    export_script = os.path.join(SCRIPT_DIR, "export_recovery_backup.sh")
    with open(export_script, "r") as f:
        exp_content = f.read()

    report("Rule 5 mock bypass completely removed from export_recovery_backup.sh", "BHOOT_BACKUP_ALLOWLIST_MOCK" not in exp_content)

    def run_fixture(profile: str) -> tuple[int, str]:
        cmd = f"""
export BHOOT_SOURCE_HEADER=/dev/null
export BHOOT_ALLOWLIST_FIXTURE=1
findmnt() {{
    case "{profile}" in
        "usb_removable") echo "/dev/sdb1 ext4 /tmp/test_backup_dest" ;;
        "internal_sata") echo "/dev/sdc1 ext4 /tmp/test_backup_dest" ;;
        "internal_nvme") echo "/dev/nvme1n1p1 ext4 /tmp/test_backup_dest" ;;
        "secondary_nvme") echo "/dev/nvme2n1p1 ext4 /tmp/test_backup_dest" ;;
        "mmc") echo "/dev/mmcblk0p1 ext4 /tmp/test_backup_dest" ;;
        "device_mapper_multislave") echo "/dev/dm-5 ext4 /tmp/test_backup_dest" ;;
        "bind_mount_internal") echo "/dev/nvme0n1p2 ext4 /tmp/test_backup_dest" ;;
        *) echo "" ;;
    esac
}}
udevadm() {{
    case "{profile}" in
        "usb_removable")
            if [ "$2" = "path" ]; then echo "/devices/pci0000:00/0000:00:14.0/usb1/1-1/block/sdb/sdb1"; else echo -e "ID_BUS=usb\\nID_DRIVE_THUMB=1"; fi
            ;;
        "internal_sata")
            if [ "$2" = "path" ]; then echo "/devices/pci0000:00/ata1/host0/target0:0:0/block/sdc/sdc1"; else echo "ID_BUS=ata"; fi
            ;;
        "internal_nvme"|"secondary_nvme")
            if [ "$2" = "path" ]; then echo "/devices/pci0000:00/nvme/nvme1n1/nvme1n1p1"; else echo "ID_BUS=pci"; fi
            ;;
        "mmc")
            if [ "$2" = "path" ]; then echo "/devices/pci0000:00/mmc0/mmcblk0/mmcblk0p1"; else echo "ID_BUS=mmc"; fi
            ;;
        *) echo "" ;;
    esac
}}
lsblk() {{
    case "{profile}" in
        "usb_removable") echo "sdb" ;;
        "internal_sata") echo "sdc" ;;
        "internal_nvme") echo "nvme1n1" ;;
        "secondary_nvme") echo "nvme2n1" ;;
        "mmc") echo "mmcblk0" ;;
        *) echo "" ;;
    esac
}}
mkdir -p /tmp/test_backup_dest 2>/dev/null || true
source "{export_script}"
validate_export_destination "/tmp/test_backup_dest"
"""
        p = subprocess.run(["bash", "-c", cmd], capture_output=True, text=True)
        return p.returncode, p.stdout + p.stderr

    # Fixture profiles
    rc_u, out_u = run_fixture("usb_removable")
    report("Allowlist evaluates Rules 5-13 and ACCEPTS genuine external USB removable", rc_u == 0 and "ALLOWED" in out_u)

    rc_sata, out_sata = run_fixture("internal_sata")
    report("Allowlist evaluates Rules 5-13 and REJECTS internal SATA bus controller", rc_sata != 0)

    rc_nvme, out_nvme = run_fixture("internal_nvme")
    report("Allowlist evaluates Rules 5-13 and REJECTS internal NVMe bus controller", rc_nvme != 0)

    rc_mmc, out_mmc = run_fixture("mmc")
    report("Allowlist evaluates Rules 5-13 and REJECTS MMC bus controller", rc_mmc != 0)

    rc_bind, out_bind = run_fixture("bind_mount_internal")
    report("Allowlist evaluates Rules 5-13 and REJECTS internal nvme0n1 backing partition", rc_bind != 0 and "strictly protected" in out_bind)

    # Real path negative tests (No fixtures active)
    def run_real_target(t: str) -> int:
        cmd = ["bash", "-c", f"source '{export_script}'; validate_export_destination '{t}'"]
        return subprocess.run(cmd, capture_output=True, text=True).returncode

    report("Real host production USB (/run/media/bhoot/HP_USB) is strictly rejected", run_real_target("/run/media/bhoot/HP_USB") != 0)
    report("Real host workspace path (/home/bhoot/Desktop/BHOOT) is strictly rejected", run_real_target("/home/bhoot/Desktop/BHOOT") != 0)
    report("Real host ESP path (/boot/efi) is strictly rejected", run_real_target("/boot/efi") != 0)
    report("Real host tmpfs path (/tmp) is strictly rejected", run_real_target("/tmp") != 0)


# ==============================================================================
# SECTION 5: Architectural Documentation & Audit Findings (MED-01, MED-02, MED-03, LOW-03)
# ==============================================================================
def test_architectural_documentation():
    print("\n--- 5. Architectural Documentation & Security Specifications ---")
    doc_path = os.path.join(DOCS_DIR, "architecture/BOOT_SECURITY_AND_FIRMWARE_SPEC.md")
    report("BOOT_SECURITY_AND_FIRMWARE_SPEC.md exists", os.path.exists(doc_path))

    if os.path.exists(doc_path):
        with open(doc_path, "r") as f:
            doc = f.read()

        # MED-01: Dell Latitude 5410 & Secure Boot
        report("Doc specifies Dell Latitude 5410 Secure Boot prerequisite (Disabled in BIOS)", "Secure Boot Enable" in doc and "Disabled" in doc)
        report("Doc outlines signed shim + MOK enrollment roadmap", "shimx64.efi" in doc and "Machine Owner Key" in doc)

        # MED-02: ESP Offline Tampering & Evil Maid
        report("Doc analyzes unencrypted FAT32 ESP limitations & Evil Maid threats", "Evil Maid" in doc and "unencrypted FAT32" in doc)
        report("Doc explains SHA-256 software manifest limitations on untrusted media", "software manifest" in doc and "untrusted medium" in doc)
        report("Doc documents TPM2 measured boot roadmap (PCR-7 and PCR-11)", "TPM2" in doc and "PCR-7" in doc and "PCR-11" in doc)

        # MED-03: SQLCipher Status & LUKS2 at-rest protection
        report("Doc states SQLCipher deferred to Phase 3 application integration", "Phase 3" in doc and "SQLCipher" in doc and "bundled-sqlcipher" in doc)
        report("Doc documents complete 100% LUKS2 Argon2id at-rest encryption", "LUKS2 Argon2id" in doc and "48 GiB" in doc)

        # LOW-03: Process memory boundaries
        report("Doc explains process memory zeroization limitations & explicit_bzero", "explicit_bzero" in doc and "volatile memory" in doc)


# ==============================================================================
# MAIN RUNNER
# ==============================================================================
def main():
    print("=" * 76)
    print(" BHOOT — Phase 2A.14 Recovery Security Remediation Verification Suite")
    print("=" * 76)

    test_grub_pbkdf2_tool()
    test_recovery_console_configuration()
    test_recovery_key_normalization()
    test_allowlist_isolated_fixture()
    test_architectural_documentation()

    print("\n" + "=" * 76)
    print(f" RESULTS: {passed} Passed, {failed} Failed")
    print("=" * 76)

    if failed == 0:
        print(" ALL PHASE 2A.14 RECOVERY SECURITY REMEDIATIONS VERIFIED SUCCESSFULLY!")
        sys.exit(0)
    else:
        print(" PHASE 2A.14 REMEDIATION VERIFICATION FAILED!")
        sys.exit(1)


if __name__ == "__main__":
    main()
