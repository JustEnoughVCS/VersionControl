use std::net::{AddrParseError, IpAddr, Ipv4Addr, SocketAddr};
use std::str::FromStr;
use tokio::net::lookup_host;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct TcpServerTarget {

    /// Server port
    port: u16,

    /// Bind addr
    bind_addr: IpAddr,
}

impl Default for TcpServerTarget {
    fn default() -> Self {
        Self {
            port: 80,
            bind_addr: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        }
    }
}

impl From<SocketAddr> for TcpServerTarget {

    /// Convert SocketAddr to TcpServerTarget
    fn from(value: SocketAddr) -> Self {
        Self {
            port: value.port(),
            bind_addr: value.ip(),
        }
    }
}

impl TcpServerTarget {

    /// Create target by address
    pub fn from_addr(addr: impl Into<IpAddr>, port: impl Into<u16>) -> Self {
        Self {
            port: port.into(),
            bind_addr: addr.into(),
        }
    }

    /// Try to create target by string
    pub fn from_str<'a>(addr_str: impl AsRef<&'a str>) -> Result<Self, AddrParseError> {
        let socket_addr = SocketAddr::from_str(addr_str.as_ref());
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
    pub async fn from_domain<'a>(domain: impl AsRef<&'a str>) -> Result<Self, std::io::Error> {
        match domain_to_addr(domain).await {
            Ok(domain_addr) => Ok(Self::from(domain_addr)),
            Err(e) => Err(e),
        }
    }
}

/// Parse Domain Name to IpAddr via DNS
async fn domain_to_addr<'a>(domain: impl AsRef<&'a str>) -> Result<SocketAddr, std::io::Error> {
    let domain = domain.as_ref();
    let default_port: u16 = 80;

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
        (&domain[..], None)
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