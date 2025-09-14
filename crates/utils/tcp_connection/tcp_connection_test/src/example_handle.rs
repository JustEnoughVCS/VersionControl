use tcp_connection::{
    handle::{ClientHandle, ServerHandle},
    instance::ConnectionInstance,
};

pub(crate) struct ExampleClientHandle;

impl ClientHandle<ExampleServerHandle> for ExampleClientHandle {
    fn process(
        instance: ConnectionInstance,
    ) -> impl std::future::Future<Output = ()> + Send + Sync {
        let _ = instance;
        async {}
    }
}

pub(crate) struct ExampleServerHandle;

impl ServerHandle<ExampleClientHandle> for ExampleServerHandle {
    fn process(
        instance: ConnectionInstance,
    ) -> impl std::future::Future<Output = ()> + Send + Sync {
        let _ = instance;
        async {}
    }
}
