use std::time::Duration;

use cfg_file::config::ConfigFile;
use env::workspace::{
    member::Member,
    vault::{Vault, config::VaultConfig, vitrual_file::VirtualFileVersionDesciption},
};
use tcp_connection::{
    handle::{ClientHandle, ServerHandle},
    target::TcpServerTarget,
    target_configure::ServerTargetConfig,
};
use tokio::{
    join,
    time::{sleep, timeout},
};

use crate::get_and_correct_test_dir;

struct VirtualFileCreateClientHandle;
struct VirtualFileCreateServerHandle;

impl ClientHandle<VirtualFileCreateServerHandle> for VirtualFileCreateClientHandle {
    fn process(
        instance: tcp_connection::instance::ConnectionInstance,
    ) -> impl Future<Output = ()> + Send + Sync {
        async move {}
    }
}

impl ServerHandle<VirtualFileCreateClientHandle> for VirtualFileCreateServerHandle {
    fn process(
        mut instance: tcp_connection::instance::ConnectionInstance,
    ) -> impl Future<Output = ()> + Send + Sync {
        async move {
            let dir = get_and_correct_test_dir("virtual_file_creation_and_update")
                .await
                .unwrap();

            // Setup vault
            Vault::setup_vault(dir.clone()).await.unwrap();

            // Read vault
            let Some(vault) = Vault::init_current_dir(VaultConfig::read().await.unwrap()) else {
                panic!("No vault found!");
            };

            // Register member
            let member_id = "test_member";
            vault.register_member_to_vault(Member::new(member_id)).await;

            // Create visual file
            let virtual_file_id = vault
                .create_virtual_file_from_connection(&mut instance, member_id.to_string())
                .await
                .unwrap();

            // Update visual file
            vault
                .update_virtual_file_from_connection(
                    &mut instance,
                    member_id.to_string(),
                    virtual_file_id,
                    "2".to_string(),
                    VirtualFileVersionDesciption {
                        creator: member_id.to_string(),
                        description: "Update".to_string(),
                    },
                )
                .await
                .unwrap();
        }
    }
}

#[tokio::test]
async fn test_virtual_file_creation_and_update() -> Result<(), std::io::Error> {
    let host = "localhost:5009";

    // Server setup
    let Ok(server_target) = TcpServerTarget::<
        VirtualFileCreateClientHandle,
        VirtualFileCreateServerHandle,
    >::from_domain(host)
    .await
    else {
        panic!("Test target built failed from a domain named `{}`", host);
    };

    // Client setup
    let Ok(client_target) = TcpServerTarget::<
        VirtualFileCreateClientHandle,
        VirtualFileCreateServerHandle,
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

    let test_timeout = Duration::from_secs(15);

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
