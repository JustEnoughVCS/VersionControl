use crate::instance::ConnectionInstance;
use std::future::Future;

pub trait ClientHandle<RequestServer> {
    fn process(instance: ConnectionInstance) -> impl Future<Output = ()> + Send;
}

pub trait ServerHandle<RequestClient> {
    fn process(instance: ConnectionInstance) -> impl Future<Output = ()> + Send;
}
