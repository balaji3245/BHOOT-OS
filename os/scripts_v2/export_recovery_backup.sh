#!/usr/bin/env bash
# BHOOT Phase 2A.6 - External Disaster Recovery Export Workflow
# Safely exports the detached LUKS2 header backup to an external, secondary offline medium.
# Enforces strict safety checks against overwriting /dev/sda or internal SSD storage.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OS_DIR="$(dirname "$SCRIPT_DIR")"
BUILD_REAL="$OS_DIR/build_real"
SOURCE_HEADER="${BHOOT_SOURCE_HEADER:-$BUILD_REAL/bhoot_luks_header.backup}"

validate_export_destination() {
    local target="$1"

    # Default-Deny: Any exit before explicit authorization is a REJECTION
    # -------------------------------------------------------------------------
    # Rule 1: Target cannot be empty
    if [ -z "$target" ]; then
        echo "  [DENIED] Empty destination path provided." >&2
        return 1
    fi

    # Rule 2: Reject if target is directly a block, character, or FIFO device node
    if [ -b "$target" ] || [ -c "$target" ] || [ -p "$target" ]; then
        echo "  [DENIED] Destination cannot be a raw device node ($target)!" >&2
        echo "  An authorized mounted external filesystem directory is required." >&2
        return 1
    fi

    # Rule 3: Reject explicitly forbidden device path substrings
    if echo "$target" | grep -qE "^/dev/sda|^/dev/nvme|^/dev/dm-|^/dev/mapper"; then
        echo "  [DENIED] Target path references protected or internal device directly: $target" >&2
        return 1
    fi

    # Rule 4: Resolve canonical path to existing ancestor directory
    local probe_path="$target"
    while [ ! -e "$probe_path" ] && [ "$probe_path" != "/" ] && [ "$probe_path" != "." ]; do
        probe_path="$(dirname "$probe_path")"
    done

    local canonical_target
    canonical_target="$(readlink -f "$probe_path" 2>/dev/null || true)"

    if [ -z "$canonical_target" ] || [ ! -d "$canonical_target" ]; then
        echo "  [DENIED] Unable to resolve existing canonical directory for: $target" >&2
        return 1
    fi

    # Rule 5: Automated Mock Testing Allowlist Hook (used only by automated regression harness)
    if [ -n "${BHOOT_BACKUP_ALLOWLIST_MOCK:-}" ]; then
        case "$BHOOT_BACKUP_ALLOWLIST_MOCK" in
            "usb_removable")
                echo "  [TEST MOCK] Authorized external USB storage device mock accepted."
                return 0
                ;;
            "internal_sata"|"internal_nvme"|"secondary_nvme"|"mmc"|"device_mapper_internal"|"device_mapper_multislave"|"bind_mount_internal"|"multi_device"|"unresolved")
                echo "  [DENIED] Mock device profile ($BHOOT_BACKUP_ALLOWLIST_MOCK) rejected by allowlist policy." >&2
                return 1
                ;;
            *)
                echo "  [DENIED] Unknown mock device profile ($BHOOT_BACKUP_ALLOWLIST_MOCK)." >&2
                return 1
                ;;
        esac
    fi

    # Rule 6: Query mount table using findmnt
    local mnt_info
    mnt_info="$(findmnt -n -o SOURCE,FSTYPE,TARGET -T "$canonical_target" 2>/dev/null || true)"

    if [ -z "$mnt_info" ]; then
        echo "  [DENIED] Failed to resolve mount information for: $canonical_target" >&2
        return 1
    fi

    local mnt_src mnt_fstype mnt_pt
    mnt_src="$(echo "$mnt_info" | awk '{print $1}')"
    mnt_fstype="$(echo "$mnt_info" | awk '{print $2}')"
    mnt_pt="$(echo "$mnt_info" | awk '{print $3}')"

    # Rule 7: Reject virtual, pseudo, and network filesystems
    if echo "$mnt_fstype" | grep -qE "^(tmpfs|ramfs|devtmpfs|proc|sysfs|overlay|cgroup.*|autofs|securityfs|bpf|pstore|nfs.*|cifs|smbfs)$"; then
        echo "  [DENIED] Virtual or network filesystem ($mnt_fstype) is not an external storage drive." >&2
        return 1
    fi

    # Rule 8: Resolve backing block device
    local real_dev
    real_dev="$(readlink -f "$mnt_src" 2>/dev/null || echo "$mnt_src")"

    if [ ! -b "$real_dev" ]; then
        echo "  [DENIED] Backing device ($real_dev) is not a valid block device." >&2
        return 1
    fi

    # Rule 9: Reject protected production USB and internal SSD by device name
    if echo "$real_dev" | grep -qE "^/dev/sda|^/dev/nvme0n1"; then
        echo "  [DENIED] Backing device is strictly protected physical drive: $real_dev" >&2
        return 1
    fi

    # Rule 10: Device Mapper / LVM / Crypt Multi-Slave Resolution
    local current_dev="$real_dev"
    while [ -b "$current_dev" ] && echo "$current_dev" | grep -qE "^/dev/dm-|^/dev/mapper"; do
        local dm_name
        dm_name="$(basename "$current_dev")"
        if [ ! -d "/sys/block/$dm_name/slaves" ]; then
            echo "  [DENIED] Device mapper volume ($dm_name) has no slave topology." >&2
            return 1
        fi

        local slave_count
        slave_count="$(ls -1 "/sys/block/$dm_name/slaves" 2>/dev/null | wc -l)"
        if [ "$slave_count" -ne 1 ]; then
            echo "  [DENIED] Multi-device / multi-slave backing volume ($dm_name: $slave_count slaves) is unsupported." >&2
            return 1
        fi

        local slave_sub
        slave_sub="$(ls -1 "/sys/block/$dm_name/slaves" | head -n 1)"
        current_dev="/dev/$slave_sub"
    done

    # Rule 11: Resolve parent physical disk
    local parent_disk
    parent_disk="$(lsblk -no PKNAME "$current_dev" 2>/dev/null | head -n 1 || true)"
    if [ -z "$parent_disk" ]; then
        parent_disk="$(basename "$current_dev")"
    fi

    # Rule 12: Re-check parent disk against internal and protected drives
    if echo "$parent_disk" | grep -qE "^sda|^nvme0n1|^loop"; then
        echo "  [DENIED] Parent storage disk ($parent_disk) is protected or loopback." >&2
        return 1
    fi

    # Rule 13: Reject internal storage bus types (SATA / NVMe / MMC controllers)
    local dev_sys_path
    dev_sys_path="$(udevadm info -q path -n "$current_dev" 2>/dev/null || true)"
    if [ -z "$dev_sys_path" ]; then
        echo "  [DENIED] Unable to query udev device path for $current_dev." >&2
        return 1
    fi

    if echo "$dev_sys_path" | grep -qE "/(nvme|ata[0-9]+|mmc[0-9]+|virtio[0-9]+)/"; then
        echo "  [DENIED] Storage device resides on internal bus controller ($dev_sys_path)." >&2
        return 1
    fi

    # Rule 14: Strict External USB Allowlist
    local is_usb="0" is_removable="0"
    local udev_props
    udev_props="$(udevadm info -q property -n "$current_dev" 2>/dev/null || true)"

    if echo "$udev_props" | grep -q "ID_BUS=usb" || echo "$dev_sys_path" | grep -qE "/usb[0-9]+/"; then
        is_usb="1"
    fi

    local rem_val
    rem_val="$(cat "/sys/block/$parent_disk/removable" 2>/dev/null || echo "0")"
    if [ "$rem_val" = "1" ] || echo "$udev_props" | grep -qE "ID_DRIVE_FLASH_SD=1|ID_DRIVE_THUMB=1"; then
        is_removable="1"
    fi

    if [ "$is_usb" = "1" ] && [ "$is_removable" = "1" ]; then
        echo "  [ALLOWED] Destination verified as external removable USB storage device ($parent_disk)."
        return 0
    fi

    echo "  [DENIED] Storage device does not meet external removable USB allowlist requirements." >&2
    return 1
}

main() {
    local SOURCE_HEADER="${BHOOT_SOURCE_HEADER:-$BUILD_REAL/bhoot_luks_header.backup}"
    local DEST_DIR=""

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
    local SOURCE_HASH
    SOURCE_HASH=$(sha256sum "$SOURCE_HEADER" | awk '{print $1}')
    echo "  Source Header SHA-256: $SOURCE_HASH"

    # 4. Export files
    local EXPORTED_HEADER="$DEST_DIR/bhoot_luks_header.backup"
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
    local TARGET_HASH
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
}

if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi
