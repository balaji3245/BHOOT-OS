#!/usr/bin/env bash
# BHOOT Phase 2A.2 - Step 02: Configure & Harden Debian 12 RootFS
# Applies offline networking blocks, power-loss resilience, ephemeral logging, and service masks.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OS_DIR="$(dirname "$SCRIPT_DIR")"
BUILD_REAL="$OS_DIR/build_real"
ROOTFS="$BUILD_REAL/rootfs"
CONFIG_DIR="$OS_DIR/config"

echo "========================================================================"
echo " [Phase 2A.2 - Step 02] Hardening and Configuring Debian 12 RootFS"
echo "========================================================================"

if [ ! -d "$ROOTFS/etc" ]; then
    echo "  ERROR: Rootfs directory $ROOTFS does not exist or is incomplete!" >&2
    exit 1
fi

# 1. Base directories
echo "  Setting up core directories and permissions..."
mkdir -p "$ROOTFS"/etc/{sysctl.d,systemd/system,systemd/journald.conf.d,bhoot,nftables}
mkdir -p "$ROOTFS"/var/bhoot "$ROOTFS"/run/bhoot
mkdir -p "$ROOTFS"/boot/efi
mkdir -p "$ROOTFS"/home/bhoot
chmod 0750 "$ROOTFS"/home/bhoot
chmod 0700 "$ROOTFS"/root
chmod 1777 "$ROOTFS"/tmp

# 2. Hostname and Localhost resolution
echo "  Configuring offline hostname and loopback resolution..."
echo "bhoot-secure" > "$ROOTFS/etc/hostname"
cat << 'EOF' > "$ROOTFS/etc/hosts"
127.0.0.1   localhost bhoot-secure
::1         localhost ip6-localhost ip6-loopback
EOF

# 3. Kernel Sysctl Hardening (Offline & Zero-Trust)
echo "  Applying offline kernel sysctl settings..."
cp "$CONFIG_DIR/offline/99-bhoot-offline.conf" "$ROOTFS/etc/sysctl.d/99-bhoot-offline.conf"
chmod 0644 "$ROOTFS/etc/sysctl.d/99-bhoot-offline.conf"

# 4. Power-Loss & Flash Memory Protection
echo "  Applying flash power-loss sync parameters..."
cp "$CONFIG_DIR/system/bhoot-powerloss.conf" "$ROOTFS/etc/sysctl.d/98-bhoot-powerloss.conf"
chmod 0644 "$ROOTFS/etc/sysctl.d/98-bhoot-powerloss.conf"

# 5. Volatile Ephemeral Logging Configuration
echo "  Configuring volatile ephemeral systemd journal..."
cp "$CONFIG_DIR/system/journald-volatile.conf" "$ROOTFS/etc/systemd/journald.conf.d/volatile.conf"
chmod 0644 "$ROOTFS/etc/systemd/journald.conf.d/volatile.conf"

# 6. RFKill Wireless Hardware Hard-Block Service
echo "  Configuring RFKill hardware radio block service..."
cp "$CONFIG_DIR/offline/bhoot-rfkill.service" "$ROOTFS/etc/systemd/system/bhoot-rfkill.service"
chmod 0644 "$ROOTFS/etc/systemd/system/bhoot-rfkill.service"
mkdir -p "$ROOTFS/etc/systemd/system/basic.target.wants"
ln -sf /etc/systemd/system/bhoot-rfkill.service "$ROOTFS/etc/systemd/system/basic.target.wants/bhoot-rfkill.service"

# 7. Zero-Trust Local-Only NFTables Firewall
echo "  Configuring zero-trust nftables firewall..."
cp "$CONFIG_DIR/offline/bhoot-network-block.nft" "$ROOTFS/etc/nftables.conf"
chmod 0644 "$ROOTFS/etc/nftables.conf"
mkdir -p "$ROOTFS/etc/systemd/system/sysinit.target.wants"
ln -sf /lib/systemd/system/nftables.service "$ROOTFS/etc/systemd/system/sysinit.target.wants/nftables.service"

# 8. Storage Mount Configuration (fstab)
echo "  Configuring filesystem table (fstab)..."
cp "$CONFIG_DIR/storage/fstab.bhoot" "$ROOTFS/etc/fstab"
chmod 0644 "$ROOTFS/etc/fstab"

# 9. Crypttab Configuration (Discard STRICTLY REMOVED to prevent ciphertext metadata leakage)
echo "  Configuring dm-crypt / crypttab (DISCARD STRICTLY OMITTED)..."
cat << 'EOF' > "$ROOTFS/etc/crypttab"
# BHOOT dm-crypt / crypttab configuration
# WARNING: 'discard' (TRIM) is deliberately omitted to prevent flash controller leakage of filesystem boundaries.
# target_name   source_device                                       key_file   options
bhoot_data      UUID=B8007001-0003-4000-8000-000000000003           none       luks,no-read-workqueue,no-write-workqueue
EOF
chmod 0600 "$ROOTFS/etc/crypttab"

# 10. Mask Telemetry, Cloud-Init, and Networking Services
echo "  Masking telemetry, auto-updaters, and unneeded network services..."
while IFS= read -r service || [ -n "$service" ]; do
    [[ "$service" =~ ^#.*$ ]] && continue
    [[ -z "${service// }" ]] && continue
    ln -sf /dev/null "$ROOTFS/etc/systemd/system/$service"
done < "$CONFIG_DIR/offline/masked-services.list"

# 11. Provenance and System Metadata
GIT_COMMIT=$(git rev-parse HEAD 2>/dev/null || echo "408e0b4")
BUILD_TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

cat << EOF > "$ROOTFS/etc/bhoot/version.json"
{
  "system": "BHOOT",
  "name": "Private Financial Intelligence OS",
  "phase": "2A.2",
  "git_commit": "$GIT_COMMIT",
  "build_timestamp": "$BUILD_TIMESTAMP",
  "target_arch": "x86_64",
  "base_distribution": "Debian 12 Bookworm Minimal",
  "kernel_version": "6.1.0-50-amd64",
  "boot_mode": "UEFI Only (x86_64-efi)",
  "persistence": "LUKS2 (Argon2id + AES-256-XTS, 4096 sector size, no discard)",
  "storage_capacity_usb": "57.8 GiB",
  "telemetry_policy": "Zero-Egress Strict Offline (nftables drop-all, rfkill hard-block, services masked)"
}
EOF
chmod 0644 "$ROOTFS/etc/bhoot/version.json"

echo "========================================================================"
echo " [Phase 2A.2 - Step 02] RootFS Hardening and Configuration Complete"
echo "========================================================================"
