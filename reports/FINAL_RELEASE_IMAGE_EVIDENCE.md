# BHOOT — Final Release Image Technical Evidence Dossier

**Execution Date:** 2026-11-07  
**Auditor:** Principal Linux Security Engineer & Systems Verification Specialist  
**Target Repository:** `/home/bhoot/Desktop/BHOOT`  
**Git Baseline Commit:** `224bf301ffb99af90e7fa97cf6687bfce478a4c8`  
**Git Working Tree Status:** Clean  

---

## 1. Git Baseline & Artifact Timestamps

### Command
```bash
git status
git log -n 2 --stat
ls -la --time-style=full-iso target/release/bhoot-desktop os/build_real/
```

### Output
```text
On branch remediation/full-product-integration
nothing to commit, working tree clean

commit 224bf301ffb99af90e7fa97cf6687bfce478a4c8 (HEAD -> remediation/full-product-integration)
Author: Bhoot Architecture Team <dev@bhoot.internal>
Date:   Sat Nov 7 14:35:48 2026 +0530

    fix(security): resolve final critical blockers, session auth, pin rate-limiting, and kiosk runtime

 apps/desktop/src/api/client.ts                   |  12 +-
 crates/bhoot-core/src/report.rs                  |  19 +
 crates/bhoot-db/src/database.rs                  |   9 +-
 crates/bhoot-db/src/lib.rs                       |   1 +
 crates/bhoot-desktop/src/main.rs                 |  32 +-
 crates/bhoot-desktop/src/server.rs               | 219 ++++++++--
 reports/FINAL_CRITICAL_REMEDIATION_EVIDENCE.md   | 465 +++++++++++++++++++++
 reports/FINAL_CRITICAL_REMEDIATION_REPORT.md     | 219 ++++++++++
 reports/FINAL_RELEASE_ARTIFACT_MANIFEST.md       | 121 ++++++
 reports/INDEPENDENT_POST_REMEDIATION_AUDIT.md    | 225 +++++++++++
 reports/INDEPENDENT_POST_REMEDIATION_EVIDENCE.md | 493 +++++++++++++++++++++++
 scratch/test_adversarial_server_suite.py         | 179 ++++++++
 12 files changed, 1959 insertions(+), 35 deletions(-)

-rwxrwxr-x 2 bhoot bhoot 4119704 2026-11-07 12:59:34.771687797 +0530 target/release/bhoot-desktop

os/build_real/:
-rw-rw-r-- 1 bhoot bhoot     7090176 2026-11-06 23:58:00.564596612 +0530 BOOTX64.EFI
-rw-rw-r-- 1 bhoot bhoot 62075699200 2026-11-07 14:25:12.151860422 +0530 bhoot_disk_assembled.img
-rw-rw-r-- 1 bhoot bhoot  1073741824 2026-11-06 23:58:02.203610880 +0530 bhoot_esp.img
-r-------- 1 bhoot bhoot    16777216 2026-11-06 17:14:40.010337777 +0530 bhoot_luks_header.backup
-rw-rw-r-- 1 bhoot bhoot        1524 2026-11-06 17:14:42.659359008 +0530 bhoot_luks_header_dump.txt
-rw-rw-r-- 1 bhoot bhoot 51539607552 2026-11-06 17:14:41.934353197 +0530 bhoot_luks_prod_48g.img
-rw-rw-r-- 1 bhoot bhoot  1073741824 2026-11-06 17:14:38.472325450 +0530 bhoot_luks_test_64m.img
-rw-rw-r-- 1 bhoot bhoot    33554432 2026-11-07 14:24:31.467531214 +0530 bhoot_recovery.img
-rw-rw-r-- 1 bhoot bhoot  4294967296 2026-11-07 14:24:20.199529571 +0530 bhoot_system_ext4.img
-rw-r--r-- 1 bhoot bhoot   305590272 2026-11-07 14:24:10.655528180 +0530 filesystem.squashfs
-rw-rw-r-- 1 bhoot bhoot        4635 2026-11-07 14:25:34.137919177 +0530 manifest.json
```

---

## 2. Complete Disk Image Integrity & Cryptographic Checksums

### Commands
```bash
# 1. Full-file SHA-256 of assembled disk image
sha256sum os/build_real/bhoot_disk_assembled.img

# 2. SHA-256 of component images & binaries
sha256sum target/release/bhoot-desktop \
          os/build_real/rootfs/usr/local/bin/bhoot-desktop \
          os/build_real/filesystem.squashfs \
          os/build_real/bhoot_esp.img \
          os/build_real/bhoot_system_ext4.img

# 3. Extract and hash binary directly from squashfs
unsquashfs -f -d /tmp/verify_squash_bin os/build_real/filesystem.squashfs usr/local/bin/bhoot-desktop
sha256sum /tmp/verify_squash_bin/usr/local/bin/bhoot-desktop
```

### Empirical Results
```text
996f70a0edd469ac1a9808b19f1571a97c833377c218986abb5a26eb2044a29e  os/build_real/bhoot_disk_assembled.img
074093103148974241866f19452029a0d784b6c39adb3b3d030079a000a1fc8c  target/release/bhoot-desktop
074093103148974241866f19452029a0d784b6c39adb3b3d030079a000a1fc8c  os/build_real/rootfs/usr/local/bin/bhoot-desktop
074093103148974241866f19452029a0d784b6c39adb3b3d030079a000a1fc8c  /tmp/verify_squash_bin/usr/local/bin/bhoot-desktop
d78c479e337d45da19e470af4f3c3a4ad44331d12c3b3251590d26bb62edb0a0  os/build_real/filesystem.squashfs
aa7955c073de04cb4094fa3e89ea2e240e56bbd9a70d0dcd825c344edaee0d6e  os/build_real/bhoot_esp.img
c4728bb4253357ec4140f218afb980e8344db613bd7a59e0f6d1cd7cdd646f92  os/build_real/bhoot_system_ext4.img
```

### Partition Geometry Verification
```text
$ sfdisk -d os/build_real/bhoot_disk_assembled.img
label: gpt
label-id: B8007001-0000-4000-8000-000000000001
device: os/build_real/bhoot_disk_assembled.img
unit: sectors
first-lba: 2048
last-lba: 121241566
sector-size: 512

os/build_real/bhoot_disk_assembled.img1 : start=        2048, size=     2097152, type=C12A7328-F81F-11D2-BA4B-00A0C93EC93B, uuid=B8007001-0001-4000-8000-000000000001, name="BHOOT_EFI"
os/build_real/bhoot_disk_assembled.img2 : start=     2099200, size=     8388608, type=0FC63DAF-8483-4772-8E79-3D69D8477DE4, uuid=B8007001-0002-4000-8000-000000000002, name="BHOOT_SYSTEM"
os/build_real/bhoot_disk_assembled.img3 : start=    10487808, size=   100663296, type=0FC63DAF-8483-4772-8E79-3D69D8477DE4, uuid=B8007001-0003-4000-8000-000000000003, name="BHOOT_DATA"
os/build_real/bhoot_disk_assembled.img4 : start=   111151104, size=     8388608, type=0FC63DAF-8483-4772-8E79-3D69D8477DE4, uuid=B8007001-0004-4000-8000-000000000004, name="BHOOT_RECOVERY"
```

### Partition Slice Bit-for-Bit Identity
```python
# Hash slice of Partition 1 (sectors 2048 to 2099199)
ESP Slice SHA-256:        aa7955c073de04cb4094fa3e89ea2e240e56bbd9a70d0dcd825c344edaee0d6e
Standalone bhoot_esp.img: aa7955c073de04cb4094fa3e89ea2e240e56bbd9a70d0dcd825c344edaee0d6e
Match: 100% IDENTICAL

# Hash slice of Partition 2 (sectors 2099200 to 10487807)
System Slice SHA-256:             c4728bb4253357ec4140f218afb980e8344db613bd7a59e0f6d1cd7cdd646f92
Standalone bhoot_system_ext4.img: c4728bb4253357ec4140f218afb980e8344db613bd7a59e0f6d1cd7cdd646f92
Match: 100% IDENTICAL

# Dump live/filesystem.squashfs from Partition 2 ext4 image
debugfs -R "dump live/filesystem.squashfs /tmp/extracted.squashfs" os/build_real/bhoot_system_ext4.img
SHA-256: d78c479e337d45da19e470af4f3c3a4ad44331d12c3b3251590d26bb62edb0a0
Match: 100% IDENTICAL
```

---

## 3. Graphical Boot Execution & Root-Cause Failure Analysis

### QEMU Execution Command
```bash
qemu-system-x86_64 \
    -name BHOOT-Graphical-Verification \
    -machine q35,accel=kvm \
    -cpu host \
    -m 3072 \
    -smp 2 \
    -drive if=pflash,format=raw,readonly=on,file=/usr/share/OVMF/OVMF_CODE_4M.fd \
    -drive file=bhoot_exec.img,format=raw,if=virtio \
    -vga virtio \
    -display vnc=127.0.0.1:42 \
    -qmp unix:/tmp/qmp_graphical.sock,server,nowait \
    -net none \
    -serial stdio
```

### In-Guest Execution Trace of `bhoot-desktop`
```text
bhoot@bhoot-secure:~$ /usr/local/bin/bhoot-desktop --port 1420 --db /tmp/test.db
/usr/local/bin/bhoot-desktop: /lib/x86_64-linux-gnu/libc.so.6: version `GLIBC_2.38' not found (required by /usr/local/bin/bhoot-desktop)
```

### Forensic Symbol Verification
```bash
$ objdump -p target/release/bhoot-desktop | grep "GLIBC_2\." | sort -V | tail -n 5
    0x06969188 0x00 14 GLIBC_2.28
    0x06969189 0x00 22 GLIBC_2.29
    0x069691b8 0x00 23 GLIBC_2.38

$ nm -D target/release/bhoot-desktop | grep "GLIBC_2.38"
                 U __isoc23_strtol@GLIBC_2.38

$ strings os/build_real/rootfs/lib/x86_64-linux-gnu/libc.so.6 | grep "^GLIBC_2\." | sort -V | tail -n 5
GLIBC_2.32
GLIBC_2.33
GLIBC_2.34
GLIBC_2.35
GLIBC_2.36
```

### Process Lifecycle Trace inside Live Guest (`ps aux`)
```text
USER         PID %CPU %MEM    VSZ   RSS TTY      STAT START   TIME COMMAND
bhoot        821  0.1  0.1   4616  3520 ttyS0    S    09:31   0:00 -bash
bhoot        867 22.8  0.1  18088  5408 tty1     Ss+  09:31   0:00 /usr/bin/cage -s -- /usr/bin/cog -P wl -b #000000 --doc-viewer http://127.0.0.1:1420
```

### Graphical Rendering Findings
1. **Compositor Startup:** `cage` compositor successfully initialized DRM/KMS on `/dev/dri/card0` and Wayland socket on `tty1`.
2. **Browser Startup:** `cog` launched in kiosk mode pointing to `http://127.0.0.1:1420`.
3. **Connection Failure:** Because `bhoot-desktop` exited immediately with dynamic linker failure `GLIBC_2.38 not found`, `http://127.0.0.1:1420` connection was refused.
4. **Visual Observation:** The frontend UI could not render. The display output remained black (`#000000` background).
5. **Verdict on Graphical Application Rendering:** **`NOT VERIFIED`** / **`FAIL`**.

---

## 4. Financial Application & PIN Lockout Test Trace

Execution of `scratch/verify_financial_app_and_backup.py` against compiled binary on host:

```text
[15:10:02] Starting test server on port 16420...
[15:10:03]   [PASS] 1. API Health Check OK (HTTP 200).
[15:10:03]   [PASS] 2. System initial state is locked (Zero-Trust).
[15:10:03]   [PASS] 3. Unauthenticated mutation rejected with HTTP 401.
[15:10:03]   [TESTING SECTION 5: PIN RATE LIMITING & LOCKOUT]
[15:10:03]   [PASS] 4. Five consecutive incorrect PIN attempts triggered mandatory lockout (HTTP 429).
[15:10:03]   [PASS] 5. Sixth attempt during cooldown rejected with HTTP 429.
[15:10:03]   [PASS] 6. Correct PIN during cooldown rejected with HTTP 429.
[15:10:03]   Restarting server to test lockout persistence in SQLCipher...
[15:10:05]   [PASS] 7. Lockout persists across server restart (HTTP 429).
[15:10:05]   Waiting for 60-second cooldown period to expire...
[15:11:06]   [PASS] 8. Successfully authenticated after cooldown with session token: 8d7dad9c...
[15:11:06]   [TESTING SECTION 4: FINANCIAL APPLICATION OPERATIONS]
[15:11:06]   [PASS] 9. Created test accounts (Bank: e39c8a21-5dee-4e23-b90d-a4ce22e79edb, Income: b2506035-318f-4a7d-821e-7b72066a3362).
[15:11:06]   [PASS] 10. Created balanced double-entry transaction (25000000 paisa, ID: 62774d46-ed8f-497e-83ca-21ac5452b61a).
[15:11:06]   [PASS] 11. Verified account balance strictly matches 25000000 paisa.
[15:11:06]   [PASS] 12. Verified dashboard net worth (44050000 paisa). Balance sheet balanced: True.
[15:11:06]   Session locked via POST /api/auth/lock.
[15:11:06]   [PASS] 13. Previous session token strictly rejected with HTTP 401 after lock.
[15:11:06]   [PASS] 14. Unauthorized mutation after lock strictly rejected with HTTP 401.
[15:11:06]   [PASS] 15. Unlocked with rotated session token.
[15:11:06]   Restarting server to test data durability...
[15:11:07]   [PASS] 16. Financial data strictly persisted across restart (25000000 paisa).
[15:11:07]   [TESTING SECTION 7: BACKUP AND RESTORE]
[15:11:08]   [PASS] 17. Encrypted backup created successfully (468445 bytes).
[15:11:08]   [PASS] 18. Backup restored and SQLCipher integrity verified (131072 bytes).

=== ALL FINANCIAL APPLICATION, RATE LIMITING & BACKUP TESTS COMPLETED ===
  - health: PASS
  - initial_locked: PASS
  - locked_mutation_401: PASS
  - pin_lockout_429: PASS
  - cooldown_6th_429: PASS
  - cooldown_correct_pin_429: PASS
  - cooldown_persists_restart_429: PASS
  - auth_after_cooldown_200: PASS
  - create_accounts: PASS
  - create_balanced_tx: PASS
  - verify_account_balance: PASS
  - verify_net_worth: PASS
  - token_revocation_401: PASS
  - locked_mutation_rejected: PASS
  - token_rotation: PASS
  - data_persistence_after_restart: PASS
  - backup_create: PASS
  - backup_restore_verified: PASS
```

---

## 5. Physical Device Safety Assertion

Throughout all verification activities:
- `/dev/sda` and `/dev/sda1` were NEVER accessed, read, mounted, or partitioned.
- `/dev/nvme0n1` and host filesystems were untouched.
- `deploy_physical_usb.sh` was NOT executed.
- All testing occurred strictly on disposable sparse images and isolated QEMU KVM virtual machines.
