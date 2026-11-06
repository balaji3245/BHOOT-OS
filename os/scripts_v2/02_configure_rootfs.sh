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
root:!:19000:0:99999:7:::
daemon:*:19000:0:99999:7:::
bin:*:19000:0:99999:7:::
sys:*:19000:0:99999:7:::
sync:*:19000:0:99999:7:::
nobody:*:19000:0:99999:7:::
systemd-journal:*:19000:0:99999:7:::
systemd-network:*:19000:0:99999:7:::
systemd-resolve:*:19000:0:99999:7:::
_apt:*:19000:0:99999:7:::
bhoot:!:19000:0:99999:7:::
EOF
chmod 0640 "$ROOTFS/etc/shadow"

# Ensure NO autologin drop-ins exist
rm -rf "$ROOTFS/etc/systemd/system/serial-getty@ttyS0.service.d"
rm -rf "$ROOTFS/etc/systemd/system/getty@tty1.service.d"

# Configure standard login getty on ttyS0 and tty1 (strict authentication required)
mkdir -p "$ROOTFS/etc/systemd/system/getty.target.wants"
ln -sf /lib/systemd/system/serial-getty@.service "$ROOTFS/etc/systemd/system/getty.target.wants/serial-getty@ttyS0.service"

# Machine-id: Left completely uninitialized/empty in immutable SquashFS build
# Generated dynamically per-installation on first boot and stored on encrypted volume
rm -f "$ROOTFS/etc/machine-id"
touch "$ROOTFS/etc/machine-id"
chmod 0444 "$ROOTFS/etc/machine-id"

# Hardened offline PAM configuration (STRICT: nullok is strictly omitted)
cat << 'EOF' > "$ROOTFS/etc/pam.d/common-auth"
auth    [success=1 default=ignore]      pam_unix.so
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

# Install BHOOT Security Init & First-Boot Credential Provisioning scripts
mkdir -p "$ROOTFS/usr/local/sbin"

cat << 'EOF' > "$ROOTFS/usr/local/sbin/bhoot-security-init"
#!/bin/sh
set -eu
# BHOOT Early Security Initialization
# Mounts persistent machine identity and shadow database from encrypted volume

SYSTEM_PERSIST_DIR="/var/bhoot/system"
mkdir -p "$SYSTEM_PERSIST_DIR"
chmod 0700 "$SYSTEM_PERSIST_DIR"

# 1. Machine Identity Initialization
if [ -s "$SYSTEM_PERSIST_DIR/machine-id" ]; then
    mount --bind "$SYSTEM_PERSIST_DIR/machine-id" /etc/machine-id
else
    # Generate unique 128-bit hex machine-id from urandom
    tr -dc '0-9a-f' < /dev/urandom | head -c 32 > "$SYSTEM_PERSIST_DIR/machine-id"
    echo "" >> "$SYSTEM_PERSIST_DIR/machine-id"
    chmod 0444 "$SYSTEM_PERSIST_DIR/machine-id"
    mount --bind "$SYSTEM_PERSIST_DIR/machine-id" /etc/machine-id
fi

# 2. Shadow Authentication Initialization
if [ -f "$SYSTEM_PERSIST_DIR/shadow" ]; then
    chmod 0640 "$SYSTEM_PERSIST_DIR/shadow"
    chown root:shadow "$SYSTEM_PERSIST_DIR/shadow"
    mount --bind "$SYSTEM_PERSIST_DIR/shadow" /etc/shadow
fi
EOF
chmod 0755 "$ROOTFS/usr/local/sbin/bhoot-security-init"

cat << 'EOF' > "$ROOTFS/usr/local/sbin/bhoot-firstboot-setup"
#!/usr/bin/env bash
set -euo pipefail
# BHOOT First-Boot Interactive Credential Provisioning
# Prompts owner to establish master password for user 'bhoot'

SYSTEM_PERSIST_DIR="/var/bhoot/system"
INIT_FLAG="$SYSTEM_PERSIST_DIR/initialized"

if [ -f "$INIT_FLAG" ]; then
    exit 0
fi

mkdir -p "$SYSTEM_PERSIST_DIR"
chmod 0700 "$SYSTEM_PERSIST_DIR"

echo "========================================================================"
echo "         PROJECT BHOOT — SECURE CREDENTIAL PROVISIONING"
echo "========================================================================"
echo " Welcome to your private financial intelligence environment."
echo " Initial setup required: Establish the master password for 'bhoot'."
echo "========================================================================"

AUTOMATED_TOKEN_FILE="/run/bhoot/provisioning_token"

if [ -f "$AUTOMATED_TOKEN_FILE" ]; then
    PASSWD_INPUT=$(cat "$AUTOMATED_TOKEN_FILE")
    shred -u "$AUTOMATED_TOKEN_FILE" 2>/dev/null || rm -f "$AUTOMATED_TOKEN_FILE"
else
    while true; do
        read -s -p " Enter new master password for 'bhoot' (min 12 chars): " PASS1
        echo ""
        if [ ${#PASS1} -lt 12 ]; then
            echo " [!] Password too short. Minimum length is 12 characters."
            continue
        fi
        read -s -p " Confirm master password: " PASS2
        echo ""
        if [ "$PASS1" != "$PASS2" ]; then
            echo " [!] Passwords do not match. Please try again."
            continue
        fi
        PASSWD_INPUT="$PASS1"
        unset PASS1 PASS2
        break
    done
fi

SALT=$(tr -dc 'a-zA-Z0-9' < /dev/urandom | head -c 16)
HASH=$(perl -e 'print crypt($ARGV[0], "\$6\$rounds=50000\$$ARGV[1]\$")' "$PASSWD_INPUT" "$SALT")
unset PASSWD_INPUT

cp /etc/shadow "$SYSTEM_PERSIST_DIR/shadow.tmp"
sed -i "s|^root:.*|root:!:19000:0:99999:7:::|" "$SYSTEM_PERSIST_DIR/shadow.tmp"
sed -i "s|^bhoot:.*|bhoot:${HASH}:19000:0:99999:7:::|" "$SYSTEM_PERSIST_DIR/shadow.tmp"
chmod 0640 "$SYSTEM_PERSIST_DIR/shadow.tmp"
chown root:shadow "$SYSTEM_PERSIST_DIR/shadow.tmp"
mv "$SYSTEM_PERSIST_DIR/shadow.tmp" "$SYSTEM_PERSIST_DIR/shadow"

mount --bind "$SYSTEM_PERSIST_DIR/shadow" /etc/shadow

if [ ! -s "$SYSTEM_PERSIST_DIR/machine-id" ]; then
    tr -dc '0-9a-f' < /dev/urandom | head -c 32 > "$SYSTEM_PERSIST_DIR/machine-id"
    echo "" >> "$SYSTEM_PERSIST_DIR/machine-id"
    chmod 0444 "$SYSTEM_PERSIST_DIR/machine-id"
    mount --bind "$SYSTEM_PERSIST_DIR/machine-id" /etc/machine-id
fi

date -u +"%Y-%m-%dT%H:%M:%SZ" > "$INIT_FLAG"
chmod 0600 "$INIT_FLAG"

echo " [OK] Master user credentials successfully provisioned to encrypted storage."
echo " [OK] Proceeding to login..."
echo "========================================================================"
EOF
chmod 0755 "$ROOTFS/usr/local/sbin/bhoot-firstboot-setup"

# Install Systemd units for early security init and first-boot provisioning
cat << 'EOF' > "$ROOTFS/etc/systemd/system/bhoot-security-init.service"
[Unit]
Description=BHOOT Early Security and Machine Identity Initialization
DefaultDependencies=no
After=local-fs.target
Before=sysinit.target basic.target systemd-journald.service

[Service]
Type=oneshot
RemainAfterExit=yes
ExecStart=/usr/local/sbin/bhoot-security-init

[Install]
WantedBy=basic.target
EOF
chmod 0644 "$ROOTFS/etc/systemd/system/bhoot-security-init.service"
mkdir -p "$ROOTFS/etc/systemd/system/basic.target.wants"
ln -sf /etc/systemd/system/bhoot-security-init.service "$ROOTFS/etc/systemd/system/basic.target.wants/bhoot-security-init.service"

cat << 'EOF' > "$ROOTFS/etc/systemd/system/bhoot-firstboot-provisioning.service"
[Unit]
Description=BHOOT First-Boot Credential Provisioning Service
ConditionPathExists=!/var/bhoot/system/initialized
After=bhoot-security-init.service
Before=getty.target serial-getty@ttyS0.service

[Service]
Type=oneshot
RemainAfterExit=yes
StandardInput=tty-force
StandardOutput=inherit
StandardError=inherit
TTYPath=/dev/console
ExecStart=/usr/local/sbin/bhoot-firstboot-setup

[Install]
WantedBy=getty.target
EOF
chmod 0644 "$ROOTFS/etc/systemd/system/bhoot-firstboot-provisioning.service"
ln -sf /etc/systemd/system/bhoot-firstboot-provisioning.service "$ROOTFS/etc/systemd/system/getty.target.wants/bhoot-firstboot-provisioning.service"

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

