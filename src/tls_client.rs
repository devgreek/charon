use std::io;
use std::sync::Arc;

use log::debug;
use rustls::ClientConfig;
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use tokio_rustls::client::TlsStream;

// We use the insecure client config from lib.rs instead of implementing here

use crate::client::Client;
use crate::protocol::SocksAddr;

pub struct TlsClient {
    proxy_host: String,
    proxy_port: u16,
    auth: Option<(String, String)>,
    tls_config: Arc<ClientConfig>,
}

impl TlsClient {
    pub fn new(proxy_host: String, proxy_port: u16) -> Self {
        let tls_config = create_tls_config();

        TlsClient {
            proxy_host,
            proxy_port,
            auth: None,
            tls_config,
        }
    }

    pub fn with_auth(
        proxy_host: String,
        proxy_port: u16,
        username: String,
        password: String,
    ) -> Self {
        let tls_config = create_tls_config();

        TlsClient {
            proxy_host,
            proxy_port,
            auth: Some((username, password)),
            tls_config,
        }
    }

    pub async fn connect_to_domain(
        &self,
        domain: &str,
        port: u16,
    ) -> io::Result<TlsStream<TcpStream>> {
        // Connect to proxy server with TLS
        let proxy_addr = format!("{}:{}", self.proxy_host, self.proxy_port);
        let tcp_stream = TcpStream::connect(&proxy_addr).await?;

        debug!("Connected to SOCKS5 proxy at {}", proxy_addr);

        // Establish TLS connection to the proxy
        let connector = TlsConnector::from(Arc::clone(&self.tls_config));
        let domain_name = self
            .proxy_host
            .as_str()
            .try_into()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "Invalid domain name"))?;

        let mut tls_stream = connector.connect(domain_name, tcp_stream).await?;

        debug!("TLS connection established to proxy");

        // Create a client to perform the SOCKS5 handshake over TLS
        let client = match &self.auth {
            Some((username, password)) => Client::with_auth(
                self.proxy_host.clone(),
                self.proxy_port,
                username.clone(),
                password.clone(),
            ),
            None => Client::new(self.proxy_host.clone(), self.proxy_port),
        };

        // Create SocksAddr from parameters
        let addr = SocksAddr::Domain(domain.to_string(), port);

        // Perform handshake
        client.handshake(&mut tls_stream).await?;

        // Request connection
        client.request_connection(&mut tls_stream, addr).await?;

        Ok(tls_stream)
    }

    pub async fn connect_to_target<A: std::net::ToSocketAddrs>(
        &self,
        target_addr: A,
    ) -> io::Result<TlsStream<TcpStream>> {
        use std::net::ToSocketAddrs;

        let target = target_addr
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Could not resolve address"))?;

        // Convert to SocksAddr
        let socks_addr = match target {
            std::net::SocketAddr::V4(addr) => SocksAddr::Ipv4(*addr.ip(), addr.port()),
            std::net::SocketAddr::V6(addr) => SocksAddr::Ipv6(*addr.ip(), addr.port()),
        };

        // Connect to proxy server with TLS
        let proxy_addr = format!("{}:{}", self.proxy_host, self.proxy_port);
        let tcp_stream = TcpStream::connect(&proxy_addr).await?;

        debug!("Connected to SOCKS5 proxy at {}", proxy_addr);

        // Establish TLS connection to the proxy
        let connector = TlsConnector::from(Arc::clone(&self.tls_config));
        let domain_name = self
            .proxy_host
            .as_str()
            .try_into()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "Invalid domain name"))?;

        let mut tls_stream = connector.connect(domain_name, tcp_stream).await?;

        debug!("TLS connection established to proxy");

        // Create a client to perform the SOCKS5 handshake over TLS
        let client = match &self.auth {
            Some((username, password)) => Client::with_auth(
                self.proxy_host.clone(),
                self.proxy_port,
                username.clone(),
                password.clone(),
            ),
            None => Client::new(self.proxy_host.clone(), self.proxy_port),
        };

        // Perform handshake
        client.handshake(&mut tls_stream).await?;

        // Request connection
        client
            .request_connection(&mut tls_stream, socks_addr)
            .await?;

        Ok(tls_stream)
    }
}

fn create_tls_config() -> Arc<ClientConfig> {
    // Use the helper function from lib.rs
    crate::create_insecure_client_config()
}
