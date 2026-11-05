#!/usr/bin/env bash
# BHOOT Phase 2A - Step 02: Build EFI System Partition (ESP) and Standalone Bootloader
# Operates non-destructively on local image files using udisksctl userspace mount.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OS_DIR="$(dirname "$SCRIPT_DIR")"
BUILD_DIR="$OS_DIR/build"
CONFIG_DIR="$OS_DIR/config"

ESP_IMG="$BUILD_DIR/bhoot_esp.img"
BOOTX64_EFI="$BUILD_DIR/BOOTX64.EFI"
EARLY_CFG="$CONFIG_DIR/boot/grub-early.cfg"
GRUB_CFG="$CONFIG_DIR/boot/grub.cfg"

echo "========================================================================"
echo " [Step 02] Building EFI System Partition (ESP) & Standalone UEFI GRUB"
echo "========================================================================"

mkdir -p "$BUILD_DIR"

# 1. Allocate 1024 MiB (1 GiB) sparse ESP image
ESP_SIZE_BYTES=$(( 1024 * 1024 * 1024 ))
echo "  Creating 1 GiB sparse ESP image at: $ESP_IMG"
rm -f "$ESP_IMG"
truncate -s "$ESP_SIZE_BYTES" "$ESP_IMG"

# 2. Format as FAT32 with label BHOOT_EFI
echo "  Formatting FAT32 filesystem with label BHOOT_EFI..."
mkfs.vfat -F 32 -n "BHOOT_EFI" -i B8000001 "$ESP_IMG"

# 3. Generate standalone GRUB EFI binary (BOOTX64.EFI)
echo "  Compiling standalone UEFI GRUB binary (BOOTX64.EFI) with embedded early config..."
grub-mkstandalone \
    -O x86_64-efi \
    -o "$BOOTX64_EFI" \
    --modules="part_gpt part_msdos fat ext2 normal linux echo search search_fs_uuid search_label test all_video gfxterm font" \
    "boot/grub/grub.cfg=$EARLY_CFG"

echo "  Standalone UEFI binary generated: $(stat -c %s "$BOOTX64_EFI") bytes"

# 4. Mount ESP image via udisksctl in userspace
echo "  Attaching loop device for ESP image..."
LOOP_OUT=$(udisksctl loop-setup --file "$ESP_IMG" --no-user-interaction)
LOOP_DEV=$(echo "$LOOP_OUT" | awk '{print $NF}' | tr -d '.')
echo "  Attached as: $LOOP_DEV"

# Ensure cleanup on exit or error
cleanup() {
    echo "  Cleaning up ESP loop mount..."
    if [ -n "${MOUNT_POINT:-}" ] && [ -d "$MOUNT_POINT" ]; then
        udisksctl unmount -b "$LOOP_DEV" --no-user-interaction 2>/dev/null || true
    fi
    if [ -n "${LOOP_DEV:-}" ]; then
        udisksctl loop-delete -b "$LOOP_DEV" --no-user-interaction 2>/dev/null || true
    fi
}
trap cleanup EXIT

MOUNT_OUT=$(udisksctl mount -b "$LOOP_DEV" --no-user-interaction)
MOUNT_POINT=$(echo "$MOUNT_OUT" | awk '{print $NF}' | tr -d '.')
echo "  Mounted at: $MOUNT_POINT"

# 5. Populate ESP directory hierarchy
echo "  Staging EFI and boot structures..."
mkdir -p "$MOUNT_POINT/EFI/BOOT"
mkdir -p "$MOUNT_POINT/boot/grub"
mkdir -p "$MOUNT_POINT/live"

cp "$BOOTX64_EFI" "$MOUNT_POINT/EFI/BOOT/BOOTX64.EFI"
cp "$GRUB_CFG" "$MOUNT_POINT/EFI/BOOT/grub.cfg"
cp "$GRUB_CFG" "$MOUNT_POINT/boot/grub/grub.cfg"

# Stage kernel manifest and mock offline bootloader verification payload
cat << 'EOF' > "$MOUNT_POINT/live/kernel-manifest.txt"
BHOOT OS Target Kernel: Linux 6.8.x Debian Bookworm x86_64
Initramfs: Dracut / Debian live-boot with systemd-cryptsetup
Persistence Mode: LUKS2 encrypted partition (BHOOT_DATA)
Offline Security: Telemetry masked, rfkill blocked, IPv6 disabled
EOF

# Stage empty placeholder signatures to test boot structure integrity
truncate -s 64M "$MOUNT_POINT/live/vmlinuz"
truncate -s 64M "$MOUNT_POINT/live/initrd.img"

echo "  ESP directory tree populated successfully:"
ls -lhR "$MOUNT_POINT"

# Unmount cleanly
udisksctl unmount -b "$LOOP_DEV" --no-user-interaction
udisksctl loop-delete -b "$LOOP_DEV" --no-user-interaction
trap - EXIT

echo "========================================================================"
echo " [Step 02] Successfully Built EFI System Partition"
echo "========================================================================"

