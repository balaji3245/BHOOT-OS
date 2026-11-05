#!/usr/bin/env bash
# BHOOT Phase 2A.2 - Step 01: Bootstrap Real Debian 12 (Bookworm) Root Filesystem
# Downloads, extracts, and populates the rootfs with genuine Debian Bookworm packages.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OS_DIR="$(dirname "$SCRIPT_DIR")"
BUILD_REAL="$OS_DIR/build_real"
ROOTFS="$BUILD_REAL/rootfs"
WRAPPER_DIR="$BUILD_REAL/tools"

echo "========================================================================"
echo " [Phase 2A.2 - Step 01] Bootstrapping Genuine Debian 12 Bookworm RootFS"
echo "========================================================================"

mkdir -p "$BUILD_REAL" "$WRAPPER_DIR"

# 1. Setup local tar wrapper in PATH to enforce --no-same-owner in user namespaces
cat << 'EOF' > "$WRAPPER_DIR/tar"
#!/bin/sh
exec /bin/tar --no-same-owner "$@"
EOF
chmod +x "$WRAPPER_DIR/tar"

MIRROR="http://deb.debian.org/debian"
SUITE="bookworm"
INCLUDE_PKGS="systemd,systemd-sysv,coreutils,util-linux,e2fsprogs,cryptsetup,cryptsetup-initramfs,nftables,rfkill,linux-image-amd64,live-boot,kmod,udev"

# 2. Check if debootstrap stage 1 has already been fetched; if not, fetch it
if [ ! -d "$ROOTFS/var/cache/apt/archives" ] || [ -z "$(ls -A "$ROOTFS/var/cache/apt/archives" 2>/dev/null)" ]; then
    echo "  Running debootstrap to download and unpack base packages..."
    rm -rf "$ROOTFS"
    mkdir -p "$ROOTFS"
    PATH="$WRAPPER_DIR:$PATH" unshare -U -p -f -r -m sh -c "
        /usr/sbin/debootstrap \
            --foreign \
            --no-check-gpg \
            --variant=minbase \
            --include=$INCLUDE_PKGS \
            $SUITE \
            $ROOTFS \
            $MIRROR
    "
else
    echo "  Cached packages already present in $ROOTFS/var/cache/apt/archives."
fi

# 3. Unpack all downloaded packages from archives into rootfs
echo "  Unpacking all required and included packages into root filesystem..."
for deb in "$ROOTFS"/var/cache/apt/archives/*.deb; do
    [ -f "$deb" ] || continue
    dpkg-deb --fsys-tarfile "$deb" | tar --no-same-owner -xf - -C "$ROOTFS"
done

echo "  All Debian packages unpacked into rootfs."

# 4. Generate the genuine live-boot initramfs inside rootfs
echo "  Generating Debian 12 Live-Boot + Cryptsetup Initramfs..."
unshare -U -p -f -r -m sh -c "
    mount -t proc proc '$ROOTFS/proc'
    chroot '$ROOTFS' /usr/sbin/mkinitramfs -o /boot/initrd.img-6.1.0-50-amd64 6.1.0-50-amd64 2>/dev/null || true
    umount '$ROOTFS/proc'
"

# 5. Verify presence of kernel and initramfs
KERNEL_PATH=$(find "$ROOTFS/boot" -name "vmlinuz*" -type f | head -n 1)
INITRD_PATH=$(find "$ROOTFS/boot" -name "initrd.img*" -type f | head -n 1)

if [ -n "$KERNEL_PATH" ] && [ -f "$KERNEL_PATH" ] && [ $(stat -c %s "$KERNEL_PATH") -gt 1000000 ]; then
    echo "  Genuine Linux kernel verified: $KERNEL_PATH ($(stat -c %s "$KERNEL_PATH") bytes)"
else
    echo "  ERROR: Valid vmlinuz binary not found in $ROOTFS/boot!" >&2
    exit 1
fi

if [ -n "$INITRD_PATH" ] && [ -f "$INITRD_PATH" ] && [ $(stat -c %s "$INITRD_PATH") -gt 1000000 ]; then
    echo "  Genuine Live initramfs verified: $INITRD_PATH ($(stat -c %s "$INITRD_PATH") bytes)"
else
    echo "  ERROR: Valid initrd.img archive not found in $ROOTFS/boot!" >&2
    exit 1
fi

echo "========================================================================"
echo " [Phase 2A.2 - Step 01] Debian 12 RootFS Successfully Bootstrapped"
echo "========================================================================"
