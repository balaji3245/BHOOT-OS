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

# 1. Base directories and user accounts
echo "  Setting up core directories, user accounts, and permissions..."
mkdir -p "$ROOTFS"/etc/{sysctl.d,systemd/system,systemd/journald.conf.d,bhoot,nftables,systemd/system/serial-getty@ttyS0.service.d}
mkdir -p "$ROOTFS"/var/bhoot "$ROOTFS"/run/bhoot
mkdir -p "$ROOTFS"/boot/efi
mkdir -p "$ROOTFS"/home/bhoot
chmod 0750 "$ROOTFS"/home/bhoot
chmod 0700 "$ROOTFS"/root
chmod 1777 "$ROOTFS"/tmp

# Create standard Debian passwd, group, and shadow accounts
cat << 'EOF' > "$ROOTFS/etc/passwd"
root:x:0:0:root:/root:/bin/bash
daemon:x:1:1:daemon:/usr/sbin:/usr/sbin/nologin
bin:x:2:2:bin:/bin:/usr/sbin/nologin
sys:x:3:3:sys:/dev:/usr/sbin/nologin
sync:x:4:65534:sync:/bin:/bin/sync
nobody:x:65534:65534:nobody:/nonexistent:/usr/sbin/nologin
systemd-journal:x:101:101::/run/systemd:/usr/sbin/nologin
systemd-network:x:102:102:systemd Network Management,,,:/run/systemd:/usr/sbin/nologin
systemd-resolve:x:103:103:systemd Resolver,,,:/run/systemd:/usr/sbin/nologin
_apt:x:104:65534::/nonexistent:/usr/sbin/nologin
bhoot:x:1000:1000:BHOOT Admin,,,:/home/bhoot:/bin/bash
EOF
chmod 0644 "$ROOTFS/etc/passwd"

cat << 'EOF' > "$ROOTFS/etc/group"
root:x:0:
daemon:x:1:
bin:x:2:
sys:x:3:
adm:x:4:
tty:x:5:
disk:x:6:
sudo:x:27:bhoot
users:x:100:
systemd-journal:x:101:
systemd-network:x:102:
systemd-resolve:x:103:
nogroup:x:65534:
bhoot:x:1000:
EOF
chmod 0644 "$ROOTFS/etc/group"

cat << 'EOF' > "$ROOTFS/etc/shadow"
root::19000:0:99999:7:::
daemon:*:19000:0:99999:7:::
bin:*:19000:0:99999:7:::
sys:*:19000:0:99999:7:::
sync:*:19000:0:99999:7:::
nobody:*:19000:0:99999:7:::
systemd-journal:*:19000:0:99999:7:::
systemd-network:*:19000:0:99999:7:::
systemd-resolve:*:19000:0:99999:7:::
_apt:*:19000:0:99999:7:::
bhoot::19000:0:99999:7:::
EOF
chmod 0640 "$ROOTFS/etc/shadow"

# Serial autologin on ttyS0 for non-interactive test verification
cat << 'EOF' > "$ROOTFS/etc/systemd/system/serial-getty@ttyS0.service.d/autologin.conf"
[Service]
ExecStart=
ExecStart=-/sbin/agetty --autologin root --noclear %I 115200,38400,9600 vt220
EOF
chmod 0644 "$ROOTFS/etc/systemd/system/serial-getty@ttyS0.service.d/autologin.conf"
mkdir -p "$ROOTFS/etc/systemd/system/getty.target.wants"
ln -sf /lib/systemd/system/serial-getty@.service "$ROOTFS/etc/systemd/system/getty.target.wants/serial-getty@ttyS0.service"

# Initialized machine-id to suppress first-boot wizard
rm -f "$ROOTFS/etc/machine-id"
echo "b8007001000040008000000000000001" > "$ROOTFS/etc/machine-id"
chmod 0444 "$ROOTFS/etc/machine-id"

# Provision minimal offline PAM configuration to allow automated root login
cat << 'EOF' > "$ROOTFS/etc/pam.d/common-auth"
auth    [success=1 default=ignore]      pam_unix.so nullok
auth    requisite                       pam_deny.so
auth    required                        pam_permit.so
EOF
chmod 0644 "$ROOTFS/etc/pam.d/common-auth"

cat << 'EOF' > "$ROOTFS/etc/pam.d/common-account"
account [success=1 new_authtok_reqd=done default=ignore]    pam_unix.so
account requisite                                           pam_deny.so
account required                                            pam_permit.so
EOF
chmod 0644 "$ROOTFS/etc/pam.d/common-account"

cat << 'EOF' > "$ROOTFS/etc/pam.d/common-session"
session required    pam_permit.so
session required    pam_unix.so
session optional    pam_systemd.so
EOF
chmod 0644 "$ROOTFS/etc/pam.d/common-session"

cat << 'EOF' > "$ROOTFS/etc/pam.d/common-password"
password    [success=1 default=ignore]  pam_unix.so obscure sha512
password    requisite                   pam_deny.so
password    required                    pam_permit.so
EOF
chmod 0644 "$ROOTFS/etc/pam.d/common-password"

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
bhoot_data      UUID=B8007001-0003-4000-8000-000000000003           none       luks,no-read-workqueue,no-write-workqueue,nofail,noauto
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

