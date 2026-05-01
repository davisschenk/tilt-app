//! Library facade for the server crate. Exposes modules that other binaries
//! (`seed`) and integration tests need to reuse.

pub mod auth_mode;
pub mod models;
pub mod seed;
