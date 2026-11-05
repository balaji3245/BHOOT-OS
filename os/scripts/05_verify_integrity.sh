#!/usr/bin/env bash
# BHOOT Phase 2A - Step 05: Automated Integrity and Configuration Verification
# Performs rigorous verification of GPT tables, EFI structures, SquashFS, and LUKS2 headers.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OS_DIR="$(dirname "$SCRIPT_DIR")"
BUILD_DIR="$OS_DIR/build"

DISK_IMG="$BUILD_DIR/bhoot_disk_sparse.img"
ESP_IMG="$BUILD_DIR/bhoot_esp.img"
BOOTX64_EFI="$BUILD_DIR/BOOTX64.EFI"
SQUASHFS_IMG="$BUILD_DIR/filesystem.squashfs"
LUKS_IMG="$BUILD_DIR/bhoot_luks_data.img"
HEADER_BACKUP="$BUILD_DIR/bhoot_luks_header.backup"
MANIFEST_JSON="$BUILD_DIR/manifest.json"

echo "========================================================================"
echo " [Step 05] Automated Image Integrity & Boot Configuration Audit"
echo "========================================================================"

ERRORS=0

log_pass() {
    echo "  [PASS] $1"
}

log_fail() {
    echo "  [FAIL] $1" >&2
    ERRORS=$(( ERRORS + 1 ))
}

# 1. Audit GPT Disk Image
echo -e "\n--- [Audit 1/4] GPT Partition Table Structure ---"
if [ -f "$DISK_IMG" ]; then
    if sfdisk --verify "$DISK_IMG" >/dev/null 2>&1; then
        log_pass "GPT partition table integrity verified with sfdisk."
    else
        log_fail "GPT partition table verification failed."
    fi

    # Verify partition count and labels
    DUMP=$(sfdisk -d "$DISK_IMG")
    if echo "$DUMP" | grep -q 'name="BHOOT_EFI"' && \
       echo "$DUMP" | grep -q 'name="BHOOT_SYSTEM"' && \
       echo "$DUMP" | grep -q 'name="BHOOT_DATA"' && \
       echo "$DUMP" | grep -q 'name="BHOOT_RECOVERY"'; then
        log_pass "All four canonical partition labels (EFI, SYSTEM, DATA, RECOVERY) confirmed."
    else
        log_fail "Missing canonical partition labels in GPT table."
    fi
else
    log_fail "Disk image $DISK_IMG not found."
fi

# 2. Audit EFI System Partition and Standalone Bootloader
echo -e "\n--- [Audit 2/4] EFI Bootloader & FAT32 Structure ---"
if [ -f "$ESP_IMG" ]; then
    ESP_FILE_INFO=$(file "$ESP_IMG")
    if echo "$ESP_FILE_INFO" | grep -q "FAT (32 bit)"; then
        log_pass "ESP filesystem format is valid FAT32."
    else
        log_fail "ESP filesystem format is NOT FAT32: $ESP_FILE_INFO"
    fi
else
    log_fail "ESP image $ESP_IMG not found."
fi

if [ -f "$BOOTX64_EFI" ]; then
    EFI_FILE_INFO=$(file "$BOOTX64_EFI")
    if echo "$EFI_FILE_INFO" | grep -q "PE32+ executable for EFI"; then
        log_pass "BOOTX64.EFI is a valid 64-bit EFI executable (PE32+ x86-64)."
    else
        log_fail "BOOTX64.EFI invalid format: $EFI_FILE_INFO"
    fi
else
    log_fail "BOOTX64.EFI binary not found."
fi

# 3. Audit SquashFS System Root
echo -e "\n--- [Audit 3/4] Read-Only SquashFS System Image ---"
if [ -f "$SQUASHFS_IMG" ]; then
    SQ_FILE_INFO=$(file "$SQUASHFS_IMG")
    if echo "$SQ_FILE_INFO" | grep -q "Squashfs filesystem" && echo "$SQ_FILE_INFO" | grep -q "xz compressed"; then
        log_pass "SquashFS is valid and compressed with XZ."
    else
        log_fail "SquashFS validation failed: $SQ_FILE_INFO"
    fi
else
    log_fail "SquashFS image $SQUASHFS_IMG not found."
fi

# 4. Audit Encrypted LUKS2 Container & Headers
echo -e "\n--- [Audit 4/4] LUKS2 Cryptographic Specifications ---"
if [ -f "$LUKS_IMG" ]; then
    if cryptsetup isLuks --type luks2 "$LUKS_IMG"; then
        log_pass "Image is a valid LUKS2 container."
    else
        log_fail "Image is not a valid LUKS2 container."
    fi

    LUKS_DUMP=$(cryptsetup luksDump "$LUKS_IMG")
    
    # Check cipher
    if echo "$LUKS_DUMP" | grep -q "cipher: *aes-xts-plain64"; then
        log_pass "Encryption cipher verified: aes-xts-plain64."
    else
        log_fail "Cipher mismatch in LUKS2 header."
    fi

    # Check PBKDF
    if echo "$LUKS_DUMP" | grep -q "PBKDF: *argon2id"; then
        log_pass "Key derivation function verified: Argon2id."
    else
        log_fail "PBKDF is not Argon2id in LUKS2 header."
    fi

    # Check Sector Size
    if echo "$LUKS_DUMP" | grep -q "sector: *4096"; then
        log_pass "Native sector size verified: 4096 bytes (4K alignment)."
    else
        log_fail "Sector size is not 4096 in LUKS2 header."
    fi

    # Check Keyslots 0 and 1
    if echo "$LUKS_DUMP" | grep -q "0: luks2" && echo "$LUKS_DUMP" | grep -q "1: luks2"; then
        log_pass "Both Keyslot 0 (User Passphrase) and Keyslot 1 (Emergency Recovery Key) are active."
    else
        log_fail "Keyslot enrollment verification failed."
    fi
else
    log_fail "LUKS image $LUKS_IMG not found."
fi

# 5. Generate Cryptographic Manifest (SHA-256)
echo -e "\n--- Generating Cryptographic Artifact Manifest ---"
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

hash_file() {
    if [ -f "$1" ]; then
        sha256sum "$1" | awk '{print $1}'
    else
        echo "null"
    fi
}

hash_gpt_header() {
    if [ -f "$1" ]; then
        head -c 2097152 "$1" | sha256sum | awk '{print $1}'
    else
        echo "null"
    fi
}

DISK_GPT_SHA=$(hash_gpt_header "$DISK_IMG")
ESP_SHA=$(hash_file "$ESP_IMG")
EFI_SHA=$(hash_file "$BOOTX64_EFI")
SQ_SHA=$(hash_file "$SQUASHFS_IMG")
LUKS_SHA=$(hash_file "$LUKS_IMG")
HEADER_SHA=$(hash_file "$HEADER_BACKUP")

cat << EOF > "$MANIFEST_JSON"
{
  "project": "BHOOT",
  "phase": "2A",
  "timestamp": "$TIMESTAMP",
  "status": "$([ $ERRORS -eq 0 ] && echo "VERIFIED" || echo "FAILED")",
  "error_count": $ERRORS,
  "artifacts": {
    "bhoot_disk_sparse.img": {
      "path": "$DISK_IMG",
      "gpt_header_2m_sha256": "$DISK_GPT_SHA",
      "size_bytes": $([ -f "$DISK_IMG" ] && stat -c %s "$DISK_IMG" || echo 0)
    },
    "bhoot_esp.img": {
      "path": "$ESP_IMG",
      "sha256": "$ESP_SHA",
      "size_bytes": $([ -f "$ESP_IMG" ] && stat -c %s "$ESP_IMG" || echo 0)
    },
    "BOOTX64.EFI": {
      "path": "$BOOTX64_EFI",
      "sha256": "$EFI_SHA",
      "size_bytes": $([ -f "$BOOTX64_EFI" ] && stat -c %s "$BOOTX64_EFI" || echo 0)
    },
    "filesystem.squashfs": {
      "path": "$SQUASHFS_IMG",
      "sha256": "$SQ_SHA",
      "size_bytes": $([ -f "$SQUASHFS_IMG" ] && stat -c %s "$SQUASHFS_IMG" || echo 0)
    },
    "bhoot_luks_data.img": {
      "path": "$LUKS_IMG",
      "sha256": "$LUKS_SHA",
      "size_bytes": $([ -f "$LUKS_IMG" ] && stat -c %s "$LUKS_IMG" || echo 0)
    },
    "bhoot_luks_header.backup": {
      "path": "$HEADER_BACKUP",
      "sha256": "$HEADER_SHA",
      "size_bytes": $([ -f "$HEADER_BACKUP" ] && stat -c %s "$HEADER_BACKUP" || echo 0)
    }
  }
}
EOF

echo "  Manifest written to: $MANIFEST_JSON"
cat "$MANIFEST_JSON"

echo "========================================================================"
if [ $ERRORS -eq 0 ]; then
    echo " [Step 05] ALL AUDIT CHECKS PASSED: OS Image Pipeline 100% Verified"
    echo "========================================================================"
    exit 0
else
    echo " [Step 05] AUDIT FAILED with $ERRORS error(s)"
    echo "========================================================================"
    exit 1
fi

