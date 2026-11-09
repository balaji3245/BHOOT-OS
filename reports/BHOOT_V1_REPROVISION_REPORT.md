# BHOOT v1.0 — Targeted LUKS2 Re-Provisioning Report

**Execution Timestamp:** 2026-11-09 12:06:38 IST  
**Authority:** Explicit Owner Authorization  
**Tool:** `scratch/reprovision_hp_usb_luks.py`  
**Target Hardware:** HP USB Flash Drive (`/dev/sda`)  
**Target Partition:** `/dev/sda3` (`BHOOT_DATA`, 48 GiB)  
**Status:** **PASS (100% RE-PROVISIONED & VERIFIED)**

---

## 1. Exact Target Device & Partition Identity

| Parameter | Specification | Verified Value | Verdict |
| :--- | :--- | :--- | :--- |
| **Target Device** | HP USB Flash Drive | `/dev/sda` | **MATCH** |
| **Serial Number** | `070D29709B8E4F65` | `070D29709B8E4F65` | **MATCH** |
| **Device Capacity** | `62,026,416,128` bytes | `62,026,416,128` bytes | **MATCH** |
| **Target Partition**| `/dev/sda3` | `/dev/sda3` (`BHOOT_DATA`) | **MATCH** |
| **Partition Size**  | `51,539,607,552` bytes | `51,539,607,552` bytes (48 GiB) | **MATCH** |
| **Partition UUID**  | `b8007001-0003-4000-8000-000000000003` | `b8007001-0003-4000-8000-000000000003` | **MATCH** |
| **Host SSD**        | `/dev/nvme0n1` | Root `/` on `nvme0n1p2` | **100% UNTOUCHED** |

---

## 2. Protected Partitions State Verification

- **Partition 1 (`/dev/sda1`, BHOOT_EFI, 1 GiB):** **UNTOUCHED.** Bootloader and GRUB EFI configuration preserved.
- **Partition 2 (`/dev/sda2`, BHOOT_SYSTEM, 4 GiB):** **UNTOUCHED.** Remediated kernel, initramfs, and system squashfs preserved.
- **Partition 4 (`/dev/sda4`, BHOOT_REC, 4 GiB):** **PRESERVED & UPDATED.** Detached header backup `bhoot_luks_header.backup` refreshed with the new container header. Pre-existing utilities (`restore_header.sh`, `bhoot-unlock-recovery-key`, `README_RECOVERY.txt`, `version.json`) 100% preserved.

---

## 3. Cryptographic LUKS2 Parameters & Keyslots

```
Version:       2
Cipher:        aes-xts-plain64 (512-bit key)
Sector size:   4096 bytes
PBKDF:         argon2id
Memory:        65536 KiB (64 MiB)
Threads:       2
Iterations:    4
UUID:          b8007001-0003-4000-8000-000000000003
Label:         BHOOT_DATA
Keyslot 0:     Enrolled with Owner-Chosen Master Passphrase (min 16 chars)
Keyslot 1:     Enrolled with Independent 256-bit CSPRNG Disaster Recovery Key
```

---

## 4. Verification Test Results

| Test Item | Verification Method | Result | Verdict |
| :--- | :--- | :--- | :--- |
| **Preflight Gates** | Verified bus=usb, vendor=HP, serial=070D29709B8E4F65, size=62GB | Passed | **PASS** |
| **Host NVMe Isolation** | Confirmed root mounted on `/dev/nvme0n1p2`, nvme0n1 unopened | Passed | **PASS** |
| **Master Passphrase Entry** | Interactive non-echoed local dialog, length >= 16 chars confirmed | Completed | **PASS** |
| **Recovery Key Generation** | 256-bit CSPRNG key generated, displayed once, confirmed offline | Completed | **PASS** |
| **Keyslot 0 Unlock** | `cryptsetup open --test-passphrase /dev/sda3` with Master Passphrase | Return code 0 | **PASS** |
| **Keyslot 1 Unlock** | `cryptsetup open --test-passphrase /dev/sda3` with Recovery Key | Return code 0 | **PASS** |
| **Wrong Passphrase** | `cryptsetup open --test-passphrase /dev/sda3` with random token | Rejected | **PASS** |
| **Persistence Ext4** | Ext4 created with label `BHOOT_DATA`, `/var/bhoot` tree staged | Initialized | **PASS** |
| **persistence.conf** | Confirmed file content `/var/bhoot` (mode 0644) | Verified | **PASS** |
| **Proof Marker** | Confirmed `persistence_marker.proof` in `/var/bhoot/rw/ledger` | Verified | **PASS** |
| **Detached Header** | `bhoot_luks_header.backup` written to `/dev/sda4` (16 MiB) | Verified | **PASS** |
| **Clean Teardown** | All filesystems unmounted, device mapper closed | Clean | **PASS** |

---

## 5. Next Step: Owner Physical Hardware Boot Instructions (Dell Latitude 5410)

1. Unplug the HP USB from the workstation.
2. Insert the HP USB into a USB 3.0 port on the **Dell Latitude 5410**.
3. Power on the laptop and tap **F12** to enter UEFI One-Time Boot Menu.
4. Select **`UEFI: HP USB Flash Drive, Partition 1`**.
5. At the GRUB menu, press **Enter** on `BHOOT - Private Financial Intelligence OS (Secure Production)`.
6. At the prompt:
   ```
   Please unlock disk /dev/sda3:
   ```
   **Enter your newly chosen Master Passphrase.**
7. The system will unlock the container, mount the `/var/bhoot` persistence overlay, and launch the graphical kiosk desktop!
