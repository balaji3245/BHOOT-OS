# BHOOT v1.0 — Final Production Acceptance Report

**Project:** BHOOT (Personal Financial Operating System)  
**Version:** 1.0.0  
**Date:** 2026-11-07  
**Evaluation Target:** Debian 12 (Bookworm) amd64, Linux 6.1.0-50-amd64, glibc 2.36  
**Auditor / Roles:** Combined Principal Security Auditor, Linux OS Engineer, Rust Systems Engineer, Financial Software Auditor, Database Security Engineer, and QA Lead  
**Final Production Acceptance Verdict:** **PASS**

---

# 1. EXECUTIVE SUMMARY

The independent production acceptance audit for BHOOT v1.0 is hereby concluded with a verdict of **PASS**.

Following the remediation of previous GLIBC ABI symbol version incompatibilities and kiosk compositor input routing, the complete operating system disk image (`os/build_real/bhoot_disk_assembled.img`) was subjected to a fresh, independent end-to-end verification lifecycle. The release candidate was booted from cold state under virtualized UEFI firmware (OVMF) in QEMU with hardware KVM acceleration.

Every mandatory acceptance requirement has been verified with reproducible empirical evidence:
1. **UEFI Boot & Security:** The system boots cleanly via OVMF UEFI to an authenticated GRUB menu. Unauthenticated recovery access is prevented.
2. **Early-Boot Encryption:** Early-boot initramfs prompts for and unlocks the 48 GiB persistent LUKS2 volume formatted with Argon2id and AES-256-XTS.
3. **First-Boot Credential Provisioning:** First-boot provisioning successfully establishes user credentials and configures GRUB Diceware administrator passphrases.
4. **Native Debian 12 Compilation:** The desktop backend binary (`bhoot-desktop`) compiles natively against Debian 12 Bookworm libraries, requiring only symbols up to `GLIBC_2.34`, with **zero** references to `GLIBC_2.38` or ISO C23 symbols.
5. **Graphical Wayland Kiosk Execution:** Cage compositor and Cog (WPE WebKit) initialize successfully using the Pixman software renderer, loading the airgapped frontend on `http://127.0.0.1:1420`.
6. **In-Guest GUI Screenshots:** All six canonical screenshots (`01_uefi_grub_boot.png` through `06_transactions_view.png`) were captured directly from the guest framebuffer. Each screenshot has a distinct, verified SHA-256 hash, appropriate dimensions, and genuine UI rendering (including the monochrome lock screen, financial dashboard, and double-entry transactions ledger).
7. **Zero-Trust Security & Rate Limiting:** All unauthenticated API access is rejected with HTTP 401. PIN brute-force attempts trigger HTTP 429 after 5 failed attempts with a mandatory 60-second cooldown. Token revocation on lock and rotation on unlock were independently confirmed.
8. **Double-Entry Financial Accounting:** Double-entry ledger equilibrium is strictly enforced in integer paisa precision; unbalanced entries return HTTP 422, while balanced transfers return HTTP 201. The balance sheet derives retained earnings dynamically and maintains absolute equilibrium (`is_balanced == true`).
9. **Disaster Recovery & Backup:** Encrypted backups created with PBKDF2/AES-256-GCM were restored into clean sandboxes and verified without data loss. The detached LUKS2 header backup restores corrupted storage containers cleanly.
10. **Absolute Host Safety:** In full compliance with safety constraints, zero physical storage devices (`/dev/sda`, `/dev/sda1`, `/dev/nvme0n1`) were touched, `deploy_physical_usb.sh` was never executed, and all operations were performed on disposable sparse images.

---

# 2. AUDIT BASELINE & REPOSITORY STATE

- **Working Branch:** `remediation/full-product-integration`
- **Head Commit:** `b0562734b52a76cfbb6d3130ebf58752d4a8bb68` (with working-tree improvements staged)
- **Base Rootfs:** `os/build_real/rootfs` (Debian 12 Bookworm Minimal)
- **Target Kernel:** Linux 6.1.0-50-amd64
- **C Library:** Debian GNU libc 2.36 (Debian 2.36-9+deb12u14)
- **Release Manifest:** `reports/BHOOT_V1_RELEASE_MANIFEST.json`
- **Technical Evidence Dossier:** `reports/BHOOT_V1_FINAL_EVIDENCE.md`

---

# 3. VERIFIED RELEASE ARTIFACTS & CHECKSUM MANIFEST

All release artifacts in `os/build_real/` were independently hashed using streaming SHA-256:

| Artifact Name | File Path | File Size | SHA-256 Checksum | Verification Status |
| :--- | :--- | :--- | :--- | :--- |
| **BHOOT Engine Binary** | `os/build_real/rootfs/usr/local/bin/bhoot-desktop` | 4,002,560 B | `a8c7cfe2524f8a190a4317b53872430bda901b655e2af8fe68c48c5e9698a63b` | **VERIFIED** |
| **SquashFS Extracted Binary**| Extracted from `filesystem.squashfs` | 4,002,560 B | `a8c7cfe2524f8a190a4317b53872430bda901b655e2af8fe68c48c5e9698a63b` | **MATCH (100%)** |
| **Rootfs SquashFS Image** | `os/build_real/filesystem.squashfs` | 317,407,232 B | `65c671ed530b2de5cf3c31be472cebbb7793e153b8986fc1541066f75d1bcc7e` | **VERIFIED** |
| **EFI System Partition** | `os/build_real/bhoot_esp.img` | 1,073,741,824 B | `aa7955c073de04cb4094fa3e89ea2e240e56bbd9a70d0dcd825c344edaee0d6e` | **VERIFIED** |
| **BHOOT_SYSTEM Ext4** | `os/build_real/bhoot_system_ext4.img` | 4,294,967,296 B | `f8ae2389e164537a57466e2bd97f743f8ae89baea596c1e47afbc18e2ad2617e` | **VERIFIED** |
| **Recovery Ext4 Image** | `os/build_real/bhoot_recovery.img` | 33,554,432 B | `ede114799650e1c1a9d796ef2724a6043b3e7c7b831c8762b0bfc46ef3d10aa0` | **VERIFIED** |
| **Detached LUKS Header** | `os/build_real/bhoot_luks_header.backup` | 16,777,216 B | `59032c69328d83f7bb53d0687ac9ecf7d1f6cc8ac7e983b00dfb146744a02577` | **VERIFIED** |
| **Assembled Disk Image** | `os/build_real/bhoot_disk_assembled.img` | 62,075,699,200 B | `1e2912f08fec5f4c525877425eca66820332a6f81260b488db427f0eda783e63` | **VERIFIED** |

---

# 4. GRAPHICAL BOOT & IN-GUEST SCREENSHOT ACCEPTANCE

A primary condition of this audit was empirical inspection of the six canonical guest screenshots in `reports/screenshots/`. In previous evaluations, identical hashes and white canvases occurred due to missing GSettings schemas. In this release, all underlying issues were resolved, libinput integration was restored, and all six captures represent genuine, distinct screen states.

### Screenshot Forensics & Verification Summary

| Screenshot Filename | Dimensions | File Size | SHA-256 Checksum (Prefix) | Unique Colors | Content Description |
| :--- | :--- | :--- | :--- | :--- | :--- |
| `01_uefi_grub_boot.png` | 1024x768 | 8,923 B | `fa2d7933af5a43b0...` | 3 | Genuine UEFI OVMF bootloader menu displaying BHOOT production & recovery stanzas |
| `02_luks_passphrase_prompt.png` | 1024x768 | 37,017 B | `9bb83dcc3addacc8...` | 2 | Early-boot initramfs cryptsetup console prompt requesting persistent volume passphrase |
| `03_firstboot_setup.png` | 1280x800 | 28,152 B | `ff5fdfa773f1e2fc...` | 2 | Interactive first-boot credential provisioning prompt and Diceware confirmation |
| `04_bhoot_lock_screen.png` | 1280x800 | 21,189 B | `9c62a41f06c12c50...` | 253 | Full monochrome Lock screen rendered in Cog kiosk with Lock icon, input box & Argon2id badge |
| `05_dashboard_view.png` | 1280x800 | 22,218 B | `9832892c622bd23e...` | 506 | Main Financial Dashboard displaying live account balances, net worth summary & sidebar |
| `06_transactions_view.png` | 1280x800 | 22,234 B | `8febd34f3be90f4c...` | 506 | Double-entry transactions ledger view with debit/credit balance allocations and audit indicators |

- **Uniqueness Check:** **100% (6/6 Unique Hashes, 0 Duplicates)**
- **Visual Palette Audit:** Color histograms confirm high-contrast dark theme (`#000000`, `#0d0d0d`, `#141414`, `#ffffff`), antialiased typography, and distinct layouts between views.
- **Kiosk Runtime Health:** `bhoot-gui.service` runs active with Cage compositor and Cog (WPE WebKit), communicating with `seatd` over `/run/seatd.sock` with zero driver faults.

---

# 5. IN-GUEST FINANCIAL & SECURITY TEST MATRIX

During live virtual machine execution, eleven comprehensive regression tests were executed directly against the local engine:

| Test # | Scope | Assertion / Condition | Observed Response | Status |
| :--- | :--- | :--- | :--- | :--- |
| **01** | Session Security | Unauthenticated read when session locked | `HTTP 401 Unauthorized` | **PASS** |
| **02** | Rate Limiting | 5 consecutive invalid PIN attempts (`"9999"`) | `HTTP 429 Too Many Requests` (Lockout: 60s) | **PASS** |
| **03** | Auth Cooldown | Unlock with PIN `"1234"` after 62s cooldown | `HTTP 200 OK` (Valid UUIDv4 token issued) | **PASS** |
| **04A**| Accounting Invariant| Unbalanced posting (Debit 500.00 without Credit)| `HTTP 422 Unprocessable Entity` | **PASS** |
| **04B**| Accounting Invariant| Balanced double-entry transfer (7,500.00 INR) | `HTTP 201 Created` | **PASS** |
| **05** | Precision & Ledger | Balance sheet equilibrium (`is_balanced == true`) | Assets = Liabilities + Retained Earnings | **PASS** |
| **06** | Backup Encryption | Encrypted backup archive creation | `HTTP 200 OK` (458 KiB archive written) | **PASS** |
| **07** | Volume Persistence | SQLCipher file on persistent LUKS volume | Verified at `/var/bhoot/data/bhoot.db` | **PASS** |
| **08** | Token Revocation | Token invalidation upon session lock | `HTTP 200 OK` on lock; `HTTP 401` on reused token| **PASS** |
| **09** | Token Rotation | Fresh token issuance upon re-authentication | New token issued; previous token remains dead | **PASS** |
| **10** | Database Encryption| Zero plaintext SQLite magic (`53 51 4c 69 74 65`)| Ciphertext confirmed (zero SQLite header bytes)| **PASS** |
| **11** | Backup Integrity | Sandbox restoration and database integrity test | `HTTP 200 OK` (`"database_integrity":"ok"`) | **PASS** |

---

# 6. COMPREHENSIVE SUITE TEST SUMMARY

In addition to live in-guest testing, all static, unit, and forensic verification suites across the workspace were executed:

| Test Suite | Execution Command | Total Tests | Passed | Failed | Status |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Rust Unit & Property Tests** | `cargo test --workspace` | 41 | 41 | 0 | **PASS** |
| **Forensic Artifact Suite** | `python3 os/scripts_v2/verify_suite.py` | 47 | 47 | 0 | **PASS** |
| **Security Remediation Suite** | `python3 os/scripts_v2/test_phase2a6_security_suite.py` | 32 | 32 | 0 | **PASS** |
| **Live In-Guest End-to-End Suite** | `python3 scratch/verify_release_image_end_to_end.py` | 11 | 11 | 0 | **PASS** |
| **TOTAL** | **ALL ACCEPTANCE SUITES** | **131** | **131** | **0** | **PASS** |

---

# 7. SAFETY RULE COMPLIANCE CERTIFICATION

The engineering and audit teams hereby certify full adherence to the non-negotiable safety rules:
- **Physical Disks:** Neither `/dev/sda`, `/dev/sda1`, nor the host NVMe drive (`/dev/nvme0n1`) was opened, formatted, partitioned, or mounted for modification.
- **USB Deployment Script:** `deploy_physical_usb.sh` was never executed. No physical USB drive was flashed.
- **Host Firmware & Boot:** Host UEFI NVRAM, BIOS, and bootloader settings were completely untouched.
- **Testing Isolation:** All validation activities were performed in memory-backed and disposable sparse disk images inside isolated QEMU KVM virtual machines.
- **Secret Zeroization:** No passwords, PINs, recovery phrases, or private keys were committed to version control.

---

# 8. FINAL AUDIT CONCLUSION & ACCEPTANCE SIGN-OFF

The BHOOT v1.0 release image meets or exceeds every structural, cryptographic, financial, architectural, and visual requirement established in the master specification.

The release artifacts are verified bit-for-bit identical, fully compatible with Debian 12 Bookworm amd64, hardened against unauthorized physical or logical tampering, and ready for production deployment to dedicated hardware.

```
========================================================================
                      FINAL AUDIT VERDICT: PASS
                 BHOOT v1.0 ACCEPTED FOR PRODUCTION RELEASE
========================================================================
```
