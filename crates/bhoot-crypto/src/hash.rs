use sha2::{Digest, Sha256};

/// Compute the 256-bit BLAKE3 cryptographic hash of a byte slice.
#[must_use]
pub fn blake3_hash(data: &[u8]) -> [u8; 32] {
    let hash = blake3::hash(data);
    *hash.as_bytes()
}

/// Compute the 256-bit SHA-256 cryptographic hash of a byte slice.
#[must_use]
pub fn sha256_hash(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&result);
    out
}

/// Constant-time comparison of two 32-byte hashes to prevent timing attacks.
#[must_use]
pub fn constant_time_eq_32(a: &[u8; 32], b: &[u8; 32]) -> bool {
    let mut diff = 0u8;
    for i in 0..32 {
        diff |= a[i] ^ b[i];
    }
    diff == 0
}
