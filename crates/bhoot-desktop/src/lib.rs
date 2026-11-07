//! BHOOT OS Desktop Embedded Server
//!
//! Serves the airgapped React/TypeScript UI bundle and provides local REST APIs
//! connected to the underlying SQLCipher encrypted database.

pub mod server;

pub use server::{handle_request, run_server, seed_starter_data, AppSettings, ServerState};
