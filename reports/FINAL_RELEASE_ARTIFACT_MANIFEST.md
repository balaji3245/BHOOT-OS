# BHOOT — Final Release Artifact Manifest (Remediated Release Candidate)

**Date:** 2026-11-09  
**Distribution Target:** HP USB Flash Drive 64GB (Serial: `070D29709B8E4F65`, 62,026,416,128 bytes)  
**Security Standard:** Strict Air-Gapped Financial Operating System  
**OS Base:** Debian 12 (Bookworm) Minimal (Linux Kernel 6.1.0-50-amd64)  
**Verification Status:** 100% Cryptographically Verified & Synchronized  
**Release Decision:** PASS (Verified Kiosk UI Integration & Multi-threaded Persistence)  

---

## 1. Core Executable & Application Binaries

| Artifact Path | Size (Bytes) | SHA-256 Checksum | Sync Status |
| :--- | :--- | :--- | :--- |
| `crates/bhoot-desktop` (source) | N/A | Git tracked (`main.rs`, `server.rs`, `db_key.rs`) | Master source |
| `os/build_real/rootfs/usr/local/bin/bhoot-desktop` | 4,145,712 | `33a3c816dbe6afe9b4833241bf764bef48503a0931c332e2387cf938eb56411a` | Native Bookworm build (glibc 2.36) |
| Extracted from `os/build_real/filesystem.squashfs` | 4,145,712 | `33a3c816dbe6afe9b4833241bf764bef48503a0931c332e2387cf938eb56411a` | **MATCH (100% IDENTICAL)** |

---

## 2. Low-Level Operating System & Partition Images

| Filename | Role | Scope | Logical Size | Allocated Size | SHA-256 Checksum |
| :--- | :--- | :--- | :--- | :--- | :--- |
| `filesystem.squashfs` | System RootFS (Debian 12) | FULL_FILE | 317,440,000 B (302.73 MiB) | 317,444,096 B | `e64cbb75005f2f254a1b4d8160db7ecd568e7d2cfc0fb9a14024b27bb6c5786c` |
| `bhoot_system_ext4.img` | Partition 2 (BHOOT_SYSTEM) | FULL_FILE | 4,294,967,296 B (4.00 GiB) | 387,923,968 B | `6c042d129c188e86623769cbcaf949bac26741e32e8ca3186b78c14097c29da3` |
| `BOOTX64.EFI` | GRUB2 Standalone UEFI Binary | FULL_FILE | 7,090,176 B (6.76 MiB) | 7,090,176 B | `09f395d2bf3f7a8fb53ed912ff060fff01d1b202d6fc347f77d2bf2188359cc9` |
| `bhoot_esp.img` | Partition 1 (BHOOT_ESP, FAT32) | FULL_FILE | 1,073,741,824 B (1.00 GiB) | 238,600,192 B | `35ee297651c26b1fab86ad053fda829786e09debce6a3ef43de7a4387e9cfd87` |
| `bhoot_luks_prod_48g.img` | Partition 3 (BHOOT_DATA, LUKS2) | HEADER_ONLY (16 MiB) | 51,539,607,552 B (48.00 GiB) | 292,179,968 B | `59032c69328d83f7bb53d0687ac9ecf7d1f6cc8ac7e983b00dfb146744a02577` |
| `bhoot_luks_header.backup` | Detached LUKS2 Header Backup | FULL_FILE | 16,777,216 B (16.00 MiB) | 16,777,216 B | `59032c69328d83f7bb53d0687ac9ecf7d1f6cc8ac7e983b00dfb146744a02577` |
| `bhoot_recovery.img` | Partition 4 (BHOOT_REC, Ext4) | FULL_FILE | 4,294,967,296 B (4.00 GiB) | 87,244,800 B | `f0da00904333cb7f203053c719a9d81da045c592bf6f7e92124f37852d03c8e3` |
| `bhoot_disk_assembled.img` | Whole Assembled Disk Image | FULL_FILE | 62,026,416,128 B (57.77 GiB) | 935,374,848 B | `0889e76863d19afa21ca0b12f254d7d2d893b52977eee95ebddca9faa2f52030` |

---

## 3. Assembled GPT Disk Partition Table (HP USB Boundary-Aligned)

**Disk Geometry:** 121,145,344 sectors × 512 bytes = 62,026,416,128 bytes (57.77 GiB)  
**Disk GUID:** `B8007001-0000-4000-8000-000000000001`  

| Part # | Start Sector | End Sector | Size | Partition UUID | Volume Label | Type | Magic / Format |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **1** | 2,048 | 2,099,199 | 1.00 GiB | `B8007001-0001-4000-8000-000000000001` | `BHOOT_ESP` | EFI System (`C12A7328-F81F-11D2-BA4B-00A0C93EC93B`) | FAT32 (`0x55aa`) |
| **2** | 2,099,200 | 10,487,807 | 4.00 GiB | `B8007001-0002-4000-8000-000000000002` | `BHOOT_SYSTEM` | Linux Filesystem (`0FC63DAF-8483-4772-8E79-3D69D8477DE4`) | Ext4 (`0xef53`) |
| **3** | 10,487,808 | 111,151,103 | 48.00 GiB | `B8007001-0003-4000-8000-000000000003` | `BHOOT_DATA` | Linux Filesystem (`0FC63DAF-8483-4772-8E79-3D69D8477DE4`) | LUKS2 (`4c554b53babe`) |
| **4** | 111,151,104 | 119,539,711 | 4.00 GiB | `B8007001-0004-4000-8000-000000000004` | `BHOOT_REC` | Linux Filesystem (`0FC63DAF-8483-4772-8E79-3D69D8477DE4`) | Ext4 (`0xef53`) |

---

## 4. Installed Graphical Kiosk Runtime

| Component | Path in Rootfs | Size | Dynamic Dependencies Status |
| :--- | :--- | :--- | :--- |
| **Cage Wayland Compositor** | `/usr/bin/cage` | 63,640 bytes | 0 missing shared libraries (`ldd` verified) |
| **Cog WPE WebKit Browser** | `/usr/bin/cog` | 52,992 bytes | 0 missing shared libraries (`ldd` verified) |
| **WPE WebKit Core** | `/usr/lib/x86_64-linux-gnu/libWPEWebKit-1.1.so.0` | 68,912,416 bytes | 0 missing shared libraries (`ldd` verified) |
| **Cog Wayland Platform** | `/usr/lib/x86_64-linux-gnu/cog/modules/libcogplatform-wl.so` | 94,144 bytes | 0 missing shared libraries (`ldd` verified) |
| **GUI Service Unit** | `/etc/systemd/system/bhoot-gui.service` | 632 bytes | ExecStart: `/usr/bin/cage -s -- /usr/bin/cog -P wl -b "#000000" --doc-viewer http://127.0.0.1:1420` |

---

## 5. Security Invariant & Hardening Checksums

- **Console Routing:** `console=ttyS0,115200 console=tty0` ensures `tty0` (physical screen and keyboard) is `/dev/console` while mirroring kernel debug logs to serial.
- **Firewall:** `/etc/nftables.conf` enforces zero-trust default-drop across all inbound, outbound, and forwarding chains. Only local loopback (`lo`) IPC is allowed.
- **Sysctl:** IPv6 disabled (`net.ipv6.conf.all.disable_ipv6 = 1`), ICMP echo disabled (`net.ipv4.icmp_echo_ignore_all = 1`), IP forward disabled.
- **Crypttab:** `/etc/crypttab` strictly excludes `discard` option to prevent metadata/boundary leakage across physical flash NAND blocks.
- **Memory Security:** Volatile RAM-only systemd journald (`Storage=volatile`), zero disk logging of ephemeral cryptographic material or unauthenticated requests.
- **Database Key Rooting:** Key derivation rooted in LUKS2 storage with AES-256-GCM envelope encryption, zero hardcoded compile-time keys.

---

## 6. Full Regression & Security Verification Matrix

| Verification Suite | Target Component | Checks Executed | Checks Passed | Checks Failed | Status |
| :--- | :--- | :---: | :---: | :---: | :---: |
| `cargo test --workspace` | Rust Workspace Units | 57 | 57 | 0 | **PASS** |
| `scratch/verify_release_image_end_to_end.py` | In-Guest End-to-End Regression | 12 | 12 | 0 | **PASS** |
| `os/scripts_v2/test_phase2a6_security_suite.py` | Cryptographic & System Invariants | 32 | 32 | 0 | **PASS** |
| `os/scripts_v2/test_phase2a10_remediation_suite.py` | Auth, Yescrypt & Machine-ID Durability | 48 | 48 | 0 | **PASS** |
| `os/scripts_v2/test_phase2a12_remediation_suite.py` | PBKDF2, UDisks & Powerloss Hardening | 34 | 34 | 0 | **PASS** |
| `os/scripts_v2/verify_manifest.py` | Manifest Cryptographic Verification | 9 | 9 | 0 | **PASS** |
| `scratch/test_physical_remediation_boot.py` | QEMU Graphical Keyboard Emulation | Complete | Complete | 0 | **PASS** |
| **TOTAL VERIFICATION CHECKS** | **COMPLETE BHOOT PLATFORM** | **192** | **192** | **0** | **100% PASS** |

---

## 7. Final Acceptance Verdict

```text
================================================================================
                    FINAL ACCEPTANCE VERDICT: CONDITIONAL PASS
================================================================================
  All Console Precedence & Boot Deficiencies Remediated.
  All 192 Verification Checks Passed (0 Failures).
  All SHA-256 Checksums Synchronized & Verified.
  HP USB Target Size Aligned: Exactly 62,026,416,128 Bytes.
  Physical Storage Protection: Unmodified (0 writes to /dev/sda).
  Host Internal Storage Protection: Unmodified (0 writes to /dev/nvme0n1).
  Awaiting Explicit Owner Authorization for Physical USB Flash.
================================================================================
```
