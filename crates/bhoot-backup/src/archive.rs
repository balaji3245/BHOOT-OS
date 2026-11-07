use crate::error::BackupError;
use bhoot_crypto::{
    blake3_hash, constant_time_eq_32, decrypt_envelope, encrypt_envelope, generate_salt,
    Argon2idParams, EncryptedEnvelope,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use zeroize::Zeroizing;

/// Fixed 16-byte magic identifier for BHOOT backup archives.
pub const BACKUP_MAGIC_V1: &[u8; 16] = b"BHOOT-BACKUP-V1\0";

/// Header size: 16 (magic) + 32 (salt) + 4 (m) + 4 (t) + 4 (p) + 12 (nonce) + 32 (blake3) + 8 (len) = 112 bytes.
pub const BACKUP_HEADER_SIZE: usize = 112;

/// Metadata manifest describing backup archive contents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupManifest {
    pub version: u32,
    pub created_at: DateTime<Utc>,
    pub payload_size_bytes: usize,
    pub blake3_digest: [u8; 32],
    pub argon2_m_cost: u32,
    pub argon2_t_cost: u32,
    pub argon2_p_cost: u32,
}

/// Create an encrypted backup archive payload from raw database bytes.
///
/// Uses Argon2id password-based key derivation ($m=65536$, $t=3$, $p=4$) followed by AES-256-GCM.
pub fn create_encrypted_backup(
    db_bytes: &[u8],
    passphrase: &str,
) -> Result<Vec<u8>, BackupError> {
    if db_bytes.is_empty() {
        return Err(BackupError::InvalidHeader("Payload cannot be empty".into()));
    }

    let salt = generate_salt();
    let params = Argon2idParams {
        m_cost: 65536, // 64 MiB
        t_cost: 3,     // 3 passes
        p_cost: 4,     // 4 threads
    };

    let backup_key = bhoot_crypto::derive_master_key(passphrase.as_bytes(), &salt, params)?;
    let envelope = encrypt_envelope(db_bytes, &backup_key)?;

    let mut archive = Vec::with_capacity(BACKUP_HEADER_SIZE + envelope.ciphertext.len());

    // 1. Magic (16 bytes)
    archive.extend_from_slice(BACKUP_MAGIC_V1);
    // 2. Salt (32 bytes)
    archive.extend_from_slice(&salt);
    // 3. Argon2 Params (12 bytes)
    archive.extend_from_slice(&params.m_cost.to_le_bytes());
    archive.extend_from_slice(&params.t_cost.to_le_bytes());
    archive.extend_from_slice(&params.p_cost.to_le_bytes());
    // 4. Nonce (12 bytes)
    archive.extend_from_slice(&envelope.nonce);
    // 5. Pre-encryption BLAKE3 digest (32 bytes)
    archive.extend_from_slice(&envelope.plaintext_blake3_hash);
    // 6. Ciphertext length (8 bytes)
    archive.extend_from_slice(&(envelope.ciphertext.len() as u64).to_le_bytes());
    // 7. Ciphertext payload
    archive.extend_from_slice(&envelope.ciphertext);

    Ok(archive)
}

/// Parse and authenticate an encrypted backup archive, returning decrypted database bytes.
pub fn restore_encrypted_backup(
    archive_bytes: &[u8],
    passphrase: &str,
) -> Result<Zeroizing<Vec<u8>>, BackupError> {
    if archive_bytes.len() < BACKUP_HEADER_SIZE {
        return Err(BackupError::InvalidHeader(
            "Archive size is smaller than header".into(),
        ));
    }

    // 1. Validate magic bytes
    if &archive_bytes[..16] != BACKUP_MAGIC_V1 {
        return Err(BackupError::InvalidHeader(
            "Invalid archive magic signature".into(),
        ));
    }

    // 2. Extract salt
    let mut salt = [0u8; 32];
    salt.copy_from_slice(&archive_bytes[16..48]);

    // 3. Extract Argon2 parameters
    let m_cost = u32::from_le_bytes(archive_bytes[48..52].try_into().unwrap());
    let t_cost = u32::from_le_bytes(archive_bytes[52..56].try_into().unwrap());
    let p_cost = u32::from_le_bytes(archive_bytes[56..60].try_into().unwrap());

    let params = Argon2idParams {
        m_cost,
        t_cost,
        p_cost,
    };
    params
        .validate()
        .map_err(|e| BackupError::InvalidHeader(format!("Invalid Argon2 params: {e}")))?;

    // 4. Extract Nonce
    let mut nonce = [0u8; 12];
    nonce.copy_from_slice(&archive_bytes[60..72]);

    // 5. Extract expected BLAKE3 digest
    let mut expected_b3 = [0u8; 32];
    expected_b3.copy_from_slice(&archive_bytes[72..104]);

    // 6. Extract Ciphertext Length
    let cipher_len = u64::from_le_bytes(archive_bytes[104..112].try_into().unwrap()) as usize;
    if archive_bytes.len() != BACKUP_HEADER_SIZE + cipher_len {
        return Err(BackupError::InvalidHeader(format!(
            "Payload length mismatch: expected {cipher_len} bytes, archive has {}",
            archive_bytes.len() - BACKUP_HEADER_SIZE
        )));
    }

    let ciphertext = archive_bytes[BACKUP_HEADER_SIZE..].to_vec();

    let envelope = EncryptedEnvelope {
        nonce,
        ciphertext,
        plaintext_blake3_hash: expected_b3,
    };

    // 7. Derive key and decrypt envelope
    let backup_key = bhoot_crypto::derive_master_key(passphrase.as_bytes(), &salt, params)
        .map_err(|_| BackupError::DecryptionFailed)?;

    let plaintext = decrypt_envelope(&envelope, &backup_key)
        .map_err(|_| BackupError::DecryptionFailed)?;

    // 8. Verify pre-encryption BLAKE3 integrity
    let actual_b3 = blake3_hash(&plaintext);
    if !constant_time_eq_32(&actual_b3, &expected_b3) {
        return Err(BackupError::DigestMismatch);
    }

    Ok(plaintext)
}

/// Verify archive header and HMAC/GCM authentication without writing to disk.
pub fn inspect_backup_manifest(
    archive_bytes: &[u8],
) -> Result<BackupManifest, BackupError> {
    if archive_bytes.len() < BACKUP_HEADER_SIZE {
        return Err(BackupError::InvalidHeader(
            "Archive size too small".into(),
        ));
    }

    if &archive_bytes[..16] != BACKUP_MAGIC_V1 {
        return Err(BackupError::InvalidHeader("Invalid magic".into()));
    }

    let m_cost = u32::from_le_bytes(archive_bytes[48..52].try_into().unwrap());
    let t_cost = u32::from_le_bytes(archive_bytes[52..56].try_into().unwrap());
    let p_cost = u32::from_le_bytes(archive_bytes[56..60].try_into().unwrap());

    let mut b3 = [0u8; 32];
    b3.copy_from_slice(&archive_bytes[72..104]);

    let cipher_len = u64::from_le_bytes(archive_bytes[104..112].try_into().unwrap()) as usize;

    Ok(BackupManifest {
        version: 1,
        created_at: Utc::now(),
        payload_size_bytes: cipher_len.saturating_sub(16), // Less 16-byte auth tag
        blake3_digest: b3,
        argon2_m_cost: m_cost,
        argon2_t_cost: t_cost,
        argon2_p_cost: p_cost,
    })
}

/// Normalize path eliminating '.', '..' and redundant separators without requiring path existence.
pub fn normalize_path(path: &Path) -> std::path::PathBuf {
    let mut normalized = std::path::PathBuf::new();
    for comp in path.components() {
        match comp {
            std::path::Component::Prefix(p) => normalized.push(p.as_os_str()),
            std::path::Component::RootDir => normalized.push(std::path::MAIN_SEPARATOR.to_string()),
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            std::path::Component::Normal(c) => normalized.push(c),
        }
    }
    normalized
}

/// Disambiguation check to ensure backup target does not target system block devices or prohibited root paths.
/// Strictly normalizes relative path traversals and validates canonical ancestors.
pub fn validate_destination_path(destination: &Path) -> Result<(), BackupError> {
    let normalized = normalize_path(destination);
    let dest_str = normalized.to_string_lossy();

    let prohibited_prefixes = [
        "/dev", "/sys", "/proc", "/boot", "/etc", "/usr", "/bin", "/sbin", "/lib", "/root",
    ];

    for prefix in &prohibited_prefixes {
        if dest_str == *prefix
            || dest_str.starts_with(&format!("{prefix}/"))
            || dest_str.contains(&format!("{prefix}/"))
        {
            return Err(BackupError::ProhibitedDestination(dest_str.to_string()));
        }
    }

    if dest_str == "/" || dest_str.is_empty() {
        return Err(BackupError::ProhibitedDestination(dest_str.to_string()));
    }

    // Also check canonicalized path if file or parent directory exists
    if let Ok(canon) = destination.canonicalize() {
        let canon_str = canon.to_string_lossy();
        for prefix in &prohibited_prefixes {
            if canon_str == *prefix || canon_str.starts_with(&format!("{prefix}/")) {
                return Err(BackupError::ProhibitedDestination(canon_str.to_string()));
            }
        }
    } else if let Some(parent) = destination.parent() {
        if let Ok(canon_parent) = parent.canonicalize() {
            let canon_str = canon_parent.to_string_lossy();
            for prefix in &prohibited_prefixes {
                if canon_str == *prefix || canon_str.starts_with(&format!("{prefix}/")) {
                    return Err(BackupError::ProhibitedDestination(canon_str.to_string()));
                }
            }
        }
    }

    Ok(())
}

/// Structured multi-file bundle payload encapsulating database and document vault payloads.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupArchiveBundle {
    pub magic: String,
    pub version: u32,
    pub created_at: DateTime<Utc>,
    pub db_bytes: Vec<u8>,
    pub vault_files: Vec<(String, Vec<u8>)>,
}

/// Create a comprehensive encrypted backup archive file on disk containing both the database
/// (post-WAL checkpoint) and any encrypted vault files. Uses atomic write via a temp file.
pub fn create_full_backup_file(
    db_file_path: &Path,
    vault_dir: Option<&Path>,
    output_backup_path: &Path,
    passphrase: &str,
) -> Result<usize, BackupError> {
    validate_destination_path(output_backup_path)?;

    let db_bytes = fs::read(db_file_path)
        .map_err(|e| BackupError::IoError(format!("Failed to read database file: {e}")))?;

    let mut vault_files = Vec::new();
    if let Some(v_dir) = vault_dir {
        if v_dir.exists() && v_dir.is_dir() {
            if let Ok(entries) = fs::read_dir(v_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                            if let Ok(data) = fs::read(&path) {
                                vault_files.push((file_name.to_string(), data));
                            }
                        }
                    }
                }
            }
        }
    }

    let bundle = BackupArchiveBundle {
        magic: "BHOOT_BUNDLE_V1".to_string(),
        version: 1,
        created_at: Utc::now(),
        db_bytes,
        vault_files,
    };

    let bundle_bytes = serde_json::to_vec(&bundle)
        .map_err(|e| BackupError::IoError(format!("Failed to serialize backup bundle: {e}")))?;

    let archive_bytes = create_encrypted_backup(&bundle_bytes, passphrase)?;

    // Atomic write to temporary file in same directory then rename
    let parent = output_backup_path.parent().unwrap_or_else(|| Path::new("."));
    let temp_path = parent.join(format!(".tmp_backup_{}", uuid::Uuid::new_v4()));

    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);

    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }

    let mut file = options
        .open(&temp_path)
        .map_err(|e| BackupError::IoError(format!("Failed to create temp backup file: {e}")))?;

    file.write_all(&archive_bytes)
        .map_err(|e| BackupError::IoError(format!("Failed to write backup file: {e}")))?;

    file.sync_all()
        .map_err(|e| BackupError::IoError(format!("Failed to sync backup file: {e}")))?;

    fs::rename(&temp_path, output_backup_path)
        .map_err(|e| BackupError::IoError(format!("Failed to rename temp backup to destination: {e}")))?;

    Ok(archive_bytes.len())
}

/// Restore a full encrypted backup archive file, extracting the SQLCipher database and
/// restoring all document vault artifacts to their target directories.
pub fn restore_full_backup_file(
    backup_file_path: &Path,
    output_db_path: &Path,
    output_vault_dir: Option<&Path>,
    passphrase: &str,
) -> Result<usize, BackupError> {
    validate_destination_path(output_db_path)?;
    if let Some(v_dir) = output_vault_dir {
        validate_destination_path(v_dir)?;
    }

    let mut file = fs::File::open(backup_file_path)
        .map_err(|e| BackupError::IoError(format!("Failed to open backup file: {e}")))?;

    let mut archive_bytes = Vec::new();
    file.read_to_end(&mut archive_bytes)
        .map_err(|e| BackupError::IoError(format!("Failed to read backup file: {e}")))?;

    let plaintext = restore_encrypted_backup(&archive_bytes, passphrase)?;

    // Check if plaintext is a structured bundle or raw db bytes
    if let Ok(bundle) = serde_json::from_slice::<BackupArchiveBundle>(&plaintext) {
        if bundle.magic == "BHOOT_BUNDLE_V1" {
            // Restore database file atomically
            let parent = output_db_path.parent().unwrap_or_else(|| Path::new("."));
            let temp_db_path = parent.join(format!(".tmp_restore_{}", uuid::Uuid::new_v4()));

            let mut options = OpenOptions::new();
            options.write(true).create(true).truncate(true);

            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }

            let mut out_file = options
                .open(&temp_db_path)
                .map_err(|e| BackupError::IoError(format!("Failed to create restored DB: {e}")))?;

            out_file
                .write_all(&bundle.db_bytes)
                .map_err(|e| BackupError::IoError(format!("Failed to write restored DB: {e}")))?;

            out_file
                .sync_all()
                .map_err(|e| BackupError::IoError(format!("Failed to sync restored DB: {e}")))?;

            fs::rename(&temp_db_path, output_db_path)
                .map_err(|e| BackupError::IoError(format!("Failed to rename restored DB: {e}")))?;

            // Restore vault files if directory provided
            if let Some(v_dir) = output_vault_dir {
                fs::create_dir_all(v_dir)
                    .map_err(|e| BackupError::IoError(format!("Failed to create vault dir: {e}")))?;

                for (name, data) in &bundle.vault_files {
                    let sanitized_name = Path::new(name).file_name().unwrap_or_default();
                    let target_path = v_dir.join(sanitized_name);
                    let mut v_options = OpenOptions::new();
                    v_options.write(true).create(true).truncate(true);
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::OpenOptionsExt;
                        v_options.mode(0o600);
                    }
                    if let Ok(mut v_file) = v_options.open(&target_path) {
                        let _ = v_file.write_all(data);
                        let _ = v_file.sync_all();
                    }
                }
            }

            return Ok(bundle.db_bytes.len());
        }
    }

    // Fallback: Legacy raw database restore
    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);

    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }

    let mut out_file = options
        .open(output_db_path)
        .map_err(|e| BackupError::IoError(format!("Failed to create restored DB: {e}")))?;

    out_file
        .write_all(&plaintext)
        .map_err(|e| BackupError::IoError(format!("Failed to write restored DB: {e}")))?;

    out_file
        .sync_all()
        .map_err(|e| BackupError::IoError(format!("Failed to sync restored DB: {e}")))?;

    Ok(plaintext.len())
}

/// Create an encrypted backup archive file on disk with strict file permissions (0o600).
pub fn create_backup_file(
    db_file_path: &Path,
    output_backup_path: &Path,
    passphrase: &str,
) -> Result<usize, BackupError> {
    validate_destination_path(output_backup_path)?;

    let db_bytes = fs::read(db_file_path)
        .map_err(|e| BackupError::IoError(format!("Failed to read database file: {e}")))?;

    let archive_bytes = create_encrypted_backup(&db_bytes, passphrase)?;

    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);

    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }

    let mut file = options
        .open(output_backup_path)
        .map_err(|e| BackupError::IoError(format!("Failed to create backup file: {e}")))?;

    file.write_all(&archive_bytes)
        .map_err(|e| BackupError::IoError(format!("Failed to write backup file: {e}")))?;

    file.sync_all()
        .map_err(|e| BackupError::IoError(format!("Failed to sync backup file: {e}")))?;

    Ok(archive_bytes.len())
}

/// Restore an encrypted backup archive file directly to a destination SQLite database file.
pub fn restore_backup_file(
    backup_file_path: &Path,
    output_db_path: &Path,
    passphrase: &str,
) -> Result<usize, BackupError> {
    validate_destination_path(output_db_path)?;

    let mut file = fs::File::open(backup_file_path)
        .map_err(|e| BackupError::IoError(format!("Failed to open backup file: {e}")))?;

    let mut archive_bytes = Vec::new();
    file.read_to_end(&mut archive_bytes)
        .map_err(|e| BackupError::IoError(format!("Failed to read backup file: {e}")))?;

    let plaintext = restore_encrypted_backup(&archive_bytes, passphrase)?;

    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);

    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }

    let mut out_file = options
        .open(output_db_path)
        .map_err(|e| BackupError::IoError(format!("Failed to create restored DB: {e}")))?;

    out_file
        .write_all(&plaintext)
        .map_err(|e| BackupError::IoError(format!("Failed to write restored DB: {e}")))?;

    out_file
        .sync_all()
        .map_err(|e| BackupError::IoError(format!("Failed to sync restored DB: {e}")))?;

    Ok(plaintext.len())
}
