#!/usr/bin/env bash
# BHOOT Phase 2A.3 - Step 05: Prepare Encrypted LUKS2 Storage with Ext4 Persistence
# Formats production 48 GiB volume with Argon2id + AES-XTS, provisions Ext4 filesystem,
# verifies persistence across mounts, extracts detached header, and tests disaster recovery.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OS_DIR="$(dirname "$SCRIPT_DIR")"
BUILD_REAL="$OS_DIR/build_real"

TEST_CONTAINER="$BUILD_REAL/bhoot_luks_test_64m.img"
PROD_CONTAINER="$BUILD_REAL/bhoot_luks_prod_48g.img"
HEADER_BACKUP="$BUILD_REAL/bhoot_luks_header.backup"
HEADER_DUMP="$BUILD_REAL/bhoot_luks_header_dump.txt"
MANAGER_PY="$SCRIPT_DIR/luks_ext4_manager.py"

echo "========================================================================"
echo " [Phase 2A.3 - Step 05] Preparing Production LUKS2 Storage & Ext4 Filesystem"
echo "========================================================================"

mkdir -p "$BUILD_REAL"

# Generate ephemeral authentication tokens or use provided test tokens
EPHEMERAL_PASSPHRASE="${BHOOT_TEST_PASSPHRASE:-BhootTestPassphrase2026!}"
EPHEMERAL_RECOVERY="${BHOOT_TEST_RECOVERY:-BhootRecoveryKey2026!}"

# Save build session key for inter-step partition unlock verification
printf '%s' "$EPHEMERAL_PASSPHRASE" > "$BUILD_REAL/.session_key.tmp"
chmod 0600 "$BUILD_REAL/.session_key.tmp"

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
    --uuid "b8007001-0003-4000-8000-000000000003" \
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

# -----------------------------------------------------------------------------
# Part C: Provision Ext4 Filesystem & Verify Persistence Across Mounts
# -----------------------------------------------------------------------------
echo "  [Part C] Formatting Cleartext Ext4 Filesystem and Verifying Persistence..."
printf '%s' "$EPHEMERAL_PASSPHRASE" | python3 "$MANAGER_PY" format_and_populate "$PROD_CONTAINER"

# -----------------------------------------------------------------------------
# Part D: Disaster Recovery Simulation: Detached Header Restore Validation
# -----------------------------------------------------------------------------
echo "  [Part D] Executing Disaster Recovery Header Restoration Test..."
TEST_BACKUP="$BUILD_REAL/test_header.backup"
cryptsetup luksHeaderBackup --batch-mode "$TEST_CONTAINER" --header-backup-file "$TEST_BACKUP"
printf '%s' "$EPHEMERAL_PASSPHRASE" | python3 "$MANAGER_PY" test_disaster_recovery "$TEST_CONTAINER" "$TEST_BACKUP"
rm -f "$TEST_BACKUP"

# Wipe sensitive ephemeral variables from memory
unset EPHEMERAL_PASSPHRASE EPHEMERAL_RECOVERY

echo "========================================================================"
echo " [Phase 2A.3 - Step 05] Production LUKS2 & Ext4 Persistence Ready"
echo "========================================================================"
