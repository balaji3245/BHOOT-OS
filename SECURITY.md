# Security Policy & Cryptographic Architecture — PROJECT BHOOT

## 1. Threat Model & Security Posture

Project BHOOT is engineered to withstand adversaries with:
- **Physical possession** of the storage media (e.g., lost or seized USB drive).
- **Offline computational capabilities** (GPU clusters attempting dictionary and brute-force attacks on encrypted databases).
- **Tampering motives** (unauthorized post-hoc alterations of historical transaction logs or proof documents).

### Protected Assets
1. **Master Authentication Password & Encryption Secrets.**
2. **Double-Entry Ledger Records:** Account balances, transaction notes, merchant names, counterparty details.
3. **Sensitive Financial Documents:** Bank statements, broker contract notes, tax filings, UPI payment receipts.
4. **Historical Audit Trail:** Complete chronological sequence of accounting events.

---

## 2. Cryptographic Specifications

### 2.1 Password-Based Key Derivation (Argon2id)
- **Standard:** RFC 9106 (Argon2id profile).
- **Parameters:**
  - Memory cost: $128\text{ MiB}$ ($131,072\text{ KiB}$)
  - Time iterations: $4$ passes
  - Parallelism: $4$ lanes / threads
- **Salt:** 32 bytes derived from system CSPRNG (`OsRng`).
- **Rationale:** Optimized to balance high defense against ASIC/GPU cracking while completing within $\approx 150-300\text{ ms}$ on target multi-core x86_64 CPUs.

### 2.2 Hierarchical Key Separation (HKDF-SHA256)
- **Standard:** RFC 5869.
- **Root Secret:** 256-bit Master Key derived via Argon2id.
- **Derived Operational Subkeys:**
  - $K_{\text{db}}$: SQLCipher SQLite database key (`info: "bhoot-kdf-sqlcipher-v1"`).
  - $K_{\text{vault}}$: AES-256-GCM Document Vault key (`info: "bhoot-kdf-vault-v1"`).
  - $K_{\text{audit}}$: Tamper-evident audit chain MAC key (`info: "bhoot-kdf-audit-v1"`).
- **Guarantee:** Compromise of an individual document or database page key does not cross-contaminate other functional subsystems.

### 2.3 Authenticated Document Encryption (AES-256-GCM)
- **Standard:** NIST SP 800-38D.
- **Key Length:** 256 bits.
- **Nonce / IV:** 96 bits generated fresh per operation using CSPRNG.
- **Invariant:** **Nonces are NEVER reused under the same key.** A fresh 96-bit nonce is drawn for every document envelope.
- **Authentication Tag:** 128 bits appended to the ciphertext payload.

### 2.4 Document Integrity & Pre-Encryption Hashing
- Every document is hashed using **BLAKE3** and **SHA-256** prior to envelope encryption.
- Upon decryption, the plaintext is immediately re-hashed and compared in constant time against the expected digest. If the hash fails or the GCM authentication tag is invalid, decryption aborts immediately with zero plaintext leaked.

---

## 3. Memory Hygiene & Zeroization

- Sensitive key material, intermediate derivation buffers, and plaintext passwords are held in memory wrappers implementing `ZeroizeOnDrop` (`zeroize` and `secrecy` crates).
- Upon drop or scope exit, memory buffers are explicitly overwritten with zeroes.

### Known Memory Limitations (Host Environment)
- **Swap Space Risk:** On operating systems where swap is unencrypted, RAM pages could theoretically be paged to disk if the OS comes under high memory pressure. In BHOOT's target deployment (Phase 2), swap is either disabled or hosted inside a LUKS2 encrypted swap partition.
- **Kernel Core Dumps:** Linux core dumps must be disabled via `PR_SET_DUMPABLE = 0` / `RLIMIT_CORE = 0` to prevent key extraction in the event of an abnormal crash.
- **Webview Rendering Engine Memory:** Tauri UI rendering runs inside a WebKit process. The frontend is architected to keep only active view models in memory and flush sensitive transaction strings upon automatic locking.

---

## 4. Ledger Immutability & Anti-Tampering

- Transactions in BHOOT are strictly append-only.
- The `bhoot-audit` crate maintains a cryptographic hash chain:
  $$\text{Hash}_n = \text{SHA256}\left(\text{Hash}_{n-1} \,\|\, \text{Timestamp} \,\|\, \text{EventPayload}\right)$$
- If an attacker tampers with an existing record in SQLite, the cryptographic hash verification of the subsequent blocks fails immediately, raising an audit breach alarm.

---

## 5. Security Vulnerability Reporting

Any security vulnerability, cryptographic flaw, or accounting invariant breach should be treated with critical priority. All findings must be documented with a formal reproducible proof-of-concept test vector before remediation.

