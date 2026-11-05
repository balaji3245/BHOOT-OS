#!/usr/bin/env bash
# BHOOT Phase 2A.2 - Step 06: Assemble Whole Sparse Disk Image with Payloads
# Creates bhoot_disk_assembled.img with GPT partitioning and embeds ESP, SquashFS, LUKS2, and Recovery.

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
SQUASHFS_IMG="$BUILD_REAL/filesystem.squashfs"
PROD_LUKS="$BUILD_REAL/bhoot_luks_prod_48g.img"
RECOVERY_IMG="$BUILD_REAL/bhoot_recovery.img"
HEADER_BACKUP="$BUILD_REAL/bhoot_luks_header.backup"
RECOVERY_STAGING="$BUILD_REAL/recovery_staging"

echo "========================================================================"
echo " [Phase 2A.2 - Step 06] Assembling Whole Disk Image with Payloads"
echo "========================================================================"

# 1. Verify source payload components
for req in "$ESP_IMG" "$SQUASHFS_IMG" "$PROD_LUKS" "$HEADER_BACKUP"; do
    if [ ! -f "$req" ]; then
        echo "  ERROR: Required payload component missing: $req" >&2
        exit 1
    fi
done

# 2. Build Offline Recovery Payload Image (Partition 4)
echo "  [Part 4] Generating offline recovery ext4 image ($RECOVERY_IMG)..."
rm -rf "$RECOVERY_STAGING" "$RECOVERY_IMG"
mkdir -p "$RECOVERY_STAGING"

cp "$HEADER_BACKUP" "$RECOVERY_STAGING/bhoot_luks_header.backup"
cp "$BUILD_REAL/rootfs/etc/bhoot/version.json" "$RECOVERY_STAGING/version.json"

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
TARGET_DEV="${1:-/dev/disk/by-partuuid/B8007001-0003-4000-8000-000000000003}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BACKUP_FILE="$SCRIPT_DIR/bhoot_luks_header.backup"

if [ ! -b "$TARGET_DEV" ]; then
    echo "ERROR: Target LUKS block device $TARGET_DEV not found!" >&2
    exit 1
fi

echo "Restoring detached LUKS2 header to $TARGET_DEV..."
cryptsetup luksHeaderRestore "$TARGET_DEV" --header-backup-file "$BACKUP_FILE" --batch-mode
echo "Header restore completed successfully."
EOF
chmod +x "$RECOVERY_STAGING/restore_header.sh"

truncate -s $(( 32 * 1024 * 1024 )) "$RECOVERY_IMG"
mkfs.ext4 -F -q -L "BHOOT_REC" -d "$RECOVERY_STAGING" "$RECOVERY_IMG"
rm -rf "$RECOVERY_STAGING"
echo "  Offline recovery payload created: $(stat -c %s "$RECOVERY_IMG") bytes"

# 3. Allocate Sparse Disk Image
echo "  Allocating 57.81 GiB sparse whole-disk image at: $ASSEMBLED_DISK..."
rm -f "$ASSEMBLED_DISK"
truncate -s "$TOTAL_BYTES" "$ASSEMBLED_DISK"

# 4. Apply Canonical GPT Partition Table
echo "  Applying GPT partition layout via sfdisk..."
sfdisk --quiet "$ASSEMBLED_DISK" < "$SFDISK_LAYOUT"
sfdisk --verify "$ASSEMBLED_DISK"

# 5. Embed Partition Payloads at Exact Sector Offsets
echo "  Writing Partition 1: EFI System Partition (Offset: 1 MiB / Sector 2048)..."
dd if="$ESP_IMG" of="$ASSEMBLED_DISK" bs=1M seek=1 conv=notrunc status=none

echo "  Writing Partition 2: System Root SquashFS (Offset: 1025 MiB / Sector 2099200)..."
dd if="$SQUASHFS_IMG" of="$ASSEMBLED_DISK" bs=1M seek=1025 conv=notrunc status=none

echo "  Writing Partition 3: Encrypted LUKS2 Header Structure (Offset: 5121 MiB / Sector 10487808)..."
dd if="$PROD_LUKS" of="$ASSEMBLED_DISK" bs=1M seek=5121 count=32 conv=notrunc status=none

echo "  Writing Partition 4: Offline Recovery Payload (Offset: 54273 MiB / Sector 111151104)..."
dd if="$RECOVERY_IMG" of="$ASSEMBLED_DISK" bs=1M seek=54273 conv=notrunc status=none

# 6. Verify Partition Offsets and Magic Signatures inside Assembled Disk Image
echo "  Performing forensic byte inspection on assembled disk image..."

# Partition 1: Check FAT32 magic
P1_MAGIC=$(dd if="$ASSEMBLED_DISK" bs=512 skip=2048 count=1 2>/dev/null | xxd -p -s 510 -l 2)
if [ "$P1_MAGIC" != "55aa" ]; then
    echo "  ERROR: Partition 1 at sector 2048 does not contain valid FAT32 boot sector (magic: $P1_MAGIC)!" >&2
    exit 1
fi
echo "  Verified Partition 1 (ESP) FAT32 boot signature (0x55aa) at sector 2048."

# Partition 2: Check SquashFS magic (0x73717368)
P2_MAGIC=$(dd if="$ASSEMBLED_DISK" bs=512 skip=2099200 count=1 2>/dev/null | xxd -p -s 0 -l 4)
if [ "$P2_MAGIC" != "68737173" ]; then
    echo "  ERROR: Partition 2 at sector 2099200 does not contain SquashFS magic (found: $P2_MAGIC)!" >&2
    exit 1
fi
echo "  Verified Partition 2 (System) SquashFS magic ('hsqs') at sector 2099200."

# Partition 3: Check LUKS2 magic ('LUKS\xba\xbe') -> hex '4c554b53babe'
P3_MAGIC=$(dd if="$ASSEMBLED_DISK" bs=512 skip=10487808 count=1 2>/dev/null | xxd -p -s 0 -l 6)
if [ "$P3_MAGIC" != "4c554b53babe" ]; then
    echo "  ERROR: Partition 3 at sector 10487808 does not contain LUKS2 magic (found: $P3_MAGIC)!" >&2
    exit 1
fi
echo "  Verified Partition 3 (Data) LUKS2 magic (4c554b53babe) at sector 10487808."

# Partition 4: Check non-zero content
P4_SAMPLE=$(dd if="$ASSEMBLED_DISK" bs=512 skip=111151104 count=16 2>/dev/null | tr -d '\0' | wc -c)
if [ "$P4_SAMPLE" -eq 0 ]; then
    echo "  ERROR: Partition 4 at sector 111151104 is completely empty!" >&2
    exit 1
fi
echo "  Verified Partition 4 (Recovery) payload at sector 111151104."

echo "  Assembled disk image characteristics:"
ls -lh "$ASSEMBLED_DISK"
sfdisk -l "$ASSEMBLED_DISK"

echo "========================================================================"
echo " [Phase 2A.2 - Step 06] Whole Disk Image Assembly Complete"
echo "========================================================================"
