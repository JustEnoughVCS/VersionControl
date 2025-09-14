use tokio::net::TcpStream;

pub struct ConnectionInstance {
    stream: TcpStream,
}

impl From<TcpStream> for ConnectionInstance {
    fn from(value: TcpStream) -> Self {
        Self { stream: value }
    }
}
