#!/usr/bin/env bash
# BHOOT Phase 2A.2 - Step 05: Prepare Encrypted LUKS2 Storage Containers
# Explicitly separates the 64 MiB test-vector container from the 48 GiB sparse production volume.
# Passphrases and recovery keys are generated ephemerally and NEVER echoed to logs or stdout.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OS_DIR="$(dirname "$SCRIPT_DIR")"
BUILD_REAL="$OS_DIR/build_real"

TEST_CONTAINER="$BUILD_REAL/bhoot_luks_test_64m.img"
PROD_CONTAINER="$BUILD_REAL/bhoot_luks_prod_48g.img"
HEADER_BACKUP="$BUILD_REAL/bhoot_luks_header.backup"
HEADER_DUMP="$BUILD_REAL/bhoot_luks_header_dump.txt"

echo "========================================================================"
echo " [Phase 2A.2 - Step 05] Preparing Production & Test LUKS2 Storage"
echo "========================================================================"

mkdir -p "$BUILD_REAL"

# Generate random ephemeral authentication tokens (in-memory only, NEVER printed)
EPHEMERAL_PASSPHRASE=$(head -c 32 /dev/urandom | base64)
EPHEMERAL_RECOVERY=$(head -c 32 /dev/urandom | base64)

# -----------------------------------------------------------------------------
# Part A: 64 MiB Functional Verification Test Vector
# -----------------------------------------------------------------------------
echo "  [Part A] Preparing 64 MiB Test-Vector Container..."
TEST_SIZE_BYTES=$(( 64 * 1024 * 1024 ))
rm -f "$TEST_CONTAINER"
truncate -s "$TEST_SIZE_BYTES" "$TEST_CONTAINER"

echo "    Formatting test container with LUKS2 (Argon2id + AES-256-XTS)..."
printf '%s' "$EPHEMERAL_PASSPHRASE" | cryptsetup luksFormat \
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
    "$TEST_CONTAINER" -

echo "    Enrolling secondary recovery key in Keyslot 1..."
printf '%s' "$EPHEMERAL_RECOVERY" | cryptsetup luksAddKey \
    --type luks2 \
    --batch-mode \
    --pbkdf argon2id \
    --pbkdf-memory 65536 \
    --pbkdf-parallel 2 \
    --pbkdf-force-iterations 4 \
    --key-slot 1 \
    --key-file <(printf '%s' "$EPHEMERAL_PASSPHRASE") \
    "$TEST_CONTAINER" -

echo "    Testing udisksctl loopback activation and device mapper creation..."
LOOP_OUT=$(udisksctl loop-setup --file "$TEST_CONTAINER" --no-user-interaction)
LOOP_DEV=$(echo "$LOOP_OUT" | awk '{print $NF}' | tr -d '.')
echo "    Attached loop device: $LOOP_DEV"

cleanup_test() {
    echo "    Cleaning up test loop device..."
    udisksctl lock -b "$LOOP_DEV" --no-user-interaction 2>/dev/null || true
    udisksctl loop-delete -b "$LOOP_DEV" --no-user-interaction 2>/dev/null || true
}
trap cleanup_test EXIT

UNLOCK_OUT=$(udisksctl unlock -b "$LOOP_DEV" --key-file <(printf '%s' "$EPHEMERAL_PASSPHRASE") --no-user-interaction)
CLEAR_DEV=$(echo "$UNLOCK_OUT" | awk '{print $NF}' | tr -d '.')
echo "    Decrypted cleartext block mapping active at: $CLEAR_DEV"

if [ -e "$CLEAR_DEV" ]; then
    echo "    Kernel device mapper activation confirmed for test container."
fi

udisksctl lock -b "$LOOP_DEV" --no-user-interaction
udisksctl loop-delete -b "$LOOP_DEV" --no-user-interaction
trap - EXIT

# -----------------------------------------------------------------------------
# Part B: 48 GiB Production Storage Sparse Container
# -----------------------------------------------------------------------------
echo "  [Part B] Preparing 48 GiB Production Sparse Volume (Sector count: 100,663,296)..."
# 100,663,296 sectors * 512 bytes = 51,539,607,552 bytes (~48 GiB)
PROD_SIZE_BYTES=51539607552
rm -f "$PROD_CONTAINER" "$HEADER_BACKUP" "$HEADER_DUMP"
truncate -s "$PROD_SIZE_BYTES" "$PROD_CONTAINER"

echo "    Formatting 48 GiB production sparse container with LUKS2..."
printf '%s' "$EPHEMERAL_PASSPHRASE" | cryptsetup luksFormat \
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
    "$PROD_CONTAINER" -

echo "    Enrolling production emergency recovery key in Keyslot 1..."
printf '%s' "$EPHEMERAL_RECOVERY" | cryptsetup luksAddKey \
    --type luks2 \
    --batch-mode \
    --pbkdf argon2id \
    --pbkdf-memory 65536 \
    --pbkdf-parallel 2 \
    --pbkdf-force-iterations 4 \
    --key-slot 1 \
    --key-file <(printf '%s' "$EPHEMERAL_PASSPHRASE") \
    "$PROD_CONTAINER" -

echo "    Backing up detached LUKS2 header to: $HEADER_BACKUP..."
cryptsetup luksHeaderBackup --batch-mode "$PROD_CONTAINER" --header-backup-file "$HEADER_BACKUP"

echo "    Dumping production LUKS2 header metadata to: $HEADER_DUMP..."
cryptsetup luksDump "$PROD_CONTAINER" > "$HEADER_DUMP"

echo "    Production LUKS2 Container Characteristics:"
grep -E "Version:|cipher:|sector:|PBKDF:|Key:|Memory:|0: luks2|1: luks2" "$HEADER_DUMP"

# Confirm header size
HEADER_SIZE=$(stat -c %s "$HEADER_BACKUP")
echo "    Detached LUKS2 Header Backup Size: $HEADER_SIZE bytes"

# Wipe sensitive ephemeral variables from memory
unset EPHEMERAL_PASSPHRASE EPHEMERAL_RECOVERY

echo "========================================================================"
echo " [Phase 2A.2 - Step 05] Production & Test LUKS2 Preparation Complete"
echo "========================================================================"
