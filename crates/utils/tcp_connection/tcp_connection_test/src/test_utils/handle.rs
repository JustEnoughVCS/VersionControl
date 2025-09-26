use std::future::Future;

use tcp_connection::instance::ConnectionInstance;

pub trait ClientHandle<RequestServer> {
    fn process(instance: ConnectionInstance) -> impl Future<Output = ()> + Send;
}

pub trait ServerHandle<RequestClient> {
    fn process(instance: ConnectionInstance) -> impl Future<Output = ()> + Send;
}
