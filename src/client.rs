use log::{debug, error};
use std::io;
use std::net::{SocketAddr, ToSocketAddrs};
use tokio::io::{AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::protocol::{
    AUTH_NONE, CMD_CONNECT, REP_GENERAL_FAILURE, REP_SUCCEEDED, SOCKS_VERSION, SocksAddr,
};

pub struct Client {
    proxy_addr: String,
    proxy_port: u16,
    auth: Option<(String, String)>, // Optional username and password for authentication
}

impl Client {
    pub fn new(proxy_addr: String, proxy_port: u16) -> Self {
        Client {
            proxy_addr,
            proxy_port,
            auth: None,
        }
    }

    pub fn with_auth(
        proxy_addr: String,
        proxy_port: u16,
        username: String,
        password: String,
    ) -> Self {
        Client {
            proxy_addr,
            proxy_port,
            auth: Some((username, password)),
        }
    }

    pub async fn connect_to_target<A: ToSocketAddrs>(
        &self,
        target_addr: A,
    ) -> io::Result<TcpStream> {
        let target = target_addr
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Could not resolve address"))?;

        // Convert to SocksAddr
        let socks_addr = match target {
            SocketAddr::V4(addr) => SocksAddr::Ipv4(*addr.ip(), addr.port()),
            SocketAddr::V6(addr) => SocksAddr::Ipv6(*addr.ip(), addr.port()),
        };

        self.connect(socks_addr).await
    }

    pub async fn connect_to_domain(&self, domain: &str, port: u16) -> io::Result<TcpStream> {
        let socks_addr = SocksAddr::Domain(domain.to_string(), port);
        self.connect(socks_addr).await
    }

    async fn connect(&self, target: SocksAddr) -> io::Result<TcpStream> {
        // Connect to the SOCKS5 proxy
        let mut stream =
            TcpStream::connect(format!("{}:{}", self.proxy_addr, self.proxy_port)).await?;
        debug!(
            "Connected to SOCKS5 proxy {}:{}",
            self.proxy_addr, self.proxy_port
        );

        // Handshake with the proxy
        self.handshake(&mut stream).await?;

        // Request connection to the target
        self.request_connection(&mut stream, target).await?;

        Ok(stream)
    }

    // Make handshake method public for TLS client
    pub async fn handshake<T>(&self, stream: &mut T) -> io::Result<()>
    where
        T: AsyncReadExt + AsyncWriteExt + Unpin,
    {
        use crate::protocol::{AUTH_PASSWORD, AUTH_SUCCESS, AUTH_VERSION, UserPassAuth};

        // Send client greeting with appropriate auth methods
        let buf = if self.auth.is_some() {
            vec![SOCKS_VERSION, 2, AUTH_NONE, AUTH_PASSWORD] // Support both no-auth and username/password
        } else {
            vec![SOCKS_VERSION, 1, AUTH_NONE] // Only support no-auth
        };

        stream.write_all(&buf).await?;
        debug!("Sent handshake request");

        // Read server choice
        let mut response = [0u8; 2];
        stream.read_exact(&mut response).await?;

        if response[0] != SOCKS_VERSION {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Invalid SOCKS version from proxy",
            ));
        }

        match response[1] {
            AUTH_NONE => {
                debug!("Handshake successful: no authentication required");
                Ok(())
            }
            AUTH_PASSWORD => {
                if let Some((username, password)) = &self.auth {
                    debug!("Server requested username/password authentication");

                    // Send username/password auth
                    let auth = UserPassAuth::new(username.clone(), password.clone());
                    auth.write_to(stream).await?;

                    // Read auth response
                    let mut auth_response = [0u8; 2];
                    stream.read_exact(&mut auth_response).await?;

                    if auth_response[0] != AUTH_VERSION {
                        return Err(io::Error::new(
                            io::ErrorKind::Other,
                            "Invalid auth protocol version",
                        ));
                    }

                    if auth_response[1] == AUTH_SUCCESS {
                        debug!("Authentication successful");
                        Ok(())
                    } else {
                        error!("Authentication failed");
                        Err(io::Error::new(
                            io::ErrorKind::PermissionDenied,
                            "Authentication failed",
                        ))
                    }
                } else {
                    error!("Server requested auth but no credentials provided");
                    Err(io::Error::new(
                        io::ErrorKind::Other,
                        "Server requested auth but no credentials provided",
                    ))
                }
            }
            0xFF => {
                error!("No acceptable authentication methods");
                Err(io::Error::new(
                    io::ErrorKind::Other,
                    "No acceptable authentication methods",
                ))
            }
            _ => {
                error!("Unknown authentication method: {}", response[1]);
                Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("Unknown authentication method: {}", response[1]),
                ))
            }
        }
    }

    // Make request_connection method public for TLS client
    pub async fn request_connection<T>(&self, stream: &mut T, addr: SocksAddr) -> io::Result<()>
    where
        T: AsyncReadExt + AsyncWrite + Unpin,
    {
        // Build and send connect request
        stream.write_u8(SOCKS_VERSION).await?;
        stream.write_u8(CMD_CONNECT).await?;
        stream.write_u8(0x00).await?; // Reserved

        // Write destination address
        addr.write_to(stream).await?;
        stream.flush().await?;
        debug!("Sent connect request to {}", addr);

        // Read response
        let version = stream.read_u8().await?;
        let status = stream.read_u8().await?;
        let _reserved = stream.read_u8().await?;

        if version != SOCKS_VERSION {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Invalid protocol version in response",
            ));
        }

        if status != REP_SUCCEEDED {
            let error_msg = match status {
                REP_GENERAL_FAILURE => "General failure",
                2 => "Network unreachable",
                3 => "Host unreachable",
                4 => "Connection refused by destination",
                5 => "TTL expired",
                6 => "Command not supported / protocol error",
                7 => "Address type not supported",
                _ => "Unknown error",
            };
            error!("Connection request failed: {}", error_msg);
            return Err(io::Error::new(io::ErrorKind::Other, error_msg));
        }

        // Skip the bound address in the response
        let addr_type = stream.read_u8().await?;
        match addr_type {
            1 => {
                // IPv4
                let mut _ipv4 = [0u8; 4];
                stream.read_exact(&mut _ipv4).await?;
                let _port = stream.read_u16().await?;
            }
            3 => {
                // Domain name
                let len = stream.read_u8().await?;
                let mut _domain = vec![0u8; len as usize];
                stream.read_exact(&mut _domain).await?;
                let _port = stream.read_u16().await?;
            }
            4 => {
                // IPv6
                let mut _ipv6 = [0u8; 16];
                stream.read_exact(&mut _ipv6).await?;
                let _port = stream.read_u16().await?;
            }
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "Invalid address type in response",
                ));
            }
        }

        debug!("Connection established through proxy");
        Ok(())
    }
}
