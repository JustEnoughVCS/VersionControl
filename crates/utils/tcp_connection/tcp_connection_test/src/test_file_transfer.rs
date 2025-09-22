use std::{env::current_dir, time::Duration};

use tcp_connection::{
    handle::{ClientHandle, ServerHandle},
    instance::ConnectionInstance,
    target::TcpServerTarget,
    target_configure::ServerTargetConfig,
};
use tokio::{
    join,
    time::{sleep, timeout},
};

pub(crate) struct ExampleFileTransferClientHandle;

impl ClientHandle<ExampleFileTransferServerHandle> for ExampleFileTransferClientHandle {
    fn process(
        mut instance: ConnectionInstance,
    ) -> impl std::future::Future<Output = ()> + Send + Sync {
        async move {
            let image_path = current_dir()
                .unwrap()
                .join("res")
                .join("image")
                .join("test_transfer.png");
            instance.write_file(image_path).await.unwrap();
        }
    }
}

pub(crate) struct ExampleFileTransferServerHandle;

impl ServerHandle<ExampleFileTransferClientHandle> for ExampleFileTransferServerHandle {
    fn process(
        mut instance: ConnectionInstance,
    ) -> impl std::future::Future<Output = ()> + Send + Sync {
        async move {
            let save_path = current_dir()
                .unwrap()
                .join("res")
                .join(".temp")
                .join("image")
                .join("test_transfer.png");
            instance.read_file(save_path).await.unwrap();
        }
    }
}

#[tokio::test]
async fn test_connection_with_challenge_handle() -> Result<(), std::io::Error> {
    let host = "localhost:5010";

    // Server setup
    let Ok(server_target) = TcpServerTarget::<
        ExampleFileTransferClientHandle,
        ExampleFileTransferServerHandle,
    >::from_domain(host)
    .await
    else {
        panic!("Test target built failed from a domain named `{}`", host);
    };

    // Client setup
    let Ok(client_target) = TcpServerTarget::<
        ExampleFileTransferClientHandle,
        ExampleFileTransferServerHandle,
    >::from_domain(host)
    .await
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

    let test_timeout = Duration::from_secs(10);

    timeout(test_timeout, async { join!(future_client, future_server) })
        .await
        .map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                format!("Test timed out after {:?}", test_timeout),
            )
        })?;

    Ok(())
}
