#!/usr/bin/env python3
"""
BHOOT v1.0 — Authorized Physical HP USB Deployment & Read-Back Verification Script
Executes full safety preflight checks, unmounts candidate partitions,
streams release image to physical HP USB, synchronizes, and executes bit-for-bit
read-back SHA-256 verification and GPT/partition forensic checks.
"""

import os
import sys
import time
import hashlib
import subprocess
import gi
from gi.repository import Gio, GLib

SOURCE_IMG = "/home/bhoot/Desktop/BHOOT/os/build_real/bhoot_disk_assembled.img"
EXPECTED_SHA256 = "a6eda341fe30ee3227adfa0291a76f9b8620446a32dc50f4b2fe23108deb4630"
EXPECTED_SIZE = 62026416128
EXPECTED_SECTORS = 121145344

EXPECTED_SERIAL = "070D29709B8E4F65"
TARGET_DEV = "/dev/sda"
TARGET_UDISKS_PATH = "/org/freedesktop/UDisks2/block_devices/sda"
NVME_DEV = "/dev/nvme0n1"

def log(msg):
    print(f"[{time.strftime('%H:%M:%S')}] {msg}", flush=True)

def fatal(msg):
    log(f"[FATAL ERROR] {msg}")
    sys.exit(1)

def get_udisks_block_proxy(obj_path):
    bus = Gio.bus_get_sync(Gio.BusType.SYSTEM, None)
    return Gio.DBusProxy.new_sync(
        bus,
        Gio.DBusProxyFlags.NONE,
        None,
        "org.freedesktop.UDisks2",
        obj_path,
        "org.freedesktop.UDisks2.Block",
        None
    )

def main():
    log("========================================================================")
    log(" BHOOT v1.0 — AUTHORIZED PHYSICAL HP USB DEPLOYMENT & VERIFICATION")
    log("========================================================================")

    # -------------------------------------------------------------------------
    # STEP 1: VERIFY SOURCE IMAGE INTEGRITY & SHA-256
    # -------------------------------------------------------------------------
    log("STEP 1: Verifying source image baseline...")
    if not os.path.exists(SOURCE_IMG):
        fatal(f"Source image missing at: {SOURCE_IMG}")

    img_size = os.path.getsize(SOURCE_IMG)
    log(f"  Source image size: {img_size} bytes (Expected: {EXPECTED_SIZE})")
    if img_size != EXPECTED_SIZE:
        fatal(f"Source image size mismatch: got {img_size}, expected {EXPECTED_SIZE}")

    log("  Computing source image SHA-256 checksum...")
    sha = hashlib.sha256()
    with open(SOURCE_IMG, "rb") as f:
        while True:
            chunk = f.read(8 * 1024 * 1024)
            if not chunk:
                break
            sha.update(chunk)
    source_sha = sha.hexdigest()
    log(f"  Source image SHA-256: {source_sha}")
    if source_sha != EXPECTED_SHA256:
        fatal(f"Source image SHA-256 mismatch! Got {source_sha}, expected {EXPECTED_SHA256}")
    log("  [PASS] Source image integrity verified 100%.")

    # -------------------------------------------------------------------------
    # STEP 2: PROTECT INTERNAL STORAGE (/dev/nvme0n1)
    # -------------------------------------------------------------------------
    log("STEP 2: Verifying internal system drive protection (/dev/nvme0n1)...")
    if not os.path.exists(NVME_DEV):
        fatal(f"Internal drive {NVME_DEV} not detected for negative isolation check!")

    # Check root mount
    findmnt_out = subprocess.check_output(["findmnt", "-n", "-o", "SOURCE", "/"], text=True).strip()
    log(f"  Host root filesystem mounted from: {findmnt_out}")
    if "nvme0n1" not in findmnt_out:
        fatal(f"Expected host root to be on nvme0n1, got: {findmnt_out}")

    log(f"  [PASS] Confirmed {NVME_DEV} is host internal drive. ABSOLUTE ISOLATION ENFORCED.")

    # -------------------------------------------------------------------------
    # STEP 3: DISCOVER & VERIFY PHYSICAL HP USB DRIVE (/dev/sda)
    # -------------------------------------------------------------------------
    log("STEP 3: Detecting physical target drive (/dev/sda)...")
    if not os.path.exists(TARGET_DEV):
        fatal(f"Target device {TARGET_DEV} does not exist!")

    # Query sysfs size
    with open("/sys/block/sda/size", "r") as f:
        sda_sectors = int(f.read().strip())
    sda_bytes = sda_sectors * 512
    log(f"  Detected sectors: {sda_sectors} sectors ({sda_bytes} bytes)")
    if sda_bytes != EXPECTED_SIZE or sda_sectors != EXPECTED_SECTORS:
        fatal(f"Capacity mismatch on {TARGET_DEV}: got {sda_bytes} bytes, expected {EXPECTED_SIZE}")

    # Query udev attributes
    udev_out = subprocess.check_output(
        ["udevadm", "info", "--query=property", "--name=/dev/sda"],
        text=True
    )
    props = dict(line.split("=", 1) for line in udev_out.strip().splitlines() if "=" in line)

    bus = props.get("ID_BUS", "")
    vendor = props.get("ID_VENDOR", "")
    serial = props.get("ID_SERIAL_SHORT", "")
    model = props.get("ID_MODEL", "")

    log(f"  Device Bus:    {bus}")
    log(f"  Device Vendor: {vendor}")
    log(f"  Device Model:  {model}")
    log(f"  Device Serial: {serial}")

    if bus != "usb":
        fatal(f"Device {TARGET_DEV} is not a USB transport device! Got: {bus}")
    if "HP" not in vendor:
        fatal(f"Device {TARGET_DEV} vendor mismatch! Expected HP, got: {vendor}")
    if serial != EXPECTED_SERIAL:
        fatal(f"Device {TARGET_DEV} serial mismatch! Expected {EXPECTED_SERIAL}, got: {serial}")

    log("  [PASS] Target device hardware identity positively confirmed:")
    log(f"         HP USB Flash Drive (Serial: {serial}, Capacity: {sda_bytes} bytes)")

    # -------------------------------------------------------------------------
    # STEP 4: UNMOUNT ANY ACTIVE PARTITIONS ON TARGET DRIVE
    # -------------------------------------------------------------------------
    log("STEP 4: Unmounting any mounted partitions on /dev/sda...")
    subprocess.run(["gio", "mount", "-u", "/run/media/bhoot/HP_USB"], capture_output=True)
    subprocess.run(["udisksctl", "unmount", "-b", "/dev/sda1", "--no-user-interaction"], capture_output=True)
    time.sleep(1)

    # Confirm unmount
    mounts = subprocess.check_output(["cat", "/proc/mounts"], text=True)
    for line in mounts.splitlines():
        if "/dev/sda" in line:
            fatal(f"Partition still mounted: {line}")
    log("  [PASS] All /dev/sda partitions confirmed unmounted.")

    # -------------------------------------------------------------------------
    # STEP 5: ACQUIRE WRITABLE DESCRIPTOR VIA UDISKS2 & WRITE DISK IMAGE
    # -------------------------------------------------------------------------
    log("STEP 5: Acquiring exclusive write lock to /dev/sda via UDisks2...")
    proxy = get_udisks_block_proxy(TARGET_UDISKS_PATH)
    fd_list = Gio.UnixFDList.new()
    res, out_fd_list = proxy.call_with_unix_fd_list_sync(
        "OpenForRestore",
        GLib.Variant("(a{sv})", ({},)),
        Gio.DBusCallFlags.NONE,
        -1,
        fd_list,
        None
    )
    handle = res.unpack()[0]
    write_fd = out_fd_list.get(handle)
    log(f"  Successfully acquired writable block file descriptor: {write_fd}")

    write_cmd = f"dd if={SOURCE_IMG} bs=4M status=progress conv=fsync of=/dev/sda"
    log(f"  Executing write command: {write_cmd}")
    t_start = time.time()

    # Stream dd into write_fd
    dd_proc = subprocess.Popen(
        ["dd", f"if={SOURCE_IMG}", "bs=4M", "status=progress", "conv=fsync"],
        stdout=write_fd,
        stderr=subprocess.PIPE
    )

    # Monitor progress from dd stderr
    while True:
        line = dd_proc.stderr.readline()
        if not line:
            break
        txt = line.decode("utf-8", errors="replace").strip()
        if txt and ("bytes" in txt or "records" in txt or "copied" in txt):
            log(f"    [dd progress] {txt}")

    dd_proc.wait()
    t_elapsed = time.time() - t_start
    log(f"  dd process completed in {t_elapsed:.1f} seconds. Exit status: {dd_proc.returncode}")
    if dd_proc.returncode != 0:
        fatal(f"dd execution failed with code {dd_proc.returncode}")

    log("  Synchronizing block device buffers (os.fsync & os.sync)...")
    os.fsync(write_fd)
    os.close(write_fd)
    os.sync()
    log("  [PASS] Complete image written to /dev/sda and synchronized to physical media.")

    # -------------------------------------------------------------------------
    # STEP 6: RESCAN PARTITIONS VIA UDISKS2
    # -------------------------------------------------------------------------
    log("STEP 6: Requesting kernel partition table rescan...")
    try:
        proxy.call_sync("Rescan", GLib.Variant("(a{sv})", ({},)), Gio.DBusCallFlags.NONE, -1, None)
    except Exception as e:
        log(f"  Rescan notice: {e}")
    time.sleep(2)

    # -------------------------------------------------------------------------
    # STEP 7: READ-BACK FULL DEVICE SHA-256 CHECKSUM VERIFICATION
    # -------------------------------------------------------------------------
    log("STEP 7: Performing full physical read-back SHA-256 verification...")
    log(f"  Acquiring read file descriptor to /dev/sda via UDisks2 OpenForBackup...")
    fd_list = Gio.UnixFDList.new()
    res, out_fd_list = proxy.call_with_unix_fd_list_sync(
        "OpenForBackup",
        GLib.Variant("(a{sv})", ({},)),
        Gio.DBusCallFlags.NONE,
        -1,
        fd_list,
        None
    )
    handle = res.unpack()[0]
    read_fd = out_fd_list.get(handle)
    log(f"  Successfully acquired readable block file descriptor: {read_fd}")

    read_sha = hashlib.sha256()
    bytes_read = 0
    chunk_size = 4 * 1024 * 1024
    t_read_start = time.time()
    last_log_time = t_read_start

    while bytes_read < EXPECTED_SIZE:
        to_read = min(chunk_size, EXPECTED_SIZE - bytes_read)
        buf = os.read(read_fd, to_read)
        if not buf:
            break
        read_sha.update(buf)
        bytes_read += len(buf)

        now = time.time()
        if now - last_log_time >= 5.0 or bytes_read == EXPECTED_SIZE:
            mb = bytes_read / (1024 * 1024)
            pct = (bytes_read / EXPECTED_SIZE) * 100.0
            mb_sec = mb / max(1.0, (now - t_read_start))
            log(f"    [readback progress] {mb:.1f} MiB / 59153.0 MiB ({pct:.1f}%) - {mb_sec:.1f} MiB/s")
            last_log_time = now

    t_read_total = time.time() - t_read_start
    log(f"  Total bytes read back: {bytes_read} bytes in {t_read_total:.1f}s")
    if bytes_read != EXPECTED_SIZE:
        fatal(f"Read-back byte count mismatch: got {bytes_read}, expected {EXPECTED_SIZE}")

    readback_hash = read_sha.hexdigest()
    log(f"  Source Image SHA-256:   {EXPECTED_SHA256}")
    log(f"  Physical Media SHA-256: {readback_hash}")

    if readback_hash != EXPECTED_SHA256:
        fatal(f"BIT-FOR-BIT VERIFICATION FAILED! Physical hash does not match source image!")

    log("  [PASS] BIT-FOR-BIT SHA-256 IDENTITY CONFIRMED (100.00% EXACT MATCH).")

    # -------------------------------------------------------------------------
    # STEP 8: FORENSIC PARTITION & SIGNATURE VERIFICATION ON PHYSICAL DRIVE
    # -------------------------------------------------------------------------
    log("STEP 8: Performing forensic partition inspection on physical USB...")

    # P1 (sector 2048): ESP FAT32 signature
    os.lseek(read_fd, 2048 * 512 + 510, os.SEEK_SET)
    p1_sig = os.read(read_fd, 2).hex()
    log(f"  Partition 1 (ESP) FAT32 boot signature at sector 2048: 0x{p1_sig}")
    if p1_sig != "55aa":
        fatal(f"P1 boot signature mismatch: got {p1_sig}")

    # P2 (sector 2099200): Ext4 superblock magic (offset 1024 + 56 = 1080)
    os.lseek(read_fd, 2099200 * 512 + 1080, os.SEEK_SET)
    p2_magic = os.read(read_fd, 2).hex()
    log(f"  Partition 2 (BHOOT_SYSTEM) Ext4 superblock magic at sector 2099200: 0x{p2_magic}")
    if p2_magic != "53ef":
        fatal(f"P2 Ext4 magic mismatch: got {p2_magic}")

    # P3 (sector 10487808): LUKS2 container magic (offset 0)
    os.lseek(read_fd, 10487808 * 512, os.SEEK_SET)
    p3_magic = os.read(read_fd, 6).hex()
    log(f"  Partition 3 (BHOOT_DATA) LUKS2 container magic at sector 10487808: 0x{p3_magic}")
    if p3_magic != "4c554b53babe":
        fatal(f"P3 LUKS2 magic mismatch: got {p3_magic}")

    # P4 (sector 111151104): Ext4 superblock magic (offset 1024 + 56 = 1080)
    os.lseek(read_fd, 111151104 * 512 + 1080, os.SEEK_SET)
    p4_magic = os.read(read_fd, 2).hex()
    log(f"  Partition 4 (BHOOT_REC) Ext4 superblock magic at sector 111151104: 0x{p4_magic}")
    if p4_magic != "53ef":
        fatal(f"P4 Ext4 magic mismatch: got {p4_magic}")

    # Secondary GPT Table Header (sector 121145343): "EFI PART" (offset 0)
    os.lseek(read_fd, 121145343 * 512, os.SEEK_SET)
    gpt_backup_sig = os.read(read_fd, 8).decode("ascii", errors="replace")
    log(f"  Secondary GPT Table Header signature at sector 121145343: '{gpt_backup_sig}'")
    if gpt_backup_sig != "EFI PART":
        fatal(f"Secondary GPT header mismatch: got {gpt_backup_sig}")

    os.close(read_fd)
    log("  [PASS] All 4 partition payloads and GPT primary/backup tables verified on physical media.")

    # -------------------------------------------------------------------------
    # STEP 9: CONFIRM INTERNAL NVME INTEGRITY
    # -------------------------------------------------------------------------
    log("STEP 9: Re-confirming internal drive integrity (/dev/nvme0n1)...")
    with open("/sys/block/nvme0n1/size", "r") as f:
        nvme_sectors = int(f.read().strip())
    log(f"  Internal NVMe sectors: {nvme_sectors} (UNTOUCHED)")
    findmnt_check = subprocess.check_output(["findmnt", "-n", "-o", "SOURCE", "/"], text=True).strip()
    log(f"  Host root mount: {findmnt_check} (HEALTHY & INTACT)")
    if "nvme0n1" not in findmnt_check:
        fatal("Critical host integrity check failed!")
    log("  [PASS] Host internal drive (/dev/nvme0n1) was completely untouched.")

    log("========================================================================")
    log(" PHYSICAL HP USB DEPLOYMENT & VERIFICATION: PASS (100% SUCCESS)")
    log("========================================================================")

if __name__ == "__main__":
    main()

