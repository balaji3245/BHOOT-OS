use crate::error::CryptoError;
use argon2::{Algorithm, Argon2, Params, Version};
use rand::rngs::OsRng;
use rand::RngCore;
use zeroize::Zeroizing;

/// Recommended security parameters for Argon2id according to RFC 9106.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Argon2idParams {
    /// Memory cost in KiB (Default: 128 MiB = 131,072 KiB).
    pub m_cost: u32,
    /// Number of passes / iterations (Default: 4).
    pub t_cost: u32,
    /// Parallelism threads (Default: 4 lanes).
    pub p_cost: u32,
}

impl Default for Argon2idParams {
    fn default() -> Self {
        Self {
            m_cost: 131_072, // 128 MiB
            t_cost: 4,       // 4 passes
            p_cost: 4,       // 4 parallel threads
        }
    }
}

impl Argon2idParams {
    /// Validate security bounds according to RFC 9106.
    ///
    /// # Errors
    /// Returns `CryptoError::KeyDerivationFailed` if parameters fall below RFC 9106 minimums.
    pub fn validate(&self) -> Result<(), CryptoError> {
        if self.m_cost < 8 * self.p_cost {
            return Err(CryptoError::KeyDerivationFailed(
                "Memory cost must be at least 8 * p_cost KiB".into(),
            ));
        }
        if self.t_cost < 1 {
            return Err(CryptoError::KeyDerivationFailed(
                "Time cost must be at least 1 iteration".into(),
            ));
        }
        if self.p_cost < 1 {
            return Err(CryptoError::KeyDerivationFailed(
                "Parallelism must be at least 1 lane".into(),
            ));
        }
        Ok(())
    }
}

/// Generate a 32-byte cryptographically secure random salt.
#[must_use]
pub fn generate_salt() -> [u8; 32] {
    let mut salt = [0u8; 32];
    OsRng.fill_bytes(&mut salt);
    salt
}

/// Derive a 256-bit (32-byte) Master Key from a user passphrase using Argon2id.
///
/// The returned key is wrapped in a `Zeroizing` guard to automatically scrub memory on drop.
///
/// # Errors
/// Returns `CryptoError::KeyDerivationFailed` if Argon2 parameters are invalid or hashing fails.
pub fn derive_master_key(
    passphrase: &[u8],
    salt: &[u8],
    params: Argon2idParams,
) -> Result<Zeroizing<[u8; 32]>, CryptoError> {
    params.validate()?;

    if salt.len() < 16 {
        return Err(CryptoError::KeyDerivationFailed(
            "Salt must be at least 16 bytes for Argon2id".into(),
        ));
    }

    let argon2_params = Params::new(params.m_cost, params.t_cost, params.p_cost, Some(32))
        .map_err(|e| CryptoError::KeyDerivationFailed(format!("Invalid Argon2 params: {e}")))?;

    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, argon2_params);

    let mut key = Zeroizing::new([0u8; 32]);
    argon2
        .hash_password_into(passphrase, salt, key.as_mut())
        .map_err(|e| CryptoError::KeyDerivationFailed(format!("Argon2 computation error: {e}")))?;

    Ok(key)
}
