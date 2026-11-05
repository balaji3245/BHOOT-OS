#!/usr/bin/env bash
# BHOOT Phase 2A.4 - Automated UEFI Virtualized Boot Test Harness
# Launches QEMU with OVMF UEFI firmware, zero networking, enters LUKS passphrase,
# and verifies all 10 stages of the boot progression and runtime security invariants.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OS_DIR="$(dirname "$SCRIPT_DIR")"
BUILD_REAL="$OS_DIR/build_real"

DISK_IMG="$BUILD_REAL/bhoot_disk_assembled.img"
SERIAL_LOG="$BUILD_REAL/qemu_boot_serial.log"

echo "========================================================================"
echo " [Phase 2A.4] BHOOT UEFI Virtualized Boot Test Harness"
echo "========================================================================"

# 1. Check prerequisites
if ! command -v qemu-system-x86_64 >/dev/null 2>&1; then
    echo "  ERROR: qemu-system-x86_64 is not installed on this host!" >&2
    echo "  Please install with: sudo apt update && sudo apt install -y qemu-system-x86 ovmf" >&2
    exit 1
fi

OVMF_PATH=""
for path in \
    "/usr/share/OVMF/OVMF_CODE_4M.fd" \
    "/usr/share/OVMF/OVMF_CODE.fd" \
    "/usr/share/ovmf/OVMF.fd" \
    "/usr/share/qemu/OVMF.fd"; do
    if [ -f "$path" ]; then
        OVMF_PATH="$path"
        break
    fi
done

if [ -z "$OVMF_PATH" ]; then
    echo "  ERROR: OVMF UEFI firmware image not found!" >&2
    echo "  Please install with: sudo apt update && sudo apt install -y ovmf" >&2
    exit 1
fi

echo "  Target Disk Image: $DISK_IMG ($(stat -c %s "$DISK_IMG") bytes)"
echo "  UEFI Firmware:     $OVMF_PATH"
echo "  Network Policy:    Strict Offline (-net none)"
echo "  Serial Log Output: $SERIAL_LOG"

# 2. Execute automated interactive boot verifier
python3 "$SCRIPT_DIR/qemu_uefi_boot_verifier.py"
