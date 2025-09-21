use std::{env::current_dir, time::Duration};

use tcp_connection::{
    handle::{ClientHandle, ServerHandle},
    instance::ConnectionInstance,
    target::TcpServerTarget,
    target_configure::ServerTargetConfig,
};
use tokio::{join, time::sleep};

pub(crate) struct ExampleChallengeClientHandle;

impl ClientHandle<ExampleChallengeServerHandle> for ExampleChallengeClientHandle {
    fn process(
        mut instance: ConnectionInstance,
    ) -> impl std::future::Future<Output = ()> + Send + Sync {
        async move {
            let key = current_dir().unwrap().join("res").join("test_key");
            let result = instance.accept_challenge(key, "test_key").await.unwrap();
            assert_eq!(true, result);
        }
    }
}

pub(crate) struct ExampleChallengeServerHandle;

impl ServerHandle<ExampleChallengeClientHandle> for ExampleChallengeServerHandle {
    fn process(
        mut instance: ConnectionInstance,
    ) -> impl std::future::Future<Output = ()> + Send + Sync {
        async move {
            let key_dir = current_dir().unwrap().join("res");
            let result = instance.challenge(key_dir).await.unwrap();
            assert_eq!(true, result);
        }
    }
}

#[tokio::test]
async fn test_connection_with_challenge_handle() -> Result<(), std::io::Error> {
    let host = "localhost";

    // Server setup
    let Ok(server_target) = TcpServerTarget::<
        ExampleChallengeClientHandle,
        ExampleChallengeServerHandle,
    >::from_domain(host)
    .await
    else {
        panic!("Test target built failed from a domain named `{}`", host);
    };

    // Client setup
    let Ok(client_target) = TcpServerTarget::<
        ExampleChallengeClientHandle,
        ExampleChallengeServerHandle,
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

    let _ = async { join!(future_client, future_server) }.await;

    Ok(())
}
