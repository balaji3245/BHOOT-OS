# PROJECT BHOOT — Private Financial Intelligence System

> **Classification:** CONFIDENTIAL / OFFLINE RECORDKEEPING  
> **Deployment Target:** Dedicated Bootable Linux USB (64 GB)  
> **Architecture:** Pure Rust Core Domain · SQLCipher Storage · AES-256-GCM Vault · Tauri Desktop UI  

---

## 1. Executive Overview

**BHOOT** is an offline-first, private personal financial management operating system engineered for complete financial sovereignty, accounting integrity, and cryptographic privacy.

BHOOT is **not** a simple budget app or expense tracker. It is a full double-entry financial recordkeeping system maintaining an immutable, tamper-evident audit trail of every rupee entering, moving across, or exiting personal, trading, banking, UPI, and business accounts.

### Absolute Privacy Guarantees
1. **Zero External Communication:** No cloud database, no external telemetry, no remote analytics, no advertising, and no public domain.
2. **Local Cryptographic Storage:** All transactional ledgers and financial documents are encrypted at rest with hardware-accelerated AES-256 and Argon2id.
3. **Double-Entry Ledger Integrity:** Every transaction maintains exact debit-credit conservation ($\sum \text{Debits} == \sum \text{Credits}$).
4. **Offline Operational Independence:** Operates completely air-gapped without internet access.

---

## 2. Workspace Crate Architecture

The system is organized into a modular, strictly decoupled Rust workspace:

| Crate | Purpose | Key Responsibilities |
| :--- | :--- | :--- |
| **`bhoot-core`** | Financial Domain & Accounting Engine | Integer Paisa arithmetic, double-entry ledger, transaction invariants, broker payouts, trading P&L. Zero I/O, pure domain logic. |
| **`bhoot-crypto`** | Cryptographic Subsystem | Argon2id KDF, AES-256-GCM authenticated encryption, HKDF key separation, memory zeroization, BLAKE3 file hashing. |
| **`bhoot-db`** | Encrypted Storage Engine | SQLCipher connection lifecycle, PRAGMA key binding, transactional migrations, zero-leak backups. |
| **`bhoot-audit`** | Tamper-Evident Ledger Log | Cryptographically chained audit event log (SHA-256 hash chains), immutable append-only verification. |
| **`bhoot-vault`** | Encrypted Proof Document Store | Content-addressable encrypted storage for bank statements, contract notes, and transaction receipts. |

---

## 3. Financial Invariants

The core accounting engine enforces the following non-negotiable mathematical invariants:

- **Exact Integer Paisa Arithmetic:** All monetary amounts are represented as 64-bit signed integers in Paisa (₹0.01). Floating-point arithmetic is forbidden.
- **Double-Entry Equilibrium:** A transaction cannot be posted unless total debits equal total credits.
- **Transfer Neutrality:** Internal transfers between owned accounts have exactly zero net effect on income or expenses.
- **Historical Immutability:** Historical records cannot be modified or deleted. Adjustments and corrections require explicit, linked reversal transactions with cryptographic audit events.

---

## 4. Building and Verification

Prerequisites:
- Rust `stable` (1.99+)
- C Compiler (`gcc` or `clang`)
- `pkg-config`, `libsqlcipher-dev`, `libwebkit2gtk-4.1-dev`, `libssl-dev`

```bash
# Sourcing Rust toolchain
source "$HOME/.cargo/env"

# Formatting and linter checks
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings

# Comprehensive test suite (unit + property-based + crypto vectors)
cargo test --workspace

# Security vulnerability audit
cargo audit
```

---

## 5. Security Policy

Refer to [`SECURITY.md`](SECURITY.md) for detailed cryptographic specifications, threat models, and memory protection limitations.

