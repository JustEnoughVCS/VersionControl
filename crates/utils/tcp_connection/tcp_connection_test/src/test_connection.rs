use std::time::Duration;

use tcp_connection::{
    handle::{ClientHandle, ServerHandle},
    instance::ConnectionInstance,
    target::TcpServerTarget,
    target_configure::ServerTargetConfig,
};
use tokio::{join, time::sleep};

pub(crate) struct ExampleClientHandle;

impl ClientHandle<ExampleServerHandle> for ExampleClientHandle {
    fn process(
        mut instance: ConnectionInstance,
    ) -> impl std::future::Future<Output = ()> + Send + Sync {
        async move {
            // Write name
            let Ok(_) = instance.write_text("Peter").await else {
                panic!("Write text failed!");
            };
            // Read msg
            let Ok(result) = instance.read_text(512 as u32).await else {
                return;
            };
            assert_eq!("Hello Peter!", result);
        }
    }
}

pub(crate) struct ExampleServerHandle;

impl ServerHandle<ExampleClientHandle> for ExampleServerHandle {
    fn process(
        mut instance: ConnectionInstance,
    ) -> impl std::future::Future<Output = ()> + Send + Sync {
        async move {
            // Read name
            let Ok(name) = instance.read_text(512 as u32).await else {
                return;
            };
            // Write msg
            let Ok(_) = instance.write_text(format!("Hello {}!", name)).await else {
                panic!("Write text failed!");
            };
        }
    }
}

#[tokio::test]
async fn test_connection_with_example_handle() {
    let host = "localhost:5012";

    // Server setup
    let Ok(server_target) =
        TcpServerTarget::<ExampleClientHandle, ExampleServerHandle>::from_domain(host).await
    else {
        panic!("Test target built failed from a domain named `{}`", host);
    };

    // Client setup
    let Ok(client_target) =
        TcpServerTarget::<ExampleClientHandle, ExampleServerHandle>::from_domain(host).await
    else {
        panic!("Test target built failed from a domain named `{}`", host);
    };

    let future_server = async move {
        // Only process once
        let configured_server = server_target.server_cfg(ServerTargetConfig::default().once());

        // Listen here
        let _ = configured_server.listen().await;
    };

    let future_client = async move {
        // Wait for server start
        let _ = sleep(Duration::from_secs_f32(1.5)).await;

        // Connect here
        let _ = client_target.connect().await;
    };

    let _ = async { join!(future_client, future_server) }.await;
}
