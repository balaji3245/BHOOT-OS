#!/usr/bin/env bash
# BHOOT Phase 2A.3 - Step 03B: Build Ext4 System Partition Image (BHOOT_SYSTEM)
# Formats Partition 2 as a 4 GiB Ext4 filesystem embedding /live/filesystem.squashfs.
# This satisfies Debian live-boot's native filesystem discovery requirements.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OS_DIR="$(dirname "$SCRIPT_DIR")"
BUILD_REAL="$OS_DIR/build_real"

SQUASHFS_IN="$BUILD_REAL/filesystem.squashfs"
SYSTEM_IMG="$BUILD_REAL/bhoot_system_ext4.img"
STAGING_DIR="$BUILD_REAL/system_ext4_staging"

echo "========================================================================"
echo " [Phase 2A.3 - Step 03B] Building BHOOT_SYSTEM Ext4 Partition Image"
echo "========================================================================"

if [ ! -f "$SQUASHFS_IN" ]; then
    echo "  ERROR: SquashFS image $SQUASHFS_IN not found! Run Step 03 first." >&2
    exit 1
fi

# Partition 2 geometry: 8,388,608 sectors * 512 bytes = 4,294,967,296 bytes (4 GiB)
PART2_SIZE_BYTES=4294967296
rm -rf "$STAGING_DIR" "$SYSTEM_IMG"
mkdir -p "$STAGING_DIR/live" "$STAGING_DIR/etc/bhoot"

echo "  Staging live system files for Partition 2..."
cp "$SQUASHFS_IN" "$STAGING_DIR/live/filesystem.squashfs"
if [ -f "$BUILD_REAL/rootfs/etc/bhoot/version.json" ]; then
    cp "$BUILD_REAL/rootfs/etc/bhoot/version.json" "$STAGING_DIR/etc/bhoot/version.json"
fi

cat << 'EOF' > "$STAGING_DIR/README_SYSTEM.txt"
========================================================================
 BHOOT System Partition (BHOOT_SYSTEM)
========================================================================
UUID: b8007001-0002-4000-8000-000000000002
Role: Read-Only System Root Image Container
Payload: /live/filesystem.squashfs (Debian 12 Bookworm Minimal)
========================================================================
EOF

echo "  Allocating 4 GiB sparse Ext4 image at: $SYSTEM_IMG..."
truncate -s "$PART2_SIZE_BYTES" "$SYSTEM_IMG"

echo "  Formatting Ext4 filesystem with label BHOOT_SYSTEM and canonical UUID..."
mkfs.ext4 -F -q \
    -L "BHOOT_SYSTEM" \
    -U "b8007001-0002-4000-8000-000000000002" \
    -d "$STAGING_DIR" \
    "$SYSTEM_IMG"

rm -rf "$STAGING_DIR"

SYSTEM_SIZE=$(stat -c %s "$SYSTEM_IMG")
echo "  BHOOT_SYSTEM image created successfully: $SYSTEM_SIZE bytes (sparse)"

echo "========================================================================"
echo " [Phase 2A.3 - Step 03B] BHOOT_SYSTEM Ext4 Partition Image Complete"
echo "========================================================================"
