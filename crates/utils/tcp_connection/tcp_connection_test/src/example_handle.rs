use tcp_connection::{
    handle::{ClientHandle, ServerHandle},
    instance::ConnectionInstance,
};

pub(crate) struct ExampleClientHandle;

impl ClientHandle<ExampleServerHandle> for ExampleClientHandle {
    fn process(
        mut instance: ConnectionInstance,
    ) -> impl std::future::Future<Output = ()> + Send + Sync {
        async move {
            let _ = instance.write_text("Hello, World!").await;
            let Ok(result) = instance.read_text(512 as u32).await else {
                return;
            };
            println!("Received: `{}`", result);
        }
    }
}

pub(crate) struct ExampleServerHandle;

impl ServerHandle<ExampleClientHandle> for ExampleServerHandle {
    fn process(
        mut instance: ConnectionInstance,
    ) -> impl std::future::Future<Output = ()> + Send + Sync {
        async move {
            let Ok(_) = instance.read_text(512 as u32).await else {
                return;
            };
            let _ = instance.write_text("Hello!").await;
        }
    }
}
