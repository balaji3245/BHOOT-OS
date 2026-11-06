#!/usr/bin/env bash
# Project BHOOT — Safe Physical USB Deployment Utility
# GATED BY MANDATORY OWNER APPROVAL. Never run against unintended block devices.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OS_DIR="$(dirname "$SCRIPT_DIR")"
REPO_ROOT="$(dirname "$OS_DIR")"
BUILD_REAL="$OS_DIR/build_real"
ASSEMBLED_DISK="$BUILD_REAL/bhoot_disk_assembled.img"
REQUIRED_PARTITION_END_BYTES=61204332544 # Sector 119539712 * 512 (Partitions 1..4)
PHYSICAL_USB_EXACT_BYTES=62026416128      # Exact physical SanDisk/HP 64GB drive geometry

echo "========================================================================"
echo "          PROJECT BHOOT — PHYSICAL USB DEPLOYMENT UTILITY"
echo "========================================================================"

TARGET_DEV="${1:-}"
APPROVED_FLAG="${2:-}"

if [ -z "$TARGET_DEV" ]; then
    echo "Usage: $0 <target_block_device> [--force-owner-approved]"
    echo ""
    echo "Examples:"
    echo "  $0 /dev/sdX                    (Dry run / inspection mode)"
    echo "  $0 /dev/sdX --force-owner-approved  (Requires explicit confirmation)"
    exit 1
fi

if [ ! -b "$TARGET_DEV" ]; then
    echo "[FATAL ERROR] Target '$TARGET_DEV' is not a valid block device node!" >&2
    exit 1
fi

# 1. Strict Target Disambiguation & Sanity Checks
DEV_NAME=$(basename "$TARGET_DEV")

# Reject NVMe drives
if [[ "$DEV_NAME" =~ ^nvme ]]; then
    echo "[FATAL ERROR] Refusing to flash NVMe storage device ($TARGET_DEV)!" >&2
    exit 1
fi

# Reject loop devices
if [[ "$DEV_NAME" =~ ^loop ]]; then
    echo "[FATAL ERROR] Refusing to flash loop device node via physical tool ($TARGET_DEV)!" >&2
    exit 1
fi

# Reject current host root and boot devices
ROOT_SRC=$(findmnt -n -o SOURCE / || true)
BOOT_SRC=$(findmnt -n -o SOURCE /boot/efi 2>/dev/null || true)
if [[ "$ROOT_SRC" == *"$DEV_NAME"* ]] || [[ "$BOOT_SRC" == *"$DEV_NAME"* ]]; then
    echo "[FATAL ERROR] Refusing to flash host root or boot device ($TARGET_DEV)!" >&2
    exit 1
fi

# Check removable attribute via sysfs
SYS_REMOVABLE="/sys/block/$DEV_NAME/removable"
if [ -f "$SYS_REMOVABLE" ]; then
    REMOVABLE_VAL=$(cat "$SYS_REMOVABLE")
    if [ "$REMOVABLE_VAL" != "1" ]; then
        echo "[SECURITY ALERT] Device $TARGET_DEV is not marked as removable storage!" >&2
        echo "Aborting for safety." >&2
        exit 1
    fi
fi

# Verify device capacity
DEV_SIZE_BYTES=$(blockdev --getsize64 "$TARGET_DEV" 2>/dev/null || lsblk -b -n -o SIZE "$TARGET_DEV" | head -n1)
echo "Target Device: $TARGET_DEV"
echo "Target Capacity: $DEV_SIZE_BYTES bytes (~$(( DEV_SIZE_BYTES / 1024 / 1024 / 1024 )) GiB)"
echo "Required Partition Payload: $REQUIRED_PARTITION_END_BYTES bytes (~57.0 GiB)"

if [ "$DEV_SIZE_BYTES" -lt "$REQUIRED_PARTITION_END_BYTES" ]; then
    echo "[FATAL ERROR] Target device is smaller than required 57.0 GiB partition payload!" >&2
    exit 1
fi

if [ ! -f "$ASSEMBLED_DISK" ]; then
    echo "[FATAL ERROR] Assembled disk image missing: $ASSEMBLED_DISK" >&2
    echo "Run assembly pipeline first." >&2
    exit 1
fi

# 2. Authorization Gate
if [ "$APPROVED_FLAG" != "--force-owner-approved" ]; then
    echo ""
    echo "========================================================================"
    echo " [SAFETY GATE] DRY RUN INSPECTION COMPLETE."
    echo " To write to physical media, explicit owner authorization is required."
    echo " Command: $0 $TARGET_DEV --force-owner-approved"
    echo "========================================================================"
    exit 0
fi

# Interactive Double Confirmation
echo ""
echo "!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!"
echo " [DANGER] ALL EXISTING DATA ON $TARGET_DEV WILL BE PERMANENTLY ERASED!"
echo "!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!"
echo "Type the exact block device name '$DEV_NAME' to confirm destruction:"
read -r CONFIRM_DEV
if [ "$CONFIRM_DEV" != "$DEV_NAME" ]; then
    echo "Confirmation mismatch. Aborting physical write."
    exit 1
fi

# Unmount any active partitions on the target drive
echo "Unmounting any existing partitions on $TARGET_DEV..."
for part in "${TARGET_DEV}"* "${TARGET_DEV}"p*; do
    if [ -b "$part" ]; then
        umount "$part" 2>/dev/null || true
    fi
done

# 3. Stream Assembled Image to Media
echo "Writing $ASSEMBLED_DISK to $TARGET_DEV (this may take several minutes)..."
dd if="$ASSEMBLED_DISK" of="$TARGET_DEV" bs=4M conv=fsync status=progress

echo "Syncing kernel block buffers..."
sync

# Reread partition table
partprobe "$TARGET_DEV" 2>/dev/null || blockdev --rereadpt "$TARGET_DEV" 2>/dev/null || true

# 4. Forensic Post-Write Verification
echo "Verifying Partition 1 FAT32 signature..."
P1_MAGIC=$(dd if="$TARGET_DEV" bs=1 skip=$((2048*512 + 510)) count=2 2>/dev/null | xxd -p)
if [ "$P1_MAGIC" != "55aa" ]; then
    echo "[ERROR] Partition 1 boot signature mismatch on physical drive!" >&2
    exit 1
fi

echo "Verifying Partition 3 LUKS2 container signature..."
P3_MAGIC=$(dd if="$TARGET_DEV" bs=1 skip=$((10487808*512)) count=6 2>/dev/null | xxd -p)
if [ "$P3_MAGIC" != "4c554b53babe" ]; then
    echo "[ERROR] Partition 3 LUKS2 signature mismatch on physical drive!" >&2
    exit 1
fi

echo "========================================================================"
echo " [SUCCESS] BHOOT OS successfully deployed to physical device $TARGET_DEV."
echo " Device is ready for UEFI boot testing on supported hardware."
echo "========================================================================"
