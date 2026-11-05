#!/usr/bin/env bash
# BHOOT Phase 2A - Step 01: Prepare Sparse Disk Image and GPT Partition Table
# Strictly non-destructive: operates ONLY on sparse image file on internal SSD.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OS_DIR="$(dirname "$SCRIPT_DIR")"
BUILD_DIR="$OS_DIR/build"
CONFIG_DIR="$OS_DIR/config"

IMAGE_PATH="$BUILD_DIR/bhoot_disk_sparse.img"
SFDISK_LAYOUT="$CONFIG_DIR/storage/gpt_layout.sfdisk"

echo "========================================================================"
echo " [Step 01] Preparing Sparse Disk Image & GPT Partition Layout"
echo "========================================================================"

mkdir -p "$BUILD_DIR"

# Total sectors matching 57.8 GiB USB: 121,241,600 sectors * 512 bytes = 62,075,699,200 bytes
TOTAL_SECTORS=121241600
SECTOR_SIZE=512
TOTAL_BYTES=$(( TOTAL_SECTORS * SECTOR_SIZE ))

echo "  Target disk geometry: $TOTAL_SECTORS sectors ($TOTAL_BYTES bytes / ~57.8 GiB)"
echo "  Creating sparse disk image at: $IMAGE_PATH"

# Allocate sparse image file (consumes minimal actual host disk space)
rm -f "$IMAGE_PATH"
truncate -s "$TOTAL_BYTES" "$IMAGE_PATH"

echo "  Applying canonical GPT partition layout via sfdisk..."
sfdisk --quiet "$IMAGE_PATH" < "$SFDISK_LAYOUT"

echo "  Verifying GPT partition table structure..."
sfdisk --verify "$IMAGE_PATH"
sfdisk -d "$IMAGE_PATH" > "$BUILD_DIR/applied_gpt_layout.dump"

echo "  GPT Partition layout summary:"
sfdisk -l "$IMAGE_PATH"

echo "========================================================================"
echo " [Step 01] Successfully Prepared Sparse Disk Image"
echo "========================================================================"

