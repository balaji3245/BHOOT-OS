#!/usr/bin/env bash
# BHOOT Phase 2A.6 - Step 05: Secure LUKS2 Storage Provisioning with Ext4 Persistence
# Provisions 48 GiB production volume with Argon2id + AES-256-XTS, independent recovery key,
# Ext4 persistence, and detached header backup without hardcoded credentials.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OS_DIR="$(dirname "$SCRIPT_DIR")"
BUILD_REAL="$OS_DIR/build_real"

TEST_CONTAINER="$BUILD_REAL/bhoot_luks_test_64m.img"
PROD_CONTAINER="$BUILD_REAL/bhoot_luks_prod_48g.img"
HEADER_BACKUP="$BUILD_REAL/bhoot_luks_header.backup"
HEADER_DUMP="$BUILD_REAL/bhoot_luks_header_dump.txt"
PROVISIONER_PY="$SCRIPT_DIR/provision_luks2_volume.py"

MODE_FLAG="${1:---test-mode}"

echo "========================================================================"
echo " [Phase 2A.6 - Step 05] Preparing Secure LUKS2 Storage & Ext4 Filesystem"
echo "========================================================================"

mkdir -p "$BUILD_REAL"

# Ensure legacy compromised artifacts are not accidentally overwritten
if [ -f "$PROD_CONTAINER" ] && [ ! -f "${PROD_CONTAINER}.phase2a4_compromised" ]; then
    echo "  Preserving legacy Phase 2A.4 compromised image for forensic reference..."
    mv "$PROD_CONTAINER" "${PROD_CONTAINER}.phase2a4_compromised"
fi

# Ensure no temporary scratch files exist
rm -f "$BUILD_REAL"/*.tmp "$BUILD_REAL"/.*.tmp 2>/dev/null || true

# -----------------------------------------------------------------------------
# Part A: Functional Verification on Disposable 64 MiB Test Vector
# -----------------------------------------------------------------------------
echo "  [Part A] Verifying LUKS2 Cryptographic Engine with 64 MiB Test Vector..."
python3 "$PROVISIONER_PY" \
    --container "$TEST_CONTAINER" \
    --size-gib 1 \
    --header-backup "$BUILD_REAL/test_header.backup" \
    --test-mode

echo "    Test vector provisioned and verified successfully."
rm -f "$BUILD_REAL/test_header.backup"

# -----------------------------------------------------------------------------
# Part B: Production 48 GiB Storage Volume Provisioning
# -----------------------------------------------------------------------------
echo "  [Part B] Provisioning 48 GiB Production Storage Volume..."
if [ "$MODE_FLAG" = "--interactive" ]; then
    echo "    Interactive owner provisioning requested."
    python3 "$PROVISIONER_PY" \
        --container "$PROD_CONTAINER" \
        --size-gib 48 \
        --header-backup "$HEADER_BACKUP"
else
    echo "    Automated test verification provisioning active (ephemeral non-hardcoded keys)."
    python3 "$PROVISIONER_PY" \
        --container "$PROD_CONTAINER" \
        --size-gib 48 \
        --header-backup "$HEADER_BACKUP" \
        --test-mode
fi

# Dump sanitized header metadata for verification suite
cryptsetup luksDump "$PROD_CONTAINER" > "$HEADER_DUMP"

echo "  Production LUKS2 Container Characteristics:"
grep -E "Version:|cipher:|sector:|PBKDF:|Key:|Memory:|0: luks2|1: luks2" "$HEADER_DUMP"

HEADER_SIZE=$(stat -c %s "$HEADER_BACKUP")
echo "  Detached LUKS2 Header Backup Size: $HEADER_SIZE bytes"

echo "========================================================================"
echo " [Phase 2A.6 - Step 05] Production LUKS2 & Ext4 Persistence Ready"
echo "========================================================================"
