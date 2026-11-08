# BHOOT v1.0 — Physical HP USB Deployment & Verification Report

**Deployment Timestamp:** 2026-11-08 19:38:10 IST  
**Deployment Authority:** Explicit Owner Authorization  
**Deployment Tool:** `scratch/deploy_hp_usb_verified.py` + `scratch/verify_physical_usb_readback.py`  
**Target Hardware:** HP USB Flash Drive (Model: `USB_Flash_Drive`, Serial: `070D29709B8E4F65`)  
**Deployment Status:** **PASS (100% BIT-FOR-BIT VERIFIED)**

---

## 1. Physical Target Identification & Preflight Safety Gates

| Parameter | Specification | Preflight Detected Value | Verification Verdict |
| :--- | :--- | :--- | :--- |
| **Device Node** | `/dev/sda` | `/dev/sda` | **MATCH** |
| **Bus Transport** | `usb` | `usb` | **MATCH** |
| **Device Vendor** | `HP` | `HP` | **MATCH** |
| **Device Model** | `USB_Flash_Drive` | `USB_Flash_Drive` | **MATCH** |
| **Serial Number** | `070D29709B8E4F65` | `070D29709B8E4F65` | **MATCH** |
| **Total Sectors** | `121,145,344` | `121,145,344` | **MATCH** |
| **Exact Capacity** | `62,026,416,128` bytes | `62,026,416,128` bytes | **MATCH** |
| **Internal Isolation**| Host NVMe (`/dev/nvme0n1`) | `/dev/nvme0n1p2` (Root `/`) | **PROTECTED / UNTOUCHED** |

---

## 2. Source Image Baseline

- **Source Image File:** `/home/bhoot/Desktop/BHOOT/os/build_real/bhoot_disk_assembled.img`
- **Expected Byte Size:** `62,026,416,128` bytes
- **Detected Byte Size:** `62,026,416,128` bytes
- **Expected SHA-256:** `a6eda341fe30ee3227adfa0291a76f9b8620446a32dc50f4b2fe23108deb4630`
- **Computed SHA-256:** `a6eda341fe30ee3227adfa0291a76f9b8620446a32dc50f4b2fe23108deb4630`
- **Preflight Verdict:** **PASS (Source verified 100%)**

---

## 3. Physical Write Operation

- **Pre-Write Partition State:** Unmounted all active partitions on `/dev/sda` (`/dev/sda1` unmounted via GIO/UDisks2).
- **Device Access:** Exclusive write block descriptor acquired via UDisks2 `OpenForRestore`.
- **Write Command:**
  ```bash
  dd if=/home/bhoot/Desktop/BHOOT/os/build_real/bhoot_disk_assembled.img bs=4M status=progress conv=fsync of=/dev/sda
  ```
- **Total Records In / Out:** `14,788+1` records in, `14,788+1` records out
- **Exact Bytes Written:** `62,026,416,128` bytes (57.77 GiB)
- **Exit Status:** `0` (Success)
- **Write Duration:** `18,347.8` seconds (~3.4 MB/s sustained USB flash write speed)
- **Buffer Synchronization:** Kernel block buffers flushed via `os.fsync(fd)` and `os.sync()`.

---

## 4. Full Physical Read-Back SHA-256 Verification

To eliminate any possibility of flash write truncation, bad sectors, or controller caching artifacts, the entire physical storage media was read back block-by-block and hashed:

- **Read-Back Method:** Streamed 62,026,416,128 bytes from `/dev/sda` via UDisks2 `OpenForBackup` in 8 MiB buffers.
- **Read-Back Byte Count:** `62,026,416,128` bytes
- **Read-Back Duration:** `2,992.9` seconds (~19.8 MB/s average read speed)
- **Source Image SHA-256:** `a6eda341fe30ee3227adfa0291a76f9b8620446a32dc50f4b2fe23108deb4630`
- **Physical Media SHA-256:** `a6eda341fe30ee3227adfa0291a76f9b8620446a32dc50f4b2fe23108deb4630`
- **Comparison Result:** **100.00% EXACT BIT-FOR-BIT MATCH**

---

## 5. Forensic Partition Table & Payload Signature Checks

Direct sector-level forensic inspection of `/dev/sda` verified the following structures:

| Structure | Sector Offset | Expected Signature | Found Signature | Forensic Verdict |
| :--- | :--- | :--- | :--- | :--- |
| **Primary GPT Header** | Sector 1 | `EFI PART` | `EFI PART` | **PASS** |
| **Partition 1 (ESP)** | Sector 2,048 | `0x55aa` (FAT32) | `0x55aa` | **PASS** |
| **Partition 2 (BHOOT_SYSTEM)** | Sector 2,099,200 | `0xef53` (Ext4) | `0x53ef` | **PASS** |
| **Partition 3 (BHOOT_DATA)** | Sector 10,487,808 | `4c554b53babe` (LUKS2) | `4c554b53babe` | **PASS** |
| **Partition 4 (BHOOT_REC)** | Sector 111,151,104 | `0xef53` (Ext4) | `0x53ef` | **PASS** |
| **Secondary GPT Header** | Sector 121,145,343 | `EFI PART` | `EFI PART` | **PASS** |

### Live Kernel Partition Layout (`lsblk /dev/sda`)
```
NAME   MAJ:MIN RM  SIZE RO TYPE MOUNTPOINTS LABEL
sda      8:0    1 57.8G  0 disk             
├─sda1   8:1    1    1G  0 part             BHOOT_EFI
├─sda2   8:2    1    4G  0 part             BHOOT_SYSTEM
├─sda3   8:3    1   48G  0 part             BHOOT_DATA
└─sda4   8:4    1    4G  0 part             BHOOT_REC
```

---

## 6. Host Internal Storage Safety Attestation

- **Internal Drive Node:** `/dev/nvme0n1` (SK Hynix 512 GB SSD)
- **Sector Count:** `1,000,215,216` sectors (`512,110,190,592` bytes)
- **Mount Verification:** `/dev/nvme0n1p2` remained mounted as host root `/` without disruption.
- **Attestation:** The internal drive `/dev/nvme0n1` was never opened, partitioned, written to, or modified in any way. Host UEFI and firmware settings remain completely unaltered.

---

## 7. Deployment Conclusion

**FINAL STATUS: PASS (DEPLOYMENT COMPLETE & VERIFIED)**

The physical HP USB Flash Drive is fully provisioned with BHOOT v1.0, verified bit-for-bit against the release manifest, and is ready for cold booting on any x86_64 UEFI machine.
