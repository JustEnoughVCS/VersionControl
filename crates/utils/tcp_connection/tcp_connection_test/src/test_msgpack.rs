use serde::{Deserialize, Serialize};
use std::time::Duration;
use tcp_connection::instance::ConnectionInstance;
use tokio::{join, time::sleep};

use crate::test_utils::{
    handle::{ClientHandle, ServerHandle},
    target::TcpServerTarget,
    target_configure::ServerTargetConfig,
};

#[derive(Debug, PartialEq, Serialize, Deserialize, Default)]
struct TestData {
    id: u32,
    name: String,
}

pub(crate) struct MsgPackClientHandle;

impl ClientHandle<MsgPackServerHandle> for MsgPackClientHandle {
    async fn process(mut instance: ConnectionInstance) {
        // Test basic MessagePack serialization
        let test_data = TestData {
            id: 42,
            name: "Test MessagePack".to_string(),
        };

        // Write MessagePack data
        if let Err(e) = instance.write_msgpack(&test_data).await {
            panic!("Write MessagePack failed: {}", e);
        }

        // Read response
        let response: TestData = match instance.read_msgpack().await {
            Ok(data) => data,
            Err(e) => panic!("Read MessagePack response failed: {}", e),
        };

        // Verify response
        assert_eq!(response.id, test_data.id * 2);
        assert_eq!(response.name, format!("Processed: {}", test_data.name));
    }
}

pub(crate) struct MsgPackServerHandle;

impl ServerHandle<MsgPackClientHandle> for MsgPackServerHandle {
    async fn process(mut instance: ConnectionInstance) {
        // Read MessagePack data
        let received_data: TestData = match instance.read_msgpack().await {
            Ok(data) => data,
            Err(_) => return,
        };

        // Process data
        let response = TestData {
            id: received_data.id * 2,
            name: format!("Processed: {}", received_data.name),
        };

        // Write response as MessagePack
        if let Err(e) = instance.write_msgpack(&response).await {
            panic!("Write MessagePack response failed: {}", e);
        }
    }
}

#[tokio::test]
async fn test_msgpack_basic() {
    let host = "localhost:5013";

    // Server setup
    let Ok(server_target) =
        TcpServerTarget::<MsgPackClientHandle, MsgPackServerHandle>::from_domain(host).await
    else {
        panic!("Test target built failed from a domain named `{}`", host);
    };

    // Client setup
    let Ok(client_target) =
        TcpServerTarget::<MsgPackClientHandle, MsgPackServerHandle>::from_domain(host).await
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
