# BHOOT — Independent Post-Remediation Verification Security Audit

**Assessment Target:** BHOOT Personal Financial Operating System  
**Audit Baseline Commit:** `32d70183f3ef2c2600ac5498bb9d63f686b7f687` (`32d7018`)  
**Audit Baseline Branch:** `remediation/full-product-integration`  
**Previous Audit Reference:** `reports/FINAL_PRODUCTION_ACCEPTANCE_SECURITY_AUDIT.md` (Verdict: FAIL)  
**Remediation Reports Under Review:** `reports/CRITICAL_REMEDIATION_REPORT.md`, `reports/FINAL_REMEDIATION_ACCEPTANCE_REPORT.md`  
**Audit Date:** 2026-11-07  
**Auditor:** Independent Senior Linux Security Auditor, Cryptographic Systems Engineer, and Financial Systems QA Specialist  

---

## EXECUTIVE SUMMARY & AUDIT VERDICT

### FINAL VERDICT: **FAIL**

Following the critical remediation executed in commit `32d7018`, an independent, adversarial, evidence-based verification audit was conducted against the repository, binary artifacts, rootfs tree, and live system images.

While significant engineering progress was made—specifically the implementation of SQLCipher database persistence, double-entry financial ledger operations, cryptographic document vault storage, and encrypted archive backup/restore routines—**the release cannot be certified for production deployment.** 

The audit verdict is **FAIL** due to two absolute mandatory acceptance gate violations and two severe API security vulnerabilities:

1. **Mandatory Acceptance Gate 1 Violation (Graphical Boot):** Neither `cage` (Wayland kiosk compositor) nor any graphical web browser (`chromium`, `firefox`, or `webkit`) is installed in the system rootfs or live SquashFS. The graphical systemd service (`bhoot-gui.service`) explicitly tests `if command -v cage ... else echo "BHOOT Graphical Kiosk (Cage) runtime not present" && sleep infinity`. In real boot, the system renders no graphical interface whatsoever.
2. **Mandatory Acceptance Gate 2 Violation (Live Rootfs/SquashFS Out of Sync):** While the filesystem rootfs directory was updated with the new binary, `os/build_real/filesystem.squashfs` was last generated on **Nov 6 23:57:36** and contains the **obsolete pre-remediation binary** (`9fd8c344...`, 3.53 MB), which lacks all mutation POST endpoints. Any user booting the system from USB or QEMU runs the broken pre-remediation version.
3. **Severe Financial UI Defect (Schema Desynchronization):** A systemic naming mismatch exists between Rust `bhoot-core` JSON serialization (e.g., `total_assets`, `total_liabilities`, `net_worth`, `balance`) and React TypeScript types in `apps/desktop/src/types.ts` (e.g., `total_assets_paisa`, `total_liabilities_paisa`, `net_worth_paisa`, `balance_paisa`). In the frontend UI, all balance sheet totals and account balances evaluate to `undefined` and default to **₹0.00**.
4. **Critical Security Vulnerability (Session Lock Mutation Bypass):** When a user explicitly locks the session (`POST /api/auth/lock`), the backend sets `session_locked: true`, but **never verifies this state or requires authentication tokens** on financial mutation endpoints (`/api/accounts`, `/api/transactions`, `/api/vault/upload`, `/api/backup/create`). Any local process can alter financial data while the application is locked.
5. **High Security Vulnerability (Lack of PIN Brute-Force Rate Limiting):** The backend tracks `failed_pin_attempts` via an atomic counter, but never checks or enforces a lockout threshold. An attacker can submit unthrottled PIN brute-force attempts indefinitely.

Per the explicit audit governance rule: *"Do not declare PASS if graphical boot or actual financial database persistence is not verified,"* the verdict must remain **FAIL** until these blockers are remediated.

---

## 1. Audit Scope & Verification Baseline

| Property | Target Specification | Verified Reality | Status |
|---|---|---|---|
| Repository Working Tree | Clean, no uncommitted changes | Clean (`git status` exit code 0) | PASS |
| Implementation Commit | `32d7018` | `32d70183f3ef2c2600ac5498bb9d63f686b7f687` | PASS |
| Host Hardware Isolation | No writes to `/dev/sda` or `/dev/nvme0n1` | Verified zero writes to host storage | PASS |
| Rust Workspace Tests | All crates pass unit & proptests | 41/41 passing across 6 crates | PASS |
| Binary Self-Test Harness | `./target/release/bhoot-desktop --test-run` | 15/15 integration tests passing | PASS |
| Live Rootfs Binary Match | Binary in rootfs matches release target | `acafeb70...` (4,085,104 bytes) matches | PASS |
| Live SquashFS Binary Match | Binary in SquashFS matches rootfs | **Mismatch:** `9fd8c344...` (3,530,776 bytes) | **FAIL** |
| Wayland Kiosk Environment | `cage` and `chromium` present in rootfs | Neither package installed | **FAIL** |
| Frontend Schema Parity | TypeScript types match Serde keys | Mismatches across all balance/total keys | **FAIL** |
| API Session Lock Enforce | Locked session rejects mutations | Accepted mutations while locked (Status 201) | **FAIL** |
| PIN Lockout Enforcement | Lockout after threshold failed attempts | Infinite failed attempts permitted | **FAIL** |
| SQLCipher Disk Encryption | Raw bytes encrypted, zero plaintext | Verified: 0 plaintext leaks, non-standard header | PASS |
| Audit Trail Hash Linking | Cryptographically linked SHA-256 chain | Verified: Strict sequential hash chain | PASS |
| Backup Archive Security | AEAD authentication, prohibited paths | Verified: Tampered archives & bad paths blocked | PASS |

---

## 2. In-Depth Technical Assessment of Blockers

### Blocker 1 — Missing Graphical Kiosk Runtime (Gate Violation)
- **Component:** `os/build_real/rootfs`, `os/scripts_v2/02_configure_rootfs.sh`, `bhoot-gui.service`
- **Severity:** CRITICAL (Mandatory Acceptance Gate Blocker)
- **Analysis:**
  The specification requires BHOOT OS to boot directly into an air-gapped, unattended graphical user interface presenting the PIN unlock screen.
  Inspection of `os/build_real/rootfs` reveals that neither `cage`, `weston`, `chromium`, `firefox`, nor any WebKit runtime is installed.
  The service file `os/build_real/rootfs/etc/systemd/system/bhoot-gui.service` contains the following execution line:
  ```ini
  ExecStart=/bin/sh -c 'if command -v cage >/dev/null 2>&1; then exec cage -- /usr/bin/chromium --kiosk http://127.0.0.1:1420; else echo "BHOOT Graphical Kiosk (Cage) runtime not present; backend server operating on http://127.0.0.1:1420" && sleep infinity; fi'
  ```
  Because `cage` is absent, the shell script prints the fallback string and executes `sleep infinity`. No graphical display is ever started. Users booting from USB are met with a black tty console and no indication of how to access the system.

### Blocker 2 — Stale Live SquashFS Image Drift
- **Component:** `os/build_real/filesystem.squashfs`, `os/build_real/bhoot_disk_assembled.img`
- **Severity:** CRITICAL
- **Analysis:**
  During the remediation at commit `32d7018`, the author compiled the new `bhoot-desktop` binary (`4,085,104` bytes, SHA256: `acafeb70...`) and copied it to `os/build_real/rootfs/usr/local/bin/bhoot-desktop`.
  However, the author **failed to rebuild `filesystem.squashfs`**.
  Extracting `usr/local/bin/bhoot-desktop` from `filesystem.squashfs` reveals:
  - File Size: `3,530,776` bytes
  - SHA256: `9fd8c344f1200af93828b5b8c383b73fd553466e681b635430d16ae269ff0ac8`
  - Timestamp: `2026-11-06 23:57:36`
  This is the old, pre-remediation binary that lacks POST mutation endpoints (`/api/accounts`, `/api/transactions`, etc.). Furthermore, `bhoot-gui.service` is missing from `filesystem.squashfs`. Booting the assembled disk image produces a completely non-functional system.

### Blocker 3 — Frontend-Backend Schema Desynchronization
- **Component:** `crates/bhoot-core/src/report.rs`, `apps/desktop/src/types.ts`, `apps/desktop/src/views/`
- **Severity:** HIGH
- **Analysis:**
  In `crates/bhoot-core/src/report.rs`, the accounting structures serialize fields without the `_paisa` suffix:
  ```rust
  pub struct BalanceSheet {
      pub total_assets: Money,       // Serialized as: "total_assets": 20500000
      pub total_liabilities: Money,  // Serialized as: "total_liabilities": 1450000
      pub total_equity: Money,       // Serialized as: "total_equity": 0
      pub retained_earnings: Money,  // Serialized as: "retained_earnings": 1972000
      pub net_worth: Money,          // Serialized as: "net_worth": 19050000
  }
  ```
  However, `apps/desktop/src/types.ts` defines the frontend interface with `_paisa` suffixes:
  ```typescript
  export interface BalanceSheet {
      total_assets_paisa: number;
      total_liabilities_paisa: number;
      total_equity_paisa: number;
      retained_earnings_paisa: number;
      net_worth_paisa: number;
  }
  ```
  In React views (`DashboardView.tsx`, `AssetsLiabilitiesView.tsx`), expressions like `balanceSheet?.net_worth_paisa ?? 0` evaluate to `undefined ?? 0` -> `0`.
  Similarly, `crates/bhoot-desktop/src/server.rs` (`GET /api/accounts`) returns a list of `Account` structs which lack any `balance` field, whereas `types.ts` expects `AccountSummaryItem` with `balance_paisa`.
  **Visual Result:** The UI renders `₹0.00` across all account balances, total assets, total liabilities, and net worth.

### Blocker 4 — Session Lock Mutation Bypass (Vulnerability)
- **Component:** `crates/bhoot-desktop/src/server.rs`
- **Severity:** HIGH (CWE-306: Missing Authentication for Critical Function)
- **Analysis:**
  The server exposes `/api/auth/lock` and `/api/auth/unlock`, and maintains `state.session_locked` (AtomicBool).
  However, in `server.rs`, `handle_request()` executes POST mutations (`/api/accounts`, `/api/transactions`, `/api/trades`, `/api/budgets`, `/api/vault/upload`, `/api/vault/delete`, `/api/backup/create`, `/api/backup/restore`) **without checking `state.session_locked` or validating an `Authorization` or `X-Session-Token` header**.
  Our adversarial probe (`test_adversarial_server_suite.py`, check `AUTH-06`) confirmed that sending a `POST /api/accounts` while `session_locked == true` resulted in **HTTP 201 Created**, successfully inserting the record into the persistent database.

### Blocker 5 — Unthrottled PIN Brute-Force (Vulnerability)
- **Component:** `crates/bhoot-desktop/src/server.rs`
- **Severity:** HIGH (CWE-307: Improper Restriction of Excessive Authentication Attempts)
- **Analysis:**
  In `ServerState::verify_pin()`:
  ```rust
  if matches {
      self.failed_pin_attempts.store(0, Ordering::SeqCst);
      ...
  } else {
      self.failed_pin_attempts.fetch_add(1, Ordering::SeqCst);
      false
  }
  ```
  While `failed_pin_attempts` is incremented on bad PIN attempts, it is **never queried**. There is no lockout condition (e.g. `if self.failed_pin_attempts.load(...) >= 5 { return Err("Locked out"); }`), no exponential backoff, and no rate limiting.

---

## 3. Verification of Remediation Successes

Despite the failing verdict, substantial high-quality systems engineering was verified in the codebase:

### 3.1 SQLCipher On-Disk Cryptographic Confidentiality
- Raw inspection of `bhoot_test.db` (4,096 bytes) and `bhoot_test.db-wal` (502,672 bytes) confirmed zero plaintext leakage.
- Standard SQLite magic header (`SQLite format 3`) was absent (randomized by SQLCipher page encryption).
- Targeted string probes for account names, transaction descriptions, and SQL schemas found 0 matches in raw disk images.
- Full process restart verified complete data persistence across restarts.

### 3.2 Double-Entry Accounting Engine
- Core accounting logic strictly enforces mathematical equilibrium ($\sum \text{Debits} = \sum \text{Credits}$).
- Fractional currency is tracked strictly in 64-bit integer paisa; floating-point math is completely eliminated.
- Reversal transactions are cryptographically created as atomic offset postings.

### 3.3 Document Vault AES-256-GCM Envelope Encryption
- Valid file upload correctly identifies PDF, PNG, and JPEG magic bytes and rejects arbitrary binaries (HTTP 422).
- Envelope encryption uses ChaCha20-Poly1305 / AES-256-GCM with distinct random nonces.
- Download endpoint successfully decrypts and returns identical plaintext bytes.
- Direct disk bit-flip tampering is detected during integrity verification.

### 3.4 Encrypted Disaster Recovery Backup
- Backup archive creation checkpoints the SQLite WAL file before archiving.
- Enforces strict destination path allowlisting, rejecting `/dev/sda`, `/etc/shadow`, `/boot/`, and path-traversal escapes (`../../dev/sda`).
- Passphrase length $\ge 12$ characters is strictly enforced (HTTP 422 on short passphrases).
- Corrupted archives and incorrect passphrases are rejected with HTTP 400.
- Restoration to an isolated temporary sandbox verifies SQLCipher database integrity before committing.

### 3.5 Cryptographic Audit Trail
- Audit events are strictly sequential ($1, 2, \dots, N$).
- Each event cryptographically binds to the previous event digest via length-prefixed domain-separated hashing.
- Chain integrity verification confirms tamper detection.

---

## 4. Test Integrity Forensics (34 vs 40 Claimed Tests)

In the prior audit report (`FINAL_PRODUCTION_ACCEPTANCE_EVIDENCE.md`), the author claimed 40 passed Rust unit tests, listing non-existent test function names:
- `test_account_crud`
- `test_sqlcipher_pragma_generation`
- `test_api_accounts_get`

Forensic analysis of Git history confirms these names were synthesized by the previous author in markdown documentation and never existed in Rust source code.
The actual Rust test suite consists of **41 automated test cases**:
- `bhoot-audit`: 4 unit tests
- `bhoot-backup`: 4 unit tests
- `bhoot-core`: 17 unit tests
- `bhoot-crypto`: 14 tests (7 unit tests + 7 proptests)
- `bhoot-db`: 1 unit test
- `bhoot-vault`: 1 unit test
In addition, `./target/release/bhoot-desktop --test-run` executes **15 standalone integration assertions**.
All 41 crate tests and 15 self-test assertions pass reliably.

---

## 5. Mandatory Remediation Checklist for Production Readiness

To achieve an unconditional **PASS** in the subsequent audit, the following engineering steps must be executed:

1. **Install Graphical Runtime in Rootfs:**
   - Install `cage` (or `weston`) and a lightweight browser runtime (`chromium` or `webkit2gtk`) into `os/build_real/rootfs`.
   - Ensure `bhoot-gui.service` successfully launches the browser in kiosk mode pointed at `http://127.0.0.1:1420`.
2. **Rebuild SquashFS & Assembled Disk Image:**
   - Execute `mksquashfs os/build_real/rootfs os/build_real/filesystem.squashfs -comp xz -noappend`.
   - Re-assemble `os/build_real/bhoot_disk_assembled.img` and update `manifest.json`.
   - Verify matching SHA256 hashes between target binary, rootfs binary, and SquashFS binary.
3. **Synchronize Frontend-Backend Data Schemas:**
   - Update `crates/bhoot-core/src/report.rs` (or `apps/desktop/src/types.ts`) so field names match exactly:
     - Map `total_assets_paisa` $\leftrightarrow$ `total_assets`
     - Map `total_liabilities_paisa` $\leftrightarrow$ `total_liabilities`
     - Map `net_worth_paisa` $\leftrightarrow$ `net_worth`
     - Update `GET /api/accounts` to return calculated balances per account.
4. **Implement Session Mutation Authorization:**
   - In `crates/bhoot-desktop/src/server.rs`, create an authentication guard middleware.
   - For all mutating routes (`POST`, `PUT`, `DELETE`), verify that:
     1. `state.session_locked.load(Ordering::SeqCst) == false`
     2. The `X-Session-Token` header matches `*state.session_token.lock().unwrap()`.
   - Return `HTTP 401 Unauthorized` if locked or if token is invalid/missing.
5. **Implement PIN Lockout Threshold & Rate Limiting:**
   - In `ServerState::verify_pin()`, if `self.failed_pin_attempts.load(...) >= 5`, reject with HTTP 429/401 lockout until a cooldown expires.
6. **Address Starter Data Equilibrium:**
   - In `seed_starter_data()`, ensure initial equity or cash settlement offsets the 19,720 INR net trading profit so that `is_balanced` is `true` upon first boot.

---

## 6. Audit Conclusion

The BHOOT project has established an exceptionally strong cryptographic foundation, featuring pristine SQLCipher integration, robust double-entry mechanics, rigorous path validation, and tamper-evident audit chaining. However, the disconnect between compiled binaries and the live bootable SquashFS, the lack of an installed Wayland kiosk runtime, frontend schema desynchronization, and the absence of session-lock mutation guards prevent this build from satisfying final production acceptance criteria.

**Verdict: FAIL**  
**Remediation Required:** Yes (Phase 2B Integration Polish).

