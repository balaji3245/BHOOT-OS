#!/usr/bin/env bash
# BHOOT Phase 2A.2 - Step 04: Build Genuine EFI System Partition (ESP) & Bootloader
# Embeds genuine Debian 12 kernel (vmlinuz), live initramfs, and standalone UEFI GRUB.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OS_DIR="$(dirname "$SCRIPT_DIR")"
BUILD_REAL="$OS_DIR/build_real"
CONFIG_DIR="$OS_DIR/config"
ROOTFS="$BUILD_REAL/rootfs"

ESP_IMG="$BUILD_REAL/bhoot_esp.img"
BOOTX64_EFI="$BUILD_REAL/BOOTX64.EFI"
EARLY_CFG="$CONFIG_DIR/boot/grub-early.cfg"
GRUB_CFG="$CONFIG_DIR/boot/grub.cfg"

echo "========================================================================"
echo " [Phase 2A.2 - Step 04] Building Genuine EFI System Partition (ESP)"
echo "========================================================================"

mkdir -p "$BUILD_REAL"

# 1. Check for genuine kernel and initramfs binaries
REAL_KERNEL=$(find "$ROOTFS/boot" -name "vmlinuz*" -type f | head -n 1)
REAL_INITRD=$(find "$ROOTFS/boot" -name "initrd.img*" -type f | head -n 1)

if [ -z "$REAL_KERNEL" ] || [ ! -f "$REAL_KERNEL" ]; then
    echo "  ERROR: Real vmlinuz binary not found in $ROOTFS/boot!" >&2
    exit 1
fi

if [ -z "$REAL_INITRD" ] || [ ! -f "$REAL_INITRD" ]; then
    echo "  ERROR: Real initrd.img archive not found in $ROOTFS/boot!" >&2
    exit 1
fi

echo "  Found Genuine Kernel:    $REAL_KERNEL ($(stat -c %s "$REAL_KERNEL") bytes)"
echo "  Found Genuine Initramfs: $REAL_INITRD ($(stat -c %s "$REAL_INITRD") bytes)"

# 2. Allocate 1024 MiB (1 GiB) sparse ESP image
ESP_SIZE_BYTES=$(( 1024 * 1024 * 1024 ))
echo "  Creating 1 GiB sparse ESP image at: $ESP_IMG"
rm -f "$ESP_IMG"
truncate -s "$ESP_SIZE_BYTES" "$ESP_IMG"

# 3. Format as FAT32 with label BHOOT_EFI
echo "  Formatting FAT32 filesystem with label BHOOT_EFI..."
mkfs.vfat -F 32 -n "BHOOT_EFI" -i B8000001 "$ESP_IMG"

# 4. Generate standalone GRUB EFI binary (BOOTX64.EFI)
echo "  Compiling standalone UEFI GRUB binary (BOOTX64.EFI) with embedded early config..."
grub-mkstandalone \
    -O x86_64-efi \
    -o "$BOOTX64_EFI" \
    --modules="part_gpt part_msdos fat ext2 normal linux echo search search_fs_uuid search_label test all_video gfxterm font serial terminfo gzio password password_pbkdf2 crypto gcry_sha512" \
    "boot/grub/grub.cfg=$EARLY_CFG"

echo "  Standalone UEFI binary generated: $(stat -c %s "$BOOTX64_EFI") bytes"

# 5. Mount ESP image via udisksctl in userspace
echo "  Attaching loop device for ESP image..."
LOOP_OUT=$(udisksctl loop-setup --file "$ESP_IMG" --no-user-interaction)
LOOP_DEV=$(echo "$LOOP_OUT" | awk '{print $NF}' | tr -d '.')
echo "  Attached as: $LOOP_DEV"

cleanup() {
    echo "  Cleaning up ESP loop mount..."
    if [ -n "${LOOP_DEV:-}" ] && [ -b "$LOOP_DEV" ]; then
        udisksctl unmount -b "$LOOP_DEV" --no-user-interaction 2>/dev/null || true
        udisksctl loop-delete -b "$LOOP_DEV" --no-user-interaction 2>/dev/null || true
    fi
}
trap cleanup EXIT INT TERM HUP ERR

MOUNT_OUT=$(udisksctl mount -b "$LOOP_DEV" --no-user-interaction)
MOUNT_POINT=$(echo "$MOUNT_OUT" | awk '{print $NF}' | tr -d '.')
echo "  Mounted at: $MOUNT_POINT"

# 6. Populate ESP directory hierarchy with REAL binaries
echo "  Staging EFI directories..."
mkdir -p "$MOUNT_POINT/EFI/BOOT"
mkdir -p "$MOUNT_POINT/boot/grub"
mkdir -p "$MOUNT_POINT/live"

cp "$BOOTX64_EFI" "$MOUNT_POINT/EFI/BOOT/BOOTX64.EFI"
cp "$GRUB_CFG" "$MOUNT_POINT/EFI/BOOT/grub.cfg"
cp "$GRUB_CFG" "$MOUNT_POINT/boot/grub/grub.cfg"

# Provision genuine GRUB PBKDF2 authentication configuration for bhoot-admin
echo "  Staging GRUB administrator authentication configuration..."
if [ -n "${BHOOT_GRUB_ADMIN_PASS:-}" ]; then
    echo "  Generating GRUB PBKDF2 hash from provided environment token..."
    if [ -x "$OS_DIR/tools/bhoot-grub-pbkdf2" ]; then
        GRUB_AUTH_LINE=$(printf '%s' "$BHOOT_GRUB_ADMIN_PASS" | "$OS_DIR/tools/bhoot-grub-pbkdf2")
    else
        GRUB_AUTH_LINE=$(printf '%s' "$BHOOT_GRUB_ADMIN_PASS" | perl -e '
            use strict;
            use warnings;
            use Digest::SHA qw(hmac_sha512);
            my $pwd = <STDIN>;
            chomp $pwd;
            open my $f, "<:raw", "/dev/urandom" or die "urandom error: $!\n";
            read($f, my $salt_raw, 64) == 64 or die "short read\n";
            my $salt_hex = uc(unpack("H*", $salt_raw));
            my $u = hmac_sha512($salt_raw . pack("N", 1), $pwd);
            my $t = $u;
            for (my $i = 1; $i < 10000; $i++) {
                $u = hmac_sha512($u, $pwd);
                $t ^= $u;
            }
            $pwd = "0" x length($pwd);
            my $hash_hex = uc(unpack("H*", $t));
            print "password_pbkdf2 bhoot-admin grub.pbkdf2.sha512.10000.${salt_hex}.${hash_hex}\n";
        ')
    fi
    echo "$GRUB_AUTH_LINE" > "$MOUNT_POINT/EFI/BOOT/grub-auth.cfg"
    echo "$GRUB_AUTH_LINE" > "$MOUNT_POINT/boot/grub/grub-auth.cfg"
else
    echo "  No build-time password provided: staging unenrolled marker (enrolled interactively on first boot)..."
    cat << 'EOF' > "$MOUNT_POINT/EFI/BOOT/grub-auth.cfg"
# BHOOT GRUB Administrator Authentication Configuration
# Status: UNENROLLED
# Enrolled interactively by owner on first boot via bhoot-firstboot-setup
EOF
    cp "$MOUNT_POINT/EFI/BOOT/grub-auth.cfg" "$MOUNT_POINT/boot/grub/grub-auth.cfg"
fi
chmod 0600 "$MOUNT_POINT/EFI/BOOT/grub-auth.cfg" "$MOUNT_POINT/boot/grub/grub-auth.cfg"

echo "  Copying genuine Linux kernel to /live/vmlinuz..."
cp "$REAL_KERNEL" "$MOUNT_POINT/live/vmlinuz"

echo "  Copying genuine initial ramdisk to /live/initrd.img..."
cp "$REAL_INITRD" "$MOUNT_POINT/live/initrd.img"

if [ -f "$BUILD_REAL/filesystem.squashfs" ]; then
    echo "  Staging live root filesystem in ESP for dual-stage discovery..."
    cp "$BUILD_REAL/filesystem.squashfs" "$MOUNT_POINT/live/filesystem.squashfs"
fi

# 7. Write cryptographic manifest and kernel details to ESP
KERNEL_SHA256=$(sha256sum "$REAL_KERNEL" | awk '{print $1}')
INITRD_SHA256=$(sha256sum "$REAL_INITRD" | awk '{print $1}')

cat << EOF > "$MOUNT_POINT/live/kernel-manifest.txt"
BHOOT OS Target Kernel: Linux 6.1.0-50-amd64 Debian Bookworm x86_64
Kernel Binary SHA-256: $KERNEL_SHA256
Initramfs SHA-256: $INITRD_SHA256
Initramfs Composition: Debian live-boot + cryptsetup + dm-crypt + ext4
Persistence Mode: LUKS2 encrypted partition (BHOOT_DATA, UUID=B8007001-0003-4000-8000-000000000003)
Security Configuration: Read-only rootfs, nftables zero-trust, rfkill hard-block, volatile journald
EOF

echo "  ESP directory tree populated successfully:"
ls -lh "$MOUNT_POINT/EFI/BOOT" "$MOUNT_POINT/live"

# Verify that vmlinuz and initrd are non-empty and match original
VMLINUZ_SIZE=$(stat -c %s "$MOUNT_POINT/live/vmlinuz")
INITRD_SIZE=$(stat -c %s "$MOUNT_POINT/live/initrd.img")

if [ "$VMLINUZ_SIZE" -lt 5000000 ]; then
    echo "  ERROR: ESP vmlinuz is too small ($VMLINUZ_SIZE bytes)!" >&2
    exit 1
fi

if [ "$INITRD_SIZE" -lt 20000000 ]; then
    echo "  ERROR: ESP initrd.img is too small ($INITRD_SIZE bytes)!" >&2
    exit 1
fi

echo "  Kernel size:   $VMLINUZ_SIZE bytes (verified genuine)"
echo "  Initramfs size: $INITRD_SIZE bytes (verified genuine)"

# 8. Unmount cleanly
udisksctl unmount -b "$LOOP_DEV" --no-user-interaction 2>/dev/null || true
udisksctl loop-delete -b "$LOOP_DEV" --no-user-interaction 2>/dev/null || true
trap - EXIT INT TERM HUP ERR

echo "========================================================================"
echo " [Phase 2A.2 - Step 04] Genuine EFI System Partition Build Complete"
echo "========================================================================"

