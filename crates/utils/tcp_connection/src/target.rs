use std::fmt::{Display, Formatter};
use std::net::{AddrParseError, IpAddr, Ipv4Addr, SocketAddr};
use std::str::FromStr;
use tokio::net::lookup_host;
use crate::handle::{ClientHandle, ServerHandle};

const DEFAULT_PORT: u16 = 8080;

#[derive(Debug, Eq, PartialEq)]
pub struct TcpServerTarget<Client, Server>
where Client: ClientHandle<Server>,
      Server: ServerHandle<Client> {

    /// Client Handle
    client_handle: Option<Client>,

    /// Server Handle
    server_handle: Option<Server>,

    /// Server port
    port: u16,

    /// Bind addr
    bind_addr: IpAddr,
}

impl<Client, Server> Default for TcpServerTarget<Client, Server>
where Client: ClientHandle<Server>,
      Server: ServerHandle<Client> {
    fn default() -> Self {
        Self {
            client_handle: None,
            server_handle: None,
            port: DEFAULT_PORT,
            bind_addr: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        }
    }
}

impl<Client, Server> From<SocketAddr> for TcpServerTarget<Client, Server>
where Client: ClientHandle<Server>,
      Server: ServerHandle<Client> {

    /// Convert SocketAddr to TcpServerTarget
    fn from(value: SocketAddr) -> Self {
        Self {
            port: value.port(),
            bind_addr: value.ip(),
            .. Self::default()
        }
    }
}

impl<Client, Server> From<TcpServerTarget<Client, Server>> for SocketAddr
where Client: ClientHandle<Server>,
      Server: ServerHandle<Client> {

    /// Convert TcpServerTarget to SocketAddr
    fn from(val: TcpServerTarget<Client, Server>) -> Self {
        SocketAddr::new(val.bind_addr, val.port)
    }
}

impl<Client, Server> Display for TcpServerTarget<Client, Server>
where Client: ClientHandle<Server>,
      Server: ServerHandle<Client> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.bind_addr, self.port)
    }
}

impl<Client, Server> TcpServerTarget<Client, Server>
where Client: ClientHandle<Server>,
      Server: ServerHandle<Client> {

    /// Create target by address
    pub fn from_addr(addr: impl Into<IpAddr>, port: impl Into<u16>) -> Self {
        Self {
            port: port.into(),
            bind_addr: addr.into(),
            .. Self::default()
        }
    }

    /// Try to create target by string
    pub fn from_str<'a>(addr_str: impl Into<&'a str>) -> Result<Self, AddrParseError> {
        let socket_addr = SocketAddr::from_str(addr_str.into());
        match socket_addr {
            Ok(socket_addr) => {
                Ok(Self::from_addr(socket_addr.ip(), socket_addr.port()))
            }
            Err(err) => {
                Err(err)
            }
        }
    }

    /// Try to create target by domain name
    pub async fn from_domain<'a>(domain: impl Into<&'a str>) -> Result<Self, std::io::Error> {
        match domain_to_addr(domain).await {
            Ok(domain_addr) => Ok(Self::from(domain_addr)),
            Err(e) => Err(e),
        }
    }
}

/// Parse Domain Name to IpAddr via DNS
async fn domain_to_addr<'a>(domain: impl Into<&'a str>) -> Result<SocketAddr, std::io::Error> {
    let domain = domain.into();
    let default_port: u16 = DEFAULT_PORT;

    if let Ok(socket_addr) = domain.parse::<SocketAddr>() {
        return Ok(match socket_addr.ip() {
            IpAddr::V4(_) => socket_addr,
            IpAddr::V6(_) => SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), socket_addr.port()),
        });
    }

    if let Ok(_v6_addr) = domain.parse::<std::net::Ipv6Addr>() {
        return Ok(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), default_port));
    }

    let (host, port_str) = if let Some((host, port)) = domain.rsplit_once(':') {
        (host.trim_matches(|c| c == '[' || c == ']'), Some(port))
    } else {
        (domain, None)
    };

    let port = port_str
        .and_then(|p| p.parse::<u16>().ok())
        .map(|p| p.clamp(0, u16::MAX))
        .unwrap_or(default_port);

    let mut socket_iter = lookup_host((host, 0)).await?;

    if let Some(addr) = socket_iter.find(|addr| addr.is_ipv4()) {
        return Ok(SocketAddr::new(addr.ip(), port));
    }

    Ok(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port))
}