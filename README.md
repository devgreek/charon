# Charon

A SOCKS5 protocol implementation in Rust.

This library implements the SOCKS5 protocol (RFC 1928) with both client and server components. 
It uses Tokio for asynchronous I/O operations.

## Features

- Full SOCKS5 protocol support
- Client and server implementations
- IPv4, IPv6, and domain name resolution
- Username/password authentication (RFC 1929)
- TLS encryption between client and proxy
- Asynchronous I/O with Tokio

## Usage

### Running the SOCKS5 server

```rust
use socks5_rs::server::{Server, ServerOptions};

#[tokio::main]
async fn main() {
    // Initialize the logger
    env_logger::init();
    
    // Create server with default options (localhost:1080)
    let server_options = ServerOptions::default();
    let server = Server::from_options(server_options);
    
    // Run the server
    if let Err(e) = server.run().await {
        eprintln!("Server error: {}", e);
    }
}
```

### Using the SOCKS5 client

```rust
use socks5_rs::client::Client;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new("127.0.0.1".to_string(), 1080);
    
    // Connect to a website through the SOCKS5 proxy
    let mut stream = client.connect_to_domain("example.com", 80).await?;

    // Use the stream for communication
    // ...

    Ok(())
}
```

## TLS Support

### Running a TLS-secured SOCKS5 server

```rust
use socks5_rs::tls::{TlsServer, TlsServerOptions, generate_self_signed_cert};
use socks5_rs::server::ServerOptions;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();
    
    // Generate certificate if needed (for testing only)
    if !std::path::Path::new("cert.pem").exists() {
        generate_self_signed_cert()?;
    }
    
    // Create server options
    let server_options = ServerOptions::default();
    
    // Create TLS server options
    let tls_options = TlsServerOptions {
        server_options,
        cert_path: "cert.pem".to_string(),
        key_path: "key.pem".to_string(),
    };
    
    // Create and run TLS server
    let server = TlsServer::new(tls_options)?;
    server.run("127.0.0.1:1081").await?;
    
    Ok(())
}
```

### Using a TLS-secured SOCKS5 client

```rust
use socks5_rs::tls_client::TlsClient;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();
    
    // Create a TLS client
    let client = TlsClient::new("localhost".to_string(), 1081);
    
    // Connect to a website through the TLS-secured SOCKS5 proxy
    let mut stream = client.connect_to_domain("example.com", 80).await?;
    
    // Use the stream for communication
    // ...
    
    Ok(())
}
```

## Example

See the `examples/simple_client.rs` file for a complete example of a client that makes an HTTP request through a SOCKS5 proxy.

## Running the examples

Start the server:

```bash
RUST_LOG=debug cargo run
```

In another terminal, run the client example:

```bash
RUST_LOG=debug cargo run --example simple_client
```

## License

MIT
