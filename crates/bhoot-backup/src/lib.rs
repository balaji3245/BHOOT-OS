pub mod archive;
pub mod error;

pub use archive::{
    create_backup_file, create_encrypted_backup, create_full_backup_file, inspect_backup_manifest,
    normalize_path, restore_backup_file, restore_encrypted_backup, restore_full_backup_file,
    validate_destination_path, BackupArchiveBundle, BackupManifest, BACKUP_HEADER_SIZE,
    BACKUP_MAGIC_V1,
};
pub use error::BackupError;

#[cfg(test)]
mod tests {
    use super::*;
    use bhoot_core::{Account, AccountSubtype, AccountType};
    use bhoot_db::Database;
    use std::path::Path;
    use tempfile::NamedTempFile;

    fn test_db_key() -> [u8; 32] {
        [0x33u8; 32]
    }

    #[test]
    fn test_backup_encryption_and_restoration_roundtrip() {
        let original_data = b"SQLITE_ENCRYPTED_DATABASE_PAYLOAD_MOCK_1234567890".to_vec();
        let passphrase = "correct-battery-horse-staple-backup-passphrase";

        let archive = create_encrypted_backup(&original_data, passphrase).unwrap();
        assert!(archive.len() > BACKUP_HEADER_SIZE);
        assert_eq!(&archive[..16], BACKUP_MAGIC_V1);

        let manifest = inspect_backup_manifest(&archive).unwrap();
        assert_eq!(manifest.version, 1);
        assert_eq!(manifest.argon2_m_cost, 65536);

        let restored = restore_encrypted_backup(&archive, passphrase).unwrap();
        assert_eq!(restored.as_slice(), original_data.as_slice());
    }

    #[test]
    fn test_wrong_passphrase_rejection_and_tamper_detection() {
        let original_data = b"SENSITIVE_FINANCIAL_DATABASE_BLOB".to_vec();
        let correct_pass = "my-super-secure-passphrase-2026";
        let wrong_pass = "wrong-passphrase-attempt-9999";

        let archive = create_encrypted_backup(&original_data, correct_pass).unwrap();

        // 1. Wrong passphrase must fail
        let res = restore_encrypted_backup(&archive, wrong_pass);
        assert!(matches!(res, Err(BackupError::DecryptionFailed)));

        // 2. Tampered ciphertext must fail
        let mut tampered = archive.clone();
        let last_idx = tampered.len() - 1;
        tampered[last_idx] ^= 0xFF; // Flip bits in auth tag / ciphertext
        let res_tampered = restore_encrypted_backup(&tampered, correct_pass);
        assert!(matches!(res_tampered, Err(BackupError::DecryptionFailed)));
    }

    #[test]
    fn test_prohibited_destination_paths() {
        assert!(matches!(
            validate_destination_path(Path::new("/dev/sda")),
            Err(BackupError::ProhibitedDestination(_))
        ));
        assert!(matches!(
            validate_destination_path(Path::new("/boot/vmlinuz")),
            Err(BackupError::ProhibitedDestination(_))
        ));
        assert!(matches!(
            validate_destination_path(Path::new("/proc/sys/vm")),
            Err(BackupError::ProhibitedDestination(_))
        ));
        assert!(matches!(
            validate_destination_path(Path::new("/var/bhoot/backups/../../dev/sda")),
            Err(BackupError::ProhibitedDestination(_))
        ));
        assert!(matches!(
            validate_destination_path(Path::new("/home/bhoot/../../dev/nvme0n1")),
            Err(BackupError::ProhibitedDestination(_))
        ));
        assert!(matches!(
            validate_destination_path(Path::new("/var/bhoot/backups/../../etc/shadow")),
            Err(BackupError::ProhibitedDestination(_))
        ));
        assert!(validate_destination_path(Path::new("/home/bhoot/backup.bhoot")).is_ok());
    }

    #[test]
    fn test_full_backup_with_vault_files_and_atomic_restore() {
        let db_file = NamedTempFile::new().unwrap();
        let backup_file = NamedTempFile::new().unwrap();
        let restored_db_file = NamedTempFile::new().unwrap();
        let vault_temp_dir = tempfile::tempdir().unwrap();
        let restored_vault_dir = tempfile::tempdir().unwrap();

        let backup_passphrase = "master-full-disaster-recovery-passphrase-2026";

        // 1. Create SQLCipher database with accounts and checkpoint
        {
            let db = Database::open(db_file.path(), &test_db_key()).unwrap();
            let acc1 = Account::new("Primary Vault Fund", AccountType::Asset, AccountSubtype::BankAccount, None);
            db.create_account(&acc1).unwrap();
            db.checkpoint().unwrap();
        }

        // 2. Create sample encrypted vault file in vault_temp_dir
        let vault_file_path = vault_temp_dir.path().join("doc_001.bin");
        std::fs::write(&vault_file_path, b"ENCRYPTED_VAULT_ATTACHMENT_BYTES").unwrap();

        // 3. Create full backup
        let written = create_full_backup_file(
            db_file.path(),
            Some(vault_temp_dir.path()),
            backup_file.path(),
            backup_passphrase,
        ).unwrap();
        assert!(written > BACKUP_HEADER_SIZE);

        // 4. Restore full backup
        let restored_bytes = restore_full_backup_file(
            backup_file.path(),
            restored_db_file.path(),
            Some(restored_vault_dir.path()),
            backup_passphrase,
        ).unwrap();
        assert!(restored_bytes > 0);

        // 5. Verify restored database
        {
            let db = Database::open(restored_db_file.path(), &test_db_key()).unwrap();
            let accounts = db.list_accounts().unwrap();
            assert_eq!(accounts.len(), 1);
            assert_eq!(accounts[0].name, "Primary Vault Fund");
            assert!(db.integrity_check().unwrap());
        }

        // 6. Verify restored vault file
        let restored_vault_file = restored_vault_dir.path().join("doc_001.bin");
        assert!(restored_vault_file.exists());
        let content = std::fs::read(restored_vault_file).unwrap();
        assert_eq!(content, b"ENCRYPTED_VAULT_ATTACHMENT_BYTES");
    }

    #[test]
    fn test_restore_into_temporary_fixture_and_sqlcipher_integrity() {
        let db_file = NamedTempFile::new().unwrap();
        let backup_file = NamedTempFile::new().unwrap();
        let restored_db_file = NamedTempFile::new().unwrap();

        let backup_passphrase = "disaster-recovery-test-key-2026";

        // 1. Create real encrypted SQLCipher database
        {
            let db = Database::open(db_file.path(), &test_db_key()).unwrap();
            let acc1 = Account::new("Checking", AccountType::Asset, AccountSubtype::BankAccount, None);
            let acc2 = Account::new("Salary", AccountType::Income, AccountSubtype::Salary, None);
            db.create_account(&acc1).unwrap();
            db.create_account(&acc2).unwrap();
            assert!(db.integrity_check().unwrap());
        }

        // 2. Create encrypted backup file on disk
        let written = create_backup_file(db_file.path(), backup_file.path(), backup_passphrase).unwrap();
        assert!(written > BACKUP_HEADER_SIZE);

        // 3. Restore backup file into a fresh temporary destination file
        let restored_bytes = restore_backup_file(backup_file.path(), restored_db_file.path(), backup_passphrase).unwrap();
        assert!(restored_bytes > 0);

        // 4. Open restored database with SQLCipher key and verify integrity
        {
            let db = Database::open(restored_db_file.path(), &test_db_key()).unwrap();
            let accounts = db.list_accounts().unwrap();
            assert_eq!(accounts.len(), 2);
            assert_eq!(accounts[0].name, "Checking");
            assert_eq!(accounts[1].name, "Salary");
            assert!(db.integrity_check().unwrap());
        }
    }
}
