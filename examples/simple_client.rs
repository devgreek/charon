use socks5_rs::client::Client;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the logger
    env_logger::init();

    // Create a new SOCKS5 client
    let client = Client::new("127.0.0.1".to_string(), 1080);
    
    // Connect to example.com through the SOCKS5 proxy
    let mut stream = client.connect_to_domain("example.com", 80).await?;

    // Send an HTTP request
    let request = b"GET / HTTP/1.1\r\nHost: example.com\r\nConnection: close\r\n\r\n";
    stream.write_all(request).await?;

    // Read and print the response
    let mut buffer = Vec::new();
    stream.read_to_end(&mut buffer).await?;
    println!("Response from example.com:\n{}", String::from_utf8_lossy(&buffer));

    Ok(())
}