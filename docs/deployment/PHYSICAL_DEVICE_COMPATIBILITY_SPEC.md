# Project BHOOT — Physical-Device Compatibility & Deployment Specification

**Document Version:** 1.0.0  
**Status:** Phase B Engineering Specification  
**Target Hardware:** Dell Latitude 5410 (Class 3 UEFI x86_64)  
**Target Storage Media:** Dedicated 64 GB USB 3.0 Flash Drive (SanDisk / HP 64 GB)  
**Hardware Safety Invariant:** Direct physical flashing is prohibited without explicit owner approval.

---

## 1. Hardware Storage Characterization

### 1.1 USB Media Geometry & Partition Table
The target deployment media is a dedicated 64 GB USB 3.0 flash drive with the following measured geometry:
- **Logical Sector Size:** 512 bytes
- **Physical Sector Size:** 512 bytes
- **Total Physical Sector Count:** 121,145,344 sectors
- **Total Physical Measured Capacity:** 62,026,416,128 bytes (57.76 GiB / 62.02 GB)
- **Partition Table Scheme:** GUID Partition Table (GPT) with protective MBR at LBA 0

### 1.2 Canonical Partition Layout

| Partition | Label | Type GUID | Start Sector | End Sector | Total Sectors | Size | Filesystem / Format |
|---|---|---|---|---|---|---|---|
| **1** | `BHOOT_EFI` | `C12A7328-F81F-11D2-BA4B-00A0C93EC93B` | `2048` | `2099199` | `2,097,152` | 1.0 GiB | FAT32 (`mkfs.vfat -F 32`) |
| **2** | `BHOOT_SYSTEM` | `0FC63DAF-8483-4772-8E79-3D69D8477DE4` | `2099200` | `10487807` | `8,388,608` | 4.0 GiB | Ext4 (read-only, holds SquashFS) |
| **3** | `BHOOT_DATA` | `0FC63DAF-8483-4772-8E79-3D69D8477DE4` | `10487808` | `111151103` | `100,663,296` | 48.0 GiB | LUKS2 Argon2id Container |
| **4** | `BHOOT_REC` | `0FC63DAF-8483-4772-8E79-3D69D8477DE4` | `111151104` | `119539711` | `8,388,608` | 4.0 GiB | Ext4 (holds recovery payloads) |
| **-** | *Unallocated* | - | `119539712` | `121145310` | `1,605,599` | ~784 MiB | Reserved for FTL wear-leveling |
| **-** | *Backup GPT* | - | `121145311` | `121145343` | `33` | 16.5 KiB | Secondary GPT Header & Table |

### 1.3 Alignment & Wear-Leveling Considerations
- **1 MiB Alignment:** All partition start sectors are exact multiples of 2,048 sectors (1 MiB), ensuring optimal alignment with flash erase block boundaries (typically 1 MiB to 4 MiB on commodity flash controllers).
- **Native 4K LUKS Sector Size:** Partition 3 is formatted with `--sector-size 4096`, aligning cryptographic block cipher operations with underlying flash page sizes and preventing read-modify-write amplification.
- **FTL Wear-Leveling Margin:** Approximately 831 MiB of trailing unallocated capacity is deliberately left unpartitioned, providing reserve over-provisioning space for the Flash Translation Layer (FTL).

---

## 2. Target Hardware Compatibility (Dell Latitude 5410)

### 2.1 Firmware Prerequisites
- **Platform:** Dell Latitude 5410 (Class 3 UEFI x86-64, Intel 10th Gen Core i5-10310U Comet Lake).
- **Secure Boot Status:** Currently, the standalone bootloader (`/EFI/BOOT/BOOTX64.EFI`) is unsigned.
- **Mandatory BIOS Configuration:**
  1. Turn on the Dell Latitude 5410 and immediately press **F2** to enter BIOS Setup.
  2. Navigate to **Secure Boot** $\rightarrow$ **Secure Boot Enable**.
  3. Set **Secure Boot Enable** to **Disabled**.
  4. Navigate to **System Configuration** $\rightarrow$ **SATA Operation**. Confirm **AHCI** mode is selected.
  5. Under **Boot Configuration**, ensure **UEFI Only** is selected.
  6. Save and exit.

### 2.2 Boot Menu Key
- Insert the provisioned BHOOT USB drive into any USB 3.0 port.
- Power on and immediately tap **F12** to display the One-Time Boot Menu.
- Under **UEFI Boot Devices**, select the USB flash drive (e.g. `UEFI: SanDisk / HP USB Device, Partition 1`).

---

## 3. Power-Loss Resilience & Safe Removal Guidelines

### 3.1 Commodity USB Flash Controller Limits
As analyzed in `bhoot-powerloss.conf`:
- Commodity USB flash drives feature volatile internal write caches without supercapacitor-backed Power Loss Protection (PLP).
- Abrupt physical disconnection during active write operations risks metadata corruption or FTL remap table loss.

### 3.2 Operating System Dirty Writeback Tuning
Project BHOOT enforces aggressive write flushing through kernel sysctl settings:
```ini
vm.dirty_background_ratio = 5
vm.dirty_ratio = 10
vm.dirty_writeback_centisecs = 200
vm.dirty_expire_centisecs = 500
```
Dirty pages are dispatched to media within 2 seconds.

### 3.3 Safe Shutdown Procedure
1. Always shut down cleanly via the desktop interface menu (**System Poweroff**) or terminal command `poweroff`.
2. Wait until the host screen turns off and the laptop power LED extinguishes completely before removing the USB flash drive.

---

## 4. Safe Deployment Tooling (`deploy_physical_usb.sh`)

Physical writing is guarded by `os/scripts_v2/deploy_physical_usb.sh`:
- Verifies that the target block device is a removable USB drive (rejects internal SATA, NVMe, and system boot drives).
- Verifies that device capacity matches or exceeds 57.81 GiB.
- Requires explicit interactive confirmation and owner authorization.
- Performs post-write byte verification of partition tables and boot signatures.
