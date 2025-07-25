// This file defines the SOCKS5 protocol specifications, including request and response formats.
// It exports constants and types used for protocol handling.

use std::fmt;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};
use tokio::io::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

// SOCKS protocol version
pub const SOCKS_VERSION: u8 = 5;

// SOCKS command codes
pub const CMD_CONNECT: u8 = 1;
pub const CMD_BIND: u8 = 2;
pub const CMD_UDP_ASSOCIATE: u8 = 3;

// SOCKS address types
pub const ATYP_IPV4: u8 = 1;
pub const ATYP_DOMAIN: u8 = 3;
pub const ATYP_IPV6: u8 = 4;

// SOCKS authentication methods
pub const AUTH_NONE: u8 = 0x00;
pub const AUTH_GSSAPI: u8 = 0x01;
pub const AUTH_PASSWORD: u8 = 0x02;
pub const AUTH_NOT_ACCEPTABLE: u8 = 0xFF;

// SOCKS reply codes
pub const REP_SUCCEEDED: u8 = 0;
pub const REP_GENERAL_FAILURE: u8 = 1;
pub const REP_CONNECTION_NOT_ALLOWED: u8 = 2;
pub const REP_NETWORK_UNREACHABLE: u8 = 3;
pub const REP_HOST_UNREACHABLE: u8 = 4;
pub const REP_CONNECTION_REFUSED: u8 = 5;
pub const REP_TTL_EXPIRED: u8 = 6;
pub const REP_COMMAND_NOT_SUPPORTED: u8 = 7;
pub const REP_ADDRESS_TYPE_NOT_SUPPORTED: u8 = 8;

// Username/Password Authentication (RFC 1929)
pub const AUTH_VERSION: u8 = 1;
pub const AUTH_SUCCESS: u8 = 0;
pub const AUTH_FAILURE: u8 = 1;

// SOCKS address enum for different address types
#[derive(Debug, Clone)]
pub enum SocksAddr {
    Ipv4(Ipv4Addr, u16),
    Ipv6(Ipv6Addr, u16),
    Domain(String, u16),
}

impl SocksAddr {
    pub async fn read_from<R>(r: &mut R) -> io::Result<SocksAddr>
    where
        R: AsyncRead + Unpin,
    {
        let addr_type = r.read_u8().await?;
        match addr_type {
            ATYP_IPV4 => {
                let mut addr_bytes = [0u8; 4];
                r.read_exact(&mut addr_bytes).await?;
                let port = r.read_u16().await?;
                Ok(SocksAddr::Ipv4(Ipv4Addr::from(addr_bytes), port))
            }
            ATYP_IPV6 => {
                let mut addr_bytes = [0u8; 16];
                r.read_exact(&mut addr_bytes).await?;
                let port = r.read_u16().await?;
                Ok(SocksAddr::Ipv6(Ipv6Addr::from(addr_bytes), port))
            }
            ATYP_DOMAIN => {
                let len = r.read_u8().await? as usize;
                let mut domain = vec![0u8; len];
                r.read_exact(&mut domain).await?;
                let domain = String::from_utf8(domain).map_err(|_| {
                    io::Error::new(io::ErrorKind::InvalidData, "Invalid domain name")
                })?;
                let port = r.read_u16().await?;
                Ok(SocksAddr::Domain(domain, port))
            }
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Unsupported address type",
            )),
        }
    }

    pub async fn write_to<W>(&self, w: &mut W) -> io::Result<()>
    where
        W: AsyncWrite + Unpin,
    {
        match self {
            SocksAddr::Ipv4(addr, port) => {
                w.write_u8(ATYP_IPV4).await?;
                w.write_all(&addr.octets()).await?;
                w.write_u16(*port).await?;
            }
            SocksAddr::Ipv6(addr, port) => {
                w.write_u8(ATYP_IPV6).await?;
                w.write_all(&addr.octets()).await?;
                w.write_u16(*port).await?;
            }
            SocksAddr::Domain(domain, port) => {
                if domain.len() > 255 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "Domain too long",
                    ));
                }
                w.write_u8(ATYP_DOMAIN).await?;
                w.write_u8(domain.len() as u8).await?;
                w.write_all(domain.as_bytes()).await?;
                w.write_u16(*port).await?;
            }
        }
        Ok(())
    }

    pub fn to_socket_addr(&self) -> Option<SocketAddr> {
        match self {
            SocksAddr::Ipv4(addr, port) => Some(SocketAddr::V4(SocketAddrV4::new(*addr, *port))),
            SocksAddr::Ipv6(addr, port) => {
                Some(SocketAddr::V6(SocketAddrV6::new(*addr, *port, 0, 0)))
            }
            SocksAddr::Domain(_, _) => None, // Requires DNS resolution
        }
    }
}

impl fmt::Display for SocksAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SocksAddr::Ipv4(addr, port) => write!(f, "{}:{}", addr, port),
            SocksAddr::Ipv6(addr, port) => write!(f, "[{}]:{}", addr, port),
            SocksAddr::Domain(domain, port) => write!(f, "{}:{}", domain, port),
        }
    }
}

// SOCKS handshake request structure
pub struct HandshakeRequest {
    pub version: u8,
    pub methods: Vec<u8>,
}

// SOCKS connection request structure
pub struct Request {
    pub version: u8,
    pub command: u8,
    pub addr: SocksAddr,
}

// SOCKS reply structure
pub struct Reply {
    pub version: u8,
    pub reply: u8,
    pub addr: SocksAddr,
}

// Username/Password Authentication structure
pub struct UserPassAuth {
    pub username: String,
    pub password: String,
}

impl UserPassAuth {
    pub fn new(username: String, password: String) -> Self {
        UserPassAuth { username, password }
    }

    pub async fn write_to<W>(&self, w: &mut W) -> io::Result<()>
    where
        W: AsyncWrite + Unpin,
    {
        // Authentication subversion
        w.write_u8(AUTH_VERSION).await?;

        // Username
        if self.username.len() > 255 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Username too long",
            ));
        }
        w.write_u8(self.username.len() as u8).await?;
        w.write_all(self.username.as_bytes()).await?;

        // Password
        if self.password.len() > 255 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Password too long",
            ));
        }
        w.write_u8(self.password.len() as u8).await?;
        w.write_all(self.password.as_bytes()).await?;

        w.flush().await?;
        Ok(())
    }

    pub async fn read_from<R>(r: &mut R) -> io::Result<Self>
    where
        R: AsyncRead + Unpin,
    {
        let version = r.read_u8().await?;
        if version != AUTH_VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid authentication version",
            ));
        }

        // Username
        let username_len = r.read_u8().await? as usize;
        let mut username_bytes = vec![0u8; username_len];
        r.read_exact(&mut username_bytes).await?;
        let username = String::from_utf8(username_bytes)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid username encoding"))?;

        // Password
        let password_len = r.read_u8().await? as usize;
        let mut password_bytes = vec![0u8; password_len];
        r.read_exact(&mut password_bytes).await?;
        let password = String::from_utf8(password_bytes)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid password encoding"))?;

        Ok(UserPassAuth { username, password })
    }
}

impl HandshakeRequest {
    pub async fn read_from<R>(r: &mut R) -> io::Result<Self>
    where
        R: AsyncRead + Unpin,
    {
        let version = r.read_u8().await?;
        if version != SOCKS_VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Unsupported SOCKS version",
            ));
        }

        let nmethods = r.read_u8().await?;
        let mut methods = vec![0u8; nmethods as usize];
        r.read_exact(&mut methods).await?;

        Ok(HandshakeRequest { version, methods })
    }
}

impl Request {
    pub async fn read_from<R>(r: &mut R) -> io::Result<Self>
    where
        R: AsyncRead + Unpin,
    {
        let version = r.read_u8().await?;
        if version != SOCKS_VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Unsupported SOCKS version",
            ));
        }

        let command = r.read_u8().await?;
        let _reserved = r.read_u8().await?; // Reserved byte, ignored
        let addr = SocksAddr::read_from(r).await?;

        Ok(Request {
            version,
            command,
            addr,
        })
    }
}

impl Reply {
    pub fn new(reply: u8, addr: SocksAddr) -> Self {
        Reply {
            version: SOCKS_VERSION,
            reply,
            addr,
        }
    }

    pub async fn write_to<W>(&self, w: &mut W) -> io::Result<()>
    where
        W: AsyncWrite + Unpin,
    {
        w.write_u8(self.version).await?;
        w.write_u8(self.reply).await?;
        w.write_u8(0x00).await?; // Reserved
        self.addr.write_to(w).await?;
        w.flush().await?;
        Ok(())
    }
}
