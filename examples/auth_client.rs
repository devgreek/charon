use socks5_rs::client::Client;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the logger
    env_logger::init();

    // Create a new SOCKS5 client with authentication
    let client = Client::with_auth(
        "127.0.0.1".to_string(),
        1080,
        "user1".to_string(),
        "password1".to_string(),
    );

    // Connect to example.com through the SOCKS5 proxy with authentication
    let mut stream = client.connect_to_domain("example.com", 80).await?;

    // Send an HTTP request
    let request = b"GET / HTTP/1.1\r\nHost: example.com\r\nConnection: close\r\n\r\n";
    stream.write_all(request).await?;

    // Read and print the response
    let mut buffer = Vec::new();
    stream.read_to_end(&mut buffer).await?;

    // Only print the first 1000 characters to avoid flooding the console
    let response = String::from_utf8_lossy(&buffer);
    let preview_len = response.len().min(1000);
    println!("Response from example.com:\n{}", &response[..preview_len]);
    if response.len() > 1000 {
        println!("... (response truncated)");
    }

    Ok(())
}
