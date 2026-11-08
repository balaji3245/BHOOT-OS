//! BHOOT OS Desktop Embedded Server
//!
//! Serves the airgapped React/TypeScript UI bundle and provides local REST APIs
//! connected to the underlying SQLCipher encrypted database.

pub mod db_key;
pub mod server;

pub use db_key::{decode_hex_32, resolve_or_provision_database_key, DbKeyError, WrappedDbKeyStore};
pub use server::{handle_request, run_server, seed_starter_data, AppSettings, ServerState};
