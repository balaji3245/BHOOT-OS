use serde::{Deserialize, Serialize};

/// SQLCipher security and performance configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SqlCipherConfig {
    pub cipher_page_size: u32,
    pub kdf_iter: u32,
    pub foreign_keys: bool,
    pub journal_mode: String,
}

impl Default for SqlCipherConfig {
    fn default() -> Self {
        Self {
            cipher_page_size: 4096,
            kdf_iter: 256_000,
            foreign_keys: true,
            journal_mode: "WAL".to_string(),
        }
    }
}

impl SqlCipherConfig {
    /// Generate standard SQLCipher PRAGMA statements for key attachment and hardening.
    #[must_use]
    pub fn generate_pragmas(&self, raw_key_32: &[u8; 32]) -> Vec<String> {
        let hex_key: String = raw_key_32.iter().map(|b| format!("{b:02x}")).collect();
        vec![
            format!("PRAGMA key = \"x'{hex_key}'\";"),
            format!("PRAGMA cipher_page_size = {};", self.cipher_page_size),
            format!("PRAGMA kdf_iter = {};", self.kdf_iter),
            "PRAGMA cipher_hmac_algorithm = HMAC_SHA512;".to_string(),
            "PRAGMA cipher_default_kdf_algorithm = PBKDF2_HMAC_SHA512;".to_string(),
            format!(
                "PRAGMA foreign_keys = {};",
                if self.foreign_keys { "ON" } else { "OFF" }
            ),
            format!("PRAGMA journal_mode = {};", self.journal_mode),
        ]
    }
}
