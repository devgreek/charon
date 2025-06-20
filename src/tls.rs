use std::fs::File;
use std::io::{self, BufReader};
use std::path::Path;
use std::sync::Arc;

use log::{debug, error, info};
use rustls::{Certificate, PrivateKey, ServerConfig};
use rustls_pemfile::{certs, pkcs8_private_keys};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

use crate::server::{Server, ServerOptions};

pub struct TlsServerOptions {
    pub server_options: ServerOptions,
    pub cert_path: String,
    pub key_path: String,
}

impl Default for TlsServerOptions {
    fn default() -> Self {
        TlsServerOptions {
            server_options: ServerOptions::default(),
            cert_path: "cert.pem".to_string(),
            key_path: "key.pem".to_string(),
        }
    }
}

pub struct TlsServer {
    server: Server,
    tls_config: Arc<ServerConfig>,
}

impl TlsServer {
    pub fn new(options: TlsServerOptions) -> io::Result<Self> {
        let server = Server::from_options(options.server_options);
        let tls_config = load_tls_config(&options.cert_path, &options.key_path)?;
        
        Ok(TlsServer {
            server,
            tls_config,
        })
    }

    pub async fn run(&self, bind_addr: &str) -> io::Result<()> {
        let listener = TcpListener::bind(bind_addr).await?;
        info!("SOCKS5 TLS server listening on {}", bind_addr);
        
        let acceptor = TlsAcceptor::from(Arc::clone(&self.tls_config));
        
        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    info!("Accepted connection from: {}", addr);
                    let acceptor = acceptor.clone();
                    let server = self.server.clone();
                    
                    tokio::spawn(async move {
                        match acceptor.accept(stream).await {
                            Ok(tls_stream) => {
                                if let Err(e) = server.handle_client(tls_stream).await {
                                    error!("Error handling TLS client: {}", e);
                                }
                            },
                            Err(e) => {
                                error!("TLS handshake failed: {}", e);
                            }
                        }
                    });
                },
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                }
            }
        }
    }
}

// Load certificates and private key from files
fn load_tls_config(cert_path: &str, key_path: &str) -> io::Result<Arc<ServerConfig>> {
    // Load certificates
    let cert_file = File::open(cert_path)?;
    let mut cert_reader = BufReader::new(cert_file);
    let certs = certs(&mut cert_reader)?
        .into_iter()
        .map(Certificate)
        .collect();

    // Load private key
    let key_file = File::open(key_path)?;
    let mut key_reader = BufReader::new(key_file);
    let keys = pkcs8_private_keys(&mut key_reader)?
        .into_iter()
        .map(PrivateKey)
        .collect::<Vec<_>>();

    if keys.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "No private keys found in file",
        ));
    }

    // Configure server
    let mut config = ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(certs, keys[0].clone())
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

    // Configure ALPN protocols if needed
    config.alpn_protocols = vec![b"http/1.1".to_vec()];

    Ok(Arc::new(config))
}

// Helper function to generate test certificates
#[allow(dead_code)]
pub fn generate_self_signed_cert() -> io::Result<()> {
    use std::process::Command;
    
    println!("Generating self-signed certificate...");
    
    let status = Command::new("openssl")
        .args(&[
            "req", "-x509", 
            "-newkey", "rsa:4096", 
            "-keyout", "key.pem", 
            "-out", "cert.pem", 
            "-days", "365", 
            "-nodes",
            "-subj", "/CN=localhost"
        ])
        .status()?;
    
    if !status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other, 
            "Failed to generate certificate"
        ));
    }
    
    println!("Certificate generated: cert.pem");
    println!("Key generated: key.pem");
    
    Ok(())
}