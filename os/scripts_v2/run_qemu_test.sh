#!/usr/bin/env bash
# BHOOT Phase 2A.3 - Automated UEFI Virtualized Boot Test Harness
# Launches QEMU with OVMF UEFI firmware, zero networking, and logs boot sequence.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OS_DIR="$(dirname "$SCRIPT_DIR")"
BUILD_REAL="$OS_DIR/build_real"

DISK_IMG="$BUILD_REAL/bhoot_disk_assembled.img"
SERIAL_LOG="$BUILD_REAL/qemu_boot_serial.log"

echo "========================================================================"
echo " [Phase 2A.3] BHOOT UEFI Virtualized Boot Test Harness"
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

rm -f "$SERIAL_LOG"

# 2. Launch QEMU with timeout
echo "  Launching QEMU UEFI virtual machine (60 second execution window)..."
timeout 60s qemu-system-x86_64 \
    -machine q35,accel=kvm:tcg \
    -m 4096 \
    -smp 4 \
    -drive if=pflash,format=raw,readonly=on,file="$OVMF_PATH" \
    -drive file="$DISK_IMG",format=raw,if=virtio \
    -net none \
    -display none \
    -serial file:"$SERIAL_LOG" \
    -monitor none || true

echo "  QEMU test run concluded. Inspecting serial boot logs..."
if [ -f "$SERIAL_LOG" ] && [ -s "$SERIAL_LOG" ]; then
    echo "  Serial log captured ($(wc -l < "$SERIAL_LOG") lines, $(stat -c %s "$SERIAL_LOG") bytes):"
    head -n 25 "$SERIAL_LOG"
    echo "  [...]"
    tail -n 25 "$SERIAL_LOG"
else
    echo "  WARNING: Serial log is empty or was not generated."
fi
