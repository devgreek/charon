use log::{debug, info};
use socks5_rs::tls_client::TlsClient;
use std::error::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logging
    env_logger::init();

    // Create a TLS client that connects to our secure SOCKS5 proxy
    // Note: Using localhost:1081 (the TLS server port)
    let client = TlsClient::new("localhost".to_string(), 1081);

    // Connect to example.com through the TLS-secured SOCKS5 proxy
    info!("Connecting to example.com via TLS-secured SOCKS5 proxy...");
    let mut stream = client.connect_to_domain("example.com", 80).await?;
    info!("Connected to example.com through TLS-secured SOCKS5 proxy");

    // Send an HTTP request
    info!("Sending HTTP request to example.com");
    stream
        .write_all(b"GET / HTTP/1.0\r\nHost: example.com\r\nConnection: close\r\n\r\n")
        .await?;

    // Read and print the response
    let mut buffer = Vec::new();
    let bytes_read = stream.read_to_end(&mut buffer).await?;

    info!("Received {} bytes from example.com", bytes_read);

    // Print first 500 characters of response
    let preview_size = buffer.len().min(500);
    println!(
        "Response preview:\n{}",
        String::from_utf8_lossy(&buffer[..preview_size])
    );

    if buffer.len() > 500 {
        println!(
            "... (response truncated, total size: {} bytes)",
            buffer.len()
        );
    }

    Ok(())
}
