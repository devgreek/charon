use log::{debug, info};
use socks5_rs::tls_client::TlsClient;
use std::error::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logging
    env_logger::init();

    // Create a TLS client with authentication
    let client = TlsClient::with_auth(
        "localhost".to_string(),
        1081,
        "user1".to_string(),
        "password1".to_string(),
    );

    info!("Connecting to example.com via authenticated TLS SOCKS5 proxy...");

    // Connect to example.com through the authenticated TLS SOCKS5 proxy
    let mut stream = client.connect_to_domain("example.com", 80).await?;
    info!("Connected to example.com through authenticated TLS SOCKS5 proxy");

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
