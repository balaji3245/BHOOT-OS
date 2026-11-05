pub mod cipher;
pub mod error;
pub mod hash;
pub mod hkdf;
pub mod kdf;

pub use cipher::{
    decrypt_envelope, encrypt_envelope, encrypt_envelope_with_nonce, generate_nonce,
    EncryptedEnvelope,
};
pub use error::CryptoError;
pub use hash::{blake3_hash, constant_time_eq_32, sha256_hash};
pub use hkdf::{derive_subkeys, SubKeys};
pub use kdf::{derive_master_key, generate_salt, Argon2idParams};

#[cfg(test)]
mod tests {
    use super::*;
    use ::hkdf::Hkdf;
    use sha2::Sha256;
    use std::collections::HashSet;

    #[test]
    fn test_nist_sp800_38d_aes256_gcm_test_case_13() {
        // NIST SP 800-38D Test Case 13 for AES-256-GCM:
        // Key: 32 bytes 0x00
        // IV: 12 bytes 0x00
        // Plaintext: empty (0 bytes)
        // Expected Tag: 530f8afbc74536b9a963b4f1c4cb738b
        let key = [0u8; 32];
        let nonce = [0u8; 12];
        let plaintext = b"";

        let envelope =
            encrypt_envelope_with_nonce(plaintext, &key, nonce).expect("encryption succeeds");

        let expected_tag = vec![
            0x53, 0x0f, 0x8a, 0xfb, 0xc7, 0x45, 0x36, 0xb9, 0xa9, 0x63, 0xb4, 0xf1, 0xc4, 0xcb,
            0x73, 0x8b,
        ];

        assert_eq!(envelope.ciphertext, expected_tag);

        // Decryption roundtrip
        let decrypted = decrypt_envelope(&envelope, &key).expect("decryption succeeds");
        assert_eq!(decrypted.as_slice(), plaintext);
    }

    #[test]
    fn test_rfc5869_hkdf_sha256_test_case_1() {
        // RFC 5869 Test Case 1:
        // IKM = 0x0b repeated 22 times
        // Salt = 0x000102030405060708090a0b0c (13 bytes)
        // info = 0xf0f1f2f3f4f5f6f7f8f9 (10 bytes)
        // L = 42 bytes
        let ikm = [0x0bu8; 22];
        let salt = [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c,
        ];
        let info = [0xf0, 0xf1, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8, 0xf9];

        let hk = Hkdf::<Sha256>::new(Some(&salt), &ikm);
        let mut okm = [0u8; 42];
        hk.expand(&info, &mut okm).expect("HKDF expansion succeeds");

        let expected_okm = [
            0x3c, 0xb2, 0x5f, 0x25, 0xfa, 0xac, 0xd5, 0x7a, 0x90, 0x43, 0x4f, 0x64, 0xd0, 0x36,
            0x2f, 0x2a, 0x2d, 0x2d, 0x0a, 0x90, 0xcf, 0x1a, 0x5a, 0x4c, 0x5d, 0xb0, 0x2d, 0x56,
            0xec, 0xc4, 0xc5, 0xbf, 0x34, 0x00, 0x72, 0x08, 0xd5, 0xb8, 0x87, 0x18, 0x58, 0x65,
        ];

        assert_eq!(okm, expected_okm);
    }

    #[test]
    fn test_argon2id_kdf_and_subkey_separation() {
        let password = b"bhoot-master-super-secret-passphrase-2026!";
        let salt = [0x42u8; 32];
        // Test parameters calibrated for unit test speed
        let test_params = Argon2idParams {
            m_cost: 4096, // 4 MiB in test mode
            t_cost: 2,
            p_cost: 1,
        };

        let master_key =
            derive_master_key(password, &salt, test_params).expect("derive master key");
        assert_ne!(*master_key, [0u8; 32]);

        let subkeys = derive_subkeys(&master_key).expect("derive subkeys");

        // Subkeys must be strictly distinct from each other and from the master key
        assert_ne!(*subkeys.db_key, *master_key);
        assert_ne!(*subkeys.vault_key, *master_key);
        assert_ne!(*subkeys.audit_key, *master_key);
        assert_ne!(*subkeys.db_key, *subkeys.vault_key);
        assert_ne!(*subkeys.db_key, *subkeys.audit_key);
        assert_ne!(*subkeys.vault_key, *subkeys.audit_key);
    }

    #[test]
    fn test_envelope_encryption_tampering_detection() {
        let key = [0x55u8; 32];
        let document = b"CONFIDENTIAL FINANCIAL RECORD: BANK STATEMENT HDFC INR 5,00,000.00";

        let mut envelope = encrypt_envelope(document, &key).expect("encrypt");

        // Legitimate decryption
        let decrypted = decrypt_envelope(&envelope, &key).expect("decrypt");
        assert_eq!(decrypted.as_slice(), document);

        // Tamper with 1 byte in ciphertext -> Must fail with DecryptionFailed
        envelope.ciphertext[0] ^= 0x01;
        assert_eq!(
            decrypt_envelope(&envelope, &key),
            Err(CryptoError::DecryptionFailed)
        );

        // Restore ciphertext, but tamper with pre-encryption BLAKE3 hash -> Must fail with IntegrityBreach
        envelope.ciphertext[0] ^= 0x01; // restored
        envelope.plaintext_blake3_hash[0] ^= 0x01; // corrupted hash
        assert_eq!(
            decrypt_envelope(&envelope, &key),
            Err(CryptoError::IntegrityBreach)
        );
    }

    #[test]
    fn test_nonce_uniqueness_property() {
        let mut nonces = HashSet::new();
        for _ in 0..1000 {
            let n = generate_nonce();
            assert!(nonces.insert(n), "CRITICAL: Nonce collision detected!");
        }
    }
}
