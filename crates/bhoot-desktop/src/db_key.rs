//! BHOOT Database Key Management & Cryptographic Key-Wrapping Store
//!
//! Provides secure at-rest wrapping and persistent lifecycle management for
//! SQLCipher 256-bit database encryption keys.
//!
//! Rooted in the LUKS2-encrypted filesystem (/var/bhoot/system), each installation
//! generates a cryptographically random 256-bit database key. The key is wrapped
//! using AES-256-GCM under a Key-Wrapping Key (KWK) derived via HKDF-SHA256 from
//! the host machine secret and a per-key 256-bit random salt.
//!
//! Transient key buffers are scrubbed on drop via zeroize::Zeroizing.

use bhoot_crypto::EncryptedEnvelope;
use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use thiserror::Error;
use zeroize::Zeroizing;

/// Error conditions encountered during database key resolution, wrapping, or recovery.
#[derive(Debug, Error)]
pub enum DbKeyError {
    #[error("Invalid BHOOT_DB_KEY environment variable: {0}")]
    EnvInvalidHex(String),

    #[error(
        "Database exists at '{0}' but wrapped key file '{1}' is missing and BHOOT_DB_KEY is not set. Refusing to initialize new key over existing database. Failing closed."
    )]
    MissingKeyForExistingDatabase(PathBuf, PathBuf),

    #[error("I/O error during key management: {0}")]
    Io(#[from] std::io::Error),

    #[error("Cryptographic error during key wrapping/unwrapping: {0}")]
    Crypto(String),

    #[error("Wrapped key file corrupted at '{0}': {1}")]
    CorruptedKeyFile(PathBuf, String),

    #[error("Failed to unwrap database key: {0}")]
    UnwrapFailed(String),

    #[error("Unsupported wrapped key version: {0}")]
    UnsupportedVersion(u32),

    #[error("Invalid unwrapped key length: expected 32 bytes, got {0}")]
    InvalidKeyLength(usize),
}

/// At-rest encrypted database key envelope store.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WrappedDbKeyStore {
    /// Format version identifier (must be 1).
    pub version: u32,
    /// 256-bit CSPRNG salt for HKDF key derivation.
    pub salt: [u8; 32],
    /// AES-256-GCM envelope containing encrypted 256-bit key + BLAKE3 hash.
    pub envelope: EncryptedEnvelope,
}

/// Decode an exact 64-character hexadecimal string into a 32-byte key.
pub fn decode_hex_32(s: &str) -> Result<[u8; 32], String> {
    let s = s.trim();
    if s.len() != 64 {
        return Err("Hex string must be exactly 64 characters (32 bytes)".into());
    }
    let mut bytes = [0u8; 32];
    for (i, byte) in bytes.iter_mut().enumerate() {
        let chunk = &s[i * 2..i * 2 + 2];
        *byte = u8::from_str_radix(chunk, 16)
            .map_err(|e| format!("Invalid hex at byte index {i}: {e}"))?;
    }
    Ok(bytes)
}

/// Retrieve or create the persistent machine root secret for key wrapping.
///
/// Combines the local system installation token (/var/bhoot/system/machine-id)
/// with the OS machine ID (/etc/machine-id) using BLAKE3.
pub fn get_or_create_machine_secret(system_dir: &Path) -> Result<Zeroizing<[u8; 32]>, DbKeyError> {
    let machine_id_path = system_dir.join("machine-id");

    let local_token: Vec<u8> = if machine_id_path.exists() {
        let data = fs::read(&machine_id_path)?;
        if data.is_empty() {
            // Regenerate if empty
            let mut fresh = vec![0u8; 32];
            OsRng.fill_bytes(&mut fresh);
            save_file_atomic(&machine_id_path, &fresh, 0o600)?;
            fresh
        } else {
            data
        }
    } else {
        let mut fresh = vec![0u8; 32];
        OsRng.fill_bytes(&mut fresh);
        save_file_atomic(&machine_id_path, &fresh, 0o600)?;
        fresh
    };

    let mut hasher = blake3::Hasher::new();
    hasher.update(b"BHOOT_MACHINE_ROOT_SECRET_V1:");
    hasher.update(&local_token);

    if let Ok(etc_id) = fs::read_to_string("/etc/machine-id") {
        let trimmed = etc_id.trim();
        if !trimmed.is_empty() {
            hasher.update(b":etc_machine_id:");
            hasher.update(trimmed.as_bytes());
        }
    }

    let hash = hasher.finalize();
    let mut secret = Zeroizing::new([0u8; 32]);
    secret.copy_from_slice(hash.as_bytes());
    Ok(secret)
}

/// Derive the Key-Wrapping Key (KWK) from machine secret and salt using HKDF-SHA256.
fn derive_kwk(salt: &[u8; 32], machine_secret: &[u8; 32]) -> Result<Zeroizing<[u8; 32]>, DbKeyError> {
    let hk = hkdf::Hkdf::<sha2::Sha256>::new(Some(salt), machine_secret);
    let mut kwk = Zeroizing::new([0u8; 32]);
    hk.expand(b"bhoot-db-key-wrap-kwk-v1", kwk.as_mut())
        .map_err(|e| DbKeyError::Crypto(format!("HKDF KWK expansion failed: {e}")))?;
    Ok(kwk)
}

/// Wrap a 256-bit database key into an authenticated, encrypted envelope.
pub fn wrap_key(
    key: &[u8; 32],
    machine_secret: &[u8; 32],
) -> Result<WrappedDbKeyStore, DbKeyError> {
    let mut salt = [0u8; 32];
    OsRng.fill_bytes(&mut salt);

    let kwk = derive_kwk(&salt, machine_secret)?;

    let envelope = bhoot_crypto::encrypt_envelope(key, &kwk)
        .map_err(|e| DbKeyError::Crypto(format!("Envelope encryption failed: {e}")))?;

    Ok(WrappedDbKeyStore {
        version: 1,
        salt,
        envelope,
    })
}

/// Unwrap a 256-bit database key from its authenticated envelope store.
pub fn unwrap_key(
    store: &WrappedDbKeyStore,
    machine_secret: &[u8; 32],
) -> Result<Zeroizing<[u8; 32]>, DbKeyError> {
    if store.version != 1 {
        return Err(DbKeyError::UnsupportedVersion(store.version));
    }

    let kwk = derive_kwk(&store.salt, machine_secret)?;

    let decrypted = bhoot_crypto::decrypt_envelope(&store.envelope, &kwk)
        .map_err(|e| DbKeyError::UnwrapFailed(format!("Integrity check or decryption failed: {e}")))?;

    if decrypted.len() != 32 {
        return Err(DbKeyError::InvalidKeyLength(decrypted.len()));
    }

    let mut key = Zeroizing::new([0u8; 32]);
    key.copy_from_slice(&decrypted);
    Ok(key)
}

/// Atomically write data to target path with strict Unix permissions (0600).
fn save_file_atomic(path: &Path, data: &[u8], _mode: u32) -> Result<(), DbKeyError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let temp_path = path.with_extension(format!("tmp.{}", std::process::id()));

    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(_mode)
            .open(&temp_path)?;
        file.write_all(data)?;
        file.sync_all()?;
    }

    #[cfg(not(unix))]
    {
        let mut file = fs::File::create(&temp_path)?;
        file.write_all(data)?;
        file.sync_all()?;
    }

    fs::rename(&temp_path, path)?;
    Ok(())
}

/// Save wrapped key store to disk atomically with mode 0600.
pub fn save_wrapped_store(path: &Path, store: &WrappedDbKeyStore) -> Result<(), DbKeyError> {
    let json = serde_json::to_string_pretty(store)
        .map_err(|e| DbKeyError::Crypto(format!("Failed to serialize wrapped store: {e}")))?;
    save_file_atomic(path, json.as_bytes(), 0o600)
}

/// Main entry point to resolve or provision the SQLCipher database encryption key.
///
/// Priority:
/// 1. `BHOOT_DB_KEY` environment variable:
///    If present, must be an exact 64-character hex string. Fails closed on any formatting error.
/// 2. Wrapped key file on disk:
///    - If the file exists, it is unwrapped using the machine secret. Fails closed on unwrap failure.
///    - If the file does NOT exist:
///      - If the database file ALREADY exists: FAILS CLOSED with fatal error to prevent unauthorized
///        access or database corruption.
///      - If the database file does NOT exist: provisions a fresh 256-bit CSPRNG key, wraps it,
///        atomically saves it with 0600 permissions, and returns the key.
pub fn resolve_or_provision_database_key(
    db_path: &Path,
    explicit_key_file: Option<&Path>,
) -> Result<Zeroizing<[u8; 32]>, DbKeyError> {
    // 1. Check environment variable override
    if let Ok(env_key) = std::env::var("BHOOT_DB_KEY") {
        let trimmed = env_key.trim();
        if !trimmed.is_empty() {
            let raw = decode_hex_32(trimmed).map_err(DbKeyError::EnvInvalidHex)?;
            println!("[BHOOT] Using database key provided via BHOOT_DB_KEY environment variable.");
            return Ok(Zeroizing::new(raw));
        }
    }

    // 2. Determine target key file path and system directory
    let (key_file, system_dir) = if let Some(kf) = explicit_key_file {
        let parent = kf.parent().unwrap_or_else(|| Path::new(".")).to_path_buf();
        (kf.to_path_buf(), parent)
    } else if db_path == Path::new("/var/bhoot/data/bhoot.db") || Path::new("/var/bhoot/system").is_dir() {
        let sys = PathBuf::from("/var/bhoot/system");
        (sys.join("db_key.wrapped"), sys)
    } else {
        let parent = db_path.parent().unwrap_or_else(|| Path::new(".")).to_path_buf();
        (parent.join("db_key.wrapped"), parent)
    };

    fs::create_dir_all(&system_dir)?;

    // 3. Resolve key file
    if key_file.exists() {
        let content = fs::read_to_string(&key_file)?;
        let store: WrappedDbKeyStore = serde_json::from_str(&content)
            .map_err(|e| DbKeyError::CorruptedKeyFile(key_file.clone(), e.to_string()))?;
        let machine_secret = get_or_create_machine_secret(&system_dir)?;
        let key = unwrap_key(&store, &machine_secret)?;
        println!(
            "[BHOOT] Successfully loaded and unwrapped persistent database key from {}",
            key_file.display()
        );
        Ok(key)
    } else {
        // Key file does NOT exist
        if db_path.exists() {
            eprintln!(
                "[BHOOT FATAL] Database exists at {} but wrapped key file {} is missing and BHOOT_DB_KEY is not set.",
                db_path.display(),
                key_file.display()
            );
            eprintln!(
                "[BHOOT FATAL] Refusing to initialize fresh key over existing database. Failing closed."
            );
            return Err(DbKeyError::MissingKeyForExistingDatabase(
                db_path.to_path_buf(),
                key_file,
            ));
        }

        // Fresh installation: generate 256-bit CSPRNG key
        let mut new_key = Zeroizing::new([0u8; 32]);
        OsRng.fill_bytes(new_key.as_mut());

        let machine_secret = get_or_create_machine_secret(&system_dir)?;
        let store = wrap_key(&new_key, &machine_secret)?;
        save_wrapped_store(&key_file, &store)?;
        println!(
            "[BHOOT] Fresh installation: provisioned new 256-bit database key wrapped at {}",
            key_file.display()
        );
        Ok(new_key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_hex_decode_valid() {
        let hex = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let bytes = decode_hex_32(hex).expect("valid hex");
        assert_eq!(bytes[0], 0x01);
        assert_eq!(bytes[15], 0xef);
        assert_eq!(bytes[31], 0xef);
    }

    #[test]
    fn test_hex_decode_invalid_length() {
        let short = "0123456789abcdef";
        assert!(decode_hex_32(short).is_err());
        let long = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef00";
        assert!(decode_hex_32(long).is_err());
    }

    #[test]
    fn test_hex_decode_invalid_chars() {
        let invalid = "zzzz456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        assert!(decode_hex_32(invalid).is_err());
    }

    #[test]
    fn test_wrap_unwrap_roundtrip() {
        let machine_secret = [0x42u8; 32];
        let original_key = [0x7au8; 32];

        let store = wrap_key(&original_key, &machine_secret).expect("wrap succeeds");
        assert_eq!(store.version, 1);

        let unwrapped = unwrap_key(&store, &machine_secret).expect("unwrap succeeds");
        assert_eq!(*unwrapped, original_key);
    }

    #[test]
    fn test_tampered_envelope_fails() {
        let machine_secret = [0x42u8; 32];
        let original_key = [0x7au8; 32];

        let mut store = wrap_key(&original_key, &machine_secret).expect("wrap succeeds");
        // Tamper with ciphertext
        if let Some(byte) = store.envelope.ciphertext.first_mut() {
            *byte ^= 0xFF;
        }

        let result = unwrap_key(&store, &machine_secret);
        assert!(result.is_err());
    }

    #[test]
    fn test_wrong_machine_secret_fails() {
        let machine_secret_a = [0x42u8; 32];
        let machine_secret_b = [0x99u8; 32];
        let original_key = [0x7au8; 32];

        let store = wrap_key(&original_key, &machine_secret_a).expect("wrap succeeds");
        let result = unwrap_key(&store, &machine_secret_b);
        assert!(result.is_err());
    }

    #[test]
    fn test_existing_db_missing_key_fails_closed() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("existing.db");
        fs::write(&db_path, b"SQLite format 3\0").unwrap();

        let key_file = dir.path().join("db_key.wrapped");
        assert!(!key_file.exists());

        let res = resolve_or_provision_database_key(&db_path, Some(&key_file));
        assert!(res.is_err());
        match res.unwrap_err() {
            DbKeyError::MissingKeyForExistingDatabase(p_db, p_kf) => {
                assert_eq!(p_db, db_path);
                assert_eq!(p_kf, key_file);
            }
            other => panic!("Expected MissingKeyForExistingDatabase, got {other:?}"),
        }
    }

    #[test]
    fn test_fresh_installation_provisions_key_and_persists() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("new_db.db");
        let key_file = dir.path().join("db_key.wrapped");

        assert!(!db_path.exists());
        assert!(!key_file.exists());

        // Stage 1: Fresh provision
        let key1 = resolve_or_provision_database_key(&db_path, Some(&key_file)).expect("provisions fresh key");
        assert_ne!(*key1, [0u8; 32]);
        assert!(key_file.exists());

        // Simulate database creation with key1
        fs::write(&db_path, b"test_db_content").unwrap();

        // Stage 2: Restart / reload from wrapped store
        let key2 = resolve_or_provision_database_key(&db_path, Some(&key_file)).expect("recovers key");
        assert_eq!(*key1, *key2);
    }

    #[test]
    fn test_independent_installations_have_distinct_keys() {
        let dir1 = tempdir().unwrap();
        let dir2 = tempdir().unwrap();

        let db1 = dir1.path().join("db.db");
        let kf1 = dir1.path().join("db_key.wrapped");

        let db2 = dir2.path().join("db.db");
        let kf2 = dir2.path().join("db_key.wrapped");

        let key1 = resolve_or_provision_database_key(&db1, Some(&kf1)).unwrap();
        let key2 = resolve_or_provision_database_key(&db2, Some(&kf2)).unwrap();

        assert_ne!(*key1, *key2);
    }

    #[cfg(unix)]
    #[test]
    fn test_file_permissions_mode_0600() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("db.db");
        let key_file = dir.path().join("db_key.wrapped");

        let _key = resolve_or_provision_database_key(&db_path, Some(&key_file)).unwrap();
        let metadata = fs::metadata(&key_file).unwrap();
        let mode = metadata.permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "Wrapped key file must have strict 0600 permissions");

        let machine_id_path = dir.path().join("machine-id");
        let machine_metadata = fs::metadata(&machine_id_path).unwrap();
        let machine_mode = machine_metadata.permissions().mode() & 0o777;
        assert_eq!(machine_mode, 0o600, "Machine ID token must have strict 0600 permissions");
    }
}

