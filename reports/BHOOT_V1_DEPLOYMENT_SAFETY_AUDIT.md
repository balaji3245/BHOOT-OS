# BHOOT v1.0 — Pre-Deployment Safety Audit Report (Targeted Partition 2 Strategy)

**Audit Date:** 2026-11-09 13:48:00 IST  
**Authority:** Pre-Deployment Safety Verification Gate  
**Target Hardware:** HP USB Flash Drive 64GB  
**Physical Target Device:** `/dev/sda`  
**Host Internal Storage:** `/dev/nvme0n1` (Protected Host SSD)  
**Safety Verdict:** **STRICT CONDITIONAL GATE — WHOLE-DISK FLASH CATEGORICALLY BLOCKED; TARGETED P2 UPDATE PREPARED**  
**Physical Write Status:** **ZERO BYTES WRITTEN (AWAITING OWNER APPROVAL)**  

---

## 1. Executive Summary & Critical Safety Invariant

During this audit, the physical storage configuration of the attached HP USB flash drive (`/dev/sda`) was inspected and compared against the release build artifacts. 

> [!CAUTION]
> **CRITICAL FINDING: WHOLE-DISK FLASHING (`bhoot_disk_assembled.img`) IS CATEGORICALLY BLOCKED**
> 
> The physical HP USB drive was authoritatively re-provisioned earlier today (`reports/BHOOT_V1_REPROVISION_REPORT.md` at 12:06:38 IST). Partition 3 (`/dev/sda3`) contains the owner's **new Master Passphrase in Keyslot 0**, the **independent 256-bit Recovery Key in Keyslot 1**, and the **active `/var/bhoot` persistence ext4 ledger**.
>
> In contrast, the assembled disk image (`bhoot_disk_assembled.img`) and legacy components (`bhoot_luks_prod_48g.img`, `bhoot_recovery.img`) contain **stale factory test metadata from Phase 2A**. 
>
> Flashing the whole 57.81 GiB assembled image would overwrite Partition 3 and Partition 4, permanently obliterating the owner's LUKS2 encryption keys, disaster recovery capability, and persisted ledger data.

### Authorized Strategy: Targeted Partition 2 (`/dev/sda2`) Deployment
To safely deliver the remediated BHOOT v1.0 functional integration release (featuring the 8-worker thread pool, persistent SQLCipher settings, and kiosk-safe controls) without risking user credentials or financial data, deployment must be strictly restricted to **Partition 2 (`/dev/sda2`, BHOOT_SYSTEM)**.

---

## 2. Physical Device Identity & Partition Table Verification

### 2.1 Hardware Identity
- **Device Path:** `/dev/sda`
- **Bus Type:** USB (`ID_BUS=usb`)
- **Vendor:** HP (`ID_VENDOR=HP`, `03f0`)
- **Model:** USB Flash Drive (`ID_MODEL=USB_Flash_Drive`, `1985`)
- **Serial Number:** `070D29709B8E4F65` (`ID_SERIAL_SHORT=070D29709B8E4F65`)
- **Total Sectors:** `121,145,344` (512-byte logical sectors)
- **Total Capacity:** `62,026,416,128` bytes (57.77 GiB)
- **Removable Flag:** `1` (True)

### 2.2 Host Isolation
- **Host Internal SSD:** `/dev/nvme0n1` (Capacity: 476.94 GiB, Model: PC SN740 NVMe WDC)
- **Host Root Filesystem:** Mounted on `/dev/nvme0n1p2` (`bef9fb98-6193-4226-9806-104c01827945`)
- **Host EFI System Partition:** Mounted on `/dev/nvme0n1p1` (`3896-2C36`)
- **Isolation Status:** **100% UNTOUCHED & DISCONNECTED FROM WRITE PATHS**

### 2.3 Physical Partition Geometry & Allocation
| Partition | Role | Start Sector | End Sector | Sector Count | Capacity (Bytes) | Filesystem / Format | Partition UUID | Volume UUID | Target Action |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `/dev/sda1` | `BHOOT_EFI` | `2,048` | `2,099,199` | `2,097,152` | `1,073,741,824` (1 GiB) | FAT32 (0x55aa) | `b8007001-0001-4000-8000-000000000001` | `B800-0001` | **PRESERVE (DO NOT TOUCH)** |
| `/dev/sda2` | `BHOOT_SYSTEM`| `2,099,200` | `10,487,807`| `8,388,608` | `4,294,967,296` (4 GiB) | Ext4 (0xef53) | `b8007001-0002-4000-8000-000000000002` | `b8007001-0002-4000-8000-000000000002` | **TARGETED UPDATE ONLY** |
| `/dev/sda3` | `BHOOT_DATA` | `10,487,808`| `111,151,103`| `100,663,296`| `51,539,607,552` (48 GiB)| LUKS2 (Argon2id)| `b8007001-0003-4000-8000-000000000003` | `b8007001-0003-4000-8000-000000000003` | **PRESERVE (DO NOT TOUCH)** |
| `/dev/sda4` | `BHOOT_REC` | `111,151,104`| `119,539,711`| `8,388,608` | `4,294,967,296` (4 GiB) | Ext4 (0xef53) | `b8007001-0004-4000-8000-000000000004` | `bf801836-c200-4873-9754-d9a7df9a6bde` | **PRESERVE (DO NOT TOUCH)** |

---

## 3. Cryptographic Forensic Comparison: Physical USB vs Assembled Image

An exhaustive forensic comparison between the live physical USB media and the repository images revealed the exact points of divergence:

### 3.1 Partition 3 (`BHOOT_DATA`, 48 GiB)
- **Current Physical USB (`/dev/sda3`):**
  - Container Type: LUKS2, AES-XTS-512, Argon2id.
  - Keyslot 0: Owner's Master Passphrase (enrolled interactively at 12:06:38 IST).
  - Keyslot 1: Independent 256-bit CSPRNG Disaster Recovery Key.
  - Payload: `/var/bhoot` persistence ext4 overlay holding user credentials and financial records.
- **Assembled Image (`bhoot_disk_assembled.img` Partition 3):**
  - Contains factory/test LUKS2 container from early builds.
  - Keyslot 0: Outdated build passphrase.
  - **Verdict:** Flashing the whole disk would irrevocably destroy the owner's newly established passphrase and persistence storage.

### 3.2 Partition 4 (`BHOOT_REC`, 4 GiB)
- **Current Physical USB (`/dev/sda4`):**
  - Contains `bhoot_luks_header.backup` written today at **Nov 9 12:06 IST** matching `/dev/sda3`'s active header.
  - Contains recovery utilities: `restore_header.sh`, `bhoot-unlock-recovery-key`.
- **Assembled Image (`bhoot_disk_assembled.img` Partition 4):**
  - Contains stale detached header backup from Nov 8 (`59032c69328d83f7bb53d0687ac9ecf7d1f6cc8ac7e983b00dfb146744a02577`).
  - **Verdict:** Overwriting P4 would destroy the valid detached backup needed for disaster recovery.

### 3.3 Partition 2 (`BHOOT_SYSTEM`, 4 GiB)
- **Current Physical USB (`/dev/sda2`):**
  - SquashFS timestamp: Nov 8 12:47.
  - SquashFS SHA-256: `2608b58b94718f82c3c40b668d1d7c587bd0637f8bebfae3dafe0b9c0d2f1a62`.
  - Binary state: Missing thread pool, non-functional controls, blocking dialogs.
- **Source Partition Image (`os/build_real/bhoot_system_ext4.img`):**
  - SquashFS timestamp: Nov 9 13:31.
  - SquashFS SHA-256: `e64cbb75005f2f254a1b4d8160db7ecd568e7d2cfc0fb9a14024b27bb6c5786c`.
  - Binary SHA-256: `33a3c816dbe6afe9b4833241bf764bef48503a0931c332e2387cf938eb56411a`.
  - Full partition SHA-256: `6c042d129c188e86623769cbcaf949bac26741e32e8ca3186b78c14097c29da3`.
  - Binary state: Native Debian 12 (glibc 2.36), 8-worker thread pool, persistent SQLCipher settings, kiosk-safe HTML modals, 100% verified.
  - **Verdict:** Updating Partition 2 is technically safe, necessary, and cleanly isolated.

---

## 4. Technical Safety Analysis of Targeted P2 Deployment

1. **Kernel Block Boundary Isolation:**
   - Linux kernel block device boundaries strictly constrain writes to the designated partition descriptor.
   - Targeting `/dev/sda2` as the destination block device limits all I/O exclusively to sector range `2,099,200` through `10,487,807` (4,294,967,296 bytes).
   - Sector `2,099,199` (P1 boundary) and Sector `10,487,808` (P3 boundary) cannot be reached or modified by operations directed at `/dev/sda2`.
2. **Identical Partition Geometry:**
   - Physical `/dev/sda2` size: `4,294,967,296` bytes.
   - Source `bhoot_system_ext4.img` size: `4,294,967,296` bytes (exact 1:1 match).
3. **Filesystem Identity:**
   - Both physical `/dev/sda2` and `bhoot_system_ext4.img` share canonical Ext4 volume label `BHOOT_SYSTEM` and UUID `b8007001-0002-4000-8000-000000000002`.

---

## 5. Mandatory Pre-Deployment Backup Plan

Prior to writing a single byte to `/dev/sda2`, execute the following defensive backup steps:

1. **Host-Side Header Snapshot:**
   Back up the active LUKS2 header directly from `/dev/sda3` to persistent host storage:
   ```bash
   mkdir -p /home/bhoot/Desktop/BHOOT/backups_host
   pkexec cryptsetup luksHeaderBackup /dev/sda3 \
     --header-backup-file /home/bhoot/Desktop/BHOOT/backups_host/bhoot_luks_header_sda3_preflash.backup
   ```
2. **P4 Detached Backup Verification:**
   Verify that `/run/media/bhoot/BHOOT_REC/bhoot_luks_header.backup` on Partition 4 is intact.
3. **Record SHA-256 Checksums:**
   Generate and archive cryptographic checksums of the backed-up headers.

---

## 6. Execution Procedure for Owner Approval

Once the owner explicitly authorizes flashing Partition 2, the execution script will run the following sequence:

```
[Phase 1] Preflight Safety Verification:
  - Verify USB device path: /dev/sda
  - Verify USB serial: 070D29709B8E4F65
  - Verify USB capacity: 62,026,416,128 bytes
  - Verify host root is NOT /dev/sda (/dev/nvme0n1p2 confirmed)
  - Ensure all /dev/sda partitions are unmounted

[Phase 2] Defensive Host-Side Backup:
  - Capture 16 MiB LUKS2 header from /dev/sda3 to /home/bhoot/Desktop/BHOOT/backups_host/
  - Verify backup file integrity

[Phase 3] Targeted Partition 2 Block Write:
  - Source: /home/bhoot/Desktop/BHOOT/os/build_real/bhoot_system_ext4.img
  - Source Size: 4,294,967,296 bytes
  - Source SHA-256: 6c042d129c188e86623769cbcaf949bac26741e32e8ca3186b78c14097c29da3
  - Target Block Device: /dev/sda2 ONLY
  - Command: dd if=bhoot_system_ext4.img of=/dev/sda2 bs=4M conv=fsync status=progress

[Phase 4] Synchronous Flush:
  - Execute sync

[Phase 5] Post-Write Forensic Verification:
  - Read back exactly 4,294,967,296 bytes from /dev/sda2
  - Compute readback SHA-256; assert exact match with 6c042d129c188e86623769cbcaf949bac26741e32e8ca3186b78c14097c29da3
  - Probe /dev/sda1 (ESP) FAT32 magic (0x55aa) -> Must be INTACT
  - Probe /dev/sda3 (DATA) LUKS2 magic (0x4c554b53babe) -> Must be INTACT
  - Probe /dev/sda4 (REC) Ext4 magic (0xef53) -> Must be INTACT
  - Verify LUKS2 keyslots on /dev/sda3 remain valid
```

---

## 7. Approval Gateway

| Gate Parameter | Required Value | Actual Status |
| :--- | :--- | :--- |
| Target Partition | `/dev/sda2` (`BHOOT_SYSTEM`) | Confirmed isolated |
| Source Image Path | `/home/bhoot/Desktop/BHOOT/os/build_real/bhoot_system_ext4.img` | Validated (4,294,967,296 B) |
| Source Image SHA-256 | `6c042d129c188e86623769cbcaf949bac26741e32e8ca3186b78c14097c29da3` | Certified |
| Protected Partitions | `/dev/sda1`, `/dev/sda3`, `/dev/sda4`, `/dev/nvme0n1` | **Zero write permitted** |
| Owner Authorization | Explicit approval for targeted `/dev/sda2` flash | **AWAITING APPROVAL** |

