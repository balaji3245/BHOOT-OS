pub mod config;
pub mod error;
pub mod schema;

pub use config::SqlCipherConfig;
pub use error::DbError;
pub use schema::MIGRATION_001_SQL;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sqlcipher_config_pragma_generation() {
        let config = SqlCipherConfig::default();
        let key = [0xabu8; 32];
        let pragmas = config.generate_pragmas(&key);

        assert_eq!(pragmas.len(), 7);
        assert!(pragmas[0].starts_with("PRAGMA key = \"x'abab"));
        assert!(pragmas[1].contains("cipher_page_size = 4096"));
        assert!(pragmas[2].contains("kdf_iter = 256000"));
        assert!(pragmas[5].contains("foreign_keys = ON"));
        assert!(pragmas[6].contains("journal_mode = WAL"));
    }

    #[test]
    fn test_migration_sql_embedded_validity() {
        assert!(MIGRATION_001_SQL.contains("CREATE TABLE IF NOT EXISTS accounts"));
        assert!(MIGRATION_001_SQL.contains("CREATE TABLE IF NOT EXISTS transactions"));
        assert!(MIGRATION_001_SQL.contains("CREATE TABLE IF NOT EXISTS postings"));
        assert!(MIGRATION_001_SQL.contains("CREATE TABLE IF NOT EXISTS trades"));
        assert!(MIGRATION_001_SQL.contains("CREATE TABLE IF NOT EXISTS payouts"));
        assert!(MIGRATION_001_SQL.contains("CREATE TABLE IF NOT EXISTS audit_events"));
        assert!(MIGRATION_001_SQL.contains("CREATE TABLE IF NOT EXISTS vault_documents"));
    }
}
