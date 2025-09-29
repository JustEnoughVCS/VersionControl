#[cfg(test)]
mod test_incremental_transfer {
    use std::path::PathBuf;

    use tokio::fs::{self, File};
    use tokio::io::AsyncWriteExt;
    use tokio::net::{TcpListener, TcpStream};

    use tcp_connection::error::TcpTargetError;
    use tcp_connection::instance::ConnectionConfig;
    use tcp_connection::instance::ConnectionInstance;

    const TEST_PORT: u16 = 54321;
    const TEST_FILE_CONTENT: &str =
        "Hello, this is a test file content for incremental transfer testing.";
    const TEST_FILE_CONTENT_UPDATED: &str = "Hello, this is UPDATED test file content for incremental transfer testing with more changes.";
    const TEST_FILE_CONTENT_SECOND_UPDATE: &str = "Second update with completely different content";

    async fn setup_test_file(file_path: &PathBuf, content: &str) -> Result<(), TcpTargetError> {
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let mut file = File::create(file_path).await?;
        file.write_all(content.as_bytes()).await?;
        file.flush().await?;
        Ok(())
    }

    async fn cleanup_test_file(file_path: &PathBuf) -> Result<(), TcpTargetError> {
        if file_path.exists() {
            fs::remove_file(file_path).await?;
        }

        // Clean up version files
        let version_file = file_path.with_extension("v");
        if version_file.exists() {
            fs::remove_file(version_file).await?;
        }

        // Clean up version history directory
        let version_dir = file_path.parent().unwrap().join("diff");
        if version_dir.exists() {
            fs::remove_dir_all(version_dir).await?;
        }

        Ok(())
    }

    async fn read_file_content(file_path: &PathBuf) -> Result<String, TcpTargetError> {
        let content = fs::read_to_string(file_path).await?;
        Ok(content)
    }

    #[tokio::test]
    async fn test_incremental_transfer_basic_flow() {
        let server_file = PathBuf::from("res/.temp/test_data/server_file_basic.txt");
        let client_file = PathBuf::from("res/.temp/test_data/client_file_basic.txt");

        // Setup test files
        setup_test_file(&server_file, TEST_FILE_CONTENT)
            .await
            .unwrap();
        setup_test_file(&client_file, TEST_FILE_CONTENT)
            .await
            .unwrap();

        let listener = TcpListener::bind(format!("127.0.0.1:{}", TEST_PORT))
            .await
            .unwrap();
        let server_addr = listener.local_addr().unwrap();

        // Server task
        let server_file_clone = server_file.clone();
        let server_handle = tokio::spawn(async move {
            println!("Server: Waiting for client connection...");
            let (stream, _) = listener.accept().await.unwrap();
            println!("Server: Client connected");
            let config = ConnectionConfig::default();
            let mut server_instance = ConnectionInstance::with_config(stream, config);

            println!("Server: Handling client update request...");
            // Handle client update request
            server_instance
                .server_handle_client_update(&server_file_clone)
                .await
                .unwrap();
            println!("Server: Client update handled successfully");
        });

        // Client task
        let client_file_clone = client_file.clone();
        let client_handle = tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            println!("Client: Connecting to server...");
            let stream = TcpStream::connect(server_addr).await.unwrap();
            let config = ConnectionConfig::default();
            let mut client_instance = ConnectionInstance::with_config(stream, config);

            println!("Client: Starting update to version 1...");
            // Update client file to version 1
            client_instance
                .client_update_to_version(&client_file_clone, 1)
                .await
                .unwrap();
            println!("Client: Update completed successfully");
        });

        // Wait for both tasks to complete
        let (server_result, client_result) = tokio::join!(server_handle, client_handle);
        server_result.unwrap();
        client_result.unwrap();

        // Verify both files still have the same content
        let server_content = read_file_content(&server_file).await.unwrap();
        let client_content = read_file_content(&client_file).await.unwrap();

        assert_eq!(server_content, client_content);
        assert_eq!(server_content, TEST_FILE_CONTENT);

        // Cleanup
        cleanup_test_file(&server_file).await.unwrap();
        cleanup_test_file(&client_file).await.unwrap();
    }

    #[tokio::test]
    async fn test_incremental_upload_basic_flow() {
        let server_file = PathBuf::from("res/.temp/test_data/server_file_upload.txt");
        let client_file = PathBuf::from("res/.temp/test_data/client_file_upload.txt");

        // Setup test files - client has updated content
        setup_test_file(&server_file, TEST_FILE_CONTENT)
            .await
            .unwrap();
        setup_test_file(&client_file, TEST_FILE_CONTENT_UPDATED)
            .await
            .unwrap();

        let listener = TcpListener::bind(format!("127.0.0.1:{}", TEST_PORT + 1))
            .await
            .unwrap();
        let server_addr = listener.local_addr().unwrap();

        // Server task
        let server_file_clone = server_file.clone();
        let server_handle = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let config = ConnectionConfig::default();
            let mut server_instance = ConnectionInstance::with_config(stream, config);

            // Handle client upload request
            server_instance
                .server_handle_client_update(&server_file_clone)
                .await
                .unwrap();
        });

        // Client task
        let client_file_clone = client_file.clone();
        let client_handle = tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            let stream = TcpStream::connect(server_addr).await.unwrap();
            let config = ConnectionConfig::default();
            let mut client_instance = ConnectionInstance::with_config(stream, config);

            // Upload client changes to server
            let new_version = client_instance
                .client_upload(&client_file_clone)
                .await
                .unwrap();
            assert_eq!(new_version, 1); // First upload should create version 1
        });

        // Wait for both tasks to complete
        let (server_result, client_result) = tokio::join!(server_handle, client_handle);
        server_result.unwrap();
        client_result.unwrap();

        // Verify server file was updated with client content
        let server_content = read_file_content(&server_file).await.unwrap();
        let expected_content = TEST_FILE_CONTENT_UPDATED;

        assert_eq!(server_content, expected_content);

        // Cleanup
        cleanup_test_file(&server_file).await.unwrap();
        cleanup_test_file(&client_file).await.unwrap();
    }

    #[tokio::test]
    async fn test_version_increment_after_upload() {
        let server_file = PathBuf::from("res/.temp/test_data/server_file_version.txt");
        let client_file = PathBuf::from("res/.temp/test_data/client_file_version.txt");

        setup_test_file(&server_file, TEST_FILE_CONTENT)
            .await
            .unwrap();
        setup_test_file(&client_file, TEST_FILE_CONTENT_UPDATED)
            .await
            .unwrap();

        let listener = TcpListener::bind(format!("127.0.0.1:{}", TEST_PORT + 2))
            .await
            .unwrap();
        let server_addr = listener.local_addr().unwrap();

        // First upload
        let server_file_clone = server_file.clone();
        let server_handle1 = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let config = ConnectionConfig::default();
            let mut server_instance = ConnectionInstance::with_config(stream, config);
            server_instance
                .server_handle_client_update(&server_file_clone)
                .await
                .unwrap();
        });

        let client_file_clone = client_file.clone();
        let client_handle1 = tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            let stream = TcpStream::connect(server_addr).await.unwrap();
            let config = ConnectionConfig::default();
            let mut client_instance = ConnectionInstance::with_config(stream, config);
            let version = client_instance
                .client_upload(&client_file_clone)
                .await
                .unwrap();
            assert_eq!(version, 1);
        });

        let (server_result1, client_result1) = tokio::join!(server_handle1, client_handle1);
        server_result1.unwrap();
        client_result1.unwrap();

        // Second upload with different content
        let updated_content2 = TEST_FILE_CONTENT_SECOND_UPDATE;
        setup_test_file(&client_file, updated_content2)
            .await
            .unwrap();

        let listener2 = TcpListener::bind(format!("127.0.0.1:{}", TEST_PORT + 3))
            .await
            .unwrap();
        let server_addr2 = listener2.local_addr().unwrap();

        let server_file_clone2 = server_file.clone();
        let server_handle2 = tokio::spawn(async move {
            let (stream, _) = listener2.accept().await.unwrap();
            let config = ConnectionConfig::default();
            let mut server_instance = ConnectionInstance::with_config(stream, config);
            server_instance
                .server_handle_client_update(&server_file_clone2)
                .await
                .unwrap();
        });

        let client_file_clone2 = client_file.clone();
        let client_handle2 = tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            let stream = TcpStream::connect(server_addr2).await.unwrap();
            let config = ConnectionConfig::default();
            let mut client_instance = ConnectionInstance::with_config(stream, config);
            let version = client_instance
                .client_upload(&client_file_clone2)
                .await
                .unwrap();
            assert_eq!(version, 2); // Should increment to version 2
        });

        let (server_result2, client_result2) = tokio::join!(server_handle2, client_handle2);
        server_result2.unwrap();
        client_result2.unwrap();

        // Verify final content
        let server_content = read_file_content(&server_file).await.unwrap();
        assert_eq!(server_content, updated_content2);

        cleanup_test_file(&server_file).await.unwrap();
        cleanup_test_file(&client_file).await.unwrap();
    }
}
