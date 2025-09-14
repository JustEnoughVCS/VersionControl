use tokio::net::TcpStream;
use tcp_connection::handle::{ClientHandle, ServerHandle};

pub(crate) struct ExampleClientHandle;

impl ClientHandle<ExampleServerHandle> for ExampleClientHandle {
    fn process(stream: TcpStream) {

    }
}

pub(crate) struct ExampleServerHandle;

impl ServerHandle<ExampleClientHandle> for ExampleServerHandle {
    fn process(stream: TcpStream) {

    }
}