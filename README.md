# socks5-rs

A SOCKS5 protocol implementation in Rust.

This library implements the SOCKS5 protocol (RFC 1928) with both client and server components. 
It uses Tokio for asynchronous I/O operations.

## Features

- Full SOCKS5 protocol support
- Client and server implementations
- IPv4, IPv6, and domain name resolution
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
