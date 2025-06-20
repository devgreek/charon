use std::env;
use socks5_rs::client::Client;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the logger
    env_logger::init();

    // Get command line arguments
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 3 {
        eprintln!("Usage: {} <host> <port>", args[0]);
        eprintln!("Example: {} example.com 80", args[0]);
        return Ok(());
    }
    
    let host = &args[1];
    let port: u16 = args[2].parse()?;
    
    println!("Connecting to {}:{} through SOCKS5 proxy at 127.0.0.1:1080", host, port);

    // Create a new SOCKS5 client
    let client = Client::new("127.0.0.1".to_string(), 1080);
    
    // Connect to the specified host through the SOCKS5 proxy
    let mut stream = client.connect_to_domain(host, port).await?;

    // For HTTP, send a basic GET request
    if port == 80 || port == 8080 || port == 443 {
        println!("Sending HTTP GET request to {}", host);
        
        // Send an HTTP request
        let request = format!("GET / HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n", host);
        stream.write_all(request.as_bytes()).await?;

        // Read and print the response
        let mut buffer = Vec::new();
        let read_bytes = stream.read_to_end(&mut buffer).await?;
        
        println!("Received {} bytes in response:", read_bytes);
        
        // Try to print as string if it looks like text
        if let Ok(response_text) = String::from_utf8(buffer.clone()) {
            // Only print first 1000 characters to avoid flooding the console
            let preview_len = response_text.len().min(1000);
            println!("{}", &response_text[..preview_len]);
            if response_text.len() > 1000 {
                println!("... (response truncated)");
            }
        } else {
            println!("Response contains binary data");
        }
    } else {
        println!("Connected successfully to {}:{} through SOCKS5 proxy", host, port);
        println!("This example only supports HTTP protocol on port 80, 8080, or 443");
        println!("Connection established but no data sent");
    }

    Ok(())
}
