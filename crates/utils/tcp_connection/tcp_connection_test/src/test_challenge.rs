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

pub(crate) struct ExampleChallengeClientHandle;

impl ClientHandle<ExampleChallengeServerHandle> for ExampleChallengeClientHandle {
    fn process(mut instance: ConnectionInstance) -> impl std::future::Future<Output = ()> + Send {
        async move {
            // Accept challenge with correct key
            let key = current_dir()
                .unwrap()
                .join("res")
                .join("key")
                .join("test_key_private.pem");
            let result = instance.accept_challenge(key, "test_key").await.unwrap();

            // Sent success
            assert_eq!(true, result);
            let response = instance.read_text().await.unwrap();

            // Verify success
            assert_eq!("OK", response);

            // Accept challenge with wrong key
            let key = current_dir()
                .unwrap()
                .join("res")
                .join("key")
                .join("wrong_key_private.pem");
            let result = instance.accept_challenge(key, "test_key").await.unwrap();

            // Sent success
            assert_eq!(true, result);
            let response = instance.read_text().await.unwrap();

            // Verify fail
            assert_eq!("ERROR", response);

            // Accept challenge with wrong name
            let key = current_dir()
                .unwrap()
                .join("res")
                .join("key")
                .join("test_key_private.pem");
            let result = instance.accept_challenge(key, "test_key__").await.unwrap();

            // Sent success
            assert_eq!(true, result);
            let response = instance.read_text().await.unwrap();

            // Verify fail
            assert_eq!("ERROR", response);
        }
    }
}

pub(crate) struct ExampleChallengeServerHandle;

impl ServerHandle<ExampleChallengeClientHandle> for ExampleChallengeServerHandle {
    fn process(mut instance: ConnectionInstance) -> impl std::future::Future<Output = ()> + Send {
        async move {
            // Challenge with correct key
            let key_dir = current_dir().unwrap().join("res").join("key");
            let result = instance.challenge(key_dir).await.unwrap();
            assert_eq!(true, result);

            // Send response
            instance
                .write_text(if result { "OK" } else { "ERROR" })
                .await
                .unwrap();

            // Challenge again
            let key_dir = current_dir().unwrap().join("res").join("key");
            let result = instance.challenge(key_dir).await.unwrap();
            assert_eq!(false, result);

            // Send response
            instance
                .write_text(if result { "OK" } else { "ERROR" })
                .await
                .unwrap();

            // Challenge again
            let key_dir = current_dir().unwrap().join("res").join("key");
            let result = instance.challenge(key_dir).await.unwrap();
            assert_eq!(false, result);

            // Send response
            instance
                .write_text(if result { "OK" } else { "ERROR" })
                .await
                .unwrap();
        }
    }
}

#[tokio::test]
async fn test_connection_with_challenge_handle() -> Result<(), std::io::Error> {
    let host = "localhost:5011";

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
