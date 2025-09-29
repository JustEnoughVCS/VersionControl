use std::path::{Path, PathBuf};

use tokio::fs::{File, OpenOptions, copy, create_dir_all, read, read_to_string, remove_file};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufWriter};

use crate::{error::TcpTargetError, instance::ConnectionInstance};

// 增量传输协议版本
const INCREMENTAL_TRANSFER_VERSION: u64 = 1;
// 块大小（字节）
const DEFAULT_CHUNK_SIZE: usize = 8192;
// 哈希大小（字节）
const HASH_SIZE: usize = 32; // blake3 produces 32-byte hashes
// 版本文件扩展名
const VERSION_FILE_EXTENSION: &str = "ver";
// 版本历史目录名
const VERSION_HISTORY_DIR: &str = "diff";
// 差异文件扩展名
const DELTA_FILE_EXTENSION: &str = "delta";

// 协议模式常量
const SERVER_DELTA_MODE: u8 = 1;
const SERVER_FULL_MODE: u8 = 2;
const CLIENT_UPDATE_MODE: u8 = 1;
const CLIENT_UPLOAD_MODE: u8 = 2;
const NO_CHANGE_MODE: u8 = 3;

// 协议错误消息
const ERR_INVALID_SERVER_RESPONSE: &str = "Invalid server response format";
const ERR_VERSION_MISMATCH: &str = "Version mismatch detected";
const ERR_CHUNK_INDEX_OUT_OF_BOUNDS: &str = "Chunk index out of bounds";
const ERR_DELTA_FILE_CORRUPTED: &str = "Delta file corrupted or incomplete";

impl ConnectionInstance {
    // ==================== 客户端功能 ====================

    /// 客户端：增量更新到指定版本
    pub async fn client_update_to_version(
        &mut self,
        file_path: impl AsRef<Path>,
        target_version: i32,
    ) -> Result<(), TcpTargetError> {
        let path = file_path.as_ref();
        let current_version = self.get_current_version(path).await?;
        println!(
            "Client update: current_version={}, target_version={}",
            current_version, target_version
        );

        if current_version >= target_version {
            println!("Client update: Already up to date, skipping");
            return Ok(()); // 已经是最新版本
        }

        // 发送更新请求
        println!(
            "Client update: Sending protocol version: {}",
            INCREMENTAL_TRANSFER_VERSION
        );
        self.stream
            .write_all(&INCREMENTAL_TRANSFER_VERSION.to_be_bytes())
            .await?;
        println!("Client update: Sending mode: {}", CLIENT_UPDATE_MODE);
        self.stream.write_all(&[CLIENT_UPDATE_MODE]).await?; // 客户端更新模式
        println!("Client update: Sending target version: {}", target_version);
        self.stream.write_all(&target_version.to_be_bytes()).await?;
        self.stream.flush().await?;
        println!("Client update: Request header sent");

        // 执行增量更新
        println!("Client update: Starting incremental update...");
        let result = self
            .client_perform_incremental_update(path, current_version, target_version)
            .await;
        println!(
            "Client update: Incremental update completed, result: {:?}",
            result
        );
        result
    }

    /// 客户端：增量上传变化到服务器
    pub async fn client_upload(
        &mut self,
        file_path: impl AsRef<Path>,
    ) -> Result<i32, TcpTargetError> {
        let path = file_path.as_ref();
        let current_version = self.get_current_version(path).await?;

        // 发送上传请求
        self.stream
            .write_all(&INCREMENTAL_TRANSFER_VERSION.to_be_bytes())
            .await?;
        self.stream.write_all(&[2u8]).await?; // 客户端上传模式
        self.stream
            .write_all(&current_version.to_be_bytes())
            .await?;
        self.stream.flush().await?;

        // 执行增量上传
        let new_version = self
            .client_perform_incremental_upload(path, current_version)
            .await?;

        // 更新本地版本
        self.save_version(path, new_version).await?;

        Ok(new_version)
    }

    // ==================== 服务端功能 ====================

    /// 服务端：处理客户端更新请求
    pub async fn server_handle_client_update(
        &mut self,
        file_path: impl AsRef<Path>,
    ) -> Result<(), TcpTargetError> {
        let path = file_path.as_ref();

        println!("Server: Reading protocol version...");
        // 读取协议版本
        let mut version_buf = [0u8; 8];
        self.stream.read_exact(&mut version_buf).await?;
        let version = u64::from_be_bytes(version_buf);
        println!("Server: Received protocol version: {}", version);

        if version != INCREMENTAL_TRANSFER_VERSION {
            return Err(TcpTargetError::Protocol(
                "Unsupported incremental transfer version".to_string(),
            ));
        }

        // 读取请求模式
        let mut mode_buf = [0u8; 1];
        self.stream.read_exact(&mut mode_buf).await?;
        println!("Server: Received mode: {}", mode_buf[0]);

        match mode_buf[0] {
            CLIENT_UPDATE_MODE => {
                // 客户端更新模式
                println!("Server: Client update mode detected");
                let mut version_buf = [0u8; 4];
                self.stream.read_exact(&mut version_buf).await?;
                let target_version = i32::from_be_bytes(version_buf);
                println!("Server: Target version: {}", target_version);

                println!("Server: Sending version delta...");
                let result = self.server_send_version_delta(path, target_version).await;
                println!("Server: Version delta sent, result: {:?}", result);
                result
            }
            CLIENT_UPLOAD_MODE => {
                // 客户端上传模式
                let mut version_buf = [0u8; 4];
                self.stream.read_exact(&mut version_buf).await?;
                let client_version = i32::from_be_bytes(version_buf);

                self.server_receive_client_changes(path, client_version)
                    .await
            }
            _ => {
                return Err(TcpTargetError::Protocol(format!(
                    "{}: unknown mode {}",
                    ERR_INVALID_SERVER_RESPONSE, mode_buf[0]
                )));
            }
        }
    }

    /// 服务端：发送指定版本的增量数据
    pub async fn server_send_delta_to_version(
        &mut self,
        file_path: impl AsRef<Path>,
        from_version: i32,
        to_version: i32,
    ) -> Result<(), TcpTargetError> {
        let path = file_path.as_ref();

        // 发送响应头
        self.stream
            .write_all(&INCREMENTAL_TRANSFER_VERSION.to_be_bytes())
            .await?;
        self.stream.write_all(&[1u8]).await?; // 服务端增量模式

        self.server_send_version_delta_internal(path, from_version, to_version)
            .await
    }

    // ==================== 内部实现 ====================

    /// 客户端：执行增量更新
    async fn client_perform_incremental_update(
        &mut self,
        file_path: impl AsRef<Path>,
        current_version: i32,
        target_version: i32,
    ) -> Result<(), TcpTargetError> {
        let path = file_path.as_ref();

        // 发送客户端当前版本给服务器
        self.stream
            .write_all(&current_version.to_be_bytes())
            .await?;
        self.stream.flush().await?;

        // 读取服务器响应
        let mut version_buf = [0u8; 8];
        self.stream.read_exact(&mut version_buf).await?;
        let version = u64::from_be_bytes(version_buf);

        if version != INCREMENTAL_TRANSFER_VERSION {
            return Err(TcpTargetError::Protocol(
                "Unsupported incremental transfer version".to_string(),
            ));
        }

        let mut mode_buf = [0u8; 1];
        self.stream.read_exact(&mut mode_buf).await?;

        match mode_buf[0] {
            SERVER_DELTA_MODE => {
                // 服务端增量模式
                self.client_apply_delta(path, target_version).await
            }
            NO_CHANGE_MODE => {
                // 无变化模式，不需要更新
                Ok(())
            }
            _ => Err(TcpTargetError::Protocol(
                "Invalid server response".to_string(),
            )),
        }
    }

    /// 客户端：应用增量数据
    async fn client_apply_delta(
        &mut self,
        file_path: impl AsRef<Path>,
        target_version: i32,
    ) -> Result<(), TcpTargetError> {
        let path = file_path.as_ref();

        // 读取增量类型
        let mut delta_type_buf = [0u8; 1];
        self.stream.read_exact(&mut delta_type_buf).await?;

        match delta_type_buf[0] {
            SERVER_FULL_MODE => {
                // 完整文件传输
                self.read_file(path).await?;
            }
            SERVER_DELTA_MODE => {
                // 增量块传输
                self.client_receive_and_apply_chunks(path).await?;
            }
            NO_CHANGE_MODE => {
                // 无变化模式
                // 不需要做任何操作
            }
            _ => {
                return Err(TcpTargetError::Protocol(format!(
                    "{}: unknown mode {}",
                    ERR_INVALID_SERVER_RESPONSE, delta_type_buf[0]
                )));
            }
        }

        // 更新本地版本
        self.save_version(path, target_version).await?;

        // 发送确认
        self.stream.write_all(&[1u8]).await?;
        self.stream.flush().await?;

        Ok(())
    }

    /// 客户端：接收并应用增量块
    async fn client_receive_and_apply_chunks(
        &mut self,
        file_path: impl AsRef<Path>,
    ) -> Result<(), TcpTargetError> {
        self.receive_and_apply_chunks_internal(file_path, None)
            .await
    }

    /// 客户端：执行增量上传
    async fn client_perform_incremental_upload(
        &mut self,
        file_path: impl AsRef<Path>,
        current_version: i32,
    ) -> Result<i32, TcpTargetError> {
        let path = file_path.as_ref();

        // 读取服务器响应
        println!("Client upload: Reading server response version...");
        let mut version_buf = [0u8; 8];
        self.stream.read_exact(&mut version_buf).await?;
        let version = u64::from_be_bytes(version_buf);
        println!("Client upload: Received protocol version: {}", version);

        if version != INCREMENTAL_TRANSFER_VERSION {
            return Err(TcpTargetError::Protocol(
                "Unsupported incremental transfer version".to_string(),
            ));
        }

        println!("Client upload: Reading server response mode...");
        let mut mode_buf = [0u8; 1];
        self.stream.read_exact(&mut mode_buf).await?;
        println!("Client upload: Received mode: {}", mode_buf[0]);

        match mode_buf[0] {
            SERVER_FULL_MODE => {
                // 服务端接收模式
                let new_version = self.client_send_changes(path, current_version).await?;
                Ok(new_version)
            }
            _ => Err(TcpTargetError::Protocol(
                "Invalid server response".to_string(),
            )),
        }
    }

    /// 客户端：发送变化到服务器
    async fn client_send_changes(
        &mut self,
        file_path: impl AsRef<Path>,
        client_version: i32,
    ) -> Result<i32, TcpTargetError> {
        let path = file_path.as_ref();
        println!(
            "Client send changes: Starting to send changes, client_version={}",
            client_version
        );

        // 发送确认给服务器
        println!("Client: Sending acknowledgment to server...");
        self.stream.write_all(&[1u8]).await?;
        self.stream.flush().await?;
        println!("Client: Acknowledgment sent");

        // 发送客户端版本给服务器进行验证
        self.stream.write_all(&client_version.to_be_bytes()).await?;
        self.stream.flush().await?;

        // 计算当前文件块哈希
        let current_hashes = self.calculate_file_chunk_hashes(path).await?;
        println!(
            "Client: Calculated {} current chunk hashes",
            current_hashes.len()
        );

        // 发送块哈希
        println!("Client: Sending chunk hashes to server...");
        match self.send_chunk_hashes(&current_hashes).await {
            Ok(_) => println!("Client: Successfully sent chunk hashes"),
            Err(e) => {
                println!("Client: ERROR sending chunk hashes: {:?}", e);
                return Err(e);
            }
        }
        // Extra flush to ensure data is sent before server reads
        match self.stream.flush().await {
            Ok(_) => println!("Client: Successfully flushed stream"),
            Err(e) => {
                println!("Client: ERROR flushing stream: {:?}", e);
                return Err(e.into());
            }
        }

        // 读取服务器需要的块列表
        println!("Client: Reading chunks needed from server...");
        let chunks_to_send = self.receive_chunks_to_send().await?;
        println!("Client: Received chunks to send: {:?}", chunks_to_send);

        // 发送需要的块
        println!("Client send changes: Sending file chunks to server...");
        self.send_file_chunks(path, &chunks_to_send).await?;
        println!("Client send changes: File chunks sent");

        // 读取新版本号
        println!("Client send changes: Reading new version from server...");
        let mut version_buf = [0u8; 4];
        self.stream.read_exact(&mut version_buf).await?;
        let new_version = i32::from_be_bytes(version_buf);
        println!("Client send changes: Received new version: {}", new_version);

        Ok(new_version)
    }

    /// 服务端：发送版本增量数据
    async fn server_send_version_delta(
        &mut self,
        file_path: impl AsRef<Path>,
        target_version: i32,
    ) -> Result<(), TcpTargetError> {
        let path = file_path.as_ref();

        println!("Server send version delta: Reading client version...");
        // 获取客户端当前版本（需要从协议中读取）
        let mut client_version_buf = [0u8; 4];
        self.stream.read_exact(&mut client_version_buf).await?;
        let client_version = i32::from_be_bytes(client_version_buf);
        println!(
            "Server send version delta: Client version: {}",
            client_version
        );

        // 发送响应头
        println!(
            "Server send version delta: Sending protocol version: {}",
            INCREMENTAL_TRANSFER_VERSION
        );
        self.stream
            .write_all(&INCREMENTAL_TRANSFER_VERSION.to_be_bytes())
            .await?;
        println!(
            "Server send version delta: Sending mode: {}",
            SERVER_DELTA_MODE
        );
        self.stream.write_all(&[SERVER_DELTA_MODE]).await?; // 服务端增量模式
        self.stream.flush().await?;
        println!("Server send version delta: Response header sent");

        println!("Server send version delta: Calling internal delta function...");
        let result = self
            .server_send_version_delta_internal(path, client_version, target_version)
            .await;
        println!(
            "Server send version delta: Internal function completed, result: {:?}",
            result
        );
        result
    }

    /// 服务端：内部发送版本增量
    async fn server_send_version_delta_internal(
        &mut self,
        file_path: impl AsRef<Path>,
        from_version: i32,
        to_version: i32,
    ) -> Result<(), TcpTargetError> {
        let path = file_path.as_ref();

        println!(
            "Server send version delta internal: from_version={}, to_version={}",
            from_version, to_version
        );

        if from_version == to_version {
            // 版本相同，不需要传输
            println!(
                "Server send version delta internal: Versions are the same, sending NO_CHANGE_MODE"
            );
            self.stream.write_all(&[NO_CHANGE_MODE]).await?; // 无变化模式
            return Ok(());
        }

        // 计算版本间的差异
        println!("Server send version delta internal: Calculating version delta...");
        let delta_chunks = self
            .calculate_version_delta(path, from_version, to_version)
            .await?;
        println!(
            "Server send version delta internal: Delta chunks count: {}",
            delta_chunks.len()
        );

        if delta_chunks.is_empty() {
            // 没有变化或目标版本不存在
            println!("Server send version delta internal: No changes, sending NO_CHANGE_MODE");
            self.stream.write_all(&[NO_CHANGE_MODE]).await?; // 无变化模式
        } else {
            // 发送增量块
            println!(
                "Server send version delta internal: Sending delta chunks with SERVER_DELTA_MODE"
            );
            self.stream.write_all(&[SERVER_DELTA_MODE]).await?; // 增量块模式
            self.send_file_chunks(path, &delta_chunks).await?;
        }
        println!("Server send version delta internal: Completed successfully");
        Ok(())
    }

    /// 服务端：接收客户端变化
    async fn server_receive_client_changes(
        &mut self,
        file_path: impl AsRef<Path>,
        client_version: i32,
    ) -> Result<(), TcpTargetError> {
        let path = file_path.as_ref();
        println!("Server receive changes: Client version: {}", client_version);

        // 验证客户端版本是否匹配服务器当前版本
        let server_version = self.get_current_version(path).await?;
        if client_version != server_version {
            return Err(TcpTargetError::Protocol(format!(
                "{}: client {}, server {}",
                ERR_VERSION_MISMATCH, client_version, server_version
            )));
        }

        // 发送响应头
        println!(
            "Server receive changes: Sending protocol version: {}",
            INCREMENTAL_TRANSFER_VERSION
        );
        self.stream
            .write_all(&INCREMENTAL_TRANSFER_VERSION.to_be_bytes())
            .await?;
        println!("Server receive changes: Sending mode: {}", SERVER_FULL_MODE);
        self.stream.write_all(&[SERVER_FULL_MODE]).await?; // 服务端完整文件模式
        self.stream.flush().await?;
        println!("Server receive changes: Response header sent");

        // 接收客户端块哈希
        println!("Server receive changes: Receiving client chunk hashes...");
        // 等待客户端确认
        let mut ack_buf = [0u8; 1];
        self.stream.read_exact(&mut ack_buf).await?;
        if ack_buf[0] != 1 {
            return Err(TcpTargetError::Protocol(
                "Client acknowledgment failed".to_string(),
            ));
        }
        println!("Server receive changes: Client acknowledged, reading client version...");

        // 读取客户端版本进行验证
        let mut client_version_buf = [0u8; 4];
        self.stream.read_exact(&mut client_version_buf).await?;
        let received_client_version = i32::from_be_bytes(client_version_buf);
        println!(
            "Server receive changes: Received client version: {}",
            received_client_version
        );

        // 验证客户端版本是否匹配
        if received_client_version != client_version {
            return Err(TcpTargetError::Protocol(format!(
                "{}: expected {}, got {}",
                ERR_VERSION_MISMATCH, client_version, received_client_version
            )));
        }

        println!("Server receive changes: Reading chunk hashes...");
        let client_chunk_hashes = match self.receive_chunk_hashes().await {
            Ok(hashes) => {
                println!(
                    "Server receive changes: Successfully received {} client chunk hashes",
                    hashes.len()
                );
                hashes
            }
            Err(e) => {
                println!(
                    "Server receive changes: ERROR receiving chunk hashes: {:?}",
                    e
                );
                return Err(e);
            }
        };

        // 计算服务器当前块哈希
        let server_hashes = self.calculate_file_chunk_hashes(path).await?;
        println!(
            "Server: Calculated {} server chunk hashes",
            server_hashes.len()
        );

        // 比较差异，确定需要更新的块
        let chunks_needed = self.compare_chunk_hashes(&client_chunk_hashes, &server_hashes);
        println!(
            "Server: Chunks needed after comparison: {:?}",
            chunks_needed
        );

        // 发送需要的块列表
        println!("Server: Sending chunks needed list: {:?}", chunks_needed);
        self.send_chunks_needed(&chunks_needed).await?;

        // 接收并应用客户端块
        self.receive_and_apply_chunks(path, &chunks_needed).await?;

        // 生成新版本号（这里简化：版本号+1）
        let current_version = self.get_current_version(path).await?;
        let new_version = current_version + 1;

        // 保存差异而不是完整文件
        self.save_version_delta(path, current_version, new_version, &chunks_needed)
            .await?;
        self.save_version(path, new_version).await?;

        // 发送新版本号给客户端
        println!(
            "Server receive changes: Sending new version to client: {}",
            new_version
        );
        self.stream.write_all(&new_version.to_be_bytes()).await?;
        self.stream.flush().await?;
        println!("Server receive changes: New version sent to client");

        Ok(())
    }

    // ==================== 工具函数 ====================

    /// 计算文件的块哈希
    async fn calculate_file_chunk_hashes(
        &self,
        file_path: impl AsRef<Path>,
    ) -> Result<Vec<[u8; HASH_SIZE]>, TcpTargetError> {
        let path = file_path.as_ref();
        let mut file = File::open(path).await?;
        let file_size = file.metadata().await?.len();

        let mut hashes = Vec::new();
        let mut buffer = vec![0u8; DEFAULT_CHUNK_SIZE];
        let mut bytes_read = 0;

        while bytes_read < file_size {
            let bytes_to_read = (file_size - bytes_read).min(DEFAULT_CHUNK_SIZE as u64) as usize;
            let chunk = &mut buffer[..bytes_to_read];

            file.read_exact(chunk).await?;

            let hash = self.simple_chunk_hash(chunk);
            hashes.push(hash);

            bytes_read += bytes_to_read as u64;
        }

        Ok(hashes)
    }

    /// 简单的块哈希函数
    fn simple_chunk_hash(&self, data: &[u8]) -> [u8; HASH_SIZE] {
        // 使用稳定的blake3哈希算法，确保跨平台一致性
        let hash = blake3::hash(data);
        let mut hash_bytes = [0u8; HASH_SIZE];
        hash_bytes[..32].copy_from_slice(hash.as_bytes());
        hash_bytes
    }

    /// 发送块哈希列表
    async fn send_chunk_hashes(
        &mut self,
        hashes: &[[u8; HASH_SIZE]],
    ) -> Result<(), TcpTargetError> {
        let hash_count = hashes.len() as u32;
        println!("Client: Sending {} chunk hashes", hash_count);

        match self.stream.write_all(&hash_count.to_be_bytes()).await {
            Ok(_) => println!("Client: Successfully sent hash count"),
            Err(e) => {
                println!("Client: ERROR sending hash count: {:?}", e);
                return Err(e.into());
            }
        }
        // Extra flush to ensure hash count is sent before server reads
        match self.stream.flush().await {
            Ok(_) => println!("Client: Successfully flushed hash count"),
            Err(e) => {
                println!("Client: ERROR flushing hash count: {:?}", e);
                return Err(e.into());
            }
        }

        for (i, hash) in hashes.iter().enumerate() {
            println!("Client: Sending chunk hash {}: {:?}", i, &hash[..8]); // Show first 8 bytes for debugging
            match self.stream.write_all(hash).await {
                Ok(_) => println!("Client: Successfully sent chunk hash {}", i),
                Err(e) => {
                    println!("Client: ERROR sending chunk hash {}: {:?}", i, e);
                    return Err(e.into());
                }
            }
        }

        match self.stream.flush().await {
            Ok(_) => println!("Client: Successfully flushed after sending hashes"),
            Err(e) => {
                println!("Client: ERROR flushing after sending hashes: {:?}", e);
                return Err(e.into());
            }
        }
        println!("Client: All chunk hashes sent");
        // Extra flush to ensure data is sent before server reads
        match self.stream.flush().await {
            Ok(_) => println!("Client: Successfully flushed final"),
            Err(e) => {
                println!("Client: ERROR flushing final: {:?}", e);
                return Err(e.into());
            }
        }
        Ok(())
    }

    /// 接收块哈希列表
    async fn receive_chunk_hashes(&mut self) -> Result<Vec<[u8; HASH_SIZE]>, TcpTargetError> {
        println!("Server: Starting to receive chunk hashes...");

        let mut count_buf = [0u8; 4];
        println!("Server: Reading chunk hash count...");
        self.stream.read_exact(&mut count_buf).await?;
        let hash_count = u32::from_be_bytes(count_buf) as usize;
        println!(
            "Server: Reading {} chunk hashes, raw bytes: {:?}",
            hash_count, count_buf
        );

        let mut hashes = Vec::with_capacity(hash_count);
        let mut hash_buf = [0u8; HASH_SIZE];

        for i in 0..hash_count {
            println!("Server: Reading chunk hash {}...", i);
            self.stream.read_exact(&mut hash_buf).await?;
            println!("Server: Received chunk hash {}: {:?}", i, &hash_buf[..8]); // Show first 8 bytes for debugging
            hashes.push(hash_buf);
        }

        println!("Server: All {} chunk hashes received", hashes.len());
        Ok(hashes)
    }

    /// 比较块哈希并返回需要更新的块索引
    fn compare_chunk_hashes(
        &self,
        new_hashes: &[[u8; HASH_SIZE]],
        old_hashes: &[[u8; HASH_SIZE]],
    ) -> Vec<usize> {
        let mut chunks_needed = Vec::new();

        for (i, (new_hash, old_hash)) in new_hashes.iter().zip(old_hashes.iter()).enumerate() {
            if new_hash != old_hash {
                chunks_needed.push(i);
            }
        }

        // 如果新文件有更多块，也需要更新
        for i in old_hashes.len()..new_hashes.len() {
            chunks_needed.push(i);
        }

        chunks_needed
    }

    /// 接收需要传输的块列表
    async fn receive_chunks_to_send(&mut self) -> Result<Vec<usize>, TcpTargetError> {
        let mut count_buf = [0u8; 4];
        self.stream.read_exact(&mut count_buf).await?;
        let chunk_count = u32::from_be_bytes(count_buf) as usize;

        let mut chunks = Vec::with_capacity(chunk_count);
        let mut index_buf = [0u8; 4];

        for _ in 0..chunk_count {
            self.stream.read_exact(&mut index_buf).await?;
            let index = u32::from_be_bytes(index_buf) as usize;
            chunks.push(index);
        }

        Ok(chunks)
    }

    /// 发送需要接收的块列表
    async fn send_chunks_needed(&mut self, chunks: &[usize]) -> Result<(), TcpTargetError> {
        let chunk_count = chunks.len() as u32;
        self.stream.write_all(&chunk_count.to_be_bytes()).await?;

        for &chunk_index in chunks {
            self.stream
                .write_all(&(chunk_index as u32).to_be_bytes())
                .await?;
        }

        self.stream.flush().await?;
        Ok(())
    }

    /// 发送指定的文件块
    async fn send_file_chunks(
        &mut self,
        file_path: impl AsRef<Path>,
        chunk_indices: &[usize],
    ) -> Result<(), TcpTargetError> {
        let path = file_path.as_ref();
        let mut file = File::open(path).await?;
        let file_size = file.metadata().await?.len();

        // 发送块数量
        self.stream
            .write_all(&(chunk_indices.len() as u32).to_be_bytes())
            .await?;

        if chunk_indices.is_empty() {
            self.stream.flush().await?;
            return Ok(());
        }

        for &chunk_index in chunk_indices {
            let chunk_offset = (chunk_index * DEFAULT_CHUNK_SIZE) as u64;
            if chunk_offset >= file_size {
                continue; // 跳过超出文件范围的块
            }

            // 定位到块位置
            tokio::io::AsyncSeekExt::seek(&mut file, std::io::SeekFrom::Start(chunk_offset))
                .await?;

            // 计算块大小
            let remaining_bytes = file_size - chunk_offset;
            let chunk_size = remaining_bytes.min(DEFAULT_CHUNK_SIZE as u64) as usize;

            // 发送块索引和大小
            self.stream
                .write_all(&(chunk_index as u32).to_be_bytes())
                .await?;
            self.stream
                .write_all(&(chunk_size as u32).to_be_bytes())
                .await?;

            // 发送块数据
            let mut buffer = vec![0u8; chunk_size];
            file.read_exact(&mut buffer).await?;
            self.stream.write_all(&buffer).await?;
        }

        self.stream.flush().await?;
        Ok(())
    }

    /// 接收并应用增量块
    async fn receive_and_apply_chunks(
        &mut self,
        file_path: impl AsRef<Path>,
        chunk_indices: &[usize],
    ) -> Result<(), TcpTargetError> {
        self.receive_and_apply_chunks_internal(file_path, Some(chunk_indices))
            .await
    }

    /// 内部工具函数：接收并应用块数据
    async fn receive_and_apply_chunks_internal(
        &mut self,
        file_path: impl AsRef<Path>,
        expected_indices: Option<&[usize]>,
    ) -> Result<(), TcpTargetError> {
        let path = file_path.as_ref();

        // 读取块数量
        let mut count_buf = [0u8; 4];
        self.stream.read_exact(&mut count_buf).await?;
        let chunk_count = u32::from_be_bytes(count_buf) as usize;

        // 创建临时文件来接收完整内容
        let temp_path = path.with_extension("tmp");
        let temp_file = File::create(&temp_path).await?;
        let mut temp_writer = BufWriter::new(temp_file);

        // 记录接收到的块数据
        let mut received_chunks = Vec::new();

        for i in 0..chunk_count {
            // 读取块索引和大小
            let mut index_buf = [0u8; 4];
            self.stream.read_exact(&mut index_buf).await?;
            let chunk_index = u32::from_be_bytes(index_buf) as usize;

            // 验证块索引（如果提供了预期索引）
            if let Some(expected) = expected_indices {
                if i >= expected.len() || chunk_index != expected[i] {
                    return Err(TcpTargetError::Protocol(format!(
                        "{}: expected {:?}, got {}",
                        ERR_CHUNK_INDEX_OUT_OF_BOUNDS,
                        expected.get(i),
                        chunk_index
                    )));
                }
            }

            let mut size_buf = [0u8; 4];
            self.stream.read_exact(&mut size_buf).await?;
            let chunk_size = u32::from_be_bytes(size_buf) as usize;

            // 读取块数据
            let mut chunk_data = vec![0u8; chunk_size];
            self.stream.read_exact(&mut chunk_data).await?;

            // 记录接收到的块
            received_chunks.push((chunk_index, chunk_data));
        }

        // 按块索引排序并写入临时文件
        received_chunks.sort_by_key(|(index, _)| *index);

        for (chunk_index, chunk_data) in received_chunks {
            let chunk_offset = (chunk_index * DEFAULT_CHUNK_SIZE) as u64;
            tokio::io::AsyncSeekExt::seek(&mut temp_writer, std::io::SeekFrom::Start(chunk_offset))
                .await?;
            temp_writer.write_all(&chunk_data).await?;
        }

        temp_writer.flush().await?;

        // 替换原文件
        tokio::fs::rename(&temp_path, path).await?;

        Ok(())
    }

    /// 获取当前文件版本
    async fn get_current_version(
        &self,
        file_path: impl AsRef<Path>,
    ) -> Result<i32, TcpTargetError> {
        let path = file_path.as_ref();
        let version_file_path = path.with_extension(VERSION_FILE_EXTENSION);

        if !version_file_path.exists() {
            return Ok(0); // 默认版本为0
        }

        let version_content = read_to_string(&version_file_path)
            .await
            .map_err(|e| TcpTargetError::File(format!("Failed to read version file: {}", e)))?;

        version_content
            .trim()
            .parse::<i32>()
            .map_err(|e| TcpTargetError::File(format!("Invalid version format: {}", e)))
    }

    /// 保存文件版本
    async fn save_version(
        &self,
        file_path: impl AsRef<Path>,
        version: i32,
    ) -> Result<(), TcpTargetError> {
        let path = file_path.as_ref();
        let version_file_path = path.with_extension(VERSION_FILE_EXTENSION);

        tokio::fs::write(&version_file_path, version.to_string())
            .await
            .map_err(|e| TcpTargetError::File(format!("Failed to write version file: {}", e)))?;

        Ok(())
    }

    /// 计算版本间的差异
    async fn calculate_version_delta(
        &self,
        file_path: impl AsRef<Path>,
        from_version: i32,
        to_version: i32,
    ) -> Result<Vec<usize>, TcpTargetError> {
        let path = file_path.as_ref();

        // 如果目标版本不存在，返回空差异
        let current_version = self.get_current_version(path).await?;
        if to_version > current_version {
            return Ok(Vec::new());
        }

        // 获取源版本和目标版本的块哈希
        let from_hashes = self.get_version_chunk_hashes(path, from_version).await?;
        let to_hashes = self.get_version_chunk_hashes(path, to_version).await?;

        // 比较差异
        let delta_chunks = self.compare_chunk_hashes(&to_hashes, &from_hashes);

        Ok(delta_chunks)
    }

    /// 获取指定版本的块哈希
    async fn get_version_chunk_hashes(
        &self,
        file_path: impl AsRef<Path>,
        version: i32,
    ) -> Result<Vec<[u8; HASH_SIZE]>, TcpTargetError> {
        let path = file_path.as_ref();

        if version == 0 {
            // 版本0表示空文件
            return Ok(Vec::new());
        }

        // 从版本历史中重建文件并计算哈希
        let reconstructed_path = self.reconstruct_version(path, version).await?;
        let hashes = self
            .calculate_file_chunk_hashes(&reconstructed_path)
            .await?;

        // 清理临时文件
        let _ = remove_file(&reconstructed_path).await;

        Ok(hashes)
    }

    /// 从差异重建指定版本的文件
    async fn reconstruct_version(
        &self,
        file_path: impl AsRef<Path>,
        target_version: i32,
    ) -> Result<PathBuf, TcpTargetError> {
        let path = file_path.as_ref();
        let temp_path = path.with_extension(format!("temp_{}", target_version));

        if target_version == 0 {
            // 创建空文件
            tokio::fs::write(&temp_path, b"").await?;
            return Ok(temp_path);
        }

        // 从版本0开始，逐步应用差异
        let mut current_version = 0;
        let mut current_path = path.with_extension("temp_base");
        tokio::fs::write(&current_path, b"").await?; // 创建基础空文件

        while current_version < target_version {
            let next_version = current_version + 1;
            let (delta_chunks, chunk_data_list) = self
                .load_version_delta(path, current_version, next_version)
                .await?;

            // 应用差异到新文件
            let next_path = path.with_extension(format!("temp_{}", next_version));
            self.apply_delta_to_file(&current_path, &next_path, &delta_chunks, &chunk_data_list)
                .await?;

            // 清理旧临时文件
            if current_version > 0 {
                let _ = tokio::fs::remove_file(&current_path).await;
            }
            current_path = next_path;
            current_version = next_version;
        }

        Ok(current_path)
    }

    /// 保存版本差异
    async fn save_version_delta(
        &self,
        file_path: impl AsRef<Path>,
        from_version: i32,
        to_version: i32,
        changed_chunks: &[usize],
    ) -> Result<(), TcpTargetError> {
        let path = file_path.as_ref();

        // 创建版本历史目录
        let history_dir = path
            .parent()
            .ok_or_else(|| TcpTargetError::File("Invalid file path".to_string()))?
            .join(VERSION_HISTORY_DIR);

        if !history_dir.exists() {
            create_dir_all(&history_dir).await?;
        }

        // 保存差异信息（记录变化的块索引和实际数据）
        let delta_path = history_dir.join(format!(
            "{}_{}_{}.{}",
            path.file_name()
                .ok_or_else(|| TcpTargetError::File("Invalid file name".to_string()))?
                .to_string_lossy(),
            from_version,
            to_version,
            DELTA_FILE_EXTENSION
        ));

        // 读取当前版本文件以获取变化块的实际数据
        let current_file_data = read(path).await?;

        let mut delta_data = Vec::new();

        for &chunk_index in changed_chunks {
            // 写入块索引
            delta_data.extend_from_slice(&(chunk_index as u32).to_be_bytes());

            // 计算块的起始和结束位置
            let chunk_start = chunk_index * DEFAULT_CHUNK_SIZE;
            let chunk_end =
                std::cmp::min(chunk_start + DEFAULT_CHUNK_SIZE, current_file_data.len());

            // 写入块数据大小
            let chunk_size = (chunk_end - chunk_start) as u32;
            delta_data.extend_from_slice(&chunk_size.to_be_bytes());

            // 写入块数据
            delta_data.extend_from_slice(&current_file_data[chunk_start..chunk_end]);
        }

        tokio::fs::write(&delta_path, &delta_data).await?;

        Ok(())
    }

    /// 加载版本差异
    async fn load_version_delta(
        &self,
        file_path: impl AsRef<Path>,
        from_version: i32,
        to_version: i32,
    ) -> Result<(Vec<usize>, Vec<Vec<u8>>), TcpTargetError> {
        let path = file_path.as_ref();
        let history_dir = path.parent().unwrap().join(VERSION_HISTORY_DIR);

        let delta_path = history_dir.join(format!(
            "{}_{}_{}.{}",
            path.file_name().unwrap().to_string_lossy(),
            from_version,
            to_version,
            DELTA_FILE_EXTENSION
        ));

        if !delta_path.exists() {
            return Ok((Vec::new(), Vec::new()));
        }

        let delta_data = read(&delta_path).await?;
        let mut chunks = Vec::new();
        let mut chunk_data_list = Vec::new();

        let mut offset = 0;
        while offset < delta_data.len() {
            // 读取块索引 (4 bytes)
            if offset + 4 > delta_data.len() {
                return Err(TcpTargetError::File(format!(
                    "{}: incomplete index data",
                    ERR_DELTA_FILE_CORRUPTED
                )));
            }
            let index = u32::from_be_bytes([
                delta_data[offset],
                delta_data[offset + 1],
                delta_data[offset + 2],
                delta_data[offset + 3],
            ]) as usize;
            offset += 4;

            // 读取块大小 (4 bytes)
            if offset + 4 > delta_data.len() {
                return Err(TcpTargetError::File(format!(
                    "{}: incomplete size data",
                    ERR_DELTA_FILE_CORRUPTED
                )));
            }
            let chunk_size = u32::from_be_bytes([
                delta_data[offset],
                delta_data[offset + 1],
                delta_data[offset + 2],
                delta_data[offset + 3],
            ]) as usize;
            offset += 4;

            // 读取块数据
            if offset + chunk_size > delta_data.len() {
                return Err(TcpTargetError::File(format!(
                    "{}: incomplete chunk data",
                    ERR_DELTA_FILE_CORRUPTED
                )));
            }
            let chunk_data = delta_data[offset..offset + chunk_size].to_vec();
            offset += chunk_size;

            chunks.push(index);
            chunk_data_list.push(chunk_data);
        }

        Ok((chunks, chunk_data_list))
    }

    /// 应用差异到文件
    async fn apply_delta_to_file(
        &self,
        source_path: impl AsRef<Path>,
        target_path: impl AsRef<Path>,
        delta_chunks: &[usize],
        chunk_data_list: &[Vec<u8>],
    ) -> Result<(), TcpTargetError> {
        let source_path = source_path.as_ref();
        let target_path = target_path.as_ref();

        // 复制源文件到目标文件
        copy(source_path, target_path).await?;

        if delta_chunks.is_empty() {
            return Ok(());
        }

        // 打开目标文件进行修改
        let mut file = OpenOptions::new().write(true).open(target_path).await?;

        for (i, &chunk_index) in delta_chunks.iter().enumerate() {
            let chunk_offset = (chunk_index * DEFAULT_CHUNK_SIZE) as u64;
            tokio::io::AsyncSeekExt::seek(&mut file, std::io::SeekFrom::Start(chunk_offset))
                .await?;

            // 使用从delta文件中读取的实际块数据进行写入
            if i < chunk_data_list.len() {
                file.write_all(&chunk_data_list[i]).await?;
            }
        }

        Ok(())
    }
}
