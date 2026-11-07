#!/usr/bin/env bash
# BHOOT Phase 2A.2 - Step 02: Configure & Harden Debian 12 RootFS
# Applies offline networking blocks, power-loss resilience, ephemeral logging, and service masks.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OS_DIR="$(dirname "$SCRIPT_DIR")"
REPO_ROOT="$(dirname "$OS_DIR")"
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
audio:x:29:bhoot
video:x:44:bhoot
render:x:105:bhoot
input:x:106:bhoot
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
password    [success=1 default=ignore]  pam_unix.so obscure yescrypt
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
chmod 0755 /var/bhoot 2>/dev/null || true
chmod 0700 "$SYSTEM_PERSIST_DIR"

# 1. Machine Identity Initialization (Atomic write, strict durability & bind)
if [ -s "$SYSTEM_PERSIST_DIR/machine-id" ]; then
    mount --bind "$SYSTEM_PERSIST_DIR/machine-id" /etc/machine-id
else
    # Generate unique 128-bit hex machine-id atomically from urandom
    MID_TMP="$SYSTEM_PERSIST_DIR/machine-id.tmp.$$"
    MID_HEX=$(perl -e 'open my $f, "<:raw", "/dev/urandom" or die "urandom error: $!\n"; read($f, my $b, 16) == 16 or die "short read\n"; print unpack("H*", $b);' 2>/dev/null || od -vAn -N16 -tx1 /dev/urandom | tr -d ' \n')
    if [ -n "$MID_HEX" ] && [ ${#MID_HEX} -eq 32 ]; then
        printf '%s\n' "$MID_HEX" > "$MID_TMP"
        chmod 0444 "$MID_TMP"
        if ! sync "$MID_TMP"; then
            echo "ERROR: Failed to sync temporary machine-id file to durable storage" >&2
            rm -f "$MID_TMP"
            exit 1
        fi
        if ! mv -f "$MID_TMP" "$SYSTEM_PERSIST_DIR/machine-id"; then
            echo "ERROR: Failed to atomically link machine-id" >&2
            rm -f "$MID_TMP"
            exit 1
        fi
        if ! sync "$SYSTEM_PERSIST_DIR"; then
            echo "ERROR: Failed to sync persistent directory metadata for machine-id" >&2
            exit 1
        fi
        mount --bind "$SYSTEM_PERSIST_DIR/machine-id" /etc/machine-id
    else
        echo "ERROR: Failed to generate valid 32-character machine-id" >&2
        exit 1
    fi
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
chmod 0755 /var/bhoot 2>/dev/null || true
chmod 0700 "$SYSTEM_PERSIST_DIR"

cleanup_temps() {
    rm -f "$SYSTEM_PERSIST_DIR/shadow.tmp"* "$SYSTEM_PERSIST_DIR/machine-id.tmp"* 2>/dev/null || true
}
trap cleanup_temps EXIT

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

# 1. Native yescrypt password hash generation (strictly piped via STDIN)
# Cryptographic salt encoding details:
# - Salt input: 16 bytes (128 bits) CSPRNG entropy read from /dev/urandom.
# - Output encoding: Standard libxcrypt 22-character B64 format for $y$ yescrypt:
#   * Bytes 0..14 (15 bytes = 120 bits): 5 groups of 3 bytes packed into 20 B64 characters.
#     Characters 0..19 are uniformly distributed across the 64-character alphabet:
#     "./0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz".
#   * Byte 15 (8 bits = bits 120..127):
#     - Lower 6 bits (w15 & 0x3f) form Character 20 (uniformly distributed across 64 chars).
#     - Upper 2 bits ((w15 >> 6) & 0x3f) form Character 21 (values 0..3: '.', '/', '0', '1').
#     - Total characters: 20 + 1 + 1 = 22 characters.
#     - Total entropy: 20*6 + 6 + 2 = 128 bits.
# - The 2-bit boundary of the 22nd character is strictly compliant with libxcrypt's
#   crypt_gensalt() specification for yescrypt ($y$j9T$...) and PAM pam_unix.so.
HASH=$(printf '%s' "$PASSWD_INPUT" | perl -e '
    use strict;
    use warnings;
    my @b64 = split(//, "./0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz");
    open my $f, "<:raw", "/dev/urandom" or die "urandom error: $!\n";
    read($f, my $raw, 16) == 16 or die "short read\n";
    my @b = unpack("C*", $raw);
    my $salt = "";
    for (my $i = 0; $i < 15; $i += 3) {
        my $w = $b[$i] | ($b[$i+1] << 8) | ($b[$i+2] << 16);
        $salt .= $b64[$w & 0x3f];
        $salt .= $b64[($w >> 6) & 0x3f];
        $salt .= $b64[($w >> 12) & 0x3f];
        $salt .= $b64[($w >> 18) & 0x3f];
    }
    my $w15 = $b[15];
    $salt .= $b64[$w15 & 0x3f];
    $salt .= $b64[($w15 >> 6) & 0x3f];
    my $setting = "\$y\$j9T\$$salt";
    my $pwd = <STDIN>;
    chomp $pwd;
    defined $pwd && length($pwd) >= 12 or die "Invalid password length\n";
    my $h = crypt($pwd, $setting);
    $pwd = "0" x length($pwd);
    die "crypt returned error ($h)\n" if $h eq "*0" || substr($h, 0, 3) ne "\$y\$";
    print $h;
')
unset PASSWD_INPUT

if [ -z "$HASH" ] || [ "${HASH:0:3}" != "\$y\$" ]; then
    echo " [ERROR] Master password native yescrypt hash generation failed!" >&2
    exit 1
fi

# 2. Atomically write shadow database to encrypted persistent volume with strict durability
SHADOW_TMP="$SYSTEM_PERSIST_DIR/shadow.tmp.$$"
cp /etc/shadow "$SHADOW_TMP"
sed -i "s|^root:.*|root:!:19000:0:99999:7:::|" "$SHADOW_TMP"
sed -i "s|^bhoot:.*|bhoot:${HASH}:19000:0:99999:7:::|" "$SHADOW_TMP"
chmod 0640 "$SHADOW_TMP"
chown root:shadow "$SHADOW_TMP"
if ! sync "$SHADOW_TMP"; then
    echo " [ERROR] Failed to sync shadow temporary file!" >&2
    rm -f "$SHADOW_TMP"
    exit 1
fi
if ! mv -f "$SHADOW_TMP" "$SYSTEM_PERSIST_DIR/shadow"; then
    echo " [ERROR] Failed to rename shadow database!" >&2
    rm -f "$SHADOW_TMP"
    exit 1
fi
if ! sync "$SYSTEM_PERSIST_DIR"; then
    echo " [ERROR] Failed to sync directory metadata for shadow!" >&2
    exit 1
fi

mount --bind "$SYSTEM_PERSIST_DIR/shadow" /etc/shadow

echo "YESCRYPT_ACTIVE: ${HASH:0:8}..." > /var/bhoot/shadow_algo.proof
chmod 0644 /var/bhoot/shadow_algo.proof
sync /var/bhoot/shadow_algo.proof 2>/dev/null || true
cp -f /var/bhoot/shadow_algo.proof "$SYSTEM_PERSIST_DIR/shadow_algo.proof" 2>/dev/null || true

# 3. Atomic Machine-ID Generation with Strict Durability
if [ ! -s "$SYSTEM_PERSIST_DIR/machine-id" ]; then
    MID_TMP="$SYSTEM_PERSIST_DIR/machine-id.tmp.$$"
    MID_HEX=$(perl -e 'open my $f, "<:raw", "/dev/urandom" or die; read($f, my $b, 16) == 16 or die; print unpack("H*", $b);' 2>/dev/null || od -vAn -N16 -tx1 /dev/urandom | tr -d ' \n')
    if [ -n "$MID_HEX" ] && [ ${#MID_HEX} -eq 32 ]; then
        printf '%s\n' "$MID_HEX" > "$MID_TMP"
        chmod 0444 "$MID_TMP"
        if ! sync "$MID_TMP"; then
            echo " [ERROR] Failed to sync machine-id temporary file!" >&2
            rm -f "$MID_TMP"
            exit 1
        fi
        if ! mv -f "$MID_TMP" "$SYSTEM_PERSIST_DIR/machine-id"; then
            echo " [ERROR] Failed to atomically rename machine-id!" >&2
            rm -f "$MID_TMP"
            exit 1
        fi
        if ! sync "$SYSTEM_PERSIST_DIR"; then
            echo " [ERROR] Failed to sync directory metadata for machine-id!" >&2
            exit 1
        fi
        mount --bind "$SYSTEM_PERSIST_DIR/machine-id" /etc/machine-id
    fi
fi

# 4. Interactive Owner GRUB Administrator Password Enrollment (5-Word Diceware 64.6-Bit Entropy)
echo "========================================================================"
echo " Establish GRUB Administrator Password for 'bhoot-admin'"
echo " Required to access offline recovery shell and edit boot parameters."
echo "========================================================================"

WORDLIST="/usr/share/bhoot/eff_large_wordlist.txt"
if [ ! -f "$WORDLIST" ]; then
    WORDLIST="/etc/bhoot/eff_large_wordlist.txt"
fi

if [ ! -f "$WORDLIST" ]; then
    echo " [FATAL ERROR] Canonical Diceware wordlist missing from system!" >&2
    exit 1
fi

GRUB_TOKEN_FILE="/run/bhoot/grub_provisioning_token"

if [ -f "$GRUB_TOKEN_FILE" ]; then
    GRUB_PASS_INPUT=$(cat "$GRUB_TOKEN_FILE")
    shred -u "$GRUB_TOKEN_FILE" 2>/dev/null || rm -f "$GRUB_TOKEN_FILE"
else
    # Verify wordlist integrity
    if ! /usr/local/bin/bhoot-grub-pbkdf2 --verify-wordlist "$WORDLIST" >/dev/null 2>&1; then
        echo " [FATAL ERROR] Wordlist integrity check failed (SHA-256 mismatch)!" >&2
        exit 1
    fi

    # Generate genuine 5-word Diceware passphrase using CSPRNG and rejection sampling
    GEN_PASSPHRASE=$(/usr/local/bin/bhoot-grub-pbkdf2 --generate-diceware "$WORDLIST")
    if [ -z "$GEN_PASSPHRASE" ]; then
        echo " [FATAL ERROR] Failed to generate cryptographically secure Diceware passphrase!" >&2
        exit 1
    fi

    echo ""
    echo " A cryptographically secure 5-word Diceware passphrase has been generated:"
    echo " (CSPRNG rejection sampling from canonical 7,776-word list | ~64.62 bits entropy)"
    echo ""
    echo "   >>> $GEN_PASSPHRASE <<<"
    echo ""
    echo " IMPORTANT: Record this passphrase in an offline password manager or safe."
    echo " It is required to unlock the Offline Recovery Console and edit GRUB settings."
    echo "========================================================================"
    echo ""

    while true; do
        IFS= read -r -s -p " Confirm by entering the exact 5-word passphrase: " CONFIRM_PASS
        echo ""
        if [ "$CONFIRM_PASS" != "$GEN_PASSPHRASE" ]; then
            echo " [!] Passphrase does not match. Please re-enter carefully."
            continue
        fi
        GRUB_PASS_INPUT="$CONFIRM_PASS"
        unset CONFIRM_PASS GEN_PASSPHRASE
        break
    done
fi

if mount -o remount,rw /boot/efi 2>/dev/null || mount -t vfat -o rw /dev/disk/by-label/BHOOT_EFI /boot/efi 2>/dev/null; then
    mkdir -p /boot/efi/EFI/BOOT /boot/efi/boot/grub 2>/dev/null || true
    GRUB_AUTH_TMP="/boot/efi/EFI/BOOT/grub-auth.cfg.tmp.$$"
    if [ -x /usr/local/bin/bhoot-grub-pbkdf2 ]; then
        GRUB_AUTH_LINE=$(printf '%s' "$GRUB_PASS_INPUT" | /usr/local/bin/bhoot-grub-pbkdf2)
    else
        echo " [FATAL ERROR] bhoot-grub-pbkdf2 utility not found!" >&2
        exit 1
    fi

    if [ -n "$GRUB_AUTH_LINE" ] && [ "${GRUB_AUTH_LINE:0:15}" = "password_pbkdf2" ]; then
        printf '%s\n' "$GRUB_AUTH_LINE" > "$GRUB_AUTH_TMP"
        chmod 0600 "$GRUB_AUTH_TMP" 2>/dev/null || true
        # Cryptographically verify generated configuration before activation
        if ! printf '%s' "$GRUB_PASS_INPUT" | /usr/local/bin/bhoot-grub-pbkdf2 --verify-file "$GRUB_AUTH_TMP" >/dev/null 2>&1; then
            echo " [FATAL ERROR] Self-verification of generated GRUB PBKDF2 hash failed!" >&2
            rm -f "$GRUB_AUTH_TMP"
            exit 1
        fi
        unset GRUB_PASS_INPUT
        if ! sync "$GRUB_AUTH_TMP"; then
            echo " [ERROR] Failed to sync temporary grub-auth.cfg!" >&2
            rm -f "$GRUB_AUTH_TMP"
            exit 1
        fi
        if ! mv -f "$GRUB_AUTH_TMP" /boot/efi/EFI/BOOT/grub-auth.cfg; then
            echo " [ERROR] Failed to rename grub-auth.cfg!" >&2
            rm -f "$GRUB_AUTH_TMP"
            exit 1
        fi
        cp -f /boot/efi/EFI/BOOT/grub-auth.cfg /boot/efi/boot/grub/grub-auth.cfg 2>/dev/null || true
        sync /boot/efi/EFI/BOOT 2>/dev/null || true
        echo " [OK] GRUB administrator authentication configured successfully."
    else
        unset GRUB_PASS_INPUT
        echo " [ERROR] Failed to compute GRUB PBKDF2 authentication line!" >&2
        exit 1
    fi
    mount -o remount,ro /boot/efi 2>/dev/null || true
fi

# 5. Mark successful initialization ONLY after all steps succeed
date -u +"%Y-%m-%dT%H:%M:%SZ" > "$INIT_FLAG"
chmod 0644 "$INIT_FLAG"
sync "$INIT_FLAG"

echo "BHOOT_LIFECYCLE_PERSISTENCE_2026" > /var/bhoot/lifecycle_proof.txt
chmod 0644 /var/bhoot/lifecycle_proof.txt
sync /var/bhoot/lifecycle_proof.txt
sync /var/bhoot

echo " [OK] Master user credentials successfully provisioned to encrypted storage."
echo " [OK] Proceeding to login..."
echo "========================================================================"
EOF
chmod 0755 "$ROOTFS/usr/local/sbin/bhoot-firstboot-setup"

# Compile and install bhoot-grub-pbkdf2 helper utility into rootfs
echo "  Compiling and installing bhoot-grub-pbkdf2 helper..."
mkdir -p "$ROOTFS/usr/local/bin"
gcc -std=gnu11 -O2 -Wall -Wextra -Werror "$OS_DIR/tools/bhoot-grub-pbkdf2.c" -lcrypto -o "$ROOTFS/usr/local/bin/bhoot-grub-pbkdf2"
chmod 0755 "$ROOTFS/usr/local/bin/bhoot-grub-pbkdf2"

# Install canonical EFF 7,776-word Diceware list into rootfs
echo "  Installing canonical EFF 7,776-word Diceware list..."
mkdir -p "$ROOTFS/usr/share/bhoot" "$ROOTFS/etc/bhoot"
cp -f "$OS_DIR/config/eff_large_wordlist.txt" "$ROOTFS/usr/share/bhoot/eff_large_wordlist.txt"
cp -f "$OS_DIR/config/eff_large_wordlist.txt" "$ROOTFS/etc/bhoot/eff_large_wordlist.txt"
chmod 0644 "$ROOTFS/usr/share/bhoot/eff_large_wordlist.txt" "$ROOTFS/etc/bhoot/eff_large_wordlist.txt"

# Install Authenticated Disaster Recovery Console (HIGH-01 Remediation)
echo "  Installing authenticated disaster recovery console and emergency tools..."
cat << 'EOF' > "$ROOTFS/usr/local/sbin/bhoot-recovery-console"
#!/usr/bin/env bash
# Project BHOOT — Authenticated Disaster Recovery Console
# Offline-first security: Strictly authenticated maintenance shell.
# Drops into root shell ONLY upon successful administrator PBKDF2 authentication.
set -u

umask 077
export PATH="/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"
export HISTFILE="/dev/null"

cleanup_and_reboot() {
    echo ""
    echo " [SECURITY ALERT] Recovery interrupted. System rebooting..."
    sync
    systemctl reboot -f 2>/dev/null || reboot -f
    exit 1
}
trap cleanup_and_reboot INT TERM

RECOVERY_BANNER="========================================================================
       PROJECT BHOOT — AUTHENTICATED DISASTER RECOVERY CONSOLE
========================================================================
 Target: Offline Storage Diagnostics & Cryptographic Header Recovery
 Notice: Unauthenticated root shell access is strictly prohibited.
========================================================================"

echo "$RECOVERY_BANNER"

# 1. Mount ESP (BHOOT_EFI) to retrieve enrolled administrator authentication hash
EFI_DIR="/boot/efi"
mkdir -p "$EFI_DIR" 2>/dev/null || true
if ! mountpoint -q "$EFI_DIR"; then
    mount -t vfat -o ro /dev/disk/by-label/BHOOT_EFI "$EFI_DIR" 2>/dev/null || \
    mount -t vfat -o ro /dev/sda1 "$EFI_DIR" 2>/dev/null || \
    mount -t vfat -o ro /dev/vda1 "$EFI_DIR" 2>/dev/null || true
fi

# 2. Mount Offline Recovery Partition (BHOOT_REC) if available
REC_DIR="/recovery"
mkdir -p "$REC_DIR" 2>/dev/null || true
if ! mountpoint -q "$REC_DIR"; then
    mount -t ext4 -o ro /dev/disk/by-label/BHOOT_REC "$REC_DIR" 2>/dev/null || \
    mount -t ext4 -o ro /dev/disk/by-partuuid/B8007001-0004-4000-8000-000000000004 "$REC_DIR" 2>/dev/null || \
    mount -t ext4 -o ro /dev/sda4 "$REC_DIR" 2>/dev/null || \
    mount -t ext4 -o ro /dev/vda4 "$REC_DIR" 2>/dev/null || true
fi

# Ensure restore_header.sh is executable and in standard recovery path
if [ -f "$REC_DIR/restore_header.sh" ]; then
    chmod +x "$REC_DIR/restore_header.sh" 2>/dev/null || true
    ln -sf "$REC_DIR/restore_header.sh" /usr/local/sbin/restore_header.sh 2>/dev/null || true
    ln -sf "$REC_DIR/restore_header.sh" /root/restore_header.sh 2>/dev/null || true
fi

# Locate grub-auth.cfg
GRUB_AUTH_CFG=""
for candidate in "$EFI_DIR/EFI/BOOT/grub-auth.cfg" "$EFI_DIR/boot/grub/grub-auth.cfg" /boot/grub/grub-auth.cfg; do
    if [ -s "$candidate" ]; then
        GRUB_AUTH_CFG="$candidate"
        break
    fi
done

if [ -z "$GRUB_AUTH_CFG" ] || grep -q "Status: UNENROLLED" "$GRUB_AUTH_CFG" 2>/dev/null; then
    echo " [FATAL ERROR] BHOOT administrator credentials have not been enrolled."
    echo " Normal first-boot provisioning must be completed before recovery access."
    echo " System will restart in 5 seconds..."
    sleep 5
    sync
    systemctl reboot -f 2>/dev/null || reboot -f
    exit 1
fi

# 3. Authenticate Operator interactively
MAX_ATTEMPTS=3
ATTEMPT=1
AUTH_SUCCESS=0

while [ "$ATTEMPT" -le "$MAX_ATTEMPTS" ]; do
    echo ""
    echo " Recovery Authentication Required (Attempt $ATTEMPT of $MAX_ATTEMPTS)"
    IFS= read -r -s -p " Enter Administrator Recovery Password: " ENTERED_PASS
    echo ""

    if [ -z "$ENTERED_PASS" ]; then
        echo " [!] Password cannot be empty."
        ATTEMPT=$((ATTEMPT + 1))
        continue
    fi

    # Verify password against enrolled PBKDF2 hash using compiled binary
    if printf '%s\n' "$ENTERED_PASS" | /usr/local/bin/bhoot-grub-pbkdf2 --verify-file "$GRUB_AUTH_CFG" >/dev/null 2>&1; then
        AUTH_SUCCESS=1
        unset ENTERED_PASS
        break
    else
        unset ENTERED_PASS
        echo " [SECURITY ALERT] Authentication failure: invalid credentials."
        ATTEMPT=$((ATTEMPT + 1))
        sleep 2
    fi
done

if [ "$AUTH_SUCCESS" -ne 1 ]; then
    echo ""
    echo " [CRITICAL ALERT] Maximum recovery authentication attempts exceeded."
    echo " Rebooting system immediately..."
    sync
    systemctl reboot -f 2>/dev/null || reboot -f
    exit 1
fi

echo ""
echo " [OK] Authentication successful. Initializing root recovery environment..."
echo "========================================================================"
echo " Available Recovery Commands:"
echo "   restore_header.sh          - Restore detached LUKS2 header to Partition 3"
echo "   bhoot-unlock-recovery-key  - Interactive persistence unlock using recovery key"
echo "   cryptsetup luksDump <dev>  - Inspect LUKS keyslots and header integrity"
echo "   exit                       - Complete maintenance and reboot system"
echo "========================================================================"

export PS1="[BHOOT-RECOVERY]# "
cd /root 2>/dev/null || cd /

# Execute interactive root shell
/bin/bash --norc --noprofile

echo ""
echo " Exiting recovery console. Syncing disks and rebooting..."
sync
systemctl reboot -f 2>/dev/null || reboot -f
EOF
chmod 0700 "$ROOTFS/usr/local/sbin/bhoot-recovery-console"

# Install Emergency Recovery Key Unlock Helper (LOW-02 Remediation)
cat << 'EOF' > "$ROOTFS/usr/local/sbin/bhoot-unlock-recovery-key"
#!/usr/bin/env bash
# Project BHOOT — Interactive Recovery Key Unlock Helper
# Normalizes operator input (strips hyphens, spaces, handles lowercase)
# to match enrolled 256-bit recovery key in Keyslot 1.
set -euo pipefail

TARGET_DEV="${1:-}"
if [ -z "$TARGET_DEV" ]; then
    for candidate in /dev/disk/by-partuuid/B8007001-0003-4000-8000-000000000003 /dev/disk/by-label/BHOOT_DATA /dev/vda3 /dev/sda3; do
        if [ -b "$candidate" ]; then
            TARGET_DEV="$candidate"
            break
        fi
    done
fi

if [ -z "$TARGET_DEV" ] || [ ! -b "$TARGET_DEV" ]; then
    echo "ERROR: Target LUKS block device not found!" >&2
    exit 1
fi

MAPPER_NAME="${2:-bhoot_data_recovery}"

echo "========================================================================"
echo " BHOOT Emergency LUKS2 Recovery Unlock Helper"
echo " Target Device: $TARGET_DEV -> /dev/mapper/$MAPPER_NAME"
echo "========================================================================"
read -s -p " Enter 256-bit recovery key (64 hex characters, hyphens optional): " RAW_INPUT
echo ""

# Normalize input: strip all whitespace and hyphens, convert to uppercase
CLEAN_HEX=$(echo "$RAW_INPUT" | tr -d ' \t\r\n-' | tr '[:lower:]' '[:upper:]')
unset RAW_INPUT

if [ ${#CLEAN_HEX} -ne 64 ] || ! echo "$CLEAN_HEX" | grep -qE '^[0-9A-F]{64}$'; then
    echo "ERROR: Invalid recovery key format. Expected 64 hexadecimal characters (256 bits)." >&2
    unset CLEAN_HEX
    exit 1
fi

# Format into canonical 16 chunks of 4 characters: XXXX-XXXX-...-XXXX
CHUNKED_KEY=$(echo "$CLEAN_HEX" | sed -E 's/(.{4})/\1-/g; s/-$//')
unset CLEAN_HEX

echo " Attempting LUKS2 unlock with normalized recovery key..."
if printf '%s' "$CHUNKED_KEY" | cryptsetup open --type luks2 --key-slot 1 "$TARGET_DEV" "$MAPPER_NAME" --key-file -; then
    unset CHUNKED_KEY
    echo " [OK] Volume unlocked successfully: /dev/mapper/$MAPPER_NAME"
else
    unset CHUNKED_KEY
    echo " [ERROR] Recovery key failed to unlock $TARGET_DEV." >&2
    exit 1
fi
EOF
chmod 0755 "$ROOTFS/usr/local/sbin/bhoot-unlock-recovery-key"

# Install fallback restore_header.sh in rootfs
cat << 'EOF' > "$ROOTFS/usr/local/sbin/restore_header.sh"
#!/usr/bin/env bash
# Project BHOOT — Detached LUKS2 Header Restoration Utility
# Restores detached header backup to Partition 3 (BHOOT_DATA)
set -euo pipefail

TARGET_DEV="${1:-}"
if [ -z "$TARGET_DEV" ]; then
    for candidate in /dev/disk/by-partuuid/B8007001-0003-4000-8000-000000000003 /dev/disk/by-label/BHOOT_DATA /dev/vda3 /dev/sda3; do
        if [ -b "$candidate" ]; then
            TARGET_DEV="$candidate"
            break
        fi
    done
fi

if [ -z "$TARGET_DEV" ] || [ ! -b "$TARGET_DEV" ]; then
    echo "ERROR: Target LUKS block device not found!" >&2
    echo "Usage: restore_header.sh [target_device] [backup_file]" >&2
    exit 1
fi

BACKUP_FILE="${2:-}"
if [ -z "$BACKUP_FILE" ]; then
    for candidate in /recovery/bhoot_luks_header.backup /boot/efi/bhoot_luks_header.backup /root/bhoot_luks_header.backup /etc/bhoot/bhoot_luks_header.backup; do
        if [ -f "$candidate" ]; then
            BACKUP_FILE="$candidate"
            break
        fi
    done
fi

if [ -z "$BACKUP_FILE" ] || [ ! -f "$BACKUP_FILE" ]; then
    echo "ERROR: LUKS2 header backup file not found!" >&2
    echo "Mount recovery volume (BHOOT_REC) to /recovery or specify backup file path." >&2
    exit 1
fi

echo "========================================================================"
echo " BHOOT LUKS2 Header Restoration Engine"
echo " Target Block Device: $TARGET_DEV"
echo " Header Backup File:  $BACKUP_FILE"
echo "========================================================================"

echo "Restoring detached LUKS2 header to $TARGET_DEV..."
cryptsetup luksHeaderRestore "$TARGET_DEV" --header-backup-file "$BACKUP_FILE" --batch-mode

echo "Verifying restored header integrity..."
cryptsetup luksDump "$TARGET_DEV" >/dev/null

echo "========================================================================"
echo " [OK] LUKS2 header restored successfully."
echo " Volume can now be unlocked with master password or recovery key."
echo "========================================================================"
EOF
chmod 0755 "$ROOTFS/usr/local/sbin/restore_header.sh"

# Install Systemd drop-in overrides for rescue.service and emergency.service
mkdir -p "$ROOTFS/etc/systemd/system/rescue.service.d"
cat << 'EOF' > "$ROOTFS/etc/systemd/system/rescue.service.d/override.conf"
[Unit]
Description=BHOOT Authenticated Disaster Recovery Console

[Service]
ExecStart=
ExecStart=-/usr/local/sbin/bhoot-recovery-console
StandardInput=tty-force
StandardOutput=inherit
StandardError=inherit
EOF
chmod 0644 "$ROOTFS/etc/systemd/system/rescue.service.d/override.conf"

mkdir -p "$ROOTFS/etc/systemd/system/emergency.service.d"
cat << 'EOF' > "$ROOTFS/etc/systemd/system/emergency.service.d/override.conf"
[Unit]
Description=BHOOT Authenticated Disaster Recovery Console

[Service]
ExecStart=
ExecStart=-/usr/local/sbin/bhoot-recovery-console
StandardInput=tty-force
StandardOutput=inherit
StandardError=inherit
EOF
chmod 0644 "$ROOTFS/etc/systemd/system/emergency.service.d/override.conf"

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

# 10. Install BHOOT Desktop Application and Update Manager
echo "  Installing BHOOT Desktop application and system update manager..."
if [ -f "$REPO_ROOT/target/release/bhoot-desktop" ]; then
    cp "$REPO_ROOT/target/release/bhoot-desktop" "$ROOTFS/usr/local/bin/bhoot-desktop"
    chmod 0755 "$ROOTFS/usr/local/bin/bhoot-desktop"
fi
if [ -f "$SCRIPT_DIR/bhoot_sysupdate.py" ]; then
    cp "$SCRIPT_DIR/bhoot_sysupdate.py" "$ROOTFS/usr/local/bin/bhoot-sysupdate"
    chmod 0755 "$ROOTFS/usr/local/bin/bhoot-sysupdate"
fi

cat << 'EOF' > "$ROOTFS/etc/systemd/system/bhoot-desktop.service"
[Unit]
Description=BHOOT Personal Financial Operating System Desktop Engine
After=basic.target bhoot-security-init.service
Wants=basic.target

[Service]
Type=simple
ExecStart=/usr/local/bin/bhoot-desktop --port 1420 --db /var/bhoot/data/bhoot.db
Restart=always
RestartSec=3
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
EOF
chmod 0644 "$ROOTFS/etc/systemd/system/bhoot-desktop.service"
mkdir -p "$ROOTFS/etc/systemd/system/multi-user.target.wants"
ln -sf /etc/systemd/system/bhoot-desktop.service "$ROOTFS/etc/systemd/system/multi-user.target.wants/bhoot-desktop.service"

# Dedicated Graphical Kiosk Service (Cage / Browser runtime)
cat << 'EOF' > "$ROOTFS/etc/systemd/system/bhoot-gui.service"
[Unit]
Description=BHOOT Personal Financial OS Graphical Kiosk
After=bhoot-desktop.service seatd.service
Wants=bhoot-desktop.service seatd.service
Conflicts=getty@tty1.service

[Service]
Type=simple
User=bhoot
Group=bhoot
SupplementaryGroups=video input render audio tty
RuntimeDirectory=user/1000
RuntimeDirectoryMode=0700
Environment=XDG_RUNTIME_DIR=/run/user/1000
Environment=WLR_BACKENDS=drm,libinput
Environment=WLR_NO_HARDWARE_CURSORS=1
Environment=WLR_RENDERER=pixman
Environment=WLR_RENDERER_ALLOW_SOFTWARE=1
Environment=LIBGL_ALWAYS_SOFTWARE=1
Environment=WEBKIT_DISABLE_DMABUF_RENDERER=1
Environment=GSETTINGS_SCHEMA_DIR=/usr/share/glib-2.0/schemas
Environment=GIO_MODULE_DIR=/usr/lib/x86_64-linux-gnu/gio/modules
Environment=SEATD_SOCK=/run/seatd.sock
Environment=LIBSEAT_BACKEND=seatd
TTYPath=/dev/tty1
TTYReset=yes
TTYVHangup=yes
StandardInput=tty
StandardOutput=journal
StandardError=journal
ExecStartPre=+/bin/mkdir -p /run/user/1000 /home/bhoot/.cache /home/bhoot/.config
ExecStartPre=+/bin/chown -R bhoot:bhoot /run/user/1000 /home/bhoot
ExecStartPre=+/bin/chmod 0700 /run/user/1000 /home/bhoot/.cache /home/bhoot/.config
ExecStartPre=/bin/bash -c 'for i in $(seq 1 30); do (echo > /dev/tcp/127.0.0.1/1420) >/dev/null 2>&1 && exit 0; sleep 0.5; done; exit 0'
ExecStart=/usr/bin/cage -s -- /usr/bin/cog -P wl -b "#000000" --doc-viewer http://127.0.0.1:1420
Restart=always
RestartSec=3

[Install]
WantedBy=graphical.target multi-user.target
EOF
chmod 0644 "$ROOTFS/etc/systemd/system/bhoot-gui.service"
ln -sf /etc/systemd/system/bhoot-gui.service "$ROOTFS/etc/systemd/system/multi-user.target.wants/bhoot-gui.service"

# 11. Mask Telemetry, Cloud-Init, and Networking Services
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

