use log::error;
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
        error!("Server error: {}", e);
    }
}
