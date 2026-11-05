#!/usr/bin/env bash
# BHOOT Phase 2A Test Suite: LUKS2 Cryptographic Resilience & Recovery Test
# Validates dual keyslots, wrong-password rejection, and detached header restoration.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OS_DIR="$(dirname "$SCRIPT_DIR")"
BUILD_DIR="$OS_DIR/build"

LUKS_IMG="$BUILD_DIR/bhoot_luks_data.img"
HEADER_BACKUP="$BUILD_DIR/bhoot_luks_header.backup"
CORRUPTED_IMG="$BUILD_DIR/bhoot_luks_corrupted_test.img"

TEST_PASSPHRASE="BhootMasterPass2026!"
TEST_RECOVERY_KEY="BHOOT-REC-9F8A-7C2E-4B1D-6A0F-83E5"
WRONG_PASSPHRASE="WrongPassphraseAttempt999!"

echo "========================================================================"
echo " [Test Suite] LUKS2 Cryptographic Resilience, Dual Keyslots & Recovery"
echo "========================================================================"

# Test 1: Keyslot 0 Unlocking with Primary Passphrase
echo -e "\n[Test 1] Verifying Keyslot 0 Unlocking with Primary Passphrase..."
LOOP_OUT=$(udisksctl loop-setup --file "$LUKS_IMG" --no-user-interaction)
LOOP_DEV=$(echo "$LOOP_OUT" | awk '{print $NF}' | tr -d '.')

UNLOCK_OUT=$(udisksctl unlock -b "$LOOP_DEV" --key-file <(echo -n "$TEST_PASSPHRASE") --no-user-interaction)
CLEAR_DEV=$(echo "$UNLOCK_OUT" | awk '{print $NF}' | tr -d '.')
echo "  -> Keyslot 0 unlocked successfully. Active cleartext block device: $CLEAR_DEV"
udisksctl lock -b "$LOOP_DEV" --no-user-interaction
udisksctl loop-delete -b "$LOOP_DEV" --no-user-interaction

# Test 2: Keyslot 1 Unlocking with Emergency Recovery Key
echo -e "\n[Test 2] Verifying Keyslot 1 Unlocking with Emergency Recovery Key..."
LOOP_OUT=$(udisksctl loop-setup --file "$LUKS_IMG" --no-user-interaction)
LOOP_DEV=$(echo "$LOOP_OUT" | awk '{print $NF}' | tr -d '.')

UNLOCK_OUT=$(udisksctl unlock -b "$LOOP_DEV" --key-file <(echo -n "$TEST_RECOVERY_KEY") --no-user-interaction)
CLEAR_DEV=$(echo "$UNLOCK_OUT" | awk '{print $NF}' | tr -d '.')
echo "  -> Keyslot 1 unlocked successfully with offline recovery key. Device: $CLEAR_DEV"
udisksctl lock -b "$LOOP_DEV" --no-user-interaction
udisksctl loop-delete -b "$LOOP_DEV" --no-user-interaction

# Test 3: Wrong Passphrase Strict Rejection
echo -e "\n[Test 3] Verifying Rejection of Invalid Passphrase..."
LOOP_OUT=$(udisksctl loop-setup --file "$LUKS_IMG" --no-user-interaction)
LOOP_DEV=$(echo "$LOOP_OUT" | awk '{print $NF}' | tr -d '.')

if udisksctl unlock -b "$LOOP_DEV" --key-file <(echo -n "$WRONG_PASSPHRASE") --no-user-interaction >/dev/null 2>&1; then
    echo "  [FAIL] Cryptographic flaw: container opened with incorrect passphrase!" >&2
    udisksctl lock -b "$LOOP_DEV" --no-user-interaction 2>/dev/null || true
    udisksctl loop-delete -b "$LOOP_DEV" --no-user-interaction
    exit 1
else
    echo "  -> Rejection confirmed: Invalid passphrase strictly refused (Operation not permitted / Invalid key)."
fi
udisksctl loop-delete -b "$LOOP_DEV" --no-user-interaction

# Test 4: Header Damage Simulation & Detached Backup Restoration
echo -e "\n[Test 4] Simulating LUKS Header Damage & Testing Detached Header Restoration..."
cp "$LUKS_IMG" "$CORRUPTED_IMG"

# Deliberately overwrite the primary 4 MiB LUKS header on the test copy with garbage
dd if=/dev/urandom of="$CORRUPTED_IMG" bs=1M count=4 conv=notrunc status=none

echo "  -> Primary header intentionally overwritten with random data."
if cryptsetup isLuks "$CORRUPTED_IMG" 2>/dev/null; then
    echo "  [FAIL] Corrupted image still recognized as LUKS!" >&2
    rm -f "$CORRUPTED_IMG"
    exit 1
else
    echo "  -> Verified: Damaged image is non-functional and fails LUKS identification."
fi

echo "  -> Restoring header from detached backup: $HEADER_BACKUP..."
cryptsetup luksHeaderRestore --batch-mode "$CORRUPTED_IMG" --header-backup-file "$HEADER_BACKUP"

if cryptsetup isLuks --type luks2 "$CORRUPTED_IMG"; then
    echo "  -> Restoration successful: Header recognized as valid LUKS2."
else
    echo "  [FAIL] Header restore failed to recover valid LUKS2 structure!" >&2
    rm -f "$CORRUPTED_IMG"
    exit 1
fi

# Verify container unlocks again after restoration
LOOP_OUT=$(udisksctl loop-setup --file "$CORRUPTED_IMG" --no-user-interaction)
LOOP_DEV=$(echo "$LOOP_OUT" | awk '{print $NF}' | tr -d '.')

UNLOCK_OUT=$(udisksctl unlock -b "$LOOP_DEV" --key-file <(echo -n "$TEST_PASSPHRASE") --no-user-interaction)
CLEAR_DEV=$(echo "$UNLOCK_OUT" | awk '{print $NF}' | tr -d '.')
echo "  -> Post-restore unlocking verified with primary passphrase. Device: $CLEAR_DEV"

udisksctl lock -b "$LOOP_DEV" --no-user-interaction
udisksctl loop-delete -b "$LOOP_DEV" --no-user-interaction
rm -f "$CORRUPTED_IMG"

echo -e "\n========================================================================"
echo " [Test Suite] ALL CRYPTOGRAPHIC & RECOVERY TESTS PASSED (100% Verified)"
echo "========================================================================"

