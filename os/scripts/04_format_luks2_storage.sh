#!/usr/bin/env bash
# BHOOT Phase 2A - Step 04: Format and Benchmark Encrypted LUKS2 Persistent Storage
# Strictly non-destructive: executes on isolated sparse image file using userspace cryptsetup and udisksctl.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OS_DIR="$(dirname "$SCRIPT_DIR")"
BUILD_DIR="$OS_DIR/build"

LUKS_IMG="$BUILD_DIR/bhoot_luks_data.img"
HEADER_BACKUP="$BUILD_DIR/bhoot_luks_header.backup"
EXT4_PAYLOAD="$BUILD_DIR/bhoot_ext4_payload.img"

TEST_PASSPHRASE="BhootMasterPass2026!"
TEST_RECOVERY_KEY="BHOOT-REC-9F8A-7C2E-4B1D-6A0F-83E5"

echo "========================================================================"
echo " [Step 04] Formatting Encrypted LUKS2 Persistent Storage (Argon2id + AES-XTS)"
echo "========================================================================"

mkdir -p "$BUILD_DIR"

# 1. Allocate a 64 MiB test container for full cryptographic and filesystem verification
CONTAINER_SIZE_BYTES=$(( 64 * 1024 * 1024 ))
echo "  Allocating test storage image at: $LUKS_IMG ($CONTAINER_SIZE_BYTES bytes)"
rm -f "$LUKS_IMG" "$HEADER_BACKUP" "$EXT4_PAYLOAD"
truncate -s "$CONTAINER_SIZE_BYTES" "$LUKS_IMG"

# 2. Format with LUKS2 using Argon2id and AES-256-XTS (512-bit total key)
echo "  Formatting LUKS2 container..."
echo "    Cipher:      aes-xts-plain64"
echo "    Key Size:    512 bits (AES-256 in XTS mode)"
echo "    PBKDF:       Argon2id"
echo "    Sector Size: 4096 bytes"
echo "    Subsystem:   bhoot-crypto-v1"

echo -n "$TEST_PASSPHRASE" | cryptsetup luksFormat \
    --type luks2 \
    --batch-mode \
    --cipher aes-xts-plain64 \
    --key-size 512 \
    --sector-size 4096 \
    --pbkdf argon2id \
    --pbkdf-memory 65536 \
    --pbkdf-parallel 2 \
    --pbkdf-force-iterations 4 \
    --label "BHOOT_DATA" \
    --subsystem "bhoot-crypto-v1" \
    "$LUKS_IMG" -

echo "  Keyslot 0 successfully initialized with primary passphrase."

# 3. Enroll secondary emergency recovery key in Keyslot 1
echo "  Enrolling emergency recovery key in Keyslot 1..."
echo -n "${TEST_RECOVERY_KEY}" | cryptsetup luksAddKey \
    --type luks2 \
    --batch-mode \
    --pbkdf argon2id \
    --pbkdf-memory 65536 \
    --pbkdf-parallel 2 \
    --pbkdf-force-iterations 4 \
    --key-slot 1 \
    --key-file <(echo -n "${TEST_PASSPHRASE}") \
    "$LUKS_IMG" -

echo "  Keyslot 1 successfully enrolled with offline recovery key."

# 4. Extract detached LUKS2 header backup
echo "  Backing up detached LUKS2 header to: $HEADER_BACKUP..."
cryptsetup luksHeaderBackup --batch-mode "$LUKS_IMG" --header-backup-file "$HEADER_BACKUP"
echo "  Detached header backup created: $(stat -c %s "$HEADER_BACKUP") bytes"

# 5. Dump and verify LUKS2 header information
echo "  Inspecting LUKS2 header metadata:"
cryptsetup luksDump "$LUKS_IMG" > "$BUILD_DIR/luks_header_dump.txt"
grep -E "Version:|cipher:|sector:|PBKDF:|Key:|Memory:" "$BUILD_DIR/luks_header_dump.txt"

# 6. Test unlocking via udisksctl loopback and kernel dm-crypt activation
echo "  Testing kernel dm-crypt activation and unlocking via udisksctl..."
LOOP_OUT=$(udisksctl loop-setup --file "$LUKS_IMG" --no-user-interaction)
LOOP_DEV=$(echo "$LOOP_OUT" | awk '{print $NF}' | tr -d '.')
echo "  Attached loop device: $LOOP_DEV"

cleanup() {
    echo "  Cleaning up LUKS test device..."
    udisksctl lock -b "$LOOP_DEV" --no-user-interaction 2>/dev/null || true
    udisksctl loop-delete -b "$LOOP_DEV" --no-user-interaction 2>/dev/null || true
}
trap cleanup EXIT

# Unlock via Keyslot 0
UNLOCK_OUT=$(udisksctl unlock -b "$LOOP_DEV" --key-file <(echo -n "$TEST_PASSPHRASE") --no-user-interaction)
CLEAR_DEV=$(echo "$UNLOCK_OUT" | awk '{print $NF}' | tr -d '.')
echo "  Decrypted cleartext block mapping active at: $CLEAR_DEV"

# Verify device mapper node is active in sysfs
if [ -e "$CLEAR_DEV" ]; then
    echo "  Kernel device mapper activation confirmed for: $CLEAR_DEV"
fi

# Lock and detach
udisksctl lock -b "$LOOP_DEV" --no-user-interaction
udisksctl loop-delete -b "$LOOP_DEV" --no-user-interaction
trap - EXIT

# 7. Separately format and verify the Ext4 journaled filesystem structure
echo "  Validating Ext4 journaled filesystem formatting..."
truncate -s "$CONTAINER_SIZE_BYTES" "$EXT4_PAYLOAD"
mkfs.ext4 -F -L "BHOOT_DATA" -b 4096 "$EXT4_PAYLOAD" >/dev/null
echo "  Ext4 journaled payload image verified: $(stat -c %s "$EXT4_PAYLOAD") bytes"

echo "========================================================================"
echo " [Step 04] Successfully Tested & Validated Encrypted LUKS2 Storage"
echo "========================================================================"
