# BHOOT — Final Release Image Independent Verification Report

**Date:** 2026-11-07  
**Verification Scope:** Read-Only Verification of Final Assembled Disk Image  
**Baseline Git Commit:** `224bf301ffb99af90e7fa97cf6687bfce478a4c8`  
**Branch:** `remediation/full-product-integration`  
**Distribution Target:** Dedicated 64 GB USB 3.0 Device (x86_64 UEFI)  
**Target OS Platform:** Debian 12 (Bookworm) Minimal (Linux Kernel 6.1.0-50-amd64, glibc 2.36)  
**Auditor:** Principal Linux Security Engineer & Systems Verification Specialist  

---

# FINAL VERIFICATION VERDICT

```
========================================================================
                      FINAL AUDIT VERDICT: FAIL
========================================================================
 Assembled Disk Image Integrity:              VERIFIED (100% Geometry & Hash Match)
 Low-Level UEFI Boot & LUKS2 Unlock:          VERIFIED (OVMF, GRUB, Initramfs OK)
 Wayland Kiosk Runtime Startup:               VERIFIED (Cage & Cog Launch on tty1)
 Financial Logic, Auth & Rate Limiting:       VERIFIED (Passes 100% on Host Runtime)
 Encrypted Backup & Restore:                  VERIFIED (Bit-for-bit Vault Integrity)
 In-Guest Backend Execution:                  FAILED (GLIBC_2.38 Symbol Conflict)
 In-Guest Graphical Frontend Rendering:       NOT VERIFIED (Blocked by Backend Failure)
 Physical Hardware & Host Protection:         100% COMPLIANT (Zero Host Writes)
 Deployment Recommendation:                   DO NOT DEPLOY TO PHYSICAL USB
========================================================================
```

---

## 1. Executive Summary

This independent verification was executed under strict read-only constraints at Git commit `224bf30`. No application source code, configuration files, UI components, or security architectures were modified during this task.

All low-level operating system components, partition table geometry, detached LUKS headers, and virtual machine boot sequences were verified using isolated QEMU KVM instances and disposable sparse disk images.

### Key Positive Verification Points
1. **Assembled Disk Image Integrity:** The 57.81 GiB assembled disk image (`os/build_real/bhoot_disk_assembled.img`) is structurally complete. Partition geometry, sector alignments, partition UUIDs, and payload slices match their standalone image counterparts bit-for-bit.
2. **UEFI Boot & Security Invariants:** The disk image boots reliably under UEFI OVMF, executes standalone GRUB2, prompts for and unlocks the 48 GiB LUKS2 volume via Argon2id, executes first-boot provisioning, and enforces yescrypt shadow passwords and nftables zero-trust network packet filtering.
3. **Kiosk Compositor Stack:** Debian 12 packages for `cage` (Wayland compositor) and `cog` (WPE WebKit browser) were verified installed with zero missing shared libraries (`ldd` verified). On boot, `cage` successfully claims `/dev/dri/card0` and `/dev/tty1` and launches `cog`.
4. **Financial Core & Security Middleware:** Tested against the release binary on the host, all 18 financial endpoints, 5-attempt PIN brute-force rate limiters, 60-second persistent lockout in SQLCipher, session token revocation on lock, and encrypted backup/restore passed with 100% accuracy and integer paisa precision.

### The Critical Verified Blocker
Despite the above successes, **the release image fails the primary acceptance criteria for production deployment**:
- The binary `/usr/local/bin/bhoot-desktop` was compiled on the host system (Ubuntu 24.04 with glibc 2.39) using `cargo build --release --workspace`.
- Compilation against host libc dynamically linked ISO C23 symbols: specifically `__isoc23_strtol@GLIBC_2.38`.
- The target operating system is Debian 12 (Bookworm), which ships with **glibc 2.36** as its maximum runtime version.
- When `bhoot-desktop.service` starts inside the live Debian 12 target system, the dynamic linker halts execution immediately:
  ```text
  /usr/local/bin/bhoot-desktop: /lib/x86_64-linux-gnu/libc.so.6: version `GLIBC_2.38' not found (required by /usr/local/bin/bhoot-desktop)
  ```
- Because `bhoot-desktop` cannot run inside the guest, port 1420 never opens.
- The `cog` kiosk browser launched by `cage` attempts to connect to `http://127.0.0.1:1420`, receives connection refused, and cannot render the BHOOT web frontend.
- Per the Master Specification: **"If actual GUI rendering fails or cannot be observed, mark it NOT VERIFIED. A final PASS requires actual graphical rendering."**
- Therefore, the verdict for the release image is **`FAIL`**.

---

## 2. Section-by-Section Verification Matrix

| Section # | Verification Target | Test Method | Result | Status |
| :--- | :--- | :--- | :--- | :--- |
| **Section 1** | Git Baseline & Artifact Timestamps | `git status`, `git log`, `ls -la` | Commit `224bf30`, Clean tree, Timestamps recorded | **PASS** |
| **Section 2** | Full Disk Image & Component Checksums | `sha256sum`, `sfdisk`, `debugfs`, Python slice hash | Full SHA-256 computed; exact bitwise partition match | **PASS** |
| **Section 3** | Graphical Boot & Visual Evidence | QEMU UEFI + virtio-vga + QMP Screendump | Backend failed (`GLIBC_2.38 not found`); GUI not rendered | **FAIL / NOT VERIFIED** |
| **Section 4** | Financial Engine & Integer Paisa Prec. | Synthetic account & double-entry balanced tx test | 25,000,000 paisa ledger balance; `is_balanced == true` | **PASS** |
| **Section 5** | Anti-Brute-Force PIN Rate Limiting | 5 failed attempts, 60s cooldown, restart test | HTTP 429 triggered; persisted across restart in SQLCipher | **PASS** |
| **Section 6** | Offline Loopback Operation | QEMU `-net none`, nftables default-drop | Strict zero-network enforcement verified; no external traffic | **PASS** |
| **Section 7** | Encrypted Backup & Restore | End-to-end export & restore into clean sandbox | Database integrity OK; bit-for-bit vault hash match | **PASS** |

---

## 3. Cryptographic Checksums & Partition Table Geometry

### Full File & Component Hashes

| Artifact Path | Scope / Geometry | SHA-256 Checksum | Sync Status |
| :--- | :--- | :--- | :--- |
| `os/build_real/bhoot_disk_assembled.img` | **FULL FILE** (62,075,699,200 B) | `996f70a0edd469ac1a9808b19f1571a97c833377c218986abb5a26eb2044a29e` | **FULL DISK IMAGE** |
| `os/build_real/bhoot_disk_assembled.img` | GPT Header (First 2 MiB) | `7eed65a18311fc88171dd0d64d1ffa6d455bf04e4a47f907397cebfcffb575a5` | Verified Partition Table |
| `os/build_real/bhoot_esp.img` | Partition 1 (1.00 GiB FAT32) | `aa7955c073de04cb4094fa3e89ea2e240e56bbd9a70d0dcd825c344edaee0d6e` | Verified Identical to Disk P1 |
| `os/build_real/bhoot_system_ext4.img` | Partition 2 (4.00 GiB Ext4) | `c4728bb4253357ec4140f218afb980e8344db613bd7a59e0f6d1cd7cdd646f92` | Verified Identical to Disk P2 |
| `os/build_real/filesystem.squashfs` | RootFS Image (305,590,272 B) | `d78c479e337d45da19e470af4f3c3a4ad44331d12c3b3251590d26bb62edb0a0` | Verified Identical to P2 squashfs |
| `target/release/bhoot-desktop` | Host Compiled Binary | `074093103148974241866f19452029a0d784b6c39adb3b3d030079a000a1fc8c` | Master Build Binary |
| `os/build_real/rootfs/.../bhoot-desktop`| Installed Rootfs Binary | `074093103148974241866f19452029a0d784b6c39adb3b3d030079a000a1fc8c` | Bit-for-bit Match |
| Extracted from `filesystem.squashfs` | Extracted Live Binary | `074093103148974241866f19452029a0d784b6c39adb3b3d030079a000a1fc8c` | Bit-for-bit Match |
| `os/build_real/BOOTX64.EFI` | GRUB Standalone Binary | `09f395d2bf3f7a8fb53ed912ff060fff01d1b202d6fc347f77d2bf2188359cc9` | Verified in ESP |
| `os/build_real/bhoot_luks_header.backup`| LUKS2 Detached Header Backup | `59032c69328d83f7bb53d0687ac9ecf7d1f6cc8ac7e983b00dfb146744a02577` | Verified |
| `os/build_real/bhoot_recovery.img` | Partition 4 (32.00 MiB Ext4) | `bba1c6dab845c3e37c7408322a9dab69a57359f132fe911cc65efa695280fc19` | Verified |

### Partition Geometry Verification
- **Total Sectors:** 121,241,600 (57.81 GiB @ 512 bytes/sector)
- **Disk GUID:** `B8007001-0000-4000-8000-000000000001`
- **Partition 1 (BHOOT_EFI):** Sector 2048 to 2099199 (1.00 GiB), Type: `C12A7328-F81F-11D2-BA4B-00A0C93EC93B`
- **Partition 2 (BHOOT_SYSTEM):** Sector 2099200 to 10487807 (4.00 GiB), Type: `0FC63DAF-8483-4772-8E79-3D69D8477DE4`
- **Partition 3 (BHOOT_DATA):** Sector 10487808 to 111151103 (48.00 GiB), Type: `0FC63DAF-8483-4772-8E79-3D69D8477DE4`
- **Partition 4 (BHOOT_RECOVERY):** Sector 111151104 to 119539711 (4.00 GiB), Type: `0FC63DAF-8483-4772-8E79-3D69D8477DE4`

---

## 4. In-Guest Graphical Boot Verification Trace

The assembled disk image was booted in QEMU KVM with OVMF UEFI firmware, `-vga virtio` graphics adapter, and QMP screendump enabled.

### Verified Sequence
1. **UEFI Boot:** OVMF loaded standalone GRUB bootloader successfully (`reports/screenshots/01_uefi_grub_boot.png`).
2. **LUKS2 Unlock:** Live-boot initramfs prompted on console; test passphrase accepted; container opened via cryptsetup (`reports/screenshots/02_luks_passphrase_prompt.png`).
3. **Persistent Mount:** Ext4 filesystem on `/dev/mapper/BHOOT_DATA` successfully mounted to `/var/bhoot`.
4. **First-Boot Setup:** Prompts displayed; master password hashed with native yescrypt (`$y$j9T$`); GRUB admin Diceware hash enrolled (`reports/screenshots/03_firstboot_setup.png`).
5. **Kiosk Launch:** `systemd` reached `graphical.target`; `bhoot-gui.service` launched `/usr/bin/cage -s -- /usr/bin/cog -P wl -b #000000 --doc-viewer http://127.0.0.1:1420` on `tty1` (PID 867 verified active).
6. **Backend Failure:** `bhoot-desktop.service` failed immediately upon execution:
   ```text
   /usr/local/bin/bhoot-desktop: /lib/x86_64-linux-gnu/libc.so.6: version `GLIBC_2.38' not found (required by /usr/local/bin/bhoot-desktop)
   ```
7. **Frontend Rendering Outcome:** `cog` could not connect to `127.0.0.1:1420` (Connection Refused). The browser displayed a black canvas (`#000000`), and no interactive web interface, Lock screen, or Dashboard rendered.
8. **Status:** **`NOT VERIFIED`** / **`FAIL`**.

---

## 5. Financial Core, Authentication & Recovery Verification (Host Runtime)

To independently audit the application source code logic independently of the target OS dynamic linker issue, the compiled binary was verified on the host platform across Sections 4, 5, 6, and 7:

```text
  - health: PASS (HTTP 200)
  - initial_locked: PASS (Zero-Trust by default)
  - locked_mutation_401: PASS (Mutations rejected when locked)
  - pin_lockout_429: PASS (5 failures trigger mandatory cooldown)
  - cooldown_6th_429: PASS (6th attempt rejected during cooldown)
  - cooldown_correct_pin_429: PASS (Correct PIN rejected during cooldown)
  - cooldown_persists_restart_429: PASS (Lockout state persisted in SQLCipher across restart)
  - auth_after_cooldown_200: PASS (Valid unlock after 60s cooldown)
  - create_accounts: PASS (Created bank and income accounts, HTTP 201)
  - create_balanced_tx: PASS (Balanced double-entry posting of 25,000,000 paisa, HTTP 201)
  - verify_account_balance: PASS (Bank balance verified at 25,000,000 paisa)
  - verify_net_worth: PASS (Net worth 44,050,000 paisa; is_balanced == true)
  - token_revocation_401: PASS (Locking session immediately invalidates previous token)
  - locked_mutation_rejected: PASS (Unauthorized mutations rejected with HTTP 401)
  - token_rotation: PASS (Unlocking generates fresh high-entropy token)
  - data_persistence_after_restart: PASS (Financial state fully intact across restart)
  - backup_create: PASS (Encrypted backup created: 468,445 bytes)
  - backup_restore_verified: PASS (Database restored into clean sandbox; SQLCipher integrity verified)
```

**Result:** The application business logic, double-entry accounting integrity, cryptographic session protection, and backup/restore mechanisms are architecturally sound and functionally complete.

---

## 6. Root-Cause Analysis & Remediation Path

### Why the Image Failed
The host build system is Ubuntu 24.04 (Linux glibc 2.39). When compiling with default toolchains (`cargo build --release`), the compiler dynamically binds to libc symbols available on the build host (such as `__isoc23_strtol@GLIBC_2.38`). The target OS in the assembled disk image is Debian 12 (Bookworm), which ships with glibc 2.36. This ABI mismatch prevents the binary from executing inside the live target image.

### Exact Remediation Path (For Next Milestone)
To make the assembled release image fully bootable and functional:
1. **Option A (Recommended — Static Musl Binary):** Compile `bhoot-desktop` with target `x86_64-unknown-linux-musl`:
   ```bash
   cargo build --release --target x86_64-unknown-linux-musl
   ```
   A static binary has zero glibc dependencies and runs across all Linux kernels.
2. **Option B (Debian 12 Native Build):** Compile `bhoot-desktop` directly inside the Debian 12 `rootfs` via `chroot` or a Debian 12 Bookworm build container so it dynamically links against Debian 12's native `libc.so.6` (glibc 2.36).
3. **Re-package and Assemble:** Re-sync the newly built binary to `rootfs/usr/local/bin/bhoot-desktop`, rebuild `filesystem.squashfs`, rebuild `bhoot_system_ext4.img`, and reassemble `bhoot_disk_assembled.img`.

---

## 7. Strict Physical Safety Attestation

In accordance with mandatory physical safety rules:
- `/dev/sda` and `/dev/sda1` were **NEVER** modified, written to, partitioned, or mounted.
- `/dev/nvme0n1` was **100% untouched**.
- `deploy_physical_usb.sh` was **NOT executed**.
- No physical USB flash drive was flashed.
- All testing and execution occurred strictly inside isolated QEMU KVM VMs on temporary disposable sparse disk images.

---

## 8. Conclusion

While the low-level boot foundation, disk geometry, cryptographic storage architecture, and application source logic are properly implemented, **the release image fails production acceptance because the compiled binary cannot execute in the target Debian 12 OS due to a glibc 2.38 dynamic linker dependency, preventing graphical frontend rendering**.

**Final Action:** Withhold physical USB flashing until the binary is recompiled using `x86_64-unknown-linux-musl` or Debian 12 native glibc 2.36.
