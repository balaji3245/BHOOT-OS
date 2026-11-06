#!/usr/bin/env bash
# BHOOT Phase 2A.6 - External Disaster Recovery Export Workflow
# Safely exports the detached LUKS2 header backup to an external, secondary offline medium.
# Enforces strict safety checks against overwriting /dev/sda or internal SSD storage.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OS_DIR="$(dirname "$SCRIPT_DIR")"
BUILD_REAL="$OS_DIR/build_real"
SOURCE_HEADER="${BHOOT_SOURCE_HEADER:-$BUILD_REAL/bhoot_luks_header.backup}"

DEST_DIR=""
while [ $# -gt 0 ]; do
    case "$1" in
        --source)
            SOURCE_HEADER="$2"
            shift 2
            ;;
        --target)
            DEST_DIR="$2"
            shift 2
            ;;
        *)
            if [ -z "$DEST_DIR" ]; then
                DEST_DIR="$1"
            fi
            shift
            ;;
    esac
done

if [ ! -f "$SOURCE_HEADER" ]; then
    echo "  ERROR: Detached header backup not found at: $SOURCE_HEADER" >&2
    exit 1
fi

echo "========================================================================"
echo " [BHOOT Disaster Recovery Engine] External Header Backup Exporter"
echo "========================================================================"

if [ -z "$DEST_DIR" ]; then
    echo "  This tool exports your detached LUKS2 header backup to a SECONDARY offline medium."
    echo "  (e.g., an external secondary USB drive, offline backup disk, or secure storage path)."
    echo ""
    read -r -p "  Enter destination export directory: " DEST_DIR
fi

# 1. STRICT SAFETY CHECKS
if [ -z "$DEST_DIR" ]; then
    echo "  ERROR: No destination path provided!" >&2
    exit 1
fi

validate_export_destination() {
    local target="$1"

    # Reject if target is directly a block or character device node
    if [ -b "$target" ] || [ -c "$target" ]; then
        echo "  [CRITICAL ERROR] Destination cannot be a raw device node ($target)!" >&2
        echo "  A mounted filesystem directory path is required." >&2
        return 1
    fi

    # Check if target string matches protected physical devices
    if echo "$target" | grep -qE "^/dev/sda|^/dev/nvme"; then
        echo "  [CRITICAL ERROR] Destination matches protected device pattern: $target" >&2
        return 1
    fi

    # Resolve canonical path to existing ancestor directory
    local probe_path="$target"
    while [ ! -e "$probe_path" ] && [ "$probe_path" != "/" ] && [ "$probe_path" != "." ]; do
        probe_path="$(dirname "$probe_path")"
    done

    local canonical_target
    canonical_target="$(readlink -f "$probe_path" 2>/dev/null || echo "$probe_path")"

    if [ ! -d "$canonical_target" ]; then
        echo "  [CRITICAL ERROR] Unable to resolve canonical directory for: $target" >&2
        return 1
    fi

    # Resolve mount metadata using findmnt
    local mnt_info
    mnt_info="$(findmnt -n -o SOURCE,FSTYPE,TARGET -T "$canonical_target" 2>/dev/null || true)"

    if [ -z "$mnt_info" ]; then
        echo "  [CRITICAL ERROR] Failed to determine mount information for: $canonical_target" >&2
        return 1
    fi

    local mnt_src
    mnt_src="$(echo "$mnt_info" | awk '{print $1}')"

    # Reject if mount source matches protected devices
    if echo "$mnt_src" | grep -qE "^/dev/sda|^/dev/nvme0n1"; then
        echo "  [CRITICAL SAFETY VIOLATION] Destination is mounted on protected device: $mnt_src" >&2
        return 1
    fi

    # Resolve parent disk using lsblk if block device
    if [ -b "$mnt_src" ] || [ -b "/dev/$mnt_src" ]; then
        local real_dev
        real_dev="$(readlink -f "$mnt_src" 2>/dev/null || echo "$mnt_src")"
        if echo "$real_dev" | grep -qE "^/dev/sda|^/dev/nvme0n1"; then
            echo "  [CRITICAL SAFETY VIOLATION] Destination backing device is protected: $real_dev" >&2
            return 1
        fi

        local parent_disk
        parent_disk="$(lsblk -no PKNAME "$real_dev" 2>/dev/null || true)"
        if [ -n "$parent_disk" ] && echo "$parent_disk" | grep -qE "^sda|^nvme0n1"; then
            echo "  [CRITICAL SAFETY VIOLATION] Destination parent disk is protected: $parent_disk" >&2
            return 1
        fi
    fi

    # In automated test mode with virtual mock protected devices
    if [ -n "${BHOOT_PROTECTED_DEVICES:-}" ]; then
        for prot in $BHOOT_PROTECTED_DEVICES; do
            if [ "$mnt_src" = "$prot" ] || echo "$mnt_src" | grep -qE "^${prot}"; then
                echo "  [CRITICAL SAFETY VIOLATION] Destination matches protected device: $prot" >&2
                return 1
            fi
        done
    fi

    return 0
}

# Run validation BEFORE creating or modifying any file
if ! validate_export_destination "$DEST_DIR"; then
    echo "  Export aborted due to destination device safety violation." >&2
    exit 1
fi

if [ ! -d "$DEST_DIR" ]; then
    mkdir -p "$DEST_DIR"
fi

if [ ! -w "$DEST_DIR" ]; then
    echo "  ERROR: Destination directory is not writable: $DEST_DIR" >&2
    exit 1
fi

# 2. Confirmation (if interactive)
if [ -t 0 ] && [ "${BHOOT_AUTOMATED_TEST:-0}" != "1" ]; then
    echo ""
    echo "  Destination Target: $DEST_DIR"
    read -r -p "  Confirm export of detached LUKS2 header backup to this destination? [y/N]: " CONFIRM
    if [ "$CONFIRM" != "y" ] && [ "$CONFIRM" != "Y" ]; then
        echo "  Export aborted by user."
        exit 0
    fi
fi

# 3. Compute Source Digest
SOURCE_HASH=$(sha256sum "$SOURCE_HEADER" | awk '{print $1}')
echo "  Source Header SHA-256: $SOURCE_HASH"

# 4. Export files
EXPORTED_HEADER="$DEST_DIR/bhoot_luks_header.backup"
cp "$SOURCE_HEADER" "$EXPORTED_HEADER"
chmod 0400 "$EXPORTED_HEADER"

# Write instructions
cat << EOF > "$DEST_DIR/README_EXTERNAL_RECOVERY.txt"
========================================================================
 BHOOT Detached LUKS2 Header Backup & Emergency Recovery Guide
========================================================================
 This directory contains the detached cryptographic header backup for
 your BHOOT primary financial storage partition (UUID: b8007001-0003-4000-8000-000000000003).

 If your physical USB drive suffers unexpected power loss, partition table
 corruption, or LUKS header damage, restore the header using:

   cryptsetup luksHeaderRestore /dev/disk/by-partuuid/B8007001-0003-4000-8000-000000000003 \\
       --header-backup-file bhoot_luks_header.backup

 Header SHA-256 Hash: $SOURCE_HASH
 Timestamp of Export: $(date -u +"%Y-%m-%dT%H:%M:%SZ")
========================================================================
EOF
chmod 0644 "$DEST_DIR/README_EXTERNAL_RECOVERY.txt"

# 5. Verify Target Digest
TARGET_HASH=$(sha256sum "$EXPORTED_HEADER" | awk '{print $1}')
echo "  Target Header SHA-256: $TARGET_HASH"

if [ "$SOURCE_HASH" != "$TARGET_HASH" ]; then
    echo "  CRITICAL ERROR: Checksum mismatch after export!" >&2
    rm -f "$EXPORTED_HEADER"
    exit 1
fi

echo ""
echo "========================================================================"
echo " [SUCCESS] Detached LUKS2 Header Backup Exported & Verified Successfully"
echo " Location: $EXPORTED_HEADER"
echo " SHA-256:  $TARGET_HASH"
echo "========================================================================"
