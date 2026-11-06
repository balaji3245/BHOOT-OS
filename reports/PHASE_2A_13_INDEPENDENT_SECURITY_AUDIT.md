# PROJECT BHOOT — Phase 2A.13 Independent Security Audit Report

**Date:** 2026-11-06  
**Auditor Role:** Independent Linux Security Auditor, Cryptographic Systems Reviewer, Boot Architecture Specialist  
**Repository Working Directory:** `/home/bhoot/Desktop/BHOOT`  
**Baseline Branch:** `security/phase-2a-12-remediation`  
**Baseline Commit:** `6c5deea`  
**Previous Remediation Reference:** Phase 2A.12 Remediation Report (`6c5deea`)  
**Target Hardware Platform:** Dell Latitude 5410, Class 3 UEFI (x86_64), SanDisk / HP 64 GB USB 3.0 Flash Drive  

---

## 1. Executive Summary & Audit Verdict

An independent, adversarial source-code and runtime security audit of Project BHOOT was conducted on the baseline commit `6c5deea`. Every finding in this report is based exclusively on empirical verification, disassembled/inspected binaries, live test executions, and direct source inspection. Previous self-reported PASS results were treated as unverified claims.

### Audit Verdict: **PASS WITH CONDITIONS**

The underlying cryptographic foundation, LUKS2 Argon2id volume encryption, native yescrypt 128-bit salt generation, machine-ID atomic durability, and loopback teardown mechanisms operate correctly and satisfy stringent offline financial security principles. However, **1 High-Severity Operational Finding** and **3 Medium-Severity Architectural Findings** prevent unconditional sign-off for physical deployment.

```
========================================================================================
 FINDINGS SEVERITY MATRIX
========================================================================================
 [CRITICAL] 0 Findings
 [HIGH]     1 Finding   (Recovery Shell Root Lockout under systemd rescue.target)
 [MEDIUM]   3 Findings  (Unsigned EFI Binary, ESP Offline Tampering Risk, SQLCipher Unlinked)
 [LOW]      3 Findings  (Mock Allowlist Hook, Recovery Key Normalization, Shell Zeroization)
 [INFO]     1 Note      (Volatile USB Flash FTL Limitations Disclosed)
========================================================================================
```

---

## 2. Comprehensive Findings Log

### [HIGH-01] Recovery Shell Inaccessible Due to Locked Root Account in `rescue.target`
- **Location:** `os/config/boot/grub.cfg#L79-L90` and `os/scripts_v2/02_configure_rootfs.sh#L66-L79`
- **Severity:** **HIGH**
- **Classification:** Broken Disaster Recovery Access / Operational Lockout
- **Verified Facts & Evidence:**
  1. `grub.cfg` defines the recovery menu entry:
     ```
     menuentry "BHOOT — Offline Recovery Shell & Header Restore" --users bhoot-admin --id bhoot-recovery {
         linux /live/vmlinuz boot=live components systemd.unit=rescue.target console=tty0 console=ttyS0,115200 ... bhoot.recovery=1
         initrd /live/initrd.img
     }
     ```
  2. `02_configure_rootfs.sh` configures `/etc/shadow` with `root:!:19000:0:99999:7:::` and does not install `sudo` (`/usr/bin/sudo` is absent, `/etc/sudoers` does not exist).
  3. When Debian 12 systemd boots with `systemd.unit=rescue.target`, it invokes `/lib/systemd/systemd-sulogin-shell rescue`.
  4. Standard `sulogin` queries `/etc/shadow` for `root`. When `root` has password `!` (locked), `sulogin` aborts with:
     `Cannot open access to console, the root account is locked.`
  5. The automated lifecycle test (`test_qemu_uefi_lifecycle.py`, Boot 3) verified that GRUB accepted the `bhoot-admin` PBKDF2 password and initiated recovery kernel load, but exited QEMU immediately after kernel load without verifying that a functional shell was spawned.
- **Impact:** An operator who successfully authenticates as `bhoot-admin` in GRUB to recover from a corrupted persistence volume is locked out at the console and cannot access disaster recovery utilities (`restore_header.sh`).
- **Recommended Remediation:**
  Install a systemd drop-in override for `rescue.service` (`/etc/systemd/system/rescue.service.d/override.conf`) setting `Environment=SYSTEMD_SULOGIN_FORCE=1` or configure `ExecStart` to launch an authenticated maintenance shell or `/usr/local/sbin/bhoot-recovery-console`.

---

### [MED-01] Unsigned EFI Binary Requires Secure Boot to be Disabled in Host Firmware
- **Location:** `os/scripts_v2/04_build_real_esp.sh#L53-L57` and `os/build_real/BOOTX64.EFI`
- **Severity:** **MEDIUM**
- **Classification:** Boot Architecture / Firmware Incompatibility
- **Verified Facts & Evidence:**
  1. Inspected `BOOTX64.EFI` via `sbverify`:
     ```
     $ sbverify --list os/build_real/BOOTX64.EFI
     No signature table present
     ```
  2. The EFI executable is generated directly via `grub-mkstandalone` without signing by Microsoft 3rd Party UEFI CA or a user-enrolled Machine Owner Key (MOK).
- **Impact:** On the target Dell Latitude 5410 (Class 3 UEFI) with factory default settings (Secure Boot Enabled), the UEFI firmware will reject `BOOTX64.EFI` with "Access Denied" or "Security Violation". The operating system will not boot until the owner manually enters UEFI setup and disables Secure Boot.
- **Recommended Remediation:**
  1. Formally document in hardware deployment instructions that Secure Boot must be disabled in firmware.
  2. For production Phase 2B, integrate standard Debian `shimx64.efi` with a dedicated BHOOT MOK or custom PK/KEK/db signing pipeline.

---

### [MED-02] Physical Offline Tampering Risk on Unencrypted FAT32 ESP Partition
- **Location:** `os/config/storage/gpt_layout.sfdisk#L10` and `os/scripts_v2/04_build_real_esp.sh#L86-L135`
- **Severity:** **MEDIUM**
- **Classification:** Physical Threat Model / Unmeasured Early Boot
- **Verified Facts & Evidence:**
  1. Partition 1 (`BHOOT_EFI`, 1 GiB) is formatted as standard unencrypted FAT32 (`mkfs.vfat -F 32 -n BHOOT_EFI`).
  2. It houses `BOOTX64.EFI`, `grub.cfg`, `live/vmlinuz`, and `live/initrd.img`.
  3. Because Secure Boot is disabled and no TPM2 measured boot / PCR sealing is implemented, an attacker with temporary physical possession of the USB drive can mount Partition 1 on any laptop, modify `initrd.img` to inject credential harvesting code, and calculate valid SHA-256 sums.
- **Impact:** Vulnerable to Evil Maid attacks if an attacker intercepts the physical USB drive between boot sessions.
- **Recommended Remediation:**
  Enforce physical custody controls (tamper-evident USB casing) and in future phases implement TPM2 PCR-7 / PCR-11 measured boot verification.

---

### [MED-03] SQLCipher Runtime Unlinked in Core Database Crate
- **Location:** `crates/bhoot-db/Cargo.toml#L11-L18` and `crates/bhoot-db/src/config.rs#L37-L51`
- **Severity:** **MEDIUM**
- **Classification:** Implementation Completeness / Cryptographic Architecture
- **Verified Facts & Evidence:**
  1. Inspected `crates/bhoot-db/Cargo.toml`: dependencies include `bhoot-core`, `bhoot-crypto`, `serde`, `thiserror`, `uuid`, `chrono`. There is **no dependency on `rusqlite` or `libsqlite3-sys` with the `sqlcipher` feature**.
  2. `crates/bhoot-db/src/config.rs` only generates text PRAGMA statements (`format!("PRAGMA key = ...")`).
  3. Checked Debian rootfs: neither `/usr/bin/sqlcipher` nor `/usr/lib/x86_64-linux-gnu/libsqlcipher.so*` is installed.
  4. The test `test_sqlcipher_config_pragma_generation` only checks string formatting (`assert!(pragmas[0].starts_with(...))`), not cryptographic SQLite page encryption or WAL journal encryption.
- **Impact:** Application database records currently rely entirely on whole-partition LUKS2 encryption. While LUKS2 protects at-rest data when powered off, secondary application-level defense-in-depth via SQLCipher page encryption is not yet functional.
- **Recommended Remediation:**
  In Phase 3 (Application Integration), add `rusqlite = { version = "0.31", features = ["bundled-sqlcipher"] }` to `crates/bhoot-db` and implement live database integration tests.

---

### [LOW-01] Automated Regression Suite Short-Circuits Hardware Allowlist via Mock Hook
- **Location:** `os/scripts_v2/export_recovery_backup.sh#L51-L67` and `os/scripts_v2/test_phase2a10_remediation_suite.py#L194-L208`
- **Severity:** **LOW**
- **Classification:** Testing Rigor / Mock Reliance
- **Verified Facts & Evidence:**
  1. `export_recovery_backup.sh` contains Rule 5:
     ```bash
     if [ -n "${BHOOT_BACKUP_ALLOWLIST_MOCK:-}" ]; then
         case "$BHOOT_BACKUP_ALLOWLIST_MOCK" in
             "usb_removable") return 0 ;;
     ```
  2. `test_phase2a10_remediation_suite.py` tests allowlist acceptance by setting `BHOOT_BACKUP_ALLOWLIST_MOCK="usb_removable"`.
  3. This hook causes the script to return `0` (ALLOWED) immediately, skipping Rules 6..14 (real `findmnt`, `readlink`, `udevadm` properties, `/sys/block/*/removable` flags).
- **Impact:** While negative rejections (Rules 1-4) execute directly against the real environment, positive allowlist acceptance is never validated against a real physical external USB device in CI.
- **Recommended Remediation:**
  In physical staging (Phase 2B), run `export_recovery_backup.sh` against a genuine secondary USB flash drive without `BHOOT_BACKUP_ALLOWLIST_MOCK`.

---

### [LOW-02] Recovery Key Requires Strict Format Matching Without Input Normalization
- **Location:** `os/scripts_v2/provision_luks2_volume.py#L40-L42` and `L113`
- **Severity:** **LOW**
- **Classification:** Disaster Recovery Usability
- **Verified Facts & Evidence:**
  1. `provision_luks2_volume.py` enrolls Keyslot 1 with the chunked string:
     `XXXX-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX` (79 characters including hyphens).
  2. `cryptsetup open` evaluates key inputs byte-for-byte. If an operator enters the 64 hexadecimal characters in lowercase or omits the hyphens during emergency recovery, unlock will fail.
- **Impact:** Operator input formatting errors during disaster recovery could lead to perceived recovery failure.
- **Recommended Remediation:**
  Provide an interactive pre-boot recovery unlock helper that strips whitespace/hyphens and attempts both chunked uppercase and raw forms.

---

### [LOW-03] Scripting Memory Zeroization Limitations in First-Boot Provisioning
- **Location:** `os/scripts_v2/02_configure_rootfs.sh#L210-L260`
- **Severity:** **LOW**
- **Classification:** Cryptographic Systems / Scripting Constraints
- **Verified Facts & Evidence:**
  1. `bhoot-firstboot-setup` uses Bash `unset PASSWD_INPUT`. Bash does not scrub memory pages when unsetting variables.
  2. Plaintext credentials pass through a kernel IPC pipe (`printf ... | perl`).
  3. Perl `$pwd = "0" x length($pwd)` does not scrub internal `PerlIO` read buffers.
- **Impact:** Residual password bytes remain in process heap/stack memory until the process terminates. (Mitigated: process memory is volatile, swap is unencrypted/disabled, and script executes only once).
- **Recommended Remediation:**
  Migrate first-boot provisioning to a compiled binary utilizing `explicit_bzero` (matching `bhoot-grub-pbkdf2.c`).

---

## 3. Deep-Dive Audit Areas

### 3.1 Boot Security
- **UEFI Firmware & Early Boot:** GRUB standalone binary is built with embedded modules (`part_gpt`, `fat`, `ext2`, `linux`, `password_pbkdf2`, `gcry_sha512`). It successfully boots in UEFI OVMF.
- **GRUB Authentication Policy:**
  - `set superusers="bhoot-admin"`
  - Normal production boot (`menuentry "BHOOT — ... (Secure Production)"`) is `--unrestricted` $\rightarrow$ boots unattended without operator interaction.
  - Recovery and Diagnostic entries specify `--users bhoot-admin` $\rightarrow$ require PBKDF2 authentication.
  - Editing boot commands (`e`) or accessing GRUB rescue shell (`c`) strictly requires `bhoot-admin` credentials.
  - Verified Fail-Closed Behavior: Prior to first-boot setup, `grub-auth.cfg` contains `# Status: UNENROLLED`. GRUB fails closed on restricted entries until enrolled.

### 3.2 Cryptography
- **LUKS2 Argon2id Volume:**
  - Inspected production container header backup (`bhoot_luks_header.backup`).
  - Memory: 64 MiB (`65536` KiB), Time: 4 iterations, Parallelism: 2 threads.
  - Cipher: `aes-xts-plain64`, 512-bit key size.
  - Sector size: 4096 bytes.
  - Keyslot 0 (Master Passphrase) and Keyslot 1 (Recovery Key) both verified enrolled.
- **Yescrypt Salt Generation & Entropy:**
  - Evaluated the actual generator in `02_configure_rootfs.sh`.
  - 16 bytes (128 bits) CSPRNG entropy read from `/dev/urandom`.
  - 15 bytes (120 bits) produce characters 0..19 (uniform 64 base64 chars).
  - Byte 15 lower 6 bits produce character 20 (uniform 64 chars).
  - Byte 15 upper 2 bits produce character 21 (strictly 2 bits, values $0, 1, 2, 3$ mapping to `['.', '/', '0', '1']`).
  - Total entropy: $21 \times 6 + 2 = 128$ bits. Conforms 100% to `libxcrypt` (`crypt_gensalt_rn`).
  - Statistical Verification: Executed 64,000 generator iterations. Chi-square goodness-of-fit yielded $\chi^2 = 79.66$ ($df=63$, $p > 0.001$) on pos 0..20, and $\chi^2 = 9.43$ ($df=3$, $p > 0.001$) on pos 21. Characters outside `['.', '/', '0', '1']` in pos 21: **0 (Zero)**.
- **GRUB PBKDF2 Benchmarking & Security:**
  - Evaluated `os/tools/bhoot-grub-pbkdf2.c` linked against OpenSSL 3.0 `libcrypto`.
  - 10,000 iterations of HMAC-SHA512 with 512-bit CSPRNG salt.
  - Userspace runtime: **6.62 ms average**.
  - UEFI GRUB runtime: **150 ms to 250 ms**.
  - Defensible Standard: Provides instantaneous recovery unlock while requiring $> 10^{25}$ SHA-512 computations against a 12+ character high-entropy passphrase.

### 3.3 Storage & Data Integrity
- **Detached Header Recovery Test (Independent Execution):**
  - Created disposable 64 MiB LUKS2 Argon2id container.
  - Backed up header via `luksHeaderBackup`.
  - Catastrophically corrupted primary and secondary headers by zeroing the first 8 MiB (`dd if=/dev/zero ... count=8`). Verified `cryptsetup isLuks` returned failure.
  - Executed `luksHeaderRestore` using detached backup.
  - Verified `cryptsetup isLuks` returned success; tested passphrase unlock $\rightarrow$ **PASS (100%)**.
- **Manifest Scope Differentiation:**
  - Validated `verify_manifest.py`:
    - `FULL_FILE_SHA256`: 6 artifacts verified across 100% byte range.
    - `HEADER_ONLY_SHA256` / `PARTITION_HEADER_SHA256`: 3 artifacts verified across bounded metadata offsets.
  - Empirical proof demonstrated: tampering at offset 5 MiB is detected by full-file verification while leaving header hashes intact.

### 3.4 Loop Device Lifecycle & Automount Prevention
- **Loop Teardown Verification:**
  - Step 1 cleanup executed during previous session unmounted all 4 child partitions (`loop33p2`, `loop33p4`, `loop34p2`, `loop34p4`) and detached `loop31`, `loop33`, `loop34`.
  - Live inspection confirmed: `losetup -a` contains 0 non-snap devices; `findmnt -l | grep BHOOT` contains 0 mounts.
  - Desktop PolicyKit LUKS passphrase popup condition is **PERMANENTLY RESOLVED**.
- **Automount Prevention:**
  - Verified `06_assemble_disk_image.sh`: whole-disk UDisks attachments are strictly forbidden; only bounded offset/size probing is permitted.
  - Verified cleanup traps: `04_build_real_esp.sh` and `06_assemble_disk_image.sh` register `trap ... EXIT INT TERM HUP ERR`.
  - Verified Python managers: `provision_luks2_volume.py` and `luks_ext4_manager.py` enforce `unmount`, `lock`, and `loop-delete` in `finally:` blocks.

### 3.5 Fresh Disposable QEMU 3-Stage Lifecycle Test
Conducted a clean execution of `test_qemu_uefi_lifecycle.py` on a freshly assembled disposable 57.81 GiB image:
- **Boot 1 (First-Boot Provisioning):** OVMF UEFI loaded $\rightarrow$ GRUB loaded $\rightarrow$ Kernel & initramfs booted $\rightarrow$ LUKS unlocked $\rightarrow$ Master password provisioned $\rightarrow$ GRUB admin PBKDF2 enrolled $\rightarrow$ Root login rejected $\rightarrow$ Master user logged in $\rightarrow$ Machine-ID persisted (`b0a908eb...`) $\rightarrow$ Clean shutdown $\rightarrow$ **PASS**.
- **Boot 2 (Reboot Persistence):** Cold boot $\rightarrow$ Unattended GRUB $\rightarrow$ LUKS unlocked $\rightarrow$ User logged in $\rightarrow$ Persistence marker verified $\rightarrow$ Machine-ID identical (`b0a908eb...`) $\rightarrow$ Clean shutdown $\rightarrow$ **PASS**.
- **Boot 3 (GRUB Recovery Authentication):** GRUB menu reached $\rightarrow$ Recovery entry selected $\rightarrow$ Wrong password rejected ('access denied') $\rightarrow$ Positive owner password accepted $\rightarrow$ Recovery kernel booted $\rightarrow$ **PASS**.
- **Post-Test Teardown:** All test images unlinked; verified 0 lingering loop devices.

---

## 4. Physical Safety & System Boundary Audit

- **Physical USB Target (`/dev/sda`, `/dev/sda1`, `HP_USB`):**
  - **Status: 100% UNTOUCHED.**
  - Verified via `lsblk`: Path `/dev/sda1` (57.8 GiB, FAT32) remained mounted at `/run/media/bhoot/HP_USB` without write access or partition changes.
- **Host Laptop NVMe SSD (`/dev/nvme0n1`):**
  - **Status: 100% UNTOUCHED.**
  - Host OS partitions (`nvme0n1p1` ESP, `nvme0n1p2` root, `nvme0n1p3` swap) completely unaltered.
- **Host NVRAM / UEFI Variables:**
  - No `efibootmgr` commands executed; host NVRAM untouched.

---

## 5. Required Remediations for Production Gate

Before physical USB deployment (Phase 2B) can be authorized, the following items must be remediated:

1. **Fix Recovery Shell Access (`HIGH-01`):**
   Add a systemd drop-in override for `rescue.service` or a dedicated recovery launcher script so that authenticated `bhoot-admin` operators are not locked out by `sulogin` due to the locked root account.
2. **Document Firmware Secure Boot Requirement (`MED-01`):**
   Provide explicit documentation that host UEFI Secure Boot must be disabled in Dell Latitude 5410 BIOS setup prior to booting BHOOT.
3. **Application Integration Roadmap (`MED-03`):**
   Formally schedule `rusqlite` + SQLCipher linking for Phase 3 application development.
4. **Physical USB Gating:**
   Phase 2B physical deployment remains **STRICTLY PROHIBITED** until owner review and approval of the above remediations.

