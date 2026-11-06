#!/usr/bin/env bash
# BHOOT Phase 2A.6 - External Disaster Recovery Export Workflow
# Safely exports the detached LUKS2 header backup to an external, secondary offline medium.
# Enforces strict safety checks against overwriting /dev/sda or internal SSD storage.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OS_DIR="$(dirname "$SCRIPT_DIR")"
BUILD_REAL="$OS_DIR/build_real"
SOURCE_HEADER="${BHOOT_SOURCE_HEADER:-$BUILD_REAL/bhoot_luks_header.backup}"

DEST_DIR=""
while [ $# -gt 0 ]; do
    case "$1" in
        --source)
            SOURCE_HEADER="$2"
            shift 2
            ;;
        --target)
            DEST_DIR="$2"
            shift 2
            ;;
        *)
            if [ -z "$DEST_DIR" ]; then
                DEST_DIR="$1"
            fi
            shift
            ;;
    esac
done

if [ ! -f "$SOURCE_HEADER" ]; then
    echo "  ERROR: Detached header backup not found at: $SOURCE_HEADER" >&2
    exit 1
fi

echo "========================================================================"
echo " [BHOOT Disaster Recovery Engine] External Header Backup Exporter"
echo "========================================================================"

if [ -z "$DEST_DIR" ]; then
    echo "  This tool exports your detached LUKS2 header backup to a SECONDARY offline medium."
    echo "  (e.g., an external secondary USB drive, offline backup disk, or secure storage path)."
    echo ""
    read -r -p "  Enter destination export directory: " DEST_DIR
fi

# 1. STRICT SAFETY CHECKS
if [ -z "$DEST_DIR" ]; then
    echo "  ERROR: No destination path provided!" >&2
    exit 1
fi

# Normalize path
DEST_DIR="$(readlink -f "$DEST_DIR" 2>/dev/null || echo "$DEST_DIR")"

# Prohibit writing directly to block devices or target USB device nodes
if echo "$DEST_DIR" | grep -E "^/dev/sda" >/dev/null 2>&1; then
    echo "  CRITICAL SAFETY VIOLATION: Export destination cannot be /dev/sda!" >&2
    echo "  The backup must reside on a physically separate medium." >&2
    exit 1
fi

if echo "$DEST_DIR" | grep -E "^/dev/nvme|^/dev/mapper" >/dev/null 2>&1; then
    echo "  CRITICAL SAFETY VIOLATION: Export destination cannot be internal disk /dev/nvme*!" >&2
    exit 1
fi

if [ ! -d "$DEST_DIR" ]; then
    mkdir -p "$DEST_DIR"
fi

if [ ! -w "$DEST_DIR" ]; then
    echo "  ERROR: Destination directory is not writable: $DEST_DIR" >&2
    exit 1
fi

# 2. Confirmation (if interactive)
if [ -t 0 ] && [ "${BHOOT_AUTOMATED_TEST:-0}" != "1" ]; then
    echo ""
    echo "  Destination Target: $DEST_DIR"
    read -r -p "  Confirm export of detached LUKS2 header backup to this destination? [y/N]: " CONFIRM
    if [ "$CONFIRM" != "y" ] && [ "$CONFIRM" != "Y" ]; then
        echo "  Export aborted by user."
        exit 0
    fi
fi

# 3. Compute Source Digest
SOURCE_HASH=$(sha256sum "$SOURCE_HEADER" | awk '{print $1}')
echo "  Source Header SHA-256: $SOURCE_HASH"

# 4. Export files
EXPORTED_HEADER="$DEST_DIR/bhoot_luks_header.backup"
cp "$SOURCE_HEADER" "$EXPORTED_HEADER"
chmod 0400 "$EXPORTED_HEADER"

# Write instructions
cat << EOF > "$DEST_DIR/README_EXTERNAL_RECOVERY.txt"
========================================================================
 BHOOT Detached LUKS2 Header Backup & Emergency Recovery Guide
========================================================================
 This directory contains the detached cryptographic header backup for
 your BHOOT primary financial storage partition (UUID: b8007001-0003-4000-8000-000000000003).

 If your physical USB drive suffers unexpected power loss, partition table
 corruption, or LUKS header damage, restore the header using:

   cryptsetup luksHeaderRestore /dev/disk/by-partuuid/B8007001-0003-4000-8000-000000000003 \\
       --header-backup-file bhoot_luks_header.backup

 Header SHA-256 Hash: $SOURCE_HASH
 Timestamp of Export: $(date -u +"%Y-%m-%dT%H:%M:%SZ")
========================================================================
EOF
chmod 0644 "$DEST_DIR/README_EXTERNAL_RECOVERY.txt"

# 5. Verify Target Digest
TARGET_HASH=$(sha256sum "$EXPORTED_HEADER" | awk '{print $1}')
echo "  Target Header SHA-256: $TARGET_HASH"

if [ "$SOURCE_HASH" != "$TARGET_HASH" ]; then
    echo "  CRITICAL ERROR: Checksum mismatch after export!" >&2
    rm -f "$EXPORTED_HEADER"
    exit 1
fi

echo ""
echo "========================================================================"
echo " [SUCCESS] Detached LUKS2 Header Backup Exported & Verified Successfully"
echo " Location: $EXPORTED_HEADER"
echo " SHA-256:  $TARGET_HASH"
echo "========================================================================"
