use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{self, AsyncWriteExt};
use log::{debug, error, info};

use crate::protocol::{
    AUTH_NONE, CMD_CONNECT, HandshakeRequest, Reply, Request, REP_ADDRESS_TYPE_NOT_SUPPORTED,
    REP_COMMAND_NOT_SUPPORTED, REP_CONNECTION_REFUSED, REP_HOST_UNREACHABLE, REP_NETWORK_UNREACHABLE,
    REP_SUCCEEDED, SOCKS_VERSION, SocksAddr, AUTH_PASSWORD, AUTH_VERSION, AUTH_SUCCESS, AUTH_FAILURE, UserPassAuth
};

pub struct Server {
    bind_addr: String,
    auth_required: bool,
    credentials: Option<Vec<(String, String)>>,
}

pub struct ServerOptions {
    pub bind_addr: String,
    pub auth_required: bool,
    pub credentials: Option<Vec<(String, String)>>, // username, password pairs
}

impl Default for ServerOptions {
    fn default() -> Self {
        ServerOptions {
            bind_addr: "127.0.0.1:1080".to_string(),
            auth_required: false,
            credentials: None,
        }
    }
}

impl Server {
    pub fn new(bind_addr: String) -> Self {
        Server { 
            bind_addr,
            auth_required: false,
            credentials: None,
        }
    }

    pub fn from_options(options: ServerOptions) -> Self {
        Server { 
            bind_addr: options.bind_addr,
            auth_required: options.auth_required,
            credentials: options.credentials,
        }
    }

    pub async fn run(&self) -> io::Result<()> {
        let listener = TcpListener::bind(&self.bind_addr).await?;
        info!("SOCKS5 server listening on {}", self.bind_addr);

        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    info!("New connection from {}", addr);
                    let auth_required = self.auth_required;
                    let credentials = self.credentials.clone();
                    
                    tokio::spawn(async move {
                        if let Err(e) = handle_client(stream, auth_required, credentials).await {
                            error!("Error handling client: {}", e);
                        }
                    });
                },
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                }
            }
        }
    }
}

async fn handle_client(
    mut stream: TcpStream,
    auth_required: bool,
    credentials: Option<Vec<(String, String)>>
) -> io::Result<()> {
    // SOCKS5 handshake
    let handshake = HandshakeRequest::read_from(&mut stream).await?;
    debug!("Received handshake with {} methods", handshake.methods.len());

    // Authentication handling
    if auth_required && handshake.methods.contains(&AUTH_PASSWORD) {
        // Send back auth choice (username/password auth)
        stream.write_all(&[SOCKS_VERSION, AUTH_PASSWORD]).await?;
        
        // Read auth data
        let auth = UserPassAuth::read_from(&mut stream).await?;
        
        // Validate credentials
        let auth_successful = if let Some(creds) = &credentials {
            creds.iter().any(|(username, password)| {
                username == &auth.username && password == &auth.password
            })
        } else {
            // No credentials specified, but auth required - deny all
            false
        };
        
        if !auth_successful {
            stream.write_all(&[AUTH_VERSION, AUTH_FAILURE]).await?;
            return Err(io::Error::new(io::ErrorKind::PermissionDenied, "Authentication failed"));
        }
        
        // Notify success
        stream.write_all(&[AUTH_VERSION, AUTH_SUCCESS]).await?;
        debug!("Authentication successful for user: {}", auth.username);
    } else if auth_required {
        // Auth required but no acceptable auth methods
        stream.write_all(&[SOCKS_VERSION, 0xFF]).await?;
        return Err(io::Error::new(io::ErrorKind::Other, "No acceptable auth methods"));
    } else if handshake.methods.contains(&AUTH_NONE) {
        // No auth required
        stream.write_all(&[SOCKS_VERSION, AUTH_NONE]).await?;
    } else {
        // No acceptable auth methods
        stream.write_all(&[SOCKS_VERSION, 0xFF]).await?;
        return Err(io::Error::new(io::ErrorKind::Other, "No acceptable auth methods"));
    }

    // Process the request
    let request = Request::read_from(&mut stream).await?;
    debug!("Received request for command {}", request.command);

    // Handle based on command
    match request.command {
        CMD_CONNECT => handle_connect(stream, request.addr).await,
        _ => {
            // Command not supported
            let reply = Reply::new(REP_COMMAND_NOT_SUPPORTED, request.addr);
            reply.write_to(&mut stream).await?;
            Err(io::Error::new(io::ErrorKind::Other, "Command not supported"))
        }
    }
}

async fn handle_connect(mut client: TcpStream, addr: SocksAddr) -> io::Result<()> {
    debug!("Connecting to {:?}", addr.to_string());
    
    // Resolve domain name if necessary
    let dest_addr = match &addr {
        SocksAddr::Domain(domain, port) => {
            match tokio::net::lookup_host(format!("{}:{}", domain, port)).await {
                Ok(mut addresses) => {
                    if let Some(addr) = addresses.next() {
                        addr
                    } else {
                        let reply = Reply::new(REP_HOST_UNREACHABLE, addr);
                        reply.write_to(&mut client).await?;
                        return Err(io::Error::new(io::ErrorKind::Other, "Could not resolve domain"));
                    }
                },
                Err(_) => {
                    let reply = Reply::new(REP_HOST_UNREACHABLE, addr.clone());
                    reply.write_to(&mut client).await?;
                    return Err(io::Error::new(io::ErrorKind::Other, "Could not resolve domain"));
                }
            }
        },
        _ => {
            // If it's an IP address, just convert it to a socket address
            if let Some(socket_addr) = addr.to_socket_addr() {
                socket_addr
            } else {
                let reply = Reply::new(REP_ADDRESS_TYPE_NOT_SUPPORTED, addr);
                reply.write_to(&mut client).await?;
                return Err(io::Error::new(io::ErrorKind::Other, "Address type not supported"));
            }
        }
    };
    
    // Connect to the destination
    match TcpStream::connect(dest_addr).await {
        Ok(mut server) => {
            // Send success reply
            let bind_addr = match server.local_addr() {
                Ok(addr) => {
                    if addr.is_ipv4() {
                        SocksAddr::Ipv4(addr.ip().to_string().parse().unwrap(), addr.port())
                    } else {
                        SocksAddr::Ipv6(addr.ip().to_string().parse().unwrap(), addr.port())
                    }
                },
                Err(_) => addr.clone(), // Fallback to original address
            };

            let reply = Reply::new(REP_SUCCEEDED, bind_addr);
            reply.write_to(&mut client).await?;

            // Proxy data between client and server
            match io::copy_bidirectional(&mut client, &mut server).await {
                Ok((bytes_to_server, bytes_to_client)) => {
                    debug!("Connection closed: {} bytes sent, {} bytes received", 
                          bytes_to_server, bytes_to_client);
                    Ok(())
                },
                Err(e) => {
                    error!("Error during data transfer: {}", e);
                    Err(e)
                }
            }
        },
        Err(e) => {
            let reply_code = match e.kind() {
                io::ErrorKind::ConnectionRefused => REP_CONNECTION_REFUSED,
                io::ErrorKind::NetworkUnreachable => REP_NETWORK_UNREACHABLE,
                io::ErrorKind::HostUnreachable => REP_HOST_UNREACHABLE,
                _ => REP_NETWORK_UNREACHABLE,
            };
            
            let reply = Reply::new(reply_code, addr);
            reply.write_to(&mut client).await?;
            Err(e)
        }
    }
}