#!/usr/bin/env python3
"""
BHOOT — LUKS2 Cryptographic Volume Provisioner & Credential Manager
Provisions LUKS2 storage with Argon2id + AES-256-XTS, independent recovery key,
and Ext4 persistence without hardcoded secrets or temporary plaintext key files.

Standards:
- PBKDF: Argon2id (64 MiB memory, 4 time iterations, 2 parallel threads)
- Cipher: AES-256-XTS (aes-xts-plain64, 512-bit key)
- Sector Size: 4096 bytes
- Keyslot 0: Master Passphrase (Owner defined, min 16 chars)
- Keyslot 1: Independent Recovery Key (256-bit CSPRNG, formatted base32/alphanumeric)
"""

import os
import sys
import argparse
import subprocess
import getpass
import secrets
import shutil
import time
import dbus

CANONICAL_UUID = "B8007001-0003-4000-8000-000000000003"
ARGON2_MEM_KIB = 65536
ARGON2_TIME = 4
ARGON2_THREADS = 2
SECTOR_SIZE = 4096


def generate_recovery_key() -> str:
    """Generate 256-bit cryptographically secure independent recovery key.
    
    Generates 32 cryptographically secure random bytes (256 bits of entropy).
    Formatted as 16 chunks of 4 uppercase hex characters:
    XXXX-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX
    Total: 64 hexadecimal characters (79 characters including separators).
    """
    token = secrets.token_bytes(32).hex().upper()
    chunks = [token[i:i+4] for i in range(0, 64, 4)]
    return "-".join(chunks)


def normalize_recovery_key(key_input: str) -> str:
    """Normalize recovery key input to canonical 16x4 chunked format.
    
    Accepts raw 64-character hex strings, lowercase, uppercase, with or without
    delimiters (spaces, hyphens, colons, underscores).
    Returns canonical format: XXXX-XXXX-...-XXXX (79 characters).
    """
    import re
    cleaned = re.sub(r"[^0-9a-fA-F]", "", key_input).upper()
    if len(cleaned) != 64:
        raise ValueError(
            f"Invalid recovery key length: expected 64 hex characters (256 bits of entropy), got {len(cleaned)}"
        )
    chunks = [cleaned[i:i+4] for i in range(0, 64, 4)]
    return "-".join(chunks)


def validate_recovery_key(key_input: str) -> str:
    """Validate and normalize a 256-bit recovery key.
    
    Ensures exactly 64 hexadecimal characters (256 bits of entropy).
    Allows hyphens and spaces for readability and returns canonical chunked format.
    """
    return normalize_recovery_key(key_input)


def run_cryptsetup_pipe(cmd: list, input_bytes: bytes) -> tuple[int, str, str]:
    """Execute cryptsetup command passing sensitive input directly via stdin pipe."""
    p = subprocess.Popen(
        cmd,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE
    )
    stdout, stderr = p.communicate(input=input_bytes)
    return p.returncode, stdout.decode('utf-8', errors='replace'), stderr.decode('utf-8', errors='replace')


def format_luks2_container(container_path: str, master_passphrase: str, recovery_key: str, uuid: str = CANONICAL_UUID):
    """Format target container with LUKS2, enroll master passphrase in slot 0 and recovery key in slot 1."""
    if not os.path.exists(container_path):
        raise FileNotFoundError(f"Container file does not exist: {container_path}")

    # Step 1: Format with Keyslot 0 (Master Passphrase)
    format_cmd = [
        "cryptsetup", "luksFormat",
        "--type", "luks2",
        "--batch-mode",
        "--cipher", "aes-xts-plain64",
        "--key-size", "512",
        "--sector-size", str(SECTOR_SIZE),
        "--pbkdf", "argon2id",
        "--pbkdf-memory", str(ARGON2_MEM_KIB),
        "--pbkdf-parallel", str(ARGON2_THREADS),
        "--pbkdf-force-iterations", str(ARGON2_TIME),
        "--uuid", uuid.lower(),
        "--label", "BHOOT_DATA",
        container_path
    ]

    rc, out, err = run_cryptsetup_pipe(format_cmd, master_passphrase.encode('utf-8'))
    if rc != 0:
        raise RuntimeError(f"cryptsetup luksFormat failed (exit code {rc}): {err}")

    # Step 2: Enroll Keyslot 1 with Independent Recovery Key
    # cryptsetup luksAddKey takes: <existing_passphrase>\n<new_key>\n
    addkey_cmd = [
        "cryptsetup", "luksAddKey",
        "--batch-mode",
        "--key-slot", "1",
        "--pbkdf", "argon2id",
        "--pbkdf-memory", str(ARGON2_MEM_KIB),
        "--pbkdf-parallel", str(ARGON2_THREADS),
        "--pbkdf-force-iterations", str(ARGON2_TIME),
        container_path
    ]

    key_input = f"{master_passphrase}\n{recovery_key}\n".encode('utf-8')
    rc, out, err = run_cryptsetup_pipe(addkey_cmd, key_input)
    if rc != 0:
        raise RuntimeError(f"cryptsetup luksAddKey (Keyslot 1) failed (exit code {rc}): {err}")


def extract_header_backup(container_path: str, backup_path: str):
    """Extract detached LUKS2 header backup."""
    if os.path.exists(backup_path):
        os.remove(backup_path)

    cmd = [
        "cryptsetup", "luksHeaderBackup",
        container_path,
        "--header-backup-file", backup_path,
        "--batch-mode"
    ]
    p = subprocess.run(cmd, capture_output=True, text=True)
    if p.returncode != 0:
        raise RuntimeError(f"luksHeaderBackup failed: {p.stderr}")

    os.chmod(backup_path, 0o400)


def verify_keyslot_unlock(container_path: str, secret: str) -> bool:
    """Test unlockability of a container using a specific secret without activating device mapper."""
    test_cmd = ["cryptsetup", "open", "--test-passphrase", container_path, "--batch-mode"]
    rc, out, err = run_cryptsetup_pipe(test_cmd, (secret + "\n").encode('utf-8'))
    return rc == 0


def setup_ext4_filesystem(container_path: str, master_passphrase: str):
    """Unlock container via udisksctl with stdin pipe, format as Ext4 labeled BHOOT_DATA, and stage persistence hierarchy."""
    # 1. Attach loop device
    loop_res = subprocess.run(
        ["udisksctl", "loop-setup", "--file", container_path, "--no-user-interaction"],
        capture_output=True, text=True, check=True
    )
    loop_dev = loop_res.stdout.split()[-1].rstrip(".")

    clear_dev = None
    mnt_pt = None
    try:
        # 2. Unlock using stdin pipe (zero plaintext key on disk)
        unlock_cmd = ["udisksctl", "unlock", "-b", loop_dev, "--key-file", "/dev/stdin", "--no-user-interaction"]
        rc, out, err = run_cryptsetup_pipe(unlock_cmd, master_passphrase.encode('utf-8'))
        if rc != 0:
            raise RuntimeError(f"udisksctl unlock failed (exit code {rc}): {err}")
        clear_dev = out.split()[-1].rstrip(".")

        # 3. Format as Ext4 via UDisks2 DBus interface
        dm_name = os.path.basename(clear_dev).replace("-", "_2d")
        obj_path = f"/org/freedesktop/UDisks2/block_devices/{dm_name}"

        bus = dbus.SystemBus()
        block_obj = bus.get_object("org.freedesktop.UDisks2", obj_path)
        block_iface = dbus.Interface(block_obj, "org.freedesktop.UDisks2.Block")

        block_iface.Format(
            "ext4",
            dbus.Dictionary({
                "label": "BHOOT_DATA",
                "take-ownership": dbus.Boolean(True)
            }, signature="sv")
        )
        time.sleep(0.5)

        # 4. Mount (check if automounted first)
        find_res = subprocess.run(["findmnt", "-n", "-o", "TARGET", clear_dev], capture_output=True, text=True)
        mnt_pt = find_res.stdout.strip()
        if not mnt_pt:
            mount_res = subprocess.run(
                ["udisksctl", "mount", "-b", clear_dev, "--no-user-interaction"],
                capture_output=True, text=True, check=True
            )
            mnt_pt = mount_res.stdout.split()[-1].rstrip(".")

        # 5. live-boot persistence structure
        target_dir = os.path.join(mnt_pt, "var", "bhoot", "rw")
        os.makedirs(target_dir, exist_ok=True)
        os.chmod(os.path.join(mnt_pt, "var", "bhoot"), 0o755)

        with open(os.path.join(mnt_pt, "persistence.conf"), "w") as f:
            f.write("/var/bhoot\n")
        os.chmod(os.path.join(mnt_pt, "persistence.conf"), 0o644)

        # Persistence proof token
        proof_path = os.path.join(target_dir, "ledger", "persistence_marker.proof")
        os.makedirs(os.path.dirname(proof_path), exist_ok=True)
        with open(proof_path, "w") as f:
            f.write("BHOOT_EXT4_PERSISTENCE_VALIDATED_2026\n")
        os.chmod(proof_path, 0o600)

        # System storage directory for machine-id and provisioned credentials
        sys_dir = os.path.join(target_dir, "system")
        os.makedirs(sys_dir, exist_ok=True)
        os.chmod(sys_dir, 0o700)

        # 6. Unmount
        subprocess.run(["udisksctl", "unmount", "-b", clear_dev, "--no-user-interaction"], check=True)
        mnt_pt = None
        time.sleep(0.3)

        # 7. Lock
        subprocess.run(["udisksctl", "lock", "-b", loop_dev, "--no-user-interaction"], check=True)
        clear_dev = None
        time.sleep(0.3)

    finally:
        # Strict teardown: unmount all cleartext/loop mounts, lock dm-crypt, and delete loopback device
        if clear_dev and os.path.exists(clear_dev):
            subprocess.run(["udisksctl", "unmount", "-b", clear_dev, "--no-user-interaction"], capture_output=True)
            time.sleep(0.2)
        if loop_dev and os.path.exists(loop_dev):
            find_res = subprocess.run(["findmnt", "-n", "-o", "TARGET", loop_dev], capture_output=True, text=True)
            if find_res.stdout.strip():
                subprocess.run(["udisksctl", "unmount", "-b", loop_dev, "--no-user-interaction"], capture_output=True)
            subprocess.run(["udisksctl", "lock", "-b", loop_dev, "--no-user-interaction"], capture_output=True)
            time.sleep(0.2)
            subprocess.run(["udisksctl", "loop-delete", "-b", loop_dev, "--no-user-interaction"], capture_output=True)


def main():
    parser = argparse.ArgumentParser(description="BHOOT Secure LUKS2 Volume Provisioner")
    parser.add_argument("--container", required=True, help="Path to LUKS2 container image file")
    parser.add_argument("--size-gib", type=int, default=48, help="Size of container in GiB (sparse allocation)")
    parser.add_argument("--header-backup", required=True, help="Path to write detached header backup")
    parser.add_argument("--uuid", default=CANONICAL_UUID, help="Canonical LUKS2 UUID")
    parser.add_argument("--test-mode", action="store_true", help="Non-interactive test mode using ephemeral random secrets")
    parser.add_argument("--passphrase-fd", type=int, help="Read passphrase from open file descriptor (automated test only)")
    parser.add_argument("--recovery-fd", type=int, help="Read recovery key from open file descriptor (automated test only)")

    args = parser.parse_args()

    print("========================================================================")
    print(" [BHOOT Security Engine] LUKS2 Cryptographic Volume Provisioning")
    print("========================================================================")

    master_passphrase = None
    recovery_key = None

    if args.passphrase_fd is not None:
        master_passphrase = os.fdopen(args.passphrase_fd).readline().strip()
    elif args.test_mode:
        master_passphrase = secrets.token_urlsafe(32)
    else:
        # Interactive Owner Provisioning
        while True:
            p1 = getpass.getpass(" Enter BHOOT LUKS2 Master Passphrase (min 16 chars): ")
            if len(p1) < 16:
                print(" [ERROR] Passphrase too short. Minimum 16 characters required for financial storage.")
                continue
            p2 = getpass.getpass(" Confirm BHOOT LUKS2 Master Passphrase: ")
            if p1 != p2:
                print(" [ERROR] Passphrases do not match. Please re-enter.")
                continue
            master_passphrase = p1
            del p1, p2
            break

    if args.recovery_fd is not None:
        raw_key = os.fdopen(args.recovery_fd).readline().strip()
        recovery_key = validate_recovery_key(raw_key)
        del raw_key
    else:
        recovery_key = generate_recovery_key()

    # Allocate sparse container
    size_bytes = args.size_gib * 1024 * 1024 * 1024
    print(f"  Allocating {args.size_gib} GiB container at: {args.container}...")
    if os.path.exists(args.container):
        os.remove(args.container)

    with open(args.container, "wb") as f:
        f.truncate(size_bytes)

    print("  Formatting with LUKS2 (Argon2id + AES-256-XTS, 4096-byte sectors)...")
    print("  Enrolling Master Passphrase in Keyslot 0...")
    print("  Enrolling Independent 256-bit Recovery Key in Keyslot 1...")
    format_luks2_container(args.container, master_passphrase, recovery_key, args.uuid)

    print("  Extracting detached header backup...")
    extract_header_backup(args.container, args.header_backup)

    print("  Verifying Keyslot 0 (Master Passphrase) unlock...")
    if not verify_keyslot_unlock(args.container, master_passphrase):
        raise RuntimeError("Keyslot 0 master passphrase verification FAILED!")

    print("  Verifying Keyslot 1 (Recovery Key) unlock...")
    if not verify_keyslot_unlock(args.container, recovery_key):
        raise RuntimeError("Keyslot 1 recovery key verification FAILED!")

    print("  Verifying wrong passphrase rejection...")
    wrong_key = secrets.token_urlsafe(32)
    if verify_keyslot_unlock(args.container, wrong_key):
        raise RuntimeError("CRITICAL: Container unlocked with wrong passphrase!")

    print("  Formatting Ext4 persistence filesystem (label: BHOOT_DATA)...")
    setup_ext4_filesystem(args.container, master_passphrase)

    print("========================================================================")
    print(" [SUCCESS] LUKS2 Storage Volume Provisioned Successfully")
    print("========================================================================")

    if not args.test_mode and args.passphrase_fd is None:
        print("\n" + "#" * 72)
        print(" # IMPORTANT: BHOOT INDEPENDENT DISASTER RECOVERY KEY")
        print(" #" * 36)
        print(f"   RECOVERY KEY: {recovery_key}")
        print(" #" * 36)
        print(" # This key is independent of your master passphrase.")
        print(" # It is the ONLY way to recover data if the passphrase is lost.")
        print(" # Store this key OFFLINE on paper or a secondary offline token.")
        print(" # IT WILL NEVER BE DISPLAYED AGAIN AND IS NOT STORED ON THE USB.")
        print("#" * 72 + "\n")

        while True:
            confirm = input(" Type 'CONFIRMED' to acknowledge you have recorded this key: ").strip()
            if confirm == "CONFIRMED":
                break

    # Clean secrets from memory
    del master_passphrase
    del recovery_key


if __name__ == "__main__":
    main()
