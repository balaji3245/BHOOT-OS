use crate::error::CryptoError;
use crate::hash::{blake3_hash, constant_time_eq_32};
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

/// Encrypted envelope containing authenticated ciphertext and plaintext integrity digest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncryptedEnvelope {
    /// 96-bit (12-byte) unique initialization vector / nonce.
    pub nonce: [u8; 12],
    /// AES-256-GCM ciphertext payload with appended 128-bit authentication tag.
    pub ciphertext: Vec<u8>,
    /// Pre-encryption 256-bit BLAKE3 hash of the plaintext.
    pub plaintext_blake3_hash: [u8; 32],
}

/// Generate a cryptographically secure 96-bit nonce from the OS CSPRNG.
#[must_use]
pub fn generate_nonce() -> [u8; 12] {
    let mut nonce = [0u8; 12];
    OsRng.fill_bytes(&mut nonce);
    nonce
}

/// Encrypt arbitrary data using AES-256-GCM and compute pre-encryption integrity hash.
///
/// Draws a fresh 96-bit nonce from `OsRng`. **Nonce reuse is mathematically impossible here.**
///
/// # Errors
/// Returns `CryptoError::EncryptionFailed` if AEAD encryption fails.
pub fn encrypt_envelope(
    plaintext: &[u8],
    key: &[u8; 32],
) -> Result<EncryptedEnvelope, CryptoError> {
    let nonce = generate_nonce();
    encrypt_envelope_with_nonce(plaintext, key, nonce)
}

/// Encrypt arbitrary data with an explicit nonce (used for deterministic KAT tests).
///
/// # Errors
/// Returns `CryptoError::EncryptionFailed` if AEAD encryption fails.
pub fn encrypt_envelope_with_nonce(
    plaintext: &[u8],
    key: &[u8; 32],
    nonce_bytes: [u8; 12],
) -> Result<EncryptedEnvelope, CryptoError> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| CryptoError::EncryptionFailed(format!("Invalid key length: {e}")))?;

    let nonce = Nonce::from_slice(&nonce_bytes);
    let plaintext_hash = blake3_hash(plaintext);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| CryptoError::EncryptionFailed(format!("GCM encryption failed: {e}")))?;

    Ok(EncryptedEnvelope {
        nonce: nonce_bytes,
        ciphertext,
        plaintext_blake3_hash: plaintext_hash,
    })
}

/// Decrypt an authenticated envelope and verify its cryptographic integrity.
///
/// Verifies:
/// 1. AES-256-GCM authentication tag.
/// 2. BLAKE3 plaintext digest matches pre-encryption hash in constant time.
///
/// Plaintext is returned wrapped in a `Zeroizing` buffer that wipes memory upon drop.
///
/// # Errors
/// Returns `CryptoError::DecryptionFailed` on bad tag/ciphertext.
/// Returns `CryptoError::IntegrityBreach` if plaintext hash does not match.
pub fn decrypt_envelope(
    envelope: &EncryptedEnvelope,
    key: &[u8; 32],
) -> Result<Zeroizing<Vec<u8>>, CryptoError> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| CryptoError::DecryptionFailed)?;

    let nonce = Nonce::from_slice(&envelope.nonce);

    let plaintext_raw = cipher
        .decrypt(nonce, envelope.ciphertext.as_ref())
        .map_err(|_| CryptoError::DecryptionFailed)?;

    let computed_hash = blake3_hash(&plaintext_raw);

    if !constant_time_eq_32(&computed_hash, &envelope.plaintext_blake3_hash) {
        return Err(CryptoError::IntegrityBreach);
    }

    Ok(Zeroizing::new(plaintext_raw))
}
