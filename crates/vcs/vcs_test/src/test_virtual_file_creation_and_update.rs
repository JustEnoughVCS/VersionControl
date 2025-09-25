use std::time::Duration;

use cfg_file::config::ConfigFile;
use env::{
    constants::SERVER_FILE_VAULT,
    workspace::{
        member::Member,
        vault::{Vault, config::VaultConfig, virtual_file::VirtualFileVersionDescription},
    },
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

use crate::get_test_dir;

struct VirtualFileCreateClientHandle;
struct VirtualFileCreateServerHandle;

impl ClientHandle<VirtualFileCreateServerHandle> for VirtualFileCreateClientHandle {
    fn process(
        mut instance: tcp_connection::instance::ConnectionInstance,
    ) -> impl Future<Output = ()> + Send {
        async move {
            let dir = get_test_dir("virtual_file_creation_and_update_2")
                .await
                .unwrap();
            // Create first test file for virtual file creation
            let test_content_1 = b"Test file content for virtual file creation";
            let temp_file_path_1 = dir.join("test_virtual_file_1.txt");

            tokio::fs::write(&temp_file_path_1, test_content_1)
                .await
                .unwrap();

            // Send the first file to server for virtual file creation
            instance.write_file(&temp_file_path_1).await.unwrap();

            // Create second test file for virtual file update
            let test_content_2 = b"Updated test file content for virtual file";
            let temp_file_path_2 = dir.join("test_virtual_file_2.txt");

            tokio::fs::write(&temp_file_path_2, test_content_2)
                .await
                .unwrap();

            // Send the second file to server for virtual file update
            instance.write_file(&temp_file_path_2).await.unwrap();
        }
    }
}

impl ServerHandle<VirtualFileCreateClientHandle> for VirtualFileCreateServerHandle {
    fn process(
        mut instance: tcp_connection::instance::ConnectionInstance,
    ) -> impl Future<Output = ()> + Send {
        async move {
            let dir = get_test_dir("virtual_file_creation_and_update")
                .await
                .unwrap();

            // Setup vault
            Vault::setup_vault(dir.clone()).await.unwrap();

            // Read vault
            let Some(vault) = Vault::init(
                VaultConfig::read_from(dir.join(SERVER_FILE_VAULT))
                    .await
                    .unwrap(),
                &dir,
            ) else {
                panic!("No vault found!");
            };

            // Register member
            let member_id = "test_member";
            vault
                .register_member_to_vault(Member::new(member_id))
                .await
                .unwrap();

            // Create visual file
            let virtual_file_id = vault
                .create_virtual_file_from_connection(&mut instance, &member_id.to_string())
                .await
                .unwrap();

            // Grant edit right to member
            vault
                .grant_virtual_file_edit_right(&member_id.to_string(), &virtual_file_id)
                .await
                .unwrap();

            // Update visual file
            vault
                .update_virtual_file_from_connection(
                    &mut instance,
                    &member_id.to_string(),
                    &virtual_file_id,
                    &"2".to_string(),
                    VirtualFileVersionDescription {
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
