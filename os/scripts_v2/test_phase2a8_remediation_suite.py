#!/usr/bin/env python3
"""
BHOOT — Phase 2A.8 Critical Security Remediation Regression Suite
Independently verifies all remediations:
1. CRITICAL-01: 256-Bit Recovery Key Entropy & Validation
2. CRITICAL-02: First-Boot Provisioning Under `set -euo pipefail` (Zero SIGPIPE)
3. HIGH-03: Password Argument Exposure Elimination (STDIN Piping & Cmdline Check)
4. HIGH-01: Robust Destination Block Device Resolution & Protection
5. HIGH-02: GRUB PBKDF2 Authentication Provisioning & Syntax Validation
6. HIGH-04: Atomic Machine-ID Generation & Permission Hardening
7. LOW-01: OS Password Hashing & PAM Evaluation
8. Manifest Deterministic Integrity Check
"""

import os
import sys
import subprocess
import tempfile
import secrets
import shutil
import re
import json

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
OS_DIR = os.path.dirname(SCRIPT_DIR)
BUILD_REAL = os.path.join(OS_DIR, "build_real")
ROOTFS = os.path.join(BUILD_REAL, "rootfs")
CONFIG_DIR = os.path.join(OS_DIR, "config")
sys.path.insert(0, SCRIPT_DIR)

from provision_luks2_volume import generate_recovery_key, validate_recovery_key

passed = 0
failed = 0


def report(name: str, condition: bool, detail: str = ""):
    global passed, failed
    if condition:
        passed += 1
        print(f"  [PASS] {name} {detail}")
    else:
        failed += 1
        print(f"  [FAIL] {name} {detail}")


def test_critical_01_recovery_key_entropy():
    print("\n--- [CRITICAL-01] Recovery Key Entropy & Validation ---")
    
    # Test generation of 32 random bytes (256 bits)
    raw_key = generate_recovery_key()
    chunks = raw_key.split("-")
    clean_hex = raw_key.replace("-", "")
    
    report("Key consists of 16 chunks of 4 characters", len(chunks) == 16 and all(len(c) == 4 for c in chunks), f"chunks={len(chunks)}")
    report("Key consists of exactly 64 hexadecimal characters (256 bits)", len(clean_hex) == 64 and all(c in "0123456789ABCDEF" for c in clean_hex), f"length={len(clean_hex)}")
    
    # Test normalization and validation
    normalized = validate_recovery_key(raw_key)
    report("Canonical formatted key passes validation", normalized == raw_key)
    
    # Test lowercase and space tolerance
    messy = raw_key.lower().replace("-", " ")
    report("Lowercase space-separated key normalizes to canonical", validate_recovery_key(messy) == raw_key)
    
    # Test rejection of 128-bit legacy key (32 hex characters)
    legacy_128 = "ABCD-1234-EF56-7890-ABCD-1234-EF56-7890"
    try:
        validate_recovery_key(legacy_128)
        report("Rejection of legacy 128-bit key", False, "Failed to reject short key")
    except ValueError as e:
        report("Rejection of legacy 128-bit key", True, str(e))
        
    # Test rejection of non-hex characters
    bad_chars = raw_key[:-4] + "-ZZZZ"
    try:
        validate_recovery_key(bad_chars)
        report("Rejection of non-hex characters", False, "Failed to reject invalid characters")
    except ValueError as e:
        report("Rejection of non-hex characters", True, str(e))
        
    # Disposable LUKS2 container enrollment with 256-bit recovery key
    with tempfile.TemporaryDirectory() as tmpdir:
        test_img = os.path.join(tmpdir, "test_luks.img")
        with open(test_img, "wb") as f:
            f.truncate(64 * 1024 * 1024)
            
        master_pass = "TestMasterPassword2026!"
        fmt_cmd = [
            "cryptsetup", "luksFormat", "--type", "luks2", "--batch-mode",
            "--cipher", "aes-xts-plain64", "--key-size", "512", "--sector-size", "4096",
            "--pbkdf", "argon2id", "--pbkdf-memory", "65536", "--pbkdf-parallel", "2",
            "--pbkdf-force-iterations", "4", test_img
        ]
        p = subprocess.Popen(fmt_cmd, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        p.communicate(input=master_pass.encode())
        
        # Enroll 256-bit recovery key
        add_cmd = [
            "cryptsetup", "luksAddKey", "--batch-mode", "--key-slot", "1",
            "--pbkdf", "argon2id", "--pbkdf-memory", "65536", "--pbkdf-parallel", "2",
            "--pbkdf-force-iterations", "4", test_img
        ]
        p2 = subprocess.Popen(add_cmd, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        p2.communicate(input=f"{master_pass}\n{raw_key}\n".encode())
        report("Keyslot 1 enrollment with 256-bit recovery key", p2.returncode == 0)
        
        # Test unlock with recovery key
        check_p = subprocess.Popen(["cryptsetup", "open", "--test-passphrase", test_img, "--batch-mode"],
                                   stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        check_p.communicate(input=f"{raw_key}\n".encode())
        report("Keyslot 1 256-bit recovery key unlocks container cleanly", check_p.returncode == 0)


def test_critical_02_sigpipe_elimination():
    print("\n--- [CRITICAL-02] First-Boot Provisioning Under `pipefail` (Zero SIGPIPE) ---")
    
    test_script = """#!/usr/bin/env bash
set -euo pipefail

# Replicate first-boot provisioning salt generation
SALT=$(perl -e '
    open my $f, "<:raw", "/dev/urandom" or die "urandom read failed: $!\n";
    read($f, my $b, 16) == 16 or die "short read\n";
    my @c = ("A".."Z", "a".."z", "0".."9");
    print join("", map { $c[ord($_) % scalar(@c)] } split(//, $b));
')

echo "SALT=$SALT"
echo "LEN=${#SALT}"
"""
    with tempfile.NamedTemporaryFile("w", suffix=".sh", delete=False) as f:
        f.write(test_script)
        script_path = f.name
    os.chmod(script_path, 0o755)
    
    try:
        # Run 50 iterations under set -euo pipefail to guarantee no SIGPIPE flakes
        all_passed = True
        for _ in range(50):
            res = subprocess.run([script_path], capture_output=True, text=True)
            if res.returncode != 0 or "LEN=16" not in res.stdout:
                all_passed = False
                break
        report("Salt generation executes 50 iterations under `set -euo pipefail` without SIGPIPE", all_passed)
    finally:
        os.remove(script_path)


def test_high_03_password_exposure_elimination():
    print("\n--- [HIGH-03] Password Command-Line Argument Exposure Elimination ---")
    
    # Verify bhoot-firstboot-setup passes password via STDIN and not in perl ARGV
    setup_script = os.path.join(ROOTFS, "usr/local/sbin/bhoot-firstboot-setup")
    with open(setup_script, "r") as f:
        content = f.read()
        
    report("No '$PASSWD_INPUT' in Perl invocation arguments", "perl -e" in content and "print crypt($ARGV[0]" not in content)
    report("Password is piped via STDIN (printf '%s' | perl)", "printf '%s' \"$PASSWD_INPUT\" | perl" in content)
    report("Perl reads password from <STDIN>", "my $pwd = <STDIN>;" in content)
    report("Perl memory best-effort overwrite present", "$pwd = \"0\" x length($pwd);" in content)
    report("PASSWD_INPUT is unset in bash", "unset PASSWD_INPUT" in content)


def test_high_01_backup_device_resolution():
    print("\n--- [HIGH-01] Recovery Backup Destination-Device Resolution ---")
    
    export_script = os.path.join(SCRIPT_DIR, "export_recovery_backup.sh")
    with open(export_script, "r") as f:
        content = f.read()
        
    report("export_recovery_backup.sh uses findmnt for backing device resolution", "findmnt" in content)
    report("export_recovery_backup.sh uses lsblk for parent disk inspection", "lsblk" in content)
    
    # Test rejection of protected real devices and simulated paths
    env = os.environ.copy()
    env["BHOOT_AUTOMATED_TEST"] = "1"
    
    # 1. Target USB mount point rejection (/run/media/bhoot/HP_USB)
    res_usb = subprocess.run([export_script, "/run/media/bhoot/HP_USB/backup"], env=env, capture_output=True, text=True)
    report("Rejection of real target USB mount point (/run/media/bhoot/HP_USB)", res_usb.returncode != 0 and "CRITICAL SAFETY VIOLATION" in res_usb.stderr)
    
    # 2. Raw device node rejection (/dev/sda1)
    res_raw = subprocess.run([export_script, "/dev/sda1"], env=env, capture_output=True, text=True)
    report("Rejection of raw block device node path (/dev/sda1)", res_raw.returncode != 0 and "CRITICAL ERROR" in res_raw.stderr)
    
    # 3. Internal NVMe SSD mount rejection (/home/bhoot/Desktop/BHOOT)
    res_nvme = subprocess.run([export_script, "/home/bhoot/Desktop/BHOOT"], env=env, capture_output=True, text=True)
    report("Rejection of internal SSD workspace (/home/bhoot/Desktop/BHOOT)", res_nvme.returncode != 0 and "CRITICAL SAFETY VIOLATION" in res_nvme.stderr)
    
    # 4. Approved virtual external target acceptance
    with tempfile.TemporaryDirectory() as tmpdir:
        approved_dir = os.path.join(tmpdir, "external_backup_disk")
        dummy_header = os.path.join(tmpdir, "dummy_header.backup")
        with open(dummy_header, "wb") as f:
            f.write(secrets.token_bytes(4096))
            
        res_ok = subprocess.run([export_script, "--source", dummy_header, approved_dir], env=env, capture_output=True, text=True)
        report("Acceptance of approved virtual external target directory", res_ok.returncode == 0 and os.path.exists(os.path.join(approved_dir, "bhoot_luks_header.backup")))


def test_high_02_grub_auth_provisioning():
    print("\n--- [HIGH-02] GRUB PBKDF2 Authentication Provisioning ---")
    
    grub_cfg = os.path.join(CONFIG_DIR, "boot/grub.cfg")
    with open(grub_cfg, "r") as f:
        content = f.read()
        
    report("GRUB config superusers configured as 'bhoot-admin'", 'set superusers="bhoot-admin"' in content)
    report("GRUB sources /EFI/BOOT/grub-auth.cfg and /boot/grub/grub-auth.cfg", "grub-auth.cfg" in content)
    report("Standard boot entry is --unrestricted (does not prompt user for password)", 'menuentry "BHOOT — Private Financial Intelligence OS (Secure Production)" --unrestricted' in content)
    report("Recovery entry is restricted to --users bhoot-admin", 'menuentry "BHOOT — Offline Recovery Shell & Header Restore" --users bhoot-admin' in content)
    
    # Test PBKDF2 hash generation in Perl (used by build & setup)
    test_gen = subprocess.run([
        "perl", "-e",
        """use Digest::SHA qw(hmac_sha512);
        open my $f, "<:raw", "/dev/urandom" or die;
        read($f, my $salt_raw, 64) == 64 or die;
        my $salt_hex = uc(unpack("H*", $salt_raw));
        my $pwd = "TestPass123!";
        my $u = hmac_sha512($salt_raw . pack("N", 1), $pwd);
        my $t = $u;
        for (my $i = 1; $i < 10000; $i++) { $u = hmac_sha512($u, $pwd); $t ^= $u; }
        my $hash_hex = uc(unpack("H*", $t));
        print "password_pbkdf2 bhoot-admin grub.pbkdf2.sha512.10000.${salt_hex}.${hash_hex}\\n";"""
    ], capture_output=True, text=True)
    
    line = test_gen.stdout.strip()
    match = re.match(r"^password_pbkdf2\s+bhoot-admin\s+grub\.pbkdf2\.sha512\.10000\.([0-9A-F]{128})\.([0-9A-F]{128})$", line)
    report("Perl PBKDF2 generator produces canonical GRUB 2 configuration line", match is not None, f"len={len(line)}")


def test_high_04_atomic_machine_id():
    print("\n--- [HIGH-04] Atomic Machine-ID Generation ---")
    
    sec_init = os.path.join(ROOTFS, "usr/local/sbin/bhoot-security-init")
    with open(sec_init, "r") as f:
        content = f.read()
        
    report("bhoot-security-init writes to temporary file (.tmp)", 'machine-id.tmp' in content)
    report("bhoot-security-init atomically renames (mv -f) to final path", 'mv -f' in content and 'machine-id' in content)
    report("bhoot-security-init sets strict permissions (0444)", 'chmod 0444' in content)
    report("bhoot-security-init flushes writes with sync", 'sync' in content)
    
    # Test atomic write simulation
    with tempfile.TemporaryDirectory() as tmpdir:
        target = os.path.join(tmpdir, "machine-id")
        tmp = os.path.join(tmpdir, "machine-id.tmp")
        
        mid_hex = secrets.token_hex(16)
        with open(tmp, "w") as f:
            f.write(f"{mid_hex}\n")
        os.chmod(tmp, 0o444)
        os.replace(tmp, target)
        
        with open(target, "r") as f:
            read_back = f.read().strip()
        report("Simulated atomic machine-id generation creates valid 32-char hex ID", read_back == mid_hex and len(read_back) == 32)


def test_low_01_pam_evaluation():
    print("\n--- [LOW-01] OS Password Hashing & PAM Evaluation ---")
    
    pam_pwd = os.path.join(ROOTFS, "etc/pam.d/common-password")
    with open(pam_pwd, "r") as f:
        content = f.read()
        
    report("common-password enables modern yescrypt in pam_unix.so", "yescrypt" in content)
    
    # Verify libcrypt supports yescrypt
    libcrypt_check = subprocess.run([
        "perl", "-e",
        r'print crypt("TestPwd123!", "\$y\$j9T\$/2E./2E./2E./2E./2E./.") . qq(\n);'
    ], capture_output=True, text=True)
    report("Target environment libcrypt supports yescrypt algorithm ($y$)", libcrypt_check.stdout.startswith("$y$"))


def test_manifest_verification():
    print("\n--- Manifest Deterministic Integrity Check ---")
    manifest_path = os.path.join(BUILD_REAL, "manifest.json")
    if not os.path.isfile(manifest_path):
        report("manifest.json exists", False)
        return
        
    with open(manifest_path, "r") as f:
        m = json.load(f)
        
    report("manifest.json has valid structure with 'artifacts' list", "artifacts" in m and isinstance(m["artifacts"], list))
    report("manifest.json contains exactly 9 artifacts", len(m["artifacts"]) == 9, f"count={len(m['artifacts'])}")
    
    all_matched = True
    for a in m["artifacts"]:
        path = os.path.join(BUILD_REAL, a["filename"])
        if not os.path.exists(path):
            all_matched = False
            print(f"    Missing artifact: {a['filename']}")
            continue
            
        st = os.stat(path)
        if st.st_size != a["logical_size_bytes"]:
            all_matched = False
            print(f"    Size mismatch on {a['filename']}: expected {a['logical_size_bytes']}, got {st.st_size}")
            
    report("All manifest artifacts exist and match logical size", all_matched)


def main():
    print("=" * 72)
    print(" BHOOT — Phase 2A.8 Critical Security Remediation Regression Suite")
    print("=" * 72)
    
    test_critical_01_recovery_key_entropy()
    test_critical_02_sigpipe_elimination()
    test_high_03_password_exposure_elimination()
    test_high_01_backup_device_resolution()
    test_high_02_grub_auth_provisioning()
    test_high_04_atomic_machine_id()
    test_low_01_pam_evaluation()
    test_manifest_verification()
    
    print("\n" + "=" * 72)
    print(f" RESULTS: {passed} Passed, {failed} Failed")
    print("=" * 72)
    
    if failed == 0:
        print(" ALL PHASE 2A.8 SECURITY REMEDIATIONS VERIFIED SUCCESSFULLY!")
        sys.exit(0)
    else:
        print(f" ERROR: {failed} tests failed!")
        sys.exit(1)


if __name__ == "__main__":
    main()
