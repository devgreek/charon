use anyhow::Result;
use log::{error, info};
use socks5_rs::server::ServerOptions;
use socks5_rs::tls::{TlsServer, TlsServerOptions, generate_self_signed_cert};
use std::error::Error;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();

    // Generate certificate if it doesn't exist
    if !std::path::Path::new("cert.pem").exists() || !std::path::Path::new("key.pem").exists() {
        info!("Generating self-signed certificate...");
        generate_self_signed_cert()?;
    }

    // Create server options with custom bind address for TLS
    let mut server_options = ServerOptions::default();
    server_options.bind_addr = "127.0.0.1:1081".to_string(); // Use a different port for TLS

    // Create TLS server options
    let tls_options = TlsServerOptions {
        server_options,
        cert_path: "cert.pem".to_string(),
        key_path: "key.pem".to_string(),
    };

    // Create TLS server
    let server = match TlsServer::new(tls_options) {
        Ok(server) => server,
        Err(e) => {
            error!("Failed to create TLS server: {}", e);
            return Err(anyhow::Error::from(e));
        }
    };

    info!("Starting SOCKS5 TLS server on 127.0.0.1:1081");

    // Run the server
    match server.run("127.0.0.1:1081").await {
        Ok(_) => Ok(()),
        Err(e) => {
            error!("Server error: {}", e);
            Err(anyhow::Error::from(e))
        }
    }
}
