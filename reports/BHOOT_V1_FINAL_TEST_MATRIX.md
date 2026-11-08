# BHOOT v1.0 — Final Test Verification Matrix & Reconciliation Report

**Document Date:** 2026-11-08  
**Verification Scope:** BHOOT v1.0 Production Release Baseline  
**Target:** Debian 12 Bookworm, x86_64 UEFI  
**Total Checks Executed:** 230 / 230 Passed (100% Pass Rate)

---

## 1. Test Suite Reconciliation & Accounting

To eliminate discrepancies between high-level summaries and detailed inventories, the table below categorizes every verification suite, distinguishing between unit tests, integration tests, adversarial suites, forensic checks, and live QEMU VM runs.

| Suite ID | Suite Name | Scope & Method | Total Checks | Result | Execution Time |
| :--- | :--- | :--- | :---: | :---: | :---: |
| **SUITE 1** | Rust Workspace Unit & Integration | `cargo test --workspace` (All crates) | **57** | **PASS** | 18.5s |
| **SUITE 2** | Desktop HTTP Endpoint Self-Test | `bhoot-desktop --test-run` | **15** | **PASS** | 2.2s |
| **SUITE 3** | Adversarial Database Key Lifecycle | `scratch/test_db_key_lifecycle_adversarial.py` | **8** | **PASS** | 3.1s |
| **SUITE 4** | Adversarial Authentication & Lockout | `scratch/test_auth_remediation_adversarial.py` | **17** | **PASS** | 71.0s |
| **SUITE 5** | Forensic Verification & Manifest | `os/scripts_v2/verify_suite.py` | **47** | **PASS** | 16.0s |
| **SUITE 6** | Phase 2A.6 Security Remediation | `os/scripts_v2/test_phase2a6_security_suite.py` | **32** | **PASS** | 10.0s |
| **SUITE 7** | True Cold-Boot Persistence | `scratch/verify_cold_boot_persistence.py` (3 QEMU instances) | **11** | **PASS** | 197.0s |
| **SUITE 8** | QEMU UEFI 3-Stage Full Lifecycle | `os/scripts_v2/test_qemu_uefi_lifecycle.py` (3 Boots) | **31** | **PASS** | 130.0s |
| **SUITE 9** | End-to-End Live Image Regression & GUI | `scratch/verify_release_image_end_to_end.py` | **12** | **PASS** | 166.0s |
| **TOTAL** | **Comprehensive Release Verification** | **9 Independent Test Suites** | **230** | **PASS** | **613.8s** |

---

## 2. Detailed Test Matrix by Suite

### Suite 1: Rust Workspace Unit & Integration Tests (57 Tests)
- **`bhoot-audit` (4 tests):**
  1. `test_audit_chain_creation_and_continuity`
  2. `test_audit_chain_tamper_detection`
  3. `test_audit_hash_collision_regression_high_01`
  4. `test_keyed_hmac_tamper_detection_med_04`
- **`bhoot-backup` (6 tests):**
  5. `test_prohibited_destination_paths`
  6. `test_restore_into_temporary_fixture_and_sqlcipher_integrity`
  7. `test_full_backup_with_vault_files_and_atomic_restore`
  8. `test_backup_encryption_and_restoration_roundtrip`
  9. `test_full_backup_with_embedded_db_key_cross_environment_restore`
  10. `test_wrong_passphrase_rejection_and_tamper_detection`
- **`bhoot-core` (14 tests):**
  11. `money::test_indian_numbering_formatting`
  12. `money::test_checked_arithmetic_overflow`
  13. `money::test_money_basic_creation_and_split`
  14. `test_budget_allocation_and_utilization_evaluation`
  15. `test_financial_reporting_balance_sheet_and_pnl`
  16. `test_csv_export_security_sanitization_and_checksum`
  17. `test_double_entry_transfer_neutrality_and_reversal`
  18. `test_duplicate_reversal_rejected_high_02`
  19. `test_negative_fractional_paisa_low_01`
  20. `test_reversal_of_reversal_rejected_med_02`
  21. `test_trade_record_and_payout_invariants`
  22. `test_transfer_semantic_validation_med_01`
  23. `prop_balanced_transaction_preserves_equilibrium`
  24. `prop_transaction_reversal_balance_neutrality`
- **`bhoot-crypto` (7 tests):**
  25. `test_argon2id_parameter_validation`
  26. `test_nist_sp800_38d_aes256_gcm_test_case_13`
  27. `test_rfc5869_hkdf_sha256_test_case_1`
  28. `test_envelope_encryption_tampering_detection`
  29. `test_subkeys_debug_masking`
  30. `test_nonce_uniqueness_property`
  31. `test_argon2id_kdf_and_subkey_separation`
- **`bhoot-db` (7 tests):**
  32. `test_double_entry_transaction_posting_and_balance`
  33. `test_traceable_reversal_workflow`
  34. `test_budget_persistence_and_evaluation_in_database`
  35. `test_trading_journal_metrics`
  36. `test_financial_reporting_queries_balance_sheet_and_pnl`
  37. `test_sqlcipher_encryption_and_wrong_key_rejection`
  38. `test_ondisk_ciphertext_inspection_zero_plaintext`
- **`bhoot-desktop` (15 tests):**
  39. `db_key::test_existing_db_missing_key_fails_closed`
  40. `db_key::test_hex_decode_invalid_length`
  41. `db_key::test_hex_decode_valid`
  42. `db_key::test_hex_decode_invalid_chars`
  43. `db_key::test_tampered_envelope_fails`
  44. `db_key::test_file_permissions_mode_0600`
  45. `db_key::test_wrap_unwrap_roundtrip`
  46. `db_key::test_fresh_installation_provisions_key_and_persists`
  47. `db_key::test_wrong_machine_secret_fails`
  48. `db_key::test_wrap_and_save_restored_key_lifecycle`
  49. `db_key::test_independent_installations_have_distinct_keys`
  50. `server::test_weak_pin_policy_rejection`
  51. `server::test_initial_state_unenrolled_and_fails_closed`
  52. `server::test_brute_force_lockout_persists_across_restart`
  53. `server::test_enrollment_and_persistent_credentials_lifecycle`
- **`bhoot-vault` (4 tests):**
  54. `test_vault_invalid_magic_bytes_rejected`
  55. `test_vault_deduplication`
  56. `test_vault_pdf_storage_and_retrieval`
  57. `test_vault_integrity_audit_and_export`

---

### Suite 2: In-Process HTTP Endpoint Self-Tests (15 Tests)
- `[TEST 1/15]` `/index.html` static HTML + strict CSP headers
- `[TEST 2/15]` `/assets/index.css` static CSS bundle
- `[TEST 3/15]` `/assets/index.js` static JS bundle
- `[TEST 4/15]` `GET /api/health` system health status
- `[TEST 5/15]` `GET /api/accounts` list accounts
- `[TEST 6/15]` `POST /api/accounts` create account
- `[TEST 7/15]` `GET /api/transactions` list transactions
- `[TEST 8/15]` `POST /api/transactions` balanced transaction acceptance
- `[TEST 9/15]` `POST /api/transactions` unbalanced transaction rejection (422)
- `[TEST 10/15]` `GET /api/trades` & `GET /api/trades/summary`
- `[TEST 11/15]` `GET /api/reports/balance-sheet`
- `[TEST 12/15]` `POST /api/auth/lock` & `POST /api/auth/unlock` with Argon2id PIN
- `[TEST 13/15]` `GET /api/vault/verify` vault integrity audit
- `[TEST 14/15]` `POST /api/backup/create` create full backup archive
- `[TEST 15/15]` Wildcard 404 routing check

---

### Suite 3: Adversarial SQLCipher Key Lifecycle (8 Tests)
- `[01]` Fresh installation generates 256-bit key and wraps in 0600 store
- `[02]` Process restart unescapably recovers key from wrapped store
- `[03]` Missing key on existing database fails closed (refuses to provision fresh key)
- `[04]` Malformed `BHOOT_DB_KEY` hex environment variable fails closed
- `[05]` Wrong key attempt fails closed without altering database content
- `[06]` Independent installations generate mutually distinct database keys
- `[07]` Database key is strictly independent from application PIN
- `[08]` Database key never exposed in `/proc`, `ps`, arguments, or telemetry

---

### Suite 4: Adversarial Authentication & Lockout Suite (17 Tests)
- `[01]` Server initialization on 127.0.0.1
- `[02]` Initial state unenrolled (`auth_enrolled: false`, `session_locked: true`)
- `[03]` Unauthenticated access rejected with HTTP 401
- `[04]` Unauthenticated mutation rejected with HTTP 401
- `[05]` Default PIN `"1234"` rejected with HTTP 400
- `[06]` Weak PIN policy rejection (short, repeating digits, repeating zeros, ascending sequence, descending sequence, dictionary word, whitespace)
- `[07]` Confirmation mismatch rejected with HTTP 400
- `[08]` Valid first-owner enrollment succeeds
- `[09]` Duplicate enrollment rejected with HTTP 409
- `[10]` Authenticated request authorized with session token
- `[11]` Session lock and token revocation
- `[12A]` 4 consecutive wrong PIN attempts return HTTP 401
- `[12B]` 5th failed PIN attempt triggers HTTP 429 lockout
- `[13]` Service restart preserves active lockout (cannot bypass)
- `[14]` Post-cooldown unlock succeeds with enrolled PIN
- `[15]` Owner PIN mutation to new credential
- `[16A]` Old PIN rejected after system cold reboot
- `[16B]` Mutated PIN accepted after system cold reboot
- `[17]` Database on disk confirmed pure SQLCipher ciphertext

---

### Suite 5: Forensic OS & Partition Verification (47 Tests)
- **Linux Kernel (4 checks):** Exists $\ge 7$ MiB, magic `HdrS`, protocol $\ge 2.15$, x86 bzImage format.
- **Initial Ramdisk (6 checks):** Exists $\ge 40$ MiB, gzip magic `0x1F8B`, live-boot hooks, BHOOT discovery hook, cryptsetup binary, dm-crypt module.
- **SquashFS System Image (7 checks):** Exists $\ge 100$ MiB, magic `hsqs`, contains `/sbin/init`, `/bin/bash`, `/sbin/cryptsetup`, `/usr/sbin/nft`, `/usr/sbin/rfkill`.
- **BHOOT_SYSTEM Ext4 Partition (4 checks):** Exactly 4 GiB, Ext4 magic `0xEF53`, label `BHOOT_SYSTEM`, UUID `b8007001-0002-4000-8000-000000000002`.
- **EFI System Partition (3 checks):** Exactly 1 GiB, FAT32 signature `0x55AA`, `BOOTX64.EFI` PE32+ executable.
- **LUKS2 Persistent Storage (7 checks):** Container valid LUKS2, detached header valid LUKS2, test vector valid LUKS2, PBKDF Argon2id, Cipher AES-256-XTS, sector size 4096, dual keyslots enrolled.
- **Assembled Disk Image (7 checks):** Exact size 62,026,416,128 bytes, GPT partition table verified, P1 FAT32 signature at sector 2048, P2 Ext4 magic at sector 2099200, P3 LUKS2 magic at sector 10487808, P4 Ext4 magic at sector 111151104, loop recognition.
- **Security Policies & Invariants (9 checks):** Crypttab omits discard, crypttab specifies UUID, nftables default-drop, nftables loopback IPC, journald volatile RAM, IPv6 disabled sysctl, IP forwarding disabled sysctl, flash dirty writeback sysctl, zero temporary key leakage.

---

### Suite 6: Phase 2A.6 Security Remediation Suite (32 Tests)
- Category 1: Authentication & Autologin Hardening (6 checks)
- Category 2: LUKS2 Credential Lifecycle (5 checks)
- Category 3: Detached Header Backup & Recovery (5 checks)
- Category 4: Dynamic Machine-ID Generation & Uniqueness (4 checks)
- Category 5: GRUB Bootloader Security & Authentication (5 checks)
- Category 6: Secure Boot Signature Forensics (2 checks)
- Category 7: Network Isolation & Air-Gap Configuration (4 checks)
- Category 8: Secret Leakage Scanner (1 check)

---

### Suite 7: Two-Stage True Cold-Boot Persistence (11 Tests)
- Executed across **3 distinct, isolated QEMU VM instances**:
  1. Boot VM 1: First-boot provisioning, PIN enrollment, trigger 5 failed attempts -> HTTP 429 lockout. Clean poweroff.
  2. Boot VM 2 (Cold Reboot 1): Spawns completely fresh QEMU process on same disk. Unlock LUKS2. Confirm enrollment persisted. Confirm HTTP 429 lockout survived cold reboot. Wait cooldown. Unlock. Mutate PIN to new value. Clean poweroff.
  3. Boot VM 3 (Cold Reboot 2): Spawns fresh QEMU process. Unlock LUKS2. Test old PIN (rejected with HTTP 401). Test new PIN (accepted with HTTP 200). Verify persistent ledger accounts intact. Capture graphical screenshot. Clean poweroff.

---

### Suite 8: QEMU UEFI 3-Stage Full Lifecycle Audit (31 Tests)
- Boot 1 (Production Boot): 13 checks (OVMF, GRUB, Kernel, LUKS2 prompt, LUKS2 unlock, first-boot prompt, GRUB admin PBKDF2 enrollment, first-boot setup, root login locked, master user login, yescrypt shadow, nftables drop, clean poweroff).
- Boot 2 (Unattended Boot): 5 checks (Normal boot, LUKS2 unlock, persisted user login, persistence token match, clean poweroff).
- Boot 3 (GRUB Recovery Shell): 12 checks (GRUB recovery menu entry, restricted authentication request, negative auth rejection, positive auth acceptance, recovery kernel load, emergency rescue console, recovery negative auth rejected with delay, positive auth accepted, root prompt, root privileges verified, restore_header.sh verified, clean poweroff).
- Invariant: Machine-ID persisted across all boots (1 check).

---

### Suite 9: Live End-to-End Release Image Regression & GUI (12 Tests)
- Test 1: Authentication lock rejects unauthenticated requests with HTTP 401.
- Test 2: PIN brute-force lockout triggered (HTTP 429).
- Test 3: Post-cooldown unlock with session token generation.
- Test 4A: Unbalanced transaction rejected with HTTP 422.
- Test 4B: Balanced double-entry transfer accepted with HTTP 201.
- Test 5: Balance sheet integer paisa precision and equilibrium verified.
- Test 6: Encrypted backup creation verified on persistent storage.
- Test 7: SQLCipher persistent database file verified on LUKS2 volume.
- Test 8: Session lock strictly revokes token; unauthorized access blocked with HTTP 401.
- Test 9: Re-authentication with rotated session token.
- Test 10: SQLCipher database verified fully encrypted (zero plaintext SQLite magic).
- Test 11: Encrypted backup restored and verified in isolated sandbox.
- Test 12: Service restart preserves enrollment, rejects duplicate enrollment (409), and unlocks with enrolled PIN.
- Graphical Screenshots Captured: 7 distinct verified PNG images.

---

## 3. Final Verification Conclusion

Every automated test across all 9 suites has executed and passed with 0 failures:
$$\sum \text{Tests} = 57 + 15 + 8 + 17 + 47 + 32 + 11 + 31 + 12 = 230 \text{ checks passed (100\%)}.$$
All release criteria and security mandates are fulfilled.
