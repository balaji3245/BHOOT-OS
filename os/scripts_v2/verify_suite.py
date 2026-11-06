#!/usr/bin/env python3
"""
BHOOT Phase 2A.4 - Forensic Verification Suite & Artifact Manifest Generator
Performs rigorous cryptographic and structural verification of all Phase 2A.4 OS artifacts.
"""

import hashlib
import json
import os
import struct
import subprocess
import sys

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
OS_DIR = os.path.dirname(SCRIPT_DIR)
BUILD_REAL = os.path.join(OS_DIR, "build_real")
ROOTFS = os.path.join(BUILD_REAL, "rootfs")
MANIFEST_FILE = os.path.join(BUILD_REAL, "manifest.json")

passed = 0
failed = 0

def check(description: str, condition: bool, details: str = ""):
    global passed, failed
    if condition:
        print(f"  [PASS] {description}")
        if details:
            print(f"         └─ {details}")
        passed += 1
    else:
        print(f"  [FAIL] {description}")
        if details:
            print(f"         └─ ERROR: {details}")
        failed += 1

def hash_file(path: str, max_bytes: int = None) -> str:
    h = hashlib.sha256()
    bytes_read = 0
    with open(path, "rb") as f:
        while chunk := f.read(4 * 1024 * 1024):
            if max_bytes is not None and bytes_read + len(chunk) > max_bytes:
                h.update(chunk[:max_bytes - bytes_read])
                break
            h.update(chunk)
            bytes_read += len(chunk)
    return h.hexdigest()

print("=" * 72)
print(" [Phase 2A.4 - Step 07] Forensic Verification Suite & Manifest")
print("=" * 72)

# -----------------------------------------------------------------------------
# 1. Kernel Binary Verification
# -----------------------------------------------------------------------------
print("\n--- 1. Linux Kernel Binary Verification ---")
kernel_path = os.path.join(ROOTFS, "boot", "vmlinuz-6.1.0-50-amd64")
if os.path.isfile(kernel_path):
    ksize = os.path.getsize(kernel_path)
    check("Kernel binary exists and size >= 7 MiB", ksize >= 7 * 1024 * 1024, f"{ksize} bytes")

    with open(kernel_path, "rb") as f:
        f.seek(0x202)
        magic = f.read(4)
        check("Kernel boot protocol magic is 'HdrS' (offset 0x202)", magic == b"HdrS", f"Found: {magic}")
        f.seek(0x206)
        version_bytes = f.read(2)
        protocol = struct.unpack("<H", version_bytes)[0]
        check("Kernel protocol version >= 2.15 (0x020f)", protocol >= 0x020f, f"Protocol version: 0x{protocol:04x}")

    res = subprocess.run(["file", kernel_path], capture_output=True, text=True)
    check("Kernel matches x86 bzImage format", "bzImage" in res.stdout and "x86" in res.stdout, res.stdout.strip())
else:
    check("Kernel binary exists", False, f"File not found: {kernel_path}")

# -----------------------------------------------------------------------------
# 2. Initial Ramdisk (Initramfs) Verification
# -----------------------------------------------------------------------------
print("\n--- 2. Initial Ramdisk Verification ---")
initrd_path = os.path.join(ROOTFS, "boot", "initrd.img-6.1.0-50-amd64")
if os.path.isfile(initrd_path):
    isize = os.path.getsize(initrd_path)
    check("Initramfs archive exists and size >= 40 MiB", isize >= 40 * 1024 * 1024, f"{isize} bytes")

    with open(initrd_path, "rb") as f:
        magic = f.read(2)
        check("Initramfs compression magic is gzip (0x1f8b)", magic == b"\x1f\x8b", f"Found: {magic.hex()}")

    res = subprocess.run(["lsinitramfs", initrd_path], capture_output=True, text=True)
    check("Initramfs contains live-boot framework hooks", "scripts/live" in res.stdout or "live" in res.stdout)
    check("Initramfs contains BHOOT dedicated discovery hook", "9991-bhoot-discovery" in res.stdout)
    check("Initramfs contains cryptsetup userspace binary", "cryptsetup" in res.stdout)
    check("Initramfs contains dm-crypt kernel module", "dm-crypt" in res.stdout or "aesni" in res.stdout)
else:
    check("Initramfs archive exists", False, f"File not found: {initrd_path}")

# -----------------------------------------------------------------------------
# 3. SquashFS System Image Verification
# -----------------------------------------------------------------------------
print("\n--- 3. SquashFS System Image Verification ---")
squashfs_path = os.path.join(BUILD_REAL, "filesystem.squashfs")
if os.path.isfile(squashfs_path):
    ssize = os.path.getsize(squashfs_path)
    check("SquashFS image exists and size >= 100 MiB", ssize >= 100 * 1024 * 1024, f"{ssize} bytes (~{ssize // (1024*1024)} MiB)")

    with open(squashfs_path, "rb") as f:
        magic = f.read(4)
        check("SquashFS magic is 0x73717368 ('hsqs')", magic == b"hsqs", f"Found: {magic.hex()}")

    res = subprocess.run(["unsquashfs", "-l", squashfs_path], capture_output=True, text=True)
    check("SquashFS contains systemd init (/sbin/init)", "sbin/init" in res.stdout)
    check("SquashFS contains bash (/bin/bash)", "bin/bash" in res.stdout)
    check("SquashFS contains cryptsetup (/sbin/cryptsetup)", "sbin/cryptsetup" in res.stdout)
    check("SquashFS contains nftables (/usr/sbin/nft)", "usr/sbin/nft" in res.stdout)
    check("SquashFS contains rfkill (/usr/sbin/rfkill)", "usr/sbin/rfkill" in res.stdout)
else:
    check("SquashFS image exists", False, f"File not found: {squashfs_path}")

# -----------------------------------------------------------------------------
# 3B. BHOOT_SYSTEM Ext4 Partition Image Verification
# -----------------------------------------------------------------------------
print("\n--- 3B. BHOOT_SYSTEM Ext4 Partition Image Verification ---")
sys_ext4_path = os.path.join(BUILD_REAL, "bhoot_system_ext4.img")
if os.path.isfile(sys_ext4_path):
    sys_size = os.path.getsize(sys_ext4_path)
    check("BHOOT_SYSTEM Ext4 image size is exactly 4 GiB", sys_size == 4294967296, f"{sys_size} bytes")

    with open(sys_ext4_path, "rb") as f:
        f.seek(1080)
        magic = f.read(2)
        check("BHOOT_SYSTEM has valid Ext4 superblock magic (0xef53)", magic == b"\x53\xef", f"Found: {magic.hex()}")

    tune_res = subprocess.run(["tune2fs", "-l", sys_ext4_path], capture_output=True, text=True)
    check("BHOOT_SYSTEM volume label is BHOOT_SYSTEM", "BHOOT_SYSTEM" in tune_res.stdout)
    check("BHOOT_SYSTEM UUID is b8007001-0002-4000-8000-000000000002", "b8007001-0002-4000-8000-000000000002" in tune_res.stdout)
else:
    check("BHOOT_SYSTEM Ext4 image exists", False)

# -----------------------------------------------------------------------------
# 4. EFI System Partition & Bootloader Verification
# -----------------------------------------------------------------------------
print("\n--- 4. EFI System Partition Verification ---")
esp_path = os.path.join(BUILD_REAL, "bhoot_esp.img")
bootx64_path = os.path.join(BUILD_REAL, "BOOTX64.EFI")

if os.path.isfile(esp_path) and os.path.isfile(bootx64_path):
    esize = os.path.getsize(esp_path)
    check("ESP image size is exactly 1 GiB", esize == 1024 * 1024 * 1024, f"{esize} bytes")

    with open(esp_path, "rb") as f:
        f.seek(510)
        sig = f.read(2)
        check("ESP FAT32 boot sector signature is 0x55aa", sig == b"\x55\xaa", f"Found: {sig.hex()}")

    res = subprocess.run(["file", bootx64_path], capture_output=True, text=True)
    check("BOOTX64.EFI is PE32+ UEFI executable", "PE32+ executable" in res.stdout and "EFI" in res.stdout, res.stdout.strip())
else:
    check("ESP image and BOOTX64.EFI exist", False)

# -----------------------------------------------------------------------------
# 5. LUKS2 Storage Verification
# -----------------------------------------------------------------------------
print("\n--- 5. LUKS2 Persistent Storage Verification ---")
luks_prod = os.path.join(BUILD_REAL, "bhoot_luks_prod_48g.img")
luks_backup = os.path.join(BUILD_REAL, "bhoot_luks_header.backup")
luks_test = os.path.join(BUILD_REAL, "bhoot_luks_test_64m.img")

if os.path.isfile(luks_prod) and os.path.isfile(luks_backup) and os.path.isfile(luks_test):
    res_prod = subprocess.run(["cryptsetup", "isLuks", "--type", "luks2", luks_prod])
    check("Production container is valid LUKS2", res_prod.returncode == 0)

    res_bak = subprocess.run(["cryptsetup", "isLuks", "--type", "luks2", luks_backup])
    check("Detached header backup is valid LUKS2", res_bak.returncode == 0)

    res_test = subprocess.run(["cryptsetup", "isLuks", "--type", "luks2", luks_test])
    check("Test vector container is valid LUKS2", res_test.returncode == 0)

    dump_res = subprocess.run(["cryptsetup", "luksDump", luks_prod], capture_output=True, text=True)
    dump_out = dump_res.stdout
    check("PBKDF algorithm is Argon2id", "argon2id" in dump_out)
    check("Cipher is AES-256 in XTS mode (aes-xts-plain64, 512-bit key)", "aes-xts-plain64" in dump_out and "512 bits" in dump_out)
    check("Storage sector size is 4096 bytes", "sector: 4096" in dump_out)
    check("Both Keyslot 0 and Keyslot 1 enrolled", "0: luks2" in dump_out and "1: luks2" in dump_out)
else:
    check("LUKS2 containers exist", False)

# -----------------------------------------------------------------------------
# 6. Assembled Whole Disk Image Verification
# -----------------------------------------------------------------------------
print("\n--- 6. Assembled Whole Disk Image Verification ---")
disk_path = os.path.join(BUILD_REAL, "bhoot_disk_assembled.img")
if os.path.isfile(disk_path):
    dsize = os.path.getsize(disk_path)
    check("Disk image size matches 57.81 GiB geometry (62,075,699,200 bytes)", dsize == 62075699200, f"{dsize} bytes")

    sf_res = subprocess.run(["sfdisk", "--verify", disk_path], capture_output=True, text=True)
    check("GPT partition table has no errors", "No errors detected" in sf_res.stdout or sf_res.returncode == 0)

    # Forensic checks at exact partition start offsets
    with open(disk_path, "rb") as f:
        # Partition 1: Sector 2048 (offset 1,048,576)
        f.seek(2048 * 512 + 510)
        p1_sig = f.read(2)
        check("Partition 1 (ESP) FAT32 boot signature (0x55aa) at sector 2048", p1_sig == b"\x55\xaa", f"Found: {p1_sig.hex()}")

        # Partition 2: Sector 2099200 (offset 1,074,790,400)
        # Ext4 Superblock is at offset 1024, magic 0xef53 is at offset 1080 of partition
        f.seek(2099200 * 512 + 1080)
        p2_sb_magic = f.read(2)
        check("Partition 2 (BHOOT_SYSTEM) Ext4 superblock magic (0xef53) at sector 2099200", p2_sb_magic == b"\x53\xef", f"Found: {p2_sb_magic.hex()}")

        # Partition 3: Sector 10487808 (offset 5,369,757,696)
        f.seek(10487808 * 512)
        p3_magic = f.read(6)
        check("Partition 3 (BHOOT_DATA) LUKS2 magic ('LUKS\\xba\\xbe') at sector 10487808", p3_magic == b"LUKS\xba\xbe", f"Found: {p3_magic.hex()}")

        # Partition 4: Sector 111151104 (offset 56,909,365,248)
        f.seek(111151104 * 512 + 1080)
        p4_sb_magic = f.read(2)
        check("Partition 4 (BHOOT_RECOVERY) Ext4 superblock magic (0xef53) at sector 111151104", p4_sb_magic == b"\x53\xef", f"Found: {p4_sb_magic.hex()}")
else:
    check("Assembled disk image exists", False, f"File not found: {disk_path}")

# -----------------------------------------------------------------------------
# 7. Security Hardening Configuration Verification
# -----------------------------------------------------------------------------
print("\n--- 7. Security Policies & Invariants Verification ---")
crypttab_path = os.path.join(ROOTFS, "etc", "crypttab")
if os.path.isfile(crypttab_path):
    with open(crypttab_path, "r") as f:
        content = f.read()
        active_lines = [line.strip() for line in content.splitlines() if line.strip() and not line.strip().startswith("#")]
        check("Crypttab strictly omits 'discard' in active rules (prevents metadata leakage)", all("discard" not in line for line in active_lines))
        check("Crypttab specifies UUID and no workqueue options", any("B8007001-0003-4000-8000-000000000003" in line and "no-read-workqueue" in line for line in active_lines))
else:
    check("Crypttab exists in rootfs", False)

nft_path = os.path.join(ROOTFS, "etc", "nftables.conf")
if os.path.isfile(nft_path):
    with open(nft_path, "r") as f:
        content = f.read()
        check("NFTables firewall default policy is drop for all chains", content.count("policy drop") >= 3)
        check("NFTables permits loopback interface internal IPC", 'iifname "lo" accept' in content and 'oifname "lo" accept' in content)
else:
    check("NFTables configuration exists in rootfs", False)

journal_path = os.path.join(ROOTFS, "etc", "systemd", "journald.conf.d", "volatile.conf")
if os.path.isfile(journal_path):
    with open(journal_path, "r") as f:
        content = f.read()
        check("Journald storage configured as volatile RAM-only", "Storage=volatile" in content)
else:
    check("Journald volatile configuration exists in rootfs", False)

sysctl_offline = os.path.join(ROOTFS, "etc", "sysctl.d", "99-bhoot-offline.conf")
if os.path.isfile(sysctl_offline):
    with open(sysctl_offline, "r") as f:
        content = f.read()
        check("Kernel sysctl disables IPv6 across all interfaces", "net.ipv6.conf.all.disable_ipv6 = 1" in content)
        check("Kernel sysctl disables IP forwarding and ping response", "net.ipv4.ip_forward = 0" in content and "net.ipv4.icmp_echo_ignore_all = 1" in content)
else:
    check("Offline sysctl exists in rootfs", False)

sysctl_power = os.path.join(ROOTFS, "etc", "sysctl.d", "98-bhoot-powerloss.conf")
if os.path.isfile(sysctl_power):
    with open(sysctl_power, "r") as f:
        content = f.read()
        check("Kernel sysctl configures aggressive flash dirty writeback", "vm.dirty_background_ratio = 5" in content and "vm.dirty_ratio = 10" in content)
else:
    check("Powerloss sysctl exists in rootfs", False)

# -----------------------------------------------------------------------------
# 8. Cryptographic Manifest Generation
# -----------------------------------------------------------------------------
print("\n--- 8. Cryptographic Manifest Generation ---")
artifacts_meta = [
    {
        "filename": "filesystem.squashfs",
        "description": "Read-only compressed Debian 12 Bookworm root filesystem with security hardening",
        "role": "system_root",
        "hash_mode": "full"
    },
    {
        "filename": "bhoot_system_ext4.img",
        "description": "4 GiB sparse Ext4 partition containing /live/filesystem.squashfs with canonical UUID",
        "role": "system_partition",
        "hash_mode": "gpt_header_2m"
    },
    {
        "filename": "BOOTX64.EFI",
        "description": "Standalone GRUB2 UEFI x86_64 bootloader executable",
        "role": "bootloader",
        "hash_mode": "full"
    },
    {
        "filename": "bhoot_esp.img",
        "description": "1 GiB FAT32 EFI System Partition containing bootloader, kernel, initramfs, squashfs, and grub.cfg",
        "role": "esp_partition",
        "hash_mode": "full"
    },
    {
        "filename": "bhoot_luks_prod_48g.img",
        "description": "Complete 48 GiB sparse LUKS2 volume (Argon2id + AES-256-XTS) with formatted Ext4 persistence",
        "role": "encrypted_persistence_prod",
        "hash_mode": "header_16m"
    },
    {
        "filename": "bhoot_luks_test_64m.img",
        "description": "64 MiB test-vector container for cryptographic verification and dm-crypt activation",
        "role": "encrypted_persistence_test",
        "hash_mode": "full"
    },
    {
        "filename": "bhoot_luks_header.backup",
        "description": "16 MiB detached binary backup of production LUKS2 header structure",
        "role": "header_backup",
        "hash_mode": "full"
    },
    {
        "filename": "bhoot_recovery.img",
        "description": "32 MiB ext4 offline disaster recovery image containing header backup and recovery tools",
        "role": "recovery_partition",
        "hash_mode": "full"
    },
    {
        "filename": "bhoot_disk_assembled.img",
        "description": "57.81 GiB whole sparse disk image with complete GPT table and all four partition payloads",
        "role": "whole_disk_image",
        "hash_mode": "gpt_header_2m"
    }
]

manifest_data = {
    "project": "BHOOT",
    "phase": "2A.4",
    "target_platform": "Dedicated Bootable USB (UEFI x86_64)",
    "base_distribution": "Debian 12 Bookworm amd64",
    "kernel": "Linux 6.1.0-50-amd64",
    "initramfs": "Debian live-boot 20230131 + cryptsetup 2.6.1",
    "artifacts": []
}

for item in artifacts_meta:
    path = os.path.join(BUILD_REAL, item["filename"])
    if os.path.exists(path):
        st = os.stat(path)
        if item["hash_mode"] == "full":
            digest = hash_file(path)
            digest_field = "sha256"
        elif item["hash_mode"] == "header_16m":
            digest = hash_file(path, max_bytes=16 * 1024 * 1024)
            digest_field = "header_16m_sha256"
        elif item["hash_mode"] == "gpt_header_2m":
            digest = hash_file(path, max_bytes=2 * 1024 * 1024)
            digest_field = "gpt_header_2m_sha256"

        entry = {
            "filename": item["filename"],
            "role": item["role"],
            "description": item["description"],
            "logical_size_bytes": st.st_size,
            "allocated_size_bytes": st.st_blocks * 512,
            digest_field: digest
        }
        manifest_data["artifacts"].append(entry)
        print(f"  Checksum: {item['filename']} -> {digest[:16]}... ({st.st_size} bytes)")

with open(MANIFEST_FILE, "w") as out:
    json.dump(manifest_data, out, indent=2)

check(f"Manifest written to {MANIFEST_FILE}", os.path.isfile(MANIFEST_FILE))

# Verify zero temporary session key files exist on disk
key_scratch_files = [f for f in os.listdir(BUILD_REAL) if f.startswith(".session") or f.endswith(".key")]
check("Zero temporary plaintext session key files exist on disk", len(key_scratch_files) == 0)

print("\n" + "=" * 72)
if failed == 0:
    print(f" [Phase 2A.6 - Step 07] ALL {passed} FORENSIC VERIFICATIONS PASSED!")
    print("=" * 72)
    sys.exit(0)
else:
    print(f" [Phase 2A.3 - Step 07] {failed} VERIFICATIONS FAILED ({passed} passed)!")
    print("=" * 72)
    sys.exit(1)
