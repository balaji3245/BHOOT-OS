#!/usr/bin/env bash
# BHOOT Phase 2A.2 - Step 03: Build Genuine Read-Only Debian 12 SquashFS Image
# Compresses the fully populated and hardened Debian 12 rootfs into filesystem.squashfs using XZ.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OS_DIR="$(dirname "$SCRIPT_DIR")"
BUILD_REAL="$OS_DIR/build_real"
ROOTFS="$BUILD_REAL/rootfs"
SQUASHFS_OUT="$BUILD_REAL/filesystem.squashfs"

echo "========================================================================"
echo " [Phase 2A.2 - Step 03] Building Genuine Debian 12 SquashFS Image"
echo "========================================================================"

if [ ! -d "$ROOTFS/bin" ] || [ ! -f "$ROOTFS/bin/bash" ]; then
    echo "  ERROR: Rootfs does not contain genuine system binaries!" >&2
    exit 1
fi

rm -f "$SQUASHFS_OUT"

echo "  Compressing $ROOTFS into $SQUASHFS_OUT with XZ..."
echo "  Options: -comp xz -noappend -reproducible -all-root"

mksquashfs "$ROOTFS" "$SQUASHFS_OUT" \
    -comp xz \
    -b 1048576 \
    -noappend \
    -reproducible \
    -all-root \
    -wildcards \
    -e 'var/cache/apt/archives/*.deb' \
    -e 'proc/*' \
    -e 'sys/*' \
    -e 'dev/*' \
    -e 'tmp/*'

SQUASHFS_SIZE=$(stat -c %s "$SQUASHFS_OUT")
SQUASHFS_SIZE_MB=$(( SQUASHFS_SIZE / 1024 / 1024 ))

echo "  SquashFS image built successfully: $SQUASHFS_SIZE bytes (~$SQUASHFS_SIZE_MB MiB)"

# Verify SquashFS file integrity and magic
echo "  Verifying SquashFS file magic..."
file "$SQUASHFS_OUT"

if [ "$SQUASHFS_SIZE" -lt 50000000 ]; then
    echo "  ERROR: SquashFS file is smaller than 50 MiB ($SQUASHFS_SIZE bytes)! Expected genuine OS image." >&2
    exit 1
fi

echo "  Inspecting SquashFS archive root entries..."
unsquashfs -l "$SQUASHFS_OUT" | grep -E "sbin/init|bin/bash|sbin/cryptsetup|usr/sbin/nft|usr/sbin/rfkill" | head -n 10

echo "========================================================================"
echo " [Phase 2A.2 - Step 03] Genuine Debian 12 SquashFS Build Complete"
echo "========================================================================"
