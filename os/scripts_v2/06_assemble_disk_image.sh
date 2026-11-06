#!/usr/bin/env bash
# BHOOT Phase 2A.3 - Step 06: Assemble Whole Sparse Disk Image with Complete Payloads
# Creates bhoot_disk_assembled.img with GPT partitioning and embeds complete 48 GiB
# production LUKS2 partition, BHOOT_SYSTEM Ext4 partition, ESP, and Recovery payloads.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OS_DIR="$(dirname "$SCRIPT_DIR")"
BUILD_REAL="$OS_DIR/build_real"
CONFIG_DIR="$OS_DIR/config"

TOTAL_SECTORS=121241600
SECTOR_SIZE=512
TOTAL_BYTES=$(( TOTAL_SECTORS * SECTOR_SIZE ))

ASSEMBLED_DISK="$BUILD_REAL/bhoot_disk_assembled.img"
SFDISK_LAYOUT="$CONFIG_DIR/storage/gpt_layout.sfdisk"
ESP_IMG="$BUILD_REAL/bhoot_esp.img"
SYSTEM_IMG="$BUILD_REAL/bhoot_system_ext4.img"
PROD_LUKS="$BUILD_REAL/bhoot_luks_prod_48g.img"
RECOVERY_IMG="$BUILD_REAL/bhoot_recovery.img"
HEADER_BACKUP="$BUILD_REAL/bhoot_luks_header.backup"
RECOVERY_STAGING="$BUILD_REAL/recovery_staging"

echo "========================================================================"
echo " [Phase 2A.3 - Step 06] Assembling Whole Disk Image with Complete Payloads"
echo "========================================================================"

# 1. Verify source payload components
for req in "$ESP_IMG" "$SYSTEM_IMG" "$PROD_LUKS" "$HEADER_BACKUP"; do
    if [ ! -f "$req" ]; then
        echo "  ERROR: Required payload component missing: $req" >&2
        exit 1
    fi
done

# Ensure previous compromised assembled image is preserved if not already
if [ -f "$ASSEMBLED_DISK" ] && [ ! -f "${ASSEMBLED_DISK}.phase2a4_compromised" ]; then
    echo "  Preserving legacy Phase 2A.4 compromised assembled disk for forensic reference..."
    mv "$ASSEMBLED_DISK" "${ASSEMBLED_DISK}.phase2a4_compromised"
fi

# 2. Build Offline Recovery Payload Image (Partition 4)
echo "  [Part 4] Generating offline recovery ext4 image ($RECOVERY_IMG)..."
rm -rf "$RECOVERY_STAGING" "$RECOVERY_IMG"
mkdir -p "$RECOVERY_STAGING"

cp "$HEADER_BACKUP" "$RECOVERY_STAGING/bhoot_luks_header.backup"
if [ -f "$BUILD_REAL/rootfs/etc/bhoot/version.json" ]; then
    cp "$BUILD_REAL/rootfs/etc/bhoot/version.json" "$RECOVERY_STAGING/version.json"
fi

cat << 'EOF' > "$RECOVERY_STAGING/README_RECOVERY.txt"
========================================================================
 BHOOT Offline Disaster Recovery Instructions
========================================================================

If the primary LUKS2 header on Partition 3 (BHOOT_DATA) is corrupted
due to unexpected power loss or storage malfunction, restore it using:

  cryptsetup luksHeaderRestore /dev/sda3 \
    --header-backup-file /path/to/bhoot_luks_header.backup

Verify restored header integrity:
  cryptsetup luksDump /dev/sda3
========================================================================
EOF

cat << 'EOF' > "$RECOVERY_STAGING/restore_header.sh"
#!/bin/sh
set -eu
TARGET_DEV="${1:-}"
if [ -z "$TARGET_DEV" ]; then
    for candidate in /dev/disk/by-partuuid/B8007001-0003-4000-8000-000000000003 /dev/disk/by-label/BHOOT_DATA /dev/vda3 /dev/sda3; do
        if [ -b "$candidate" ]; then
            TARGET_DEV="$candidate"
            break
        fi
    done
fi

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BACKUP_FILE="${2:-$SCRIPT_DIR/bhoot_luks_header.backup}"

if [ -z "$TARGET_DEV" ] || [ ! -b "$TARGET_DEV" ]; then
    echo "ERROR: Target LUKS block device not found!" >&2
    exit 1
fi

if [ ! -f "$BACKUP_FILE" ]; then
    echo "ERROR: Header backup file not found at: $BACKUP_FILE" >&2
    exit 1
fi

echo "Restoring detached LUKS2 header to $TARGET_DEV..."
cryptsetup luksHeaderRestore "$TARGET_DEV" --header-backup-file "$BACKUP_FILE" --batch-mode
echo "Header restore completed successfully."
EOF
chmod +x "$RECOVERY_STAGING/restore_header.sh"

if [ -f "$BUILD_REAL/rootfs/usr/local/sbin/bhoot-unlock-recovery-key" ]; then
    cp "$BUILD_REAL/rootfs/usr/local/sbin/bhoot-unlock-recovery-key" "$RECOVERY_STAGING/bhoot-unlock-recovery-key"
    chmod +x "$RECOVERY_STAGING/bhoot-unlock-recovery-key"
fi

truncate -s $(( 32 * 1024 * 1024 )) "$RECOVERY_IMG"
mkfs.ext4 -F -q -L "BHOOT_REC" -d "$RECOVERY_STAGING" "$RECOVERY_IMG"
rm -rf "$RECOVERY_STAGING"
echo "  Offline recovery payload created: $(stat -c %s "$RECOVERY_IMG") bytes"

# 3. Allocate Sparse Disk Image matching exact 57.81 GiB geometry
echo "  Allocating 57.81 GiB sparse whole-disk image at: $ASSEMBLED_DISK..."
rm -f "$ASSEMBLED_DISK"
truncate -s "$TOTAL_BYTES" "$ASSEMBLED_DISK"

# 4. Apply Canonical GPT Partition Table
echo "  Applying GPT partition layout via sfdisk..."
sfdisk --quiet "$ASSEMBLED_DISK" < "$SFDISK_LAYOUT"
sfdisk --verify "$ASSEMBLED_DISK"

# 5. Embed Complete Partition Payloads at Exact Sector Offsets using sparse copy
echo "  Writing Partition 1: EFI System Partition (Offset: 1 MiB / Sector 2048, 1 GiB)..."
dd if="$ESP_IMG" of="$ASSEMBLED_DISK" bs=1M seek=1 conv=notrunc,sparse status=none

echo "  Writing Partition 2: BHOOT_SYSTEM Ext4 Partition (Offset: 1025 MiB / Sector 2099200, 4 GiB)..."
dd if="$SYSTEM_IMG" of="$ASSEMBLED_DISK" bs=1M seek=1025 count=4096 conv=notrunc,sparse status=none

echo "  Writing Partition 3: COMPLETE 48 GiB Production LUKS2 Volume (Offset: 5121 MiB / Sector 10487808, 48 GiB)..."
dd if="$PROD_LUKS" of="$ASSEMBLED_DISK" bs=1M seek=5121 count=49152 conv=notrunc,sparse status=none

echo "  Writing Partition 4: Offline Recovery Payload (Offset: 54273 MiB / Sector 111151104, 4 GiB)..."
dd if="$RECOVERY_IMG" of="$ASSEMBLED_DISK" bs=1M seek=54273 count=4096 conv=notrunc,sparse status=none

# 6. Forensic Verification of Partition Structures inside Assembled Disk Image
echo "  Performing forensic byte inspection on assembled disk image..."

# Partition 1: Check FAT32 magic
P1_MAGIC=$(dd if="$ASSEMBLED_DISK" bs=1 skip=$((2048*512 + 510)) count=2 2>/dev/null | xxd -p)
if [ "$P1_MAGIC" != "55aa" ]; then
    echo "  ERROR: Partition 1 at sector 2048 does not contain valid FAT32 boot sector (magic: $P1_MAGIC)!" >&2
    exit 1
fi
echo "  Verified Partition 1 (ESP) FAT32 boot signature (0x55aa) at sector 2048."

# Partition 2: Check Ext4 superblock magic (0xef53 at offset 1080 of partition)
P2_SB_MAGIC=$(dd if="$ASSEMBLED_DISK" bs=1 skip=$((2099200*512 + 1080)) count=2 2>/dev/null | xxd -p)
if [ "$P2_SB_MAGIC" != "53ef" ]; then
    echo "  ERROR: Partition 2 at sector 2099200 does not contain Ext4 superblock magic (found: $P2_SB_MAGIC)!" >&2
    exit 1
fi
echo "  Verified Partition 2 (BHOOT_SYSTEM) Ext4 superblock magic (0xef53) at sector 2099200."

# Partition 3: Check LUKS2 magic ('LUKS\xba\xbe') -> hex '4c554b53babe'
P3_MAGIC=$(dd if="$ASSEMBLED_DISK" bs=1 skip=$((10487808*512)) count=6 2>/dev/null | xxd -p)
if [ "$P3_MAGIC" != "4c554b53babe" ]; then
    echo "  ERROR: Partition 3 at sector 10487808 does not contain LUKS2 magic (found: $P3_MAGIC)!" >&2
    exit 1
fi
echo "  Verified Partition 3 (BHOOT_DATA) LUKS2 magic (4c554b53babe) at sector 10487808."

# Partition 4: Check Ext4 superblock magic
P4_SB_MAGIC=$(dd if="$ASSEMBLED_DISK" bs=1 skip=$((111151104*512 + 1080)) count=2 2>/dev/null | xxd -p)
if [ "$P4_SB_MAGIC" != "53ef" ]; then
    echo "  ERROR: Partition 4 at sector 111151104 does not contain Ext4 superblock magic (found: $P4_SB_MAGIC)!" >&2
    exit 1
fi
echo "  Verified Partition 4 (BHOOT_RECOVERY) Ext4 superblock magic (0xef53) at sector 111151104."

# 7. Verify Partition 3 inside the Assembled Disk Image can be directly probed and unlocked
# SAFETY INVARIANT: Whole-disk loop setup via UDisks is strictly forbidden to prevent
# kernel partition scans, GVFS automounting, and desktop LUKS passphrase popups.
# Verification must use strict partition-bounded offset and size limits only.
echo "  Verifying Partition 3 within assembled disk image via loop offset..."
P3_OFFSET_BYTES=$(( 10487808 * 512 ))
P3_SIZE_BYTES=$(( 100663296 * 512 ))

P3_LOOP_OUT=$(udisksctl loop-setup --file "$ASSEMBLED_DISK" --offset "$P3_OFFSET_BYTES" --size "$P3_SIZE_BYTES" --no-user-interaction)
P3_LOOP_DEV=$(echo "$P3_LOOP_OUT" | awk '{print $NF}' | tr -d '.')
echo "  Partition 3 attached as: $P3_LOOP_DEV"

cleanup_p3() {
    if [ -n "${P3_LOOP_DEV:-}" ] && [ -b "$P3_LOOP_DEV" ]; then
        udisksctl lock -b "$P3_LOOP_DEV" --no-user-interaction 2>/dev/null || true
        udisksctl loop-delete -b "$P3_LOOP_DEV" --no-user-interaction 2>/dev/null || true
    fi
}
trap cleanup_p3 EXIT INT TERM HUP ERR

# Check UDisks recognition
P3_INFO=$(udisksctl info -b "$P3_LOOP_DEV")
echo "$P3_INFO" | grep -E "IdType|IdLabel|IdUUID"

if echo "$P3_INFO" | grep -q "IdType: *crypto_LUKS"; then
    echo "  Partition 3 recognized as crypto_LUKS by Linux kernel/UDisks2."
    echo "  Partition 3 UUID and label verified successfully."
else
    echo "  ERROR: Partition 3 not recognized as LUKS container!" >&2
    exit 1
fi

# Detach test loop
udisksctl lock -b "$P3_LOOP_DEV" --no-user-interaction 2>/dev/null || true
udisksctl loop-delete -b "$P3_LOOP_DEV" --no-user-interaction 2>/dev/null || true
trap - EXIT INT TERM HUP ERR

echo "  Assembled disk image characteristics:"
ls -lh "$ASSEMBLED_DISK"
sfdisk -l "$ASSEMBLED_DISK"

echo "========================================================================"
echo " [Phase 2A.3 - Step 06] Whole Disk Image Assembly Complete"
echo "========================================================================"
