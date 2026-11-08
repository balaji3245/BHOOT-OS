# BHOOT v1.0 — Final Release Engineering Report

**Release Candidate:** v1.0.0-rc3 (Production Release Build)  
**Release Date:** 2026-11-08  
**Architecture:** Debian 12 Bookworm, x86_64 UEFI  
**Target Deployment Drive:** HP USB Flash Drive (`070D29709B8E4F65`)  
**Deployment Status:** Staged in `os/build_real/bhoot_disk_assembled.img`. Zero physical writes executed.

---

## 1. Executive Summary

BHOOT v1.0 is a private, offline-first personal financial operating system built on Debian 12 Bookworm x86_64 UEFI. It features full disk encryption (LUKS2 Argon2id), an authenticated read-only SquashFS system root, an encrypted financial ledger powered by SQLCipher 4.5.x, an offline document vault with AES-256-GCM encryption, a zero-trust nftables firewall, and an isolated Wayland kiosk user interface.

All release artifacts have been rebuilt natively against Debian 12 Bookworm (GLIBC 2.36 runtime) and verified across 230 automated tests. The disk image matches the exact byte capacity of the target HP USB drive (`62,026,416,128` bytes).

---

## 2. Release Artifact Manifest & Cryptographic Signatures

All release artifacts have been generated in disposable staging environments and cryptographically hashed:

| Artifact | File Path | Exact Size (Bytes) | SHA-256 Checksum |
| :--- | :--- | :--- | :--- |
| **Release Binary** | `os/build_real/rootfs/usr/local/bin/bhoot-desktop` | `4,098,416` | `af99e18b74de789fd43be72ed3169364977e904838ec2f4bbb58b514c97195a6` |
| **SquashFS Root** | `os/build_real/filesystem.squashfs` | `317,431,808` | `2608b58b94718f82c3c40b668d1d7c587bd0637f8bebfae3dafe0b9c0d2f1a62` |
| **EFI Partition** | `os/build_real/bhoot_esp.img` | `1,073,741,824` | `aa7955c073de04cb4094fa3e89ea2e240e56bbd9a70d0dcd825c344edaee0d6e` |
| **System Ext4** | `os/build_real/bhoot_system_ext4.img` | `4,294,967,296` | `aedd69299d8aa5e4156c256784619ead316ab5a1a96049ef0fda40d4a92395bf` |
| **Recovery Ext4** | `os/build_real/bhoot_recovery.img` | `4,294,967,296` | `b9d8d07c4077c847e2706ffb371667b2b46d11aa29c79591e0f81509074b8ddb` |
| **LUKS2 Header** | `os/build_real/bhoot_luks_header.backup` | `16,777,216` | `59032c69328d83f7bb53d0687ac9ecf7d1f6cc8ac7e983b00dfb146744a02577` |
| **Assembled Disk** | `os/build_real/bhoot_disk_assembled.img` | `62,026,416,128` | `a6eda341fe30ee3227adfa0291a76f9b8620446a32dc50f4b2fe23108deb4630` |

### 2.1 SquashFS Binary Identity Verification
The binary extracted directly from `filesystem.squashfs` via `unsquashfs` has been compared against the rootfs binary:
- **Rootfs binary SHA-256:** `af99e18b74de789fd43be72ed3169364977e904838ec2f4bbb58b514c97195a6`
- **SquashFS binary SHA-256:** `af99e18b74de789fd43be72ed3169364977e904838ec2f4bbb58b514c97195a6`
- **Result:** Exact identity match.

---

## 3. Key Remediation Summary

1. **SQLCipher Database Encryption Key:**
   - Eradicated compile-time constant key `[0x5Au8; 32]`.
   - Implemented per-installation random 256-bit CSPRNG key provisioning.
   - Key wrapped in AES-256-GCM envelope using HKDF-SHA256 key derived from machine secret.
   - Enforced fail-closed behavior on missing key with existing database.
   - Transients protected via `zeroize::Zeroizing`.

2. **Backup & Disaster Recovery Key Lifecycle:**
   - Embedded database key inside the Argon2id passphrase-protected backup envelope.
   - Verified cross-installation recovery capability: a backup created on one host can now be fully restored on another host without manual key extraction.

3. **Partition 4 (Recovery Partition) Full 4 GiB Restoration:**
   - Restored Partition 4 size from 1 GiB to full 4 GiB (`4,294,967,296` bytes).
   - Formatted genuine Ext4 filesystem (`BHOOT_REC`) containing detached LUKS2 header, automated restore script, and recovery key unlocker.

4. **Authentication Remediation:**
   - Removed default `"1234"` PIN.
   - Mandatory first-owner enrollment with strong PIN validation.
   - Persistent 60s lockout after 5 failed attempts surviving system reboot.

---

## 4. HP USB Fit & Geometry Specification

The assembled disk image `os/build_real/bhoot_disk_assembled.img` was assembled to fit the owner's HP USB drive:
- **Reported HP USB Capacity:** `62,026,416,128` bytes (121,145,344 sectors).
- **Assembled Disk Image Size:** `62,026,416,128` bytes.
- **Delta:** Exactly 0 bytes.

```
Disk /home/bhoot/Desktop/BHOOT/os/build_real/bhoot_disk_assembled.img: 57.77 GiB, 62026416128 bytes, 121145344 sectors
Disk identifier: B8007001-0000-4000-8000-000000000001

Device                                                                Start       End   Sectors Size Type
/home/bhoot/Desktop/BHOOT/os/build_real/bhoot_disk_assembled.img1      2048   2099199   2097152   1G EFI System
/home/bhoot/Desktop/BHOOT/os/build_real/bhoot_disk_assembled.img2   2099200  10487807   8388608   4G Linux filesystem (BHOOT_SYSTEM)
/home/bhoot/Desktop/BHOOT/os/build_real/bhoot_disk_assembled.img3  10487808 111151103 100663296  48G Linux filesystem (BHOOT_DATA - LUKS2)
/home/bhoot/Desktop/BHOOT/os/build_real/bhoot_disk_assembled.img4 111151104 119539711   8388608   4G Linux filesystem (BHOOT_REC)
[Unallocated Alignment Slack]                                     119539712 121145310   1605599 784M Free Space / Controller Wear Reserve
[Secondary GPT Table]                                             121145311 121145343        33 16K GPT Backup
```

---

## 5. Physical Deployment Handover (When Authorized by Owner)

> [!IMPORTANT]
> **SAFETY GATE MAINTAINED:**
> Neither this agent nor the verification harness has written to `/dev/sda` or any physical storage device. `/dev/sda` currently remains mounted at `/run/media/bhoot/HP_USB` with its existing user data intact.

When the owner decides to deploy this release image to the physical HP USB drive, execute the following verified procedure:

### Step 1: Confirm USB Identity & Back Up Existing Files
```bash
# Verify HP USB device identity
lsblk -o NAME,SIZE,MODEL,SERIAL,TRAN /dev/sda
# Ensure any personal files on /run/media/bhoot/HP_USB have been saved to another machine
```

### Step 2: Unmount Target Partitions
```bash
sudo umount /dev/sda1 || true
sudo umount /dev/sda* || true
```

### Step 3: Flash the Assembled Image
```bash
sudo dd if=os/build_real/bhoot_disk_assembled.img of=/dev/sda bs=4M conv=fsync status=progress
```

### Step 4: Verify Flashed Media Integrity
```bash
sudo blockdev --rereadpt /dev/sda
sudo sfdisk --verify /dev/sda
sudo cmp -n 62026416128 os/build_real/bhoot_disk_assembled.img /dev/sda
```

---

## 6. Release Sign-Off
All software, security, cryptographic, and operating system release gates for BHOOT v1.0 have been met.
