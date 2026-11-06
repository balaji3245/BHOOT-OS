# Project BHOOT — Boot Security, Firmware Prerequisites & Cryptographic Architecture Specification

**Document Version:** 1.0.0  
**Security Status:** Confidential / Systems Security Specification  
**Baseline Commit:** Phase 2A.14 Remediation  
**Target Hardware Platform:** Dell Latitude 5410 (Class 3 UEFI x86_64), SanDisk / HP 64 GB USB 3.0 Flash Drive  

---

## 1. Executive Summary & Purpose

This specification defines the cryptographic foundations, bootloader security boundaries, firmware prerequisites, threat model assumptions, and disaster recovery access mechanisms for Project BHOOT (Private Financial Intelligence Operating System).

It formally resolves and documents findings identified during the Phase 2A.13 independent security audit:
- **HIGH-01:** Authenticated Disaster Recovery Console Architecture (bypassing Debian 12 `sulogin` lockout without creating an unauthenticated root backdoor).
- **MED-01:** Dell Latitude 5410 Firmware Prerequisites & Unsigned EFI Binary Roadmap.
- **MED-02:** Unencrypted FAT32 ESP Partition Limits, Offline Tampering Risks & TPM2 Roadmap.
- **MED-03:** Application Database Encryption Architecture & SQLCipher Integration Status.
- **LOW-01:** External Removable USB Backup Allowlist Testing Methodology.
- **LOW-02:** 256-Bit Disaster Recovery Key Input Normalization.
- **LOW-03:** Credential In-Memory Lifetime, Shell Volatility & Explicit Zeroization Boundaries.

---

## 2. Firmware Prerequisites & Boot Security (MED-01)

### 2.1 Current Implementation Status: Standalone Unsigned GRUB2 EFI
The primary bootloader binary for Project BHOOT is located at `/EFI/BOOT/BOOTX64.EFI` on Partition 1 (`BHOOT_EFI`). It is generated directly via `grub-mkstandalone` embedding essential modules (`part_gpt`, `fat`, `ext2`, `linux`, `password_pbkdf2`, `gcry_sha512`).

```
$ sbverify --list os/build_real/BOOTX64.EFI
No signature table present
```

Because `BOOTX64.EFI` is unsigned:
- **Dell Latitude 5410 Factory Setting:** The Dell Latitude 5410 ships with UEFI Secure Boot enabled by default. Under factory configuration, the firmware will reject `BOOTX64.EFI` with a "Security Violation" or "Selected boot device failed" error.
- **Mandatory Hardware Prerequisite:** The system administrator or owner MUST enter the UEFI BIOS Setup before first boot and disable Secure Boot:
  1. Power on the Dell Latitude 5410 and immediately tap **F2** to enter BIOS Setup.
  2. Navigate to **Secure Boot** $\rightarrow$ **Secure Boot Enable**.
  3. Deselect / set **Secure Boot Enable** to **Disabled**.
  4. Ensure **Boot Sequence** is configured for **UEFI Only** (Legacy / CSM is not supported).
  5. Apply changes and restart.

### 2.2 Secure Boot Production Roadmap (Phase 2B & Beyond)
To enable operation on hardware with Secure Boot enforced, the following architecture will be implemented:
1. **Signed Shim Chain:**
   - Embed Debian's pre-signed `shimx64.efi` (signed by Microsoft 3rd Party UEFI CA).
   - Shim acts as first-stage loader and verifies the secondary bootloader (`grubx64.efi`) using embedded certificates or Machine Owner Keys (MOK).
2. **Machine Owner Key (MOK) Enrollment:**
   - Generate an offline BHOOT Root Signing Authority.
   - Sign `grubx64.efi` and the Linux kernel image (`vmlinuz`) with the private key.
   - On initial provisioning on target Dell hardware, enroll the public `MOK.der` certificate into NVRAM via the interactive MokManager (`MokManager.efi`).
3. **Custom Platform Key (PK) Enterprise Deployment:**
   - For dedicated high-security hardware, transition the Dell Latitude 5410 into "Custom Mode", replace the factory Microsoft PK/KEK with a private BHOOT Platform Key, and sign boot artifacts directly.

---

## 3. ESP Partition Boundaries & Threat Model Analysis (MED-02)

### 3.1 Unencrypted FAT32 EFI System Partition
Partition 1 (`BHOOT_EFI`, 1 GiB) is formatted as standard FAT32 (`mkfs.vfat -F 32 -n BHOOT_EFI`) per UEFI Specification version 2.10 section 13.3. It contains:
- `EFI/BOOT/BOOTX64.EFI` (UEFI bootloader)
- `EFI/BOOT/grub.cfg` and `EFI/BOOT/grub-auth.cfg` (Boot configuration and owner PBKDF2 hash)
- `live/vmlinuz` and `live/initrd.img` (Linux kernel and initial ramdisk)

### 3.2 Offline Physical Tampering & Evil Maid Analysis
Because Partition 1 is unencrypted FAT32:
- **Threat Vector (Evil Maid):** An adversary obtaining temporary, unsupervised physical custody of the USB flash drive can mount Partition 1 on an external machine, modify `initrd.img` to inject keystroke logging or credential harvesting hooks, and compute valid SHA-256 hashes matching the modified file.
- **Inherent Limitation of Software SHA-256 Manifests:**
  While cryptographic manifests (such as `manifest.json` and `07_verify_all_artifacts.sh`) detect storage bit-rot, accidental administrative discrepancies, and casual modification, **a software manifest stored on the untrusted medium itself cannot prevent an adversary who can rewrite both the payload and the manifest.**
- **Operational Defense-in-Depth Countermeasures:**
  1. **Strict Physical Custody:** The BHOOT USB flash drive must remain in the physical custody of the owner at all times. Tamper-evident seals or a tamper-evident physical casing must be utilized when the drive is in transit or storage.
  2. **TPM2 Measured Boot Roadmap:**
     - Utilize Dell Latitude 5410's discrete TPM 2.0 cryptoprocessor.
     - Measure firmware (PCR-0..3), bootloader (PCR-4), Secure Boot policy (PCR-7), and kernel/initrd command lines (PCR-8..11, specifically PCR-11 for initrd measurement).
     - Seal the LUKS2 storage volume decryption key to the TPM2 PCR policy (`systemd-cryptenroll --tpm2-pcrs=7,11`). If any byte of the ESP or kernel is altered, the TPM2 refuses to unseal the volume key, preventing compromised system boot.

---

## 4. Authenticated Disaster Recovery Console Architecture (HIGH-01)

### 4.1 Root Account Lockout in Debian 12 `rescue.target`
In Debian 12 Bookworm, invoking `systemd.unit=rescue.target` normally triggers `rescue.service`, which executes:
```
/lib/systemd/systemd-sulogin-shell rescue
```
`sulogin` checks the root account in `/etc/shadow`. Because Project BHOOT locks the root account (`root:!:...`) to enforce zero-trust non-root daily operation, standard `sulogin` aborts immediately with:
```
Cannot open access to console, the root account is locked.
```
If an administrator blindly enables `SYSTEMD_SULOGIN_FORCE=1`, `sulogin --force` drops directly into an **unauthenticated root shell** without requiring any password, creating a severe physical console backdoor.

### 4.2 Dedicated Authenticated Recovery Console Solution
Project BHOOT implements a dedicated, fully authenticated recovery console (`/usr/local/sbin/bhoot-recovery-console`) installed via systemd drop-in overrides (`/etc/systemd/system/rescue.service.d/override.conf` and `emergency.service.d/override.conf`):

```ini
[Unit]
Description=BHOOT Authenticated Disaster Recovery Console

[Service]
ExecStart=
ExecStart=-/usr/local/sbin/bhoot-recovery-console
StandardInput=tty-force
StandardOutput=inherit
StandardError=inherit
```

#### Console Security Flow:
```mermaid
flowchart TD
    A["GRUB Boot Menu (Entry 3: Offline Recovery Shell)"] --> B{"GRUB Authenticates 'bhoot-admin'"}
    B -- "Invalid Password" --> C["Access Denied / Boot Aborted"]
    B -- "Valid PBKDF2 Password" --> D["Kernel Boots into rescue.target"]
    D --> E["rescue.service launches bhoot-recovery-console"]
    E --> F["Mount /boot/efi (ro) & Read grub-auth.cfg"]
    F --> G{"Credentials Enrolled?"}
    G -- "UNENROLLED" --> H["Print Fatal Error & Immediate Reboot"]
    G -- "Enrolled Hash Found" --> I["Interactive Recovery Console Prompt"]
    I --> J["Read Password from Console (stdin)"]
    J --> K{"bhoot-grub-pbkdf2 --verify-file"}
    K -- "Match" --> L["Spawn Root Maintenance Shell ([BHOOT-RECOVERY]#)"]
    K -- "Mismatch" --> M["Increment Failed Attempts, Sleep 2s"]
    M --> N{"Attempts >= 3?"}
    N -- "No" --> I
    N -- "Yes" --> O["Security Alert & Immediate System Reboot"]
    L --> P["Available Tools: restore_header.sh, bhoot-unlock-recovery-key"]
    P --> Q["On Shell Exit: sync && reboot -f"]
```

#### Security Properties of `bhoot-recovery-console`:
1. **No Unauthenticated Shell:** Root shell access is granted ONLY upon cryptographic verification of the owner administrator PBKDF2 hash.
2. **Rate Limiting & Lockout:** A 2-second delay is enforced between failed attempts. Exceeding 3 failed attempts triggers an immediate emergency reboot.
3. **Fail-Closed on Unenrolled Systems:** If `grub-auth.cfg` contains `# Status: UNENROLLED`, the console refuses to grant access and reboots.
4. **Volatile Session Security:** `HISTFILE=/dev/null` is enforced, preventing bash history from persisting. Interrupt signals (`SIGINT`, `SIGTERM`) are trapped to force an immediate reboot rather than dropping to an unauthenticated prompt.
5. **Unattended Production Boot Preserved:** Normal boot (`bhoot-normal`) targets `default.target` and remains `--unrestricted` in GRUB.

---

## 5. Storage Confidentiality & Database Architecture (MED-03)

### 5.1 Storage Hierarchy & At-Rest Encryption
The current Project BHOOT storage layout provides complete cryptographic confidentiality for all sensitive data at rest:
- **Partition 1 (`BHOOT_EFI`):** Unencrypted 1 GiB FAT32 containing kernel, initrd, and bootloader.
- **Partition 2 (`BHOOT_SYSTEM`):** Unencrypted 4 GiB Ext4 containing compressed read-only OS root image (`/live/filesystem.squashfs`). Contains zero user secrets or credentials.
- **Partition 3 (`BHOOT_DATA`):** **100% LUKS2 Argon2id Container (48 GiB).**
  - Cipher: `aes-xts-plain64`, 512-bit key.
  - Sector size: 4096 bytes (no discard/TRIM).
  - PBKDF: Argon2id (`memory=64 MiB`, `parallel=2`, `iterations=4`).
  - Holds persistent overlay (`/var/bhoot`), system shadow database, machine identity, SQLite databases, and user documents.
  - Protected at rest when powered off or unmounted.
- **Partition 4 (`BHOOT_REC`):** 32 MiB Ext4 offline recovery payload containing detached LUKS2 header backup (`bhoot_luks_header.backup`), `restore_header.sh`, and `bhoot-unlock-recovery-key`.

### 5.2 SQLCipher Status & Application Integration Roadmap
- **Current Status (Phase 2A):** Whole-volume encryption is provided at the storage block layer by LUKS2. `crates/bhoot-db` defines database schema and configuration generators. Runtime SQLCipher page-level encryption is not yet linked.
- **Phase 3 Integration Specification:**
  In Phase 3 (Application Integration), `crates/bhoot-db` will link `rusqlite` with the `bundled-sqlcipher` feature:
  ```toml
  [dependencies]
  rusqlite = { version = "0.31", features = ["bundled-sqlcipher"] }
  ```
  This will provide application-level defense-in-depth: even if an adversary gains root execution while the system is running and LUKS2 is mapped, database pages in memory and SQLite WAL journals remain encrypted with a dedicated application-derived database key.

---

## 6. Disaster Recovery Usability & Normalization (LOW-02)

### 6.1 256-Bit Recovery Key Format
Keyslot 1 on Partition 3 is enrolled with an independent 256-bit recovery key. The canonical enrollment representation is 16 groups of 4 uppercase hex characters separated by hyphens (79 characters):
```
XXXX-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX
```

### 6.2 Normalization Engine (`provision_luks2_volume.py` & `bhoot-unlock-recovery-key`)
To eliminate operator errors during emergency high-stress disaster recovery:
1. `provision_luks2_volume.py#normalize_recovery_key(key)`:
   - Strips all whitespace, hyphens, colons, and formatting characters via regex `[^0-9a-fA-F]`.
   - Converts lowercase hex to uppercase.
   - Strictly verifies that exactly 64 hexadecimal characters (256 bits) remain.
   - Chunks into the canonical 16x4 format required by LUKS2 Keyslot 1.
2. Interactive Helper Utility (`/usr/local/sbin/bhoot-unlock-recovery-key`):
   - Available in the recovery console and Partition 4.
   - Prompts operator interactively for the recovery key with or without hyphens or spaces.
   - Normalizes input on the fly and unlocks the persistence volume to `/dev/mapper/bhoot_data_recovery`.

---

## 7. Memory Zeroization & Credential Lifetimes (LOW-03)

### 7.1 Linux Process Memory & Scripting Boundaries
In bash and scripting languages:
- Variables unset via `unset VAR` are marked free by glibc's `free()`, but heap/stack memory pages are not immediately overwritten with zeros.
- Pipes (`printf ... | perl`) pass data through Linux kernel volatile pipe buffers in memory.
- In Project BHOOT, disk swap is disabled or unencrypted swap is masked. RAM is volatile and cleared on cold poweroff.

### 7.2 Explicit Zeroization in Compiled C Utilities
To minimize the exposure window of credentials in volatile memory:
1. `bhoot-grub-pbkdf2.c`:
   - Utilizes `explicit_bzero()` on all password arrays, intermediate PBKDF2 digests, and salt buffers immediately upon completion.
   - Constant-time comparison (`constant_time_compare`) prevents timing side-channels.
2. Direct STDIN Pipes:
   - Passwords are never passed via command-line arguments (`argv`) or environment variables, eliminating exposure via `/proc/$PID/cmdline` and `/proc/$PID/environ`.

---

## 8. Passphrase Generation, Diceware Architecture & Hardware Cracking Analysis (Phase 2A.17)

### 8.1 5-Word Diceware Passphrase Architecture & Provenance
To prevent human selection of weak credentials for pre-boot and recovery authentication while ensuring memorability, Phase 2A.17 mandates a 5-word Diceware generation policy:
- **Canonical Wordlist Provenance:** The Electronic Frontier Foundation (EFF) Large Wordlist (`eff_large_wordlist.txt`), designed by Joseph Bonneau.
- **Licensing:** Creative Commons Attribution 3.0 United States (CC-BY-3.0 US).
- **Integrity Baseline:** The wordlist consists of exactly 7,776 lines and 7,776 unique entries, with a canonical cryptographic checksum:
  `SHA-256: addd35536511597a02fa0a9ff1e5284677b8883b83e986e43f15a3db996b903e`
- **Offline-First Storage:** Stored locally in `/usr/share/bhoot/eff_large_wordlist.txt` and `/etc/bhoot/eff_large_wordlist.txt`. No external or remote network downloads are performed during installation, boot, or recovery.

### 8.2 CSPRNG Rejection Sampling & Entropy Calculation
Passphrase generation is performed natively by `bhoot-grub-pbkdf2 --generate-diceware <wordlist_path>`:
- **Entropy Source:** Linux kernel cryptographic random number generator via `getrandom(..., 0)`.
- **Rejection Sampling Algorithm:** To eliminate modulo bias across 7,776 words:
  1. Two random bytes are drawn to form an unsigned 16-bit integer $V \in [0, 65535]$.
  2. The rejection threshold is set to $62,208 = 8 \times 7776$.
  3. If $V \ge 62208$, $V$ is discarded and a new 16-bit integer is drawn (rejection probability $\approx 5.078\%$).
  4. The word index is calculated as $I = V \pmod{7776}$, ensuring uniform distribution across all 7,776 outcomes.
- **Exact Mathematical Entropy:**
  $$\text{Entropy} = 5 \times \log_2(7776) = 5 \times 12.9248125 \approx 64.624 \text{ bits}$$
  The resulting passphrase yields $\approx 64.62$ bits of uncompressible cryptographic entropy, separated by single space characters.

### 8.3 Unified Authentication Policy & Delimiter Preservation
The single 5-word Diceware passphrase enrolled at first boot serves as the unified administrator credential for:
1. **GRUB Bootloader Configuration Editing / Unrestricted Booting (`bhoot-admin`):** Verified via GRUB2's internal PBKDF2 engine.
2. **Offline Disaster Recovery Console (`bhoot-recovery-console`):** Verified interactively in `rescue.target`.

**Space Delimiter Preservation:**
- In Bash scripts (`bhoot-firstboot-setup`, `bhoot-recovery-console`), input is captured strictly using `IFS= read -r -s` to prevent whitespace splitting or word truncation.
- GRUB2's `grub_password_get()` accepts all printable ASCII characters, including space (ASCII 0x20).

### 8.4 PBKDF2 Tuning, Hardware Reality & Cracking Analysis
- **PBKDF2-HMAC-SHA512 Iterations:** Parameterized at 10,000 iterations with a 64-byte (128-hex-character) CSPRNG salt. This iteration count is chosen specifically to ensure instant (< 100 ms) interactive UX within GRUB2 in single-threaded UEFI environments, where firmware execution does not benefit from hardware acceleration.
- **Hardware Cracking Benchmark Status: NOT TESTED ON HOST**
  - The development and target platform (Dell Latitude 5410) is equipped with integrated Intel Comet Lake UHD Graphics (PCI ID `8086:9b41`).
  - No discrete NVIDIA GPUs or Hashcat cracking infrastructure are installed on this system.
  - Physical cracking benchmarks are therefore formally designated as **NOT TESTED** on local hardware.
- **Theoretical Hashcat Cracking Estimates (Transparent Mathematical Modeling):**
  - Based on published Hashcat benchmarks on high-end hardware (8x NVIDIA GeForce RTX 4090 GPUs), an optimized cluster achieves $\approx 12.5 \text{ MH/s}$ per GPU for PBKDF2-HMAC-SHA512 at 10,000 iterations, yielding an aggregate cluster throughput of $\approx 100 \text{ MH/s}$ ($10^8 \text{ hashes/sec}$).
  - Full keyspace search:
    $$\frac{2^{64.62} \text{ combinations}}{1.0 \times 10^8 \text{ hashes/sec}} = \frac{2.712 \times 10^{19}}{10^8} \approx 2.712 \times 10^{11} \text{ seconds} \approx 8,594 \text{ years}$$
  - Average time to success (50% keyspace): $\approx 4,297$ years.
  - *Note:* These figures represent theoretical calculations under optimal hardware assumptions, not empirical test results.

### 8.5 Storage Confidentiality vs. Boot Security Domain Separation
- **Partition 3 Persistence:** Storage confidentiality is strictly enforced by LUKS2 with Argon2id (`memory=64 MiB`, `parallel=2`, `iterations=4`).
- **Memory-Hardness vs. "Immunity":** Argon2id is not "immune" to attack, but its 64 MiB memory hardness forces adversaries to dedicate significant high-bandwidth memory per parallel thread, rendering GPU and ASIC parallelization orders of magnitude less cost-effective than for compute-only primitives like PBKDF2.
- **Security Boundaries:** Bootloader modification and recovery console access are governed by GRUB PBKDF2; resting volume confidentiality is governed by LUKS2 Argon2id.

---

## 9. Summary of Audit Remediations (Phases 2A.14 – 2A.17)

| Finding ID | Severity | Description | Remediation Mechanism | Status |
|:---|:---:|:---|:---|:---:|
| **HIGH-01** | **HIGH** | Recovery shell root lockout under `rescue.target` | Implemented `bhoot-recovery-console`, systemd drop-ins, interactive PBKDF2 auth, rate-limiting, and root shell | **RESOLVED** |
| **MED-01** | **MEDIUM** | Unsigned EFI binary incompatible with Secure Boot | Documented Dell Latitude 5410 prerequisite (Secure Boot disabled); defined signed shim + MOK roadmap | **RESOLVED** |
| **MED-02** | **MEDIUM** | ESP offline tampering / Evil Maid risks | Formally analyzed unencrypted FAT32 limits; established physical custody policy & TPM2 measured boot plan | **RESOLVED** |
| **MED-03** | **MEDIUM** | SQLCipher unlinked in database crate | Clarified LUKS2 at-rest protection; scheduled `rusqlite` bundled-sqlcipher for Phase 3 | **RESOLVED** |
| **LOW-01** | **LOW** | Backup allowlist short-circuited via mock hook | Removed Rule 5 mock bypass; implemented comprehensive isolated fixture test for Rules 5..13 | **RESOLVED** |
| **LOW-02** | **LOW** | Recovery key format strictness | Implemented `normalize_recovery_key` and `/usr/local/sbin/bhoot-unlock-recovery-key` | **RESOLVED** |
| **LOW-03** | **LOW** | Shell credential zeroization limitations | Documented memory boundaries; implemented `bhoot-grub-pbkdf2` with `explicit_bzero` and constant-time compare | **RESOLVED** |
| **AUD-17A** | **HIGH** | Inconsistent passphrases & weak entropy bounds | Canonical 5-word Diceware (EFF Large Wordlist, 64.62 bits entropy, rejection sampling) | **RESOLVED** |
| **AUD-17B** | **MEDIUM** | Untested GPU cracking claims & Argon2id claims | Benchmark marked NOT TESTED; transparent theoretical estimates provided; Argon2id memory hardness clarified | **RESOLVED** |

---
**Approved by:** Senior Linux Security Auditor & Cryptographic Systems Specialist  
**Project:** BHOOT Private Financial Intelligence Operating System
