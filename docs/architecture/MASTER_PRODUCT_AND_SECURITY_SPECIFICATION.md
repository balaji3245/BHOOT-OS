# BHOOT — Master Product & Security Specification

**Version:** 1.0  
**Status:** Architecture baseline — pending implementation and independent security review  
**Project type:** Private, offline-first personal financial operating system  
**Deployment target:** Dedicated 64 GB USB device  
**Primary principle:** Financial data remains under the owner's physical and cryptographic control.

---

## 1. Final Objective

Build BHOOT as a self-contained, bootable financial operating environment that:

- Boots independently from a dedicated USB device on compatible x86-64 UEFI hardware.
- Works without internet, cloud accounts, telemetry, advertisements, or mandatory external services.
- Stores financial records and documents inside an encrypted persistent data volume.
- Provides a secure authentication and recovery process.
- Maintains accurate double-entry accounting and a tamper-evident audit trail.
- Supports financial records, trading activity, payouts, expenses, assets, liabilities, and reports.
- Provides encrypted backup and tested restoration.
- Has a premium, simple, responsive desktop interface.
- Can be maintained and upgraded without silently risking existing financial data.

**Non-goal:** BHOOT is not initially a banking application, broker, payment processor, cloud synchronization service, or automated live-trading platform.

## 2. Non-Negotiable Principles

1. **Offline-first:** Core functionality must work without network access.
2. **Local ownership:** No mandatory cloud dependency or third-party account.
3. **Encryption by default:** Financial records and private documents must never be stored unencrypted on persistent storage.
4. **Accounting correctness:** Financial calculations use integer minor currency units, never floating-point values for ledger balances.
5. **Auditability:** Financial changes must be traceable; historical entries must not be silently overwritten.
6. **Fail closed:** Authentication, integrity, encryption, and recovery failures must not silently downgrade security.
7. **Explicit consent:** Destructive operations require clear owner approval.
8. **Reproducibility:** Builds, images, manifests, and test evidence must be versioned and reproducible.
9. **Independent verification:** Agent-generated reports are evidence to inspect, not independent certification.
10. **No deployment shortcuts:** Passing unit tests does not authorize writing to a physical USB.

## 3. Target System Architecture

### 3.1 Operating system and boot

Target architecture:

- Debian-based minimal Linux environment.
- x86-64 UEFI boot.
- GRUB bootloader.
- Read-only or integrity-verified operating-system image, such as SquashFS.
- Separate persistent encrypted data partition.
- Explicitly documented recovery and upgrade mechanisms.

Secure Boot, signed EFI components, measured boot, and TPM2 integration must be treated as separate security capabilities. They must not be described as implemented until verified on the actual boot chain and supported hardware.

### 3.2 Storage layout

The intended USB layout is:

| Area | Purpose | Security requirement |
|---|---|---|
| EFI system partition | Bootloader and required boot files | Minimize writable attack surface; integrity protection required |
| System partition | BHOOT operating system | Read-only or verified system image |
| Encrypted data partition | Database, documents, settings, and private records | LUKS2 encryption |
| Optional ordinary storage | User-accessible general files | Must not expose BHOOT secrets |

The proposed arrangement in which only approximately 4 GB appears as ordinary storage while the remainder is reserved for BHOOT is a partitioning and usability design—not guaranteed concealment or anti-format protection. Partition inspection and destructive formatting remain possible.

Do not finalize partition sizes until capacity, update requirements, backup requirements, and filesystem overhead are measured.

### 3.3 Encryption architecture

Intended cryptographic separation:

- **Boot authentication:** GRUB-compatible password hash and authenticated recovery access.
- **Persistent storage:** LUKS2 with Argon2id and a strong volume key.
- **Application encryption:** SQLCipher-backed database encryption where appropriate.
- **Document protection:** Encrypted storage for private documents.
- **Key derivation:** HKDF-SHA256 for purpose-separated application subkeys.
- **Authenticated encryption:** AES-256-GCM where application-level encryption is required.
- **Secret handling:** Minimize secret lifetime in memory; use zeroization where supported and document unavoidable limitations.

The GRUB credential and LUKS credential are separate security domains. Bootloader authentication must never be represented as a substitute for volume encryption.

SQLCipher is not considered implemented until the database is actually linked against a supported SQLCipher library and encryption is verified by reopening the database and inspecting its on-disk representation.

### 3.4 Application architecture

Target application stack:

- Frontend: React, TypeScript, and Tauri.
- Backend: Rust.
- Database: SQLite with SQLCipher.
- Core Rust modules: financial ledger, cryptography, database access, audit, and document vault.

Suggested module boundaries:

- `bhoot-core`: domain models and business rules.
- `bhoot-db`: database access, migrations, and transaction boundaries.
- `bhoot-crypto`: key derivation and encryption primitives.
- `bhoot-audit`: append-only, tamper-evident audit records.
- `bhoot-vault`: private document storage and metadata.
- Desktop application: user interface and controlled access to backend capabilities.

The frontend must not independently implement financial invariants or directly manipulate encrypted database files.

## 4. Financial Domain Requirements

### 4.1 Double-entry ledger

- Every posted transaction must balance: total debits equal total credits.
- Store money as integer minor units, such as paisa (`i64`).
- Currency must be explicit for each monetary amount.
- Use database transactions for posting and reversal.
- Prevent invalid, incomplete, or unbalanced journal entries.
- Posted entries must not be silently edited or deleted.
- Corrections must use traceable reversal and replacement entries.

### 4.2 Financial records

The application should support:

- Accounts and account categories.
- Income and expenses.
- Transfers between accounts.
- Assets and liabilities.
- Trading records and realized/unrealized P&L records.
- Payouts, deposits, and withdrawals.
- Attachments and supporting documents.
- Categories, tags, counterparties, and notes.
- Date-range reports and account statements.

Trading records are initially for recordkeeping and analysis. Broker integration and live order execution are out of initial scope.

### 4.3 Audit trail

- Record actor, operation, timestamp, and relevant record identifiers.
- Protect the integrity of audit records using a cryptographic chain or equivalent authenticated construction.
- Define how migrations, reversals, and administrative actions are represented.
- Detect missing, reordered, or modified audit records.
- Never claim that a local hash chain prevents a privileged attacker from rewriting the entire database and chain without an external trust anchor.

## 5. Security Requirements

### 5.1 Authentication and recovery

- Strong initial credential generation.
- Secure credential confirmation and enrollment.
- No secrets in command-line arguments, logs, or persistent temporary files.
- Rate limiting or lockout with documented denial-of-service implications.
- Recovery must require explicit authentication.
- Recovery credentials and recovery instructions must be tested.
- No unauthenticated root shell or passwordless recovery fallback.
- Credential changes must have a documented rollback and recovery procedure.

### 5.2 Boot integrity

- Define the trusted boot chain from firmware to application.
- Verify boot artifacts against a protected trust anchor.
- Document Secure Boot signing and key ownership.
- Address offline modification of the EFI system partition.
- Evaluate TPM2 measured boot where supported.
- Clearly distinguish checksums, signatures, Secure Boot, and measured boot; they provide different protections.

### 5.3 Data protection

- Encrypt the persistent data partition.
- Ensure private documents are encrypted at rest.
- Avoid plaintext exports and temporary copies unless explicitly requested.
- Define screen locking, session termination, and memory cleanup behavior.
- Do not expose secrets through diagnostic reports or crash logs.

### 5.4 Backup and restore

- Support encrypted backups to an owner-selected external destination.
- Validate destination paths and prevent path traversal.
- Verify backup integrity before reporting success.
- Test restoration into a disposable environment.
- Document recovery-key storage and loss scenarios.
- A backup is not considered reliable until restoration has been demonstrated.

### 5.5 Threat model

The security design must explicitly assess:

- Loss or theft of the USB.
- Unauthorized physical access.
- Offline copying and brute-force attacks.
- Malicious modification of boot files.
- Compromised or hostile host hardware.
- Malicious or accidental application changes.
- Power loss during writes.
- Filesystem corruption and USB controller failure.
- Insider misuse and accidental disclosure.
- Loss of credentials or recovery material.

No software-only design can guarantee protection against every compromised firmware, hardware implant, malicious host, or destructive physical attack.

## 6. User Interface Requirements

- Premium, modern, restrained desktop design.
- Clear financial dashboard and account summaries.
- Consistent navigation and predictable workflows.
- Responsive layouts for supported screen sizes.
- Accessible contrast, readable typography, and keyboard navigation.
- Light theme as the initial default; dark theme as a supported option.
- No decorative complexity that obscures financial information.
- Confirmations for irreversible financial and security operations.
- Clear error states that explain what failed without exposing secrets.

## 7. Development Roadmap and Approval Gates

Each phase must have a defined scope, deliverables, test evidence, and independent review. A phase is not accepted merely because an implementation agent reports success.

### Phase A — OS foundation and boot security

Complete the disposable-image boot architecture, authentication, encrypted persistence, recovery, and integrity design.

**Current position:** Phase 2A.17 implementation report received. Independent source-level audit remains outstanding.

**Exit gate:** Boot chain, authentication, recovery, persistence, and integrity claims independently verified in disposable environments.

### Phase B — Physical-device compatibility

Prepare a deployment image and validate it against the actual USB and target hardware.

Requirements:
- Written device identification and capacity verification.
- Verified image and partition layout.
- Boot and recovery tests on supported hardware.
- Power-loss and safe-removal tests.
- Explicit owner approval immediately before any destructive write.

**No physical USB writing is authorized by this specification.**

### Phase C — Database and application security

Implement and verify:
- Actual SQLCipher integration.
- Encrypted database lifecycle.
- Database migrations and recovery.
- Key derivation and secret management.
- Secure document storage.
- Audit-chain verification.

**Exit gate:** Database encryption, key handling, migration safety, and tamper detection independently tested.

### Phase D — Financial engine

Implement:
- Double-entry accounting.
- Account management.
- Income, expense, and transfer workflows.
- Trading and payout recordkeeping.
- Reversals and audit history.
- Reconciliation and financial reports.

**Exit gate:** Accounting invariants, transaction atomicity, concurrency, and edge cases pass automated and independent tests.

### Phase E — Desktop application

Implement the React/TypeScript and Tauri interface, integrated with the Rust backend.

**Exit gate:** All supported workflows operate against the real backend; UI cannot bypass domain or security invariants.

### Phase F — Backup, recovery, and resilience

Implement encrypted backup, restoration, corruption detection, safe upgrades, and power-loss recovery.

**Exit gate:** Restoration from backup is demonstrated, not merely backup creation.

### Phase G — Final security assessment

Conduct a complete audit of:
- Boot chain and firmware assumptions.
- Authentication and recovery.
- Encryption and key management.
- Database and document vault.
- Application permissions.
- Update and backup processes.
- Financial correctness.
- Privacy and information leakage.

**Exit gate:** No unresolved critical or high-severity findings. Remaining accepted risks must be documented and explicitly approved.

### Phase H — Release and maintenance

Produce:
- Versioned release image.
- Artifact checksums and signed release metadata where supported.
- Build and deployment instructions.
- Recovery documentation.
- Known limitations.
- Upgrade and rollback procedures.
- Security incident and vulnerability response process.

## 8. Mandatory Testing Policy

Testing must include:

- Unit tests for domain and cryptographic components.
- Integration tests for database and application boundaries.
- Negative authentication tests.
- Recovery and restore tests.
- Filesystem and image integrity checks.
- Disposable QEMU UEFI lifecycle tests.
- Power-loss and interrupted-operation tests.
- Static analysis and secret scanning.
- Independent source review.
- Hardware compatibility testing when explicitly authorized.

Every test report must identify:
- Exact commit and branch.
- Environment and tool versions.
- Exact commands executed.
- Raw pass/fail output.
- Assertions versus files scanned.
- Skipped tests and limitations.
- Any mocked, simulated, or untested behavior.

Agent-reported results must be labeled as agent-reported until independently reproduced.

## 9. Hardware Safety Rules

These rules are mandatory:

1. `/dev/sda` and `/dev/sda1` are protected host USB devices.
2. `/dev/nvme0n1` is the protected host internal drive.
3. Never format, partition, erase, overwrite, or modify these devices without separate explicit owner authorization.
4. Never modify host UEFI settings without separate explicit owner authorization.
5. Use disposable sparse images and QEMU for development and destructive tests.
6. Clean up loop devices and temporary mounts after tests.
7. Verify target device identity and size immediately before any approved physical write.
8. Stop if device identity is ambiguous.
9. A general project approval is not authorization for a specific destructive operation.

## 10. Release Acceptance Criteria

BHOOT is not production-ready until all of the following are true:

- Boot and recovery behavior is independently verified.
- Boot integrity claims match the actual implementation.
- Persistent data is encrypted and tested.
- SQLCipher is genuinely integrated if claimed.
- Key and credential lifecycle is reviewed.
- Financial ledger invariants are enforced.
- Audit records are tamper-evident within the documented threat model.
- Encrypted backup restoration is proven.
- Power-loss and corruption recovery are tested.
- No critical or high-severity security findings remain unresolved.
- Release artifacts are reproducible and verifiable.
- Owner has reviewed known limitations and explicitly approved deployment.

## 11. Current Project Status

| Area | Current status |
|---|---|
| Phase 2A.17 implementation | Reported complete |
| Phase 2A.17 independent audit | Pending |
| Secure Boot and signed EFI | Not yet established as complete |
| TPM2 measured boot | Not yet established as complete |
| SQLCipher integration | Deferred; must be verified in Phase C |
| Financial application | Future implementation |
| Physical USB deployment | Not authorized |
| Host disk and UEFI modification | Prohibited without separate approval |

## 12. Change Control

This document is the architectural baseline. Any change to encryption, authentication, boot integrity, partition layout, recovery, financial invariants, or hardware deployment must include:

- Reason for change.
- Threat and risk assessment.
- Affected components.
- Migration and rollback plan.
- Tests and evidence.
- Independent review for security-sensitive changes.

**Final rule:** BHOOT must prioritize financial-data integrity, privacy, recoverability, and security over feature count or development speed.
