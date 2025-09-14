use tokio::net::TcpStream;

pub trait ClientHandle<RequestServer> {

    fn process(stream: TcpStream);
}

pub trait ServerHandle<RequestClient> {

    fn process(stream: TcpStream);
}

