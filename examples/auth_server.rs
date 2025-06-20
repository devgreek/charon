use socks5_rs::server::{Server, ServerOptions};
use log::error;

#[tokio::main]
async fn main() {
    // Initialize the logger
    env_logger::init();
    
    // Create server with user authentication
    let mut server_options = ServerOptions::default();
    
    // Enable authentication and add some credentials
    server_options.auth_required = true;
    server_options.credentials = Some(vec![
        ("user1".to_string(), "password1".to_string()),
        ("user2".to_string(), "password2".to_string()),
    ]);
    
    let server = Server::from_options(server_options);
    
    // Run the server
    if let Err(e) = server.run().await {
        error!("Server error: {}", e);
    }
}
