use crate::error::CryptoError;
use hkdf::Hkdf;
use sha2::Sha256;
use zeroize::Zeroizing;

/// Cryptographically separated operational subkeys derived from the Master Key.
#[derive(Debug)]
pub struct SubKeys {
    /// 256-bit key dedicated to SQLCipher SQLite database page encryption.
    pub db_key: Zeroizing<[u8; 32]>,
    /// 256-bit key dedicated to AES-256-GCM Document Vault envelope encryption.
    pub vault_key: Zeroizing<[u8; 32]>,
    /// 256-bit key dedicated to the tamper-evident audit log HMAC verification.
    pub audit_key: Zeroizing<[u8; 32]>,
}

const INFO_DB: &[u8] = b"bhoot-sqlcipher-database-key-v1";
const INFO_VAULT: &[u8] = b"bhoot-aes256gcm-vault-key-v1";
const INFO_AUDIT: &[u8] = b"bhoot-audit-log-hmac-key-v1";

/// Expand a 256-bit Master Key into 3 isolated operational subkeys using HKDF-SHA256 (RFC 5869).
///
/// Ensures domain separation so that knowledge or leakage of one subkey
/// cannot be used to deduce keys for other subsystems.
///
/// # Errors
/// Returns `CryptoError::KeyDerivationFailed` if HKDF expansion fails.
pub fn derive_subkeys(master_key: &[u8; 32]) -> Result<SubKeys, CryptoError> {
    // Salt is None for pseudorandom master key extraction
    let hk = Hkdf::<Sha256>::new(None, master_key);

    let mut db_key = Zeroizing::new([0u8; 32]);
    let mut vault_key = Zeroizing::new([0u8; 32]);
    let mut audit_key = Zeroizing::new([0u8; 32]);

    hk.expand(INFO_DB, db_key.as_mut()).map_err(|e| {
        CryptoError::KeyDerivationFailed(format!("HKDF db_key expansion error: {e}"))
    })?;

    hk.expand(INFO_VAULT, vault_key.as_mut()).map_err(|e| {
        CryptoError::KeyDerivationFailed(format!("HKDF vault_key expansion error: {e}"))
    })?;

    hk.expand(INFO_AUDIT, audit_key.as_mut()).map_err(|e| {
        CryptoError::KeyDerivationFailed(format!("HKDF audit_key expansion error: {e}"))
    })?;

    Ok(SubKeys {
        db_key,
        vault_key,
        audit_key,
    })
}
