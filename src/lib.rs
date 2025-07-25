//! A SOCKS5 protocol implementation in Rust
//!
//! This crate provides implementation of SOCKS5 proxy protocol (RFC 1928)
//! with both client and server components, including TLS support.

pub mod client;
pub mod protocol;
pub mod server;
pub mod tls;
pub mod tls_client;

// Re-exports
pub use crate::client::Client;
pub use crate::protocol::SocksAddr;
pub use crate::server::Server;
pub use crate::tls::TlsServer;
pub use crate::tls::generate_self_signed_cert;
pub use crate::tls_client::TlsClient;

// Helper functions
pub fn create_insecure_client_config() -> std::sync::Arc<rustls::ClientConfig> {
    use std::sync::Arc;
    use rustls::ClientConfig;
    
    // Create a configuration that accepts all certificates (DANGEROUS!)
    let mut config = ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(rustls::RootCertStore::empty())
        .with_no_client_auth();
        
    // Disable certificate verification
    // Only for development/testing - NOT for production!
    // config.dangerous().set_certificate_verifier(Arc::new(
    //     rustls::dangerous_configuration::NoCertificateVerifier {}
    // ));
    
    Arc::new(config)
}
