//! A SOCKS5 protocol implementation in Rust
//!
//! This crate provides implementation of SOCKS5 proxy protocol (RFC 1928)
//! with both client and server components.

pub mod protocol;
pub mod server;
pub mod client;

// Re-exports
pub use crate::protocol::SocksAddr;
pub use crate::client::Client;
pub use crate::server::Server;