use log::info;
use std::error::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logging
    env_logger::init();

    info!("This is a placeholder for the TLS example.");
    info!("To use TLS with SOCKS5, make sure the TLS server is running on port 1081.");

    // For now, just use the standard client as a placeholder
    let client = socks5_rs::client::Client::new("127.0.0.1".to_string(), 1080);

    // Connect to example.com through the SOCKS5 proxy
    let mut stream = client.connect_to_domain("example.com", 80).await?;
    info!("Connected to example.com through SOCKS5 proxy");

    // Send an HTTP request
    stream
        .write_all(b"GET / HTTP/1.1\r\nHost: example.com\r\nConnection: close\r\n\r\n")
        .await?;

    // Read and print the response
    let mut buffer = [0; 1024];
    let n = stream.read(&mut buffer).await?;
    info!("Received {} bytes", n);
    println!("{}", String::from_utf8_lossy(&buffer[..n]));

    Ok(())
}
