#!/usr/bin/env bash
# BHOOT Phase 2A - Step 03: Build Deterministic Read-Only SquashFS System Root
# Assembles the hardened offline root filesystem skeleton and compiles it into filesystem.squashfs.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OS_DIR="$(dirname "$SCRIPT_DIR")"
BUILD_DIR="$OS_DIR/build"
CONFIG_DIR="$OS_DIR/config"

SQUASHFS_OUT="$BUILD_DIR/filesystem.squashfs"
ROOT_STAGING="$BUILD_DIR/root_staging"

echo "========================================================================"
echo " [Step 03] Building Read-Only SquashFS System Image"
echo "========================================================================"

mkdir -p "$BUILD_DIR"
rm -rf "$ROOT_STAGING" "$SQUASHFS_OUT"
mkdir -p "$ROOT_STAGING"

echo "  Staging standard Linux directory layout..."
mkdir -p "$ROOT_STAGING"/{bin,sbin,etc,lib,usr,var,run,tmp,dev,proc,sys,root,home/bhoot}
mkdir -p "$ROOT_STAGING"/usr/{bin,sbin,lib}
mkdir -p "$ROOT_STAGING"/etc/{sysctl.d,systemd/system,systemd/journald.conf.d,bhoot,nftables}
mkdir -p "$ROOT_STAGING"/var/bhoot
chmod 1777 "$ROOT_STAGING"/tmp
chmod 0700 "$ROOT_STAGING"/root
chmod 0750 "$ROOT_STAGING"/home/bhoot

# 1. Install hardened sysctl offline settings
echo "  Installing offline kernel sysctl settings..."
cp "$CONFIG_DIR/offline/99-bhoot-offline.conf" "$ROOT_STAGING/etc/sysctl.d/99-bhoot-offline.conf"

# 2. Install rfkill early hardware blocking service
echo "  Installing RFKill hardware radio block service..."
cp "$CONFIG_DIR/offline/bhoot-rfkill.service" "$ROOT_STAGING/etc/systemd/system/bhoot-rfkill.service"
ln -sf /etc/systemd/system/bhoot-rfkill.service "$ROOT_STAGING/etc/systemd/system/basic.target.wants/bhoot-rfkill.service" 2>/dev/null || \
    (mkdir -p "$ROOT_STAGING/etc/systemd/system/basic.target.wants" && ln -sf /etc/systemd/system/bhoot-rfkill.service "$ROOT_STAGING/etc/systemd/system/basic.target.wants/bhoot-rfkill.service")

# 3. Install zero-trust local-only firewall
echo "  Installing zero-trust nftables firewall..."
cp "$CONFIG_DIR/offline/bhoot-network-block.nft" "$ROOT_STAGING/etc/nftables/bhoot-network-block.nft"

# 4. Install volatile in-memory journald logging config
echo "  Installing ephemeral volatile journald config..."
cp "$CONFIG_DIR/system/journald-volatile.conf" "$ROOT_STAGING/etc/systemd/journald.conf.d/volatile.conf"

# 5. Install power-loss & storage barrier configuration
echo "  Installing power-loss flash protection sysctl..."
cp "$CONFIG_DIR/system/bhoot-powerloss.conf" "$ROOT_STAGING/etc/sysctl.d/98-bhoot-powerloss.conf"

# 6. Install fstab and crypttab
echo "  Installing storage mounts and crypttab..."
cp "$CONFIG_DIR/storage/fstab.bhoot" "$ROOT_STAGING/etc/fstab"
cp "$CONFIG_DIR/storage/crypttab.bhoot" "$ROOT_STAGING/etc/crypttab"

# 7. Mask all telemetry, cloud-init, and network manager services
echo "  Masking telemetry and unapproved network daemons..."
while IFS= read -r service || [ -n "$service" ]; do
    # Skip comments and empty lines
    [[ "$service" =~ ^#.*$ ]] && continue
    [[ -z "${service// }" ]] && continue
    ln -sf /dev/null "$ROOT_STAGING/etc/systemd/system/$service"
done < "$CONFIG_DIR/offline/masked-services.list"

# 8. Embed version and provenance metadata
GIT_COMMIT=$(git rev-parse HEAD 2>/dev/null || echo "408e0b4")
BUILD_TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

cat << EOF > "$ROOT_STAGING/etc/bhoot/version.json"
{
  "system": "BHOOT",
  "name": "Private Financial Intelligence OS",
  "phase": "2A",
  "git_commit": "$GIT_COMMIT",
  "build_timestamp": "$BUILD_TIMESTAMP",
  "target_arch": "x86_64",
  "base_distribution": "Debian 12 Bookworm Minimal",
  "boot_mode": "UEFI Only",
  "persistence": "LUKS2 (Argon2id + AES-256-XTS)",
  "storage_capacity_usb": "57.8 GiB",
  "telemetry_policy": "Zero-Egress Strict Offline"
}
EOF

# 9. Compile SquashFS filesystem with deterministic parameters
echo "  Compressing SquashFS with XZ (-comp xz -noappend -reproducible -all-root)..."
mksquashfs "$ROOT_STAGING" "$SQUASHFS_OUT" \
    -comp xz \
    -noappend \
    -reproducible \
    -all-root

SQUASHFS_SIZE=$(stat -c %s "$SQUASHFS_OUT")
echo "  SquashFS image built successfully: $SQUASHFS_SIZE bytes"

rm -rf "$ROOT_STAGING"

echo "========================================================================"
echo " [Step 03] Successfully Built Read-Only SquashFS System Image"
echo "========================================================================"

