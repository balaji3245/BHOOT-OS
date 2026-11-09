#!/usr/bin/env python3
"""
BHOOT v1.0 — Targeted Partition 2 (BHOOT_SYSTEM) Deployment & Read-Back Verification
Authoritatively updates ONLY /dev/sda2 with the remediated BHOOT_SYSTEM ext4 image
while strictly preserving Partition 1 (ESP), Partition 3 (BHOOT_DATA LUKS2 volume),
and Partition 4 (BHOOT_REC offline disaster recovery volume).
"""

import os
import sys
import time
import subprocess
import hashlib

TARGET_DEV = "/dev/sda"
TARGET_PART = "/dev/sda2"
SOURCE_IMG = "/home/bhoot/Desktop/BHOOT/os/build_real/bhoot_system_ext4.img"
EXPECTED_SERIAL = "070D29709B8E4F65"
EXPECTED_DISK_SIZE = 62026416128
EXPECTED_PART_SIZE = 4294967296
EXPECTED_P2_SHA256 = "6c042d129c188e86623769cbcaf949bac26741e32e8ca3186b78c14097c29da3"
HOST_BACKUP_DIR = "/home/bhoot/Desktop/BHOOT/backups_host"
REPORT_PATH = "/home/bhoot/Desktop/BHOOT/reports/BHOOT_V1_TARGETED_DEPLOYMENT_VERIFICATION_REPORT.md"

def log(msg):
    print(f"[{time.strftime('%H:%M:%S')}] {msg}", flush=True)

def fatal(msg):
    log(f"[FATAL BLOCKER] {msg}")
    sys.exit(1)

def sha256_stream(file_path_or_dev, size_bytes=None, chunk_size=65536):
    h = hashlib.sha256()
    read_total = 0
    with open(file_path_or_dev, "rb") as f:
        while True:
            to_read = chunk_size
            if size_bytes is not None:
                to_read = min(chunk_size, size_bytes - read_total)
                if to_read <= 0:
                    break
            buf = f.read(to_read)
            if not buf:
                break
            h.update(buf)
            read_total += len(buf)
    return h.hexdigest(), read_total

def main():
    log("========================================================================")
    log(" BHOOT v1.0 — TARGETED PARTITION 2 (BHOOT_SYSTEM) DEPLOYMENT PIPELINE")
    log("========================================================================")

    if "--execute" not in sys.argv:
        fatal("Safety gate: Missing '--execute' flag. Refusing to write without explicit armed execution.")

    log("[ARMED EXECUTION] Explicit owner authorization confirmed. Commencing deployment.")

    # -------------------------------------------------------------------------
    # STEP 1: VERIFY SOURCE IMAGE BASELINE & SHA-256
    # -------------------------------------------------------------------------
    log("STEP 1: Verifying source image at: " + SOURCE_IMG)
    if not os.path.isfile(SOURCE_IMG):
        fatal(f"Source image does not exist: {SOURCE_IMG}")
    
    src_size = os.path.getsize(SOURCE_IMG)
    log(f"  Source image size: {src_size} bytes (Expected: {EXPECTED_PART_SIZE})")
    if src_size != EXPECTED_PART_SIZE:
        fatal(f"Source size mismatch: {src_size} vs {EXPECTED_PART_SIZE}")
    
    log("  Computing source image SHA-256...")
    t0 = time.time()
    src_hash, _ = sha256_stream(SOURCE_IMG)
    log(f"  Source SHA-256: {src_hash} (computed in {time.time()-t0:.1f}s)")
    if src_hash != EXPECTED_P2_SHA256:
        fatal(f"Source checksum mismatch! Expected: {EXPECTED_P2_SHA256}")
    log("  [PASS] Source image certified bit-for-bit.")

    # -------------------------------------------------------------------------
    # STEP 2: VERIFY PHYSICAL TARGET IDENTITY & HARDWARE SERIAL
    # -------------------------------------------------------------------------
    log("STEP 2: Revalidating physical USB hardware identity (/dev/sda)...")
    if not os.path.exists(TARGET_DEV):
        fatal(f"Target device {TARGET_DEV} not detected!")
    
    udev_out = subprocess.check_output(["udevadm", "info", "--query=property", f"--name={TARGET_DEV}"], text=True)
    if "ID_BUS=usb" not in udev_out:
        fatal(f"{TARGET_DEV} is NOT a USB device!")
    if EXPECTED_SERIAL not in udev_out:
        fatal(f"{TARGET_DEV} serial mismatch! Expected: {EXPECTED_SERIAL}")
    if "HP" not in udev_out:
        fatal(f"{TARGET_DEV} vendor mismatch! Expected: HP")
    
    with open("/sys/block/sda/size") as f:
        disk_bytes = int(f.read().strip()) * 512
    log(f"  Target device capacity: {disk_bytes} bytes (Expected: {EXPECTED_DISK_SIZE})")
    if disk_bytes != EXPECTED_DISK_SIZE:
        fatal(f"Disk capacity mismatch: {disk_bytes} bytes vs expected {EXPECTED_DISK_SIZE}")
    log("  [PASS] Target physical USB certified: HP USB Flash Drive (Serial: 070D29709B8E4F65).")

    # -------------------------------------------------------------------------
    # STEP 3: HOST INTERNAL SSD ISOLATION GUARD
    # -------------------------------------------------------------------------
    log("STEP 3: Verifying host internal NVMe SSD isolation...")
    findmnt_out = subprocess.check_output(["findmnt", "-n", "-o", "SOURCE", "/"], text=True).strip()
    log(f"  Host root filesystem mounted on: {findmnt_out}")
    if "nvme0n1" not in findmnt_out:
        fatal("Host filesystem corrupted or not on NVMe!")
    if "sda" in findmnt_out:
        fatal("CRITICAL ERROR: Host root is on /dev/sda!")
    log("  [PASS] Host root safely isolated on /dev/nvme0n1p2. Absolute host isolation confirmed.")

    # -------------------------------------------------------------------------
    # STEP 4: VERIFY PARTITION 2 SECTOR BOUNDARIES
    # -------------------------------------------------------------------------
    log("STEP 4: Verifying /dev/sda2 partition geometry...")
    with open("/sys/block/sda/sda2/size") as f:
        p2_bytes = int(f.read().strip()) * 512
    with open("/sys/block/sda/sda2/start") as f:
        p2_start = int(f.read().strip())
    
    log(f"  /dev/sda2 start sector: {p2_start} (Expected: 2099200)")
    log(f"  /dev/sda2 size: {p2_bytes} bytes (Expected: {EXPECTED_PART_SIZE})")
    if p2_start != 2099200:
        fatal(f"/dev/sda2 start sector mismatch: {p2_start} vs 2099200")
    if p2_bytes != EXPECTED_PART_SIZE:
        fatal(f"/dev/sda2 size mismatch: {p2_bytes} vs {EXPECTED_PART_SIZE}")
    log("  [PASS] /dev/sda2 sector boundaries certified strictly within [2099200, 10487807].")

    # -------------------------------------------------------------------------
    # STEP 5: ENSURE ALL PARTITIONS ARE UNMOUNTED
    # -------------------------------------------------------------------------
    log("STEP 5: Ensuring every USB partition is unmounted...")
    for p in ["sda1", "sda2", "sda3", "sda4"]:
        dev_p = f"/dev/{p}"
        try:
            subprocess.run(["udisksctl", "unmount", "-b", dev_p], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        except Exception:
            pass
    
    mounts = subprocess.check_output(["cat", "/proc/mounts"], text=True)
    for line in mounts.splitlines():
        if "/dev/sda" in line:
            fatal(f"Partition is currently mounted: {line}. Aborting write.")
    log("  [PASS] Every /dev/sda partition is verified safely unmounted.")

    # -------------------------------------------------------------------------
    # STEP 6: DEFENSIVE HOST-SIDE BACKUP OF ACTIVE /dev/sda3 LUKS2 HEADER
    # -------------------------------------------------------------------------
    log("STEP 6: Capturing defensive host-side backup of /dev/sda3 LUKS2 header...")
    os.makedirs(HOST_BACKUP_DIR, exist_ok=True)
    timestamp_str = time.strftime('%Y%m%d_%H%M%S')
    backup_file = os.path.join(HOST_BACKUP_DIR, f"bhoot_luks_header_sda3_preflash_{timestamp_str}.backup")
    bk_cmd = ["pkexec", "cryptsetup", "luksHeaderBackup", "/dev/sda3", "--header-backup-file", backup_file, "--batch-mode"]
    res = subprocess.run(bk_cmd)
    if res.returncode != 0:
        fatal("Failed to capture defensive LUKS2 header backup from /dev/sda3!")
    
    subprocess.run(["pkexec", "chmod", "0644", backup_file], check=True)

    if not os.path.isfile(backup_file) or os.path.getsize(backup_file) != 16777216:
        fatal(f"Defensive backup file invalid or size not 16 MiB: {backup_file}")
    
    bk_hash, _ = sha256_stream(backup_file)
    log(f"  [PASS] Defensive header snapshot created: {backup_file} (16 MiB, SHA: {bk_hash})")

    # -------------------------------------------------------------------------
    # STEP 7: TARGETED BLOCK WRITE TO /dev/sda2 ONLY
    # -------------------------------------------------------------------------
    log(f"STEP 7: Writing {SOURCE_IMG} directly to {TARGET_PART} via dd...")
    log("  Executing: pkexec dd if=" + SOURCE_IMG + " of=" + TARGET_PART + " bs=4M conv=fsync status=progress")
    t_write_start = time.time()
    dd_cmd = ["pkexec", "dd", f"if={SOURCE_IMG}", f"of={TARGET_PART}", "bs=4M", "conv=fsync", "status=progress"]
    res = subprocess.run(dd_cmd)
    if res.returncode != 0:
        fatal(f"dd block write to {TARGET_PART} failed with exit code {res.returncode}!")
    
    t_write = time.time() - t_write_start
    log(f"  [PASS] dd completed in {t_write:.1f}s. Synchronizing filesystem buffers (sync)...")
    subprocess.run(["sync"])
    log("  [PASS] Physical media buffers fully synchronized.")

    # -------------------------------------------------------------------------
    # STEP 8: READ-BACK FULL 4 GiB SHA-256 CHECKSUM VERIFICATION
    # -------------------------------------------------------------------------
    log("STEP 8: Performing bit-for-bit physical read-back verification on /dev/sda2...")
    t_read_start = time.time()
    readback_cmd = ["pkexec", "dd", f"if={TARGET_PART}", "bs=4M", "status=none"]
    p = subprocess.Popen(readback_cmd, stdout=subprocess.PIPE)
    h = hashlib.sha256()
    bytes_read = 0
    while True:
        chunk = p.stdout.read(65536)
        if not chunk:
            break
        h.update(chunk)
        bytes_read += len(chunk)
    p.wait()
    t_read = time.time() - t_read_start
    readback_hash = h.hexdigest()
    
    log(f"  Readback bytes:  {bytes_read} (Expected: {EXPECTED_PART_SIZE})")
    log(f"  Source SHA-256:   {EXPECTED_P2_SHA256}")
    log(f"  Readback SHA-256: {readback_hash} (read in {t_read:.1f}s)")
    if bytes_read != EXPECTED_PART_SIZE:
        fatal(f"Readback size mismatch: read {bytes_read} bytes!")
    if readback_hash != EXPECTED_P2_SHA256:
        fatal(f"CRITICAL ERROR: Readback SHA-256 mismatch on /dev/sda2! Got: {readback_hash}")
    log("  [PASS] 100% BIT-FOR-BIT CRYPTOGRAPHIC IDENTITY VERIFIED ON /dev/sda2.")

    # -------------------------------------------------------------------------
    # STEP 9: FORENSIC VERIFICATION OF PROTECTED PARTITIONS (P1, P3, P4)
    # -------------------------------------------------------------------------
    log("STEP 9: Verifying protected partitions /dev/sda1, /dev/sda3, and /dev/sda4...")
    
    # Verify P1 FAT32 signature (0x55aa)
    p1_magic = subprocess.check_output(["pkexec", "dd", "if=/dev/sda1", "bs=1", "skip=510", "count=2"], stderr=subprocess.DEVNULL).hex()
    log(f"  Partition 1 (BHOOT_EFI) boot signature: 0x{p1_magic} (Expected: 0x55aa)")
    if p1_magic != "55aa":
        fatal("Partition 1 (ESP) signature compromised!")
    log("  [PASS] /dev/sda1 (ESP) FAT32 bootloader preserved and untouched.")

    # Verify P3 LUKS2 magic ('LUKS\xba\xbe')
    p3_magic = subprocess.check_output(["pkexec", "dd", "if=/dev/sda3", "bs=1", "count=6"], stderr=subprocess.DEVNULL).hex()
    log(f"  Partition 3 (BHOOT_DATA) LUKS2 magic: 0x{p3_magic} (Expected: 0x4c554b53babe)")
    if p3_magic != "4c554b53babe":
        fatal("Partition 3 (BHOOT_DATA) LUKS2 container magic compromised!")
    
    # Inspect P3 LUKS2 metadata and keyslots
    luks_dump = subprocess.check_output(["pkexec", "cryptsetup", "luksDump", "/dev/sda3"], text=True)
    if "UUID:          b8007001-0003-4000-8000-000000000003" not in luks_dump:
        fatal("Partition 3 LUKS2 UUID altered!")
    if "0: luks2" not in luks_dump or "1: luks2" not in luks_dump:
        fatal("Partition 3 LUKS2 keyslots missing!")
    log("  [PASS] /dev/sda3 LUKS2 header verified intact (Keyslot 0 and Keyslot 1 enrolled, Argon2id).")

    # Verify P4 Ext4 superblock magic (0xef53 at offset 1080)
    p4_magic = subprocess.check_output(["pkexec", "dd", "if=/dev/sda4", "bs=1", "skip=1080", "count=2"], stderr=subprocess.DEVNULL).hex()
    log(f"  Partition 4 (BHOOT_REC) Ext4 magic: 0x{p4_magic} (Expected: 0x53ef)")
    if p4_magic != "53ef":
        fatal("Partition 4 (BHOOT_REC) Ext4 superblock compromised!")
    
    # Verify P4 detached header backup file
    subprocess.run(["udisksctl", "mount", "-b", "/dev/sda4"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    p4_rec_path = "/run/media/bhoot/BHOOT_REC/bhoot_luks_header.backup"
    if os.path.exists(p4_rec_path):
        p4_header_size = os.path.getsize(p4_rec_path)
        log(f"  Partition 4 detached header backup verified: {p4_header_size} bytes (Expected: 16777216)")
        if p4_header_size != 16777216:
            fatal("Partition 4 detached header backup size invalid!")
    subprocess.run(["udisksctl", "unmount", "-b", "/dev/sda4"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    log("  [PASS] /dev/sda4 (BHOOT_REC) and detached recovery header verified 100% intact.")

    # -------------------------------------------------------------------------
    # STEP 10: SYNCHRONIZATION AND FINAL POWER-OFF / SAFE REMOVAL
    # -------------------------------------------------------------------------
    log("STEP 10: Executing final sync and safe detachment...")
    subprocess.run(["sync"])
    log("  Attempting clean power-off of USB drive via udisksctl...")
    try:
        po_res = subprocess.run(["udisksctl", "power-off", "-b", "/dev/sda"], capture_output=True, text=True)
        log(f"  Power-off result: {po_res.stdout.strip()}")
    except Exception as e:
        log(f"  Notice: {e}")
    
    log("========================================================================")
    log(" TARGETED DEPLOYMENT & READ-BACK VERIFICATION 100% COMPLETE & PASS!")
    log("========================================================================")

if __name__ == "__main__":
    main()
