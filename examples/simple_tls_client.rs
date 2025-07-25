use std::error::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use log::{info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logging
    env_logger::init();

    // Create a direct TLS connection to the server without full certificate validation
    let tls_connector = create_insecure_tls_connector();
    
    info!("Connecting to SOCKS5 TLS server at localhost:1081...");
    let tcp_stream = tokio::net::TcpStream::connect("127.0.0.1:1081").await?;
    
    // Convert the domain to DNS name
    let domain = rustls::ServerName::try_from("localhost")
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid domain name"))?;
    
    let mut tls_stream = tls_connector.connect(domain, tcp_stream).await?;
    info!("TLS connection established to proxy");
    
    // Perform SOCKS5 handshake directly
    // Step 1: Client greeting - version 5, 1 auth method (no auth)
    let greeting = &[0x05, 0x01, 0x00];
    tls_stream.write_all(greeting).await?;
    info!("Sent SOCKS5 greeting");
    
    // Step 2: Read server choice
    let mut response = [0u8; 2];
    tls_stream.read_exact(&mut response).await?;
    info!("Received auth method selection: {:?}", response);
    
    if response[0] != 0x05 || response[1] != 0x00 {
        return Err("Server did not accept no-auth method".into());
    }
    
    // Step 3: Send connection request to example.com:80
    let target = "example.com";
    let port = 80;
    
    let mut request = Vec::new();
    request.push(0x05); // SOCKS version
    request.push(0x01); // CONNECT command
    request.push(0x00); // Reserved
    request.push(0x03); // Domain name address type
    request.push(target.len() as u8); // Domain name length
    request.extend_from_slice(target.as_bytes()); // Domain name
    request.push((port >> 8) as u8); // Port high byte
    request.push(port as u8); // Port low byte
    
    tls_stream.write_all(&request).await?;
    info!("Sent connection request to {}:{}", target, port);
    
    // Step 4: Read connection response
    let mut header = [0u8; 4];
    tls_stream.read_exact(&mut header).await?;
    
    if header[1] != 0x00 {
        let error = match header[1] {
            0x01 => "General failure",
            0x02 => "Connection not allowed",
            0x03 => "Network unreachable",
            0x04 => "Host unreachable",
            0x05 => "Connection refused",
            0x06 => "TTL expired",
            0x07 => "Command not supported",
            0x08 => "Address type not supported",
            _ => "Unknown error",
        };
        return Err(format!("SOCKS server error: {}", error).into());
    }
    
    // Read the bound address (we don't use it but need to read it)
    match header[3] {
        0x01 => { // IPv4
            let mut addr = [0u8; 6]; // 4 bytes for IPv4 + 2 for port
            tls_stream.read_exact(&mut addr).await?;
        },
        0x03 => { // Domain name
            let len = tls_stream.read_u8().await? as usize;
            let mut domain = vec![0u8; len + 2]; // domain + port
            tls_stream.read_exact(&mut domain).await?;
        },
        0x04 => { // IPv6
            let mut addr = [0u8; 18]; // 16 bytes for IPv6 + 2 for port
            tls_stream.read_exact(&mut addr).await?;
        },
        _ => return Err("Unknown address type in response".into()),
    }
    
    info!("Connection to {}:{} established through SOCKS5 proxy", target, port);
    
    // Now we can send HTTP request
    let http_request = format!(
        "GET / HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n", 
        target
    );
    tls_stream.write_all(http_request.as_bytes()).await?;
    info!("Sent HTTP request");
    
    // Read the response
    let mut buffer = Vec::new();
    let bytes_read = tls_stream.read_to_end(&mut buffer).await?;
    info!("Received {} bytes from {}", bytes_read, target);
    
    // Print first 500 characters of response
    let preview_size = buffer.len().min(500);
    println!("Response preview:\n{}", String::from_utf8_lossy(&buffer[..preview_size]));
    
    if buffer.len() > 500 {
        println!("... (response truncated, total size: {} bytes)", buffer.len());
    }
    
    Ok(())
}

fn create_insecure_tls_connector() -> tokio_rustls::TlsConnector {
    // Use the helper function from our library
    let config = socks5_rs::create_insecure_client_config();
    tokio_rustls::TlsConnector::from(config)
}
