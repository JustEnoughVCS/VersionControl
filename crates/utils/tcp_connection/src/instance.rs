use std::{path::Path, time::Duration};

use serde::Serialize;
use tokio::{
    fs::{File, OpenOptions},
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, BufWriter},
    net::TcpStream,
};

use ring::signature::{self};

use crate::error::TcpTargetError;

const DEFAULT_CHUNK_SIZE: usize = 4096;
const DEFAULT_TIMEOUT_SECS: u64 = 10;

const ECDSA_P256_SHA256_ASN1_SIGNING: &signature::EcdsaSigningAlgorithm =
    &signature::ECDSA_P256_SHA256_ASN1_SIGNING;
const ECDSA_P384_SHA384_ASN1_SIGNING: &signature::EcdsaSigningAlgorithm =
    &signature::ECDSA_P384_SHA384_ASN1_SIGNING;

#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    pub chunk_size: usize,
    pub timeout_secs: u64,
    pub enable_crc_validation: bool,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            chunk_size: DEFAULT_CHUNK_SIZE,
            timeout_secs: DEFAULT_TIMEOUT_SECS,
            enable_crc_validation: false,
        }
    }
}

pub struct ConnectionInstance {
    pub(crate) stream: TcpStream,
    config: ConnectionConfig,
}

impl From<TcpStream> for ConnectionInstance {
    fn from(stream: TcpStream) -> Self {
        Self {
            stream,
            config: ConnectionConfig::default(),
        }
    }
}

impl ConnectionInstance {
    /// Create a new ConnectionInstance with custom configuration
    pub fn with_config(stream: TcpStream, config: ConnectionConfig) -> Self {
        Self { stream, config }
    }

    /// Get a reference to the current configuration
    pub fn config(&self) -> &ConnectionConfig {
        &self.config
    }

    /// Get a mutable reference to the current configuration
    pub fn config_mut(&mut self) -> &mut ConnectionConfig {
        &mut self.config
    }
    /// Serialize data and write to the target machine
    pub async fn write<Data>(&mut self, data: Data) -> Result<(), TcpTargetError>
    where
        Data: Default + Serialize,
    {
        let Ok(json_text) = serde_json::to_string(&data) else {
            return Err(TcpTargetError::Serialization(
                "Serialize failed.".to_string(),
            ));
        };
        Self::write_text(self, json_text).await?;
        Ok(())
    }

    /// Serialize data to MessagePack and write to the target machine
    pub async fn write_msgpack<Data>(&mut self, data: Data) -> Result<(), TcpTargetError>
    where
        Data: Serialize,
    {
        let msgpack_data = rmp_serde::to_vec(&data)?;
        let len = msgpack_data.len() as u32;

        self.stream.write_all(&len.to_be_bytes()).await?;
        self.stream.write_all(&msgpack_data).await?;
        Ok(())
    }

    /// Read data from target machine and deserialize from MessagePack
    pub async fn read_msgpack<Data>(&mut self) -> Result<Data, TcpTargetError>
    where
        Data: serde::de::DeserializeOwned,
    {
        let mut len_buf = [0u8; 4];
        self.stream.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;

        let mut buffer = vec![0; len];
        self.stream.read_exact(&mut buffer).await?;

        let data = rmp_serde::from_slice(&buffer)?;
        Ok(data)
    }

    /// Read data from target machine and deserialize
    pub async fn read<Data>(&mut self) -> Result<Data, TcpTargetError>
    where
        Data: Default + serde::de::DeserializeOwned,
    {
        let Ok(json_text) = Self::read_text(self).await else {
            return Err(TcpTargetError::Io("Read failed.".to_string()));
        };
        let Ok(deser_obj) = serde_json::from_str::<Data>(&json_text) else {
            return Err(TcpTargetError::Serialization(
                "Deserialize failed.".to_string(),
            ));
        };
        Ok(deser_obj)
    }

    /// Serialize data and write to the target machine
    pub async fn write_large<Data>(&mut self, data: Data) -> Result<(), TcpTargetError>
    where
        Data: Default + Serialize,
    {
        let Ok(json_text) = serde_json::to_string(&data) else {
            return Err(TcpTargetError::Serialization(
                "Serialize failed.".to_string(),
            ));
        };
        Self::write_large_text(self, json_text).await?;
        Ok(())
    }

    /// Read data from target machine and deserialize
    pub async fn read_large<Data>(
        &mut self,
        buffer_size: impl Into<u32>,
    ) -> Result<Data, TcpTargetError>
    where
        Data: Default + serde::de::DeserializeOwned,
    {
        let Ok(json_text) = Self::read_large_text(self, buffer_size).await else {
            return Err(TcpTargetError::Io("Read failed.".to_string()));
        };
        let Ok(deser_obj) = serde_json::from_str::<Data>(&json_text) else {
            return Err(TcpTargetError::Serialization(
                "Deserialize failed.".to_string(),
            ));
        };
        Ok(deser_obj)
    }

    /// Write text to the target machine
    pub async fn write_text(&mut self, text: impl Into<String>) -> Result<(), TcpTargetError> {
        let text = text.into();
        let bytes = text.as_bytes();
        let len = bytes.len() as u32;

        self.stream.write_all(&len.to_be_bytes()).await?;
        match self.stream.write_all(bytes).await {
            Ok(_) => Ok(()),
            Err(err) => Err(TcpTargetError::Io(err.to_string())),
        }
    }

    /// Read text from the target machine
    pub async fn read_text(&mut self) -> Result<String, TcpTargetError> {
        let mut len_buf = [0u8; 4];
        self.stream.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;

        let mut buffer = vec![0; len];
        self.stream.read_exact(&mut buffer).await?;

        match String::from_utf8(buffer) {
            Ok(text) => Ok(text),
            Err(err) => Err(TcpTargetError::Serialization(format!(
                "Invalid UTF-8 sequence: {}",
                err
            ))),
        }
    }

    /// Write large text to the target machine (chunked)
    pub async fn write_large_text(
        &mut self,
        text: impl Into<String>,
    ) -> Result<(), TcpTargetError> {
        let text = text.into();
        let bytes = text.as_bytes();
        let mut offset = 0;

        while offset < bytes.len() {
            let chunk = &bytes[offset..];
            let written = match self.stream.write(chunk).await {
                Ok(n) => n,
                Err(err) => return Err(TcpTargetError::Io(err.to_string())),
            };
            offset += written;
        }

        Ok(())
    }

    /// Read large text from the target machine (chunked)
    pub async fn read_large_text(
        &mut self,
        chunk_size: impl Into<u32>,
    ) -> Result<String, TcpTargetError> {
        let chunk_size = chunk_size.into() as usize;
        let mut buffer = Vec::new();
        let mut chunk_buf = vec![0; chunk_size];

        loop {
            match self.stream.read(&mut chunk_buf).await {
                Ok(0) => break, // EOF
                Ok(n) => {
                    buffer.extend_from_slice(&chunk_buf[..n]);
                }
                Err(err) => return Err(TcpTargetError::Io(err.to_string())),
            }
        }

        Ok(String::from_utf8_lossy(&buffer).to_string())
    }

    /// Write large MessagePack data to the target machine (chunked)
    pub async fn write_large_msgpack<Data>(
        &mut self,
        data: Data,
        chunk_size: impl Into<u32>,
    ) -> Result<(), TcpTargetError>
    where
        Data: Serialize,
    {
        let msgpack_data = rmp_serde::to_vec(&data)?;
        let chunk_size = chunk_size.into() as usize;
        let len = msgpack_data.len() as u32;

        // Write total length first
        self.stream.write_all(&len.to_be_bytes()).await?;

        // Write data in chunks
        let mut offset = 0;
        while offset < msgpack_data.len() {
            let end = std::cmp::min(offset + chunk_size, msgpack_data.len());
            let chunk = &msgpack_data[offset..end];
            match self.stream.write(chunk).await {
                Ok(n) => offset += n,
                Err(err) => return Err(TcpTargetError::Io(err.to_string())),
            }
        }

        Ok(())
    }

    /// Read large MessagePack data from the target machine (chunked)
    pub async fn read_large_msgpack<Data>(
        &mut self,
        chunk_size: impl Into<u32>,
    ) -> Result<Data, TcpTargetError>
    where
        Data: serde::de::DeserializeOwned,
    {
        let chunk_size = chunk_size.into() as usize;

        // Read total length first
        let mut len_buf = [0u8; 4];
        self.stream.read_exact(&mut len_buf).await?;
        let total_len = u32::from_be_bytes(len_buf) as usize;

        // Read data in chunks
        let mut buffer = Vec::with_capacity(total_len);
        let mut remaining = total_len;
        let mut chunk_buf = vec![0; chunk_size];

        while remaining > 0 {
            let read_size = std::cmp::min(chunk_size, remaining);
            let chunk = &mut chunk_buf[..read_size];

            match self.stream.read_exact(chunk).await {
                Ok(_) => {
                    buffer.extend_from_slice(chunk);
                    remaining -= read_size;
                }
                Err(err) => return Err(TcpTargetError::Io(err.to_string())),
            }
        }

        let data = rmp_serde::from_slice(&buffer)?;
        Ok(data)
    }

    /// Write file to target machine.
    pub async fn write_file(&mut self, file_path: impl AsRef<Path>) -> Result<(), TcpTargetError> {
        let path = file_path.as_ref();

        // Validate file
        if !path.exists() {
            return Err(TcpTargetError::File(format!(
                "File not found: {}",
                path.display()
            )));
        }
        if path.is_dir() {
            return Err(TcpTargetError::File(format!(
                "Path is directory: {}",
                path.display()
            )));
        }

        // Open file and get metadata
        let mut file = File::open(path).await?;
        let file_size = file.metadata().await?.len();

        // Send file header (version + size + crc)
        self.stream.write_all(&1u64.to_be_bytes()).await?;
        self.stream.write_all(&file_size.to_be_bytes()).await?;

        // Calculate and send CRC32 if enabled
        let file_crc = if self.config.enable_crc_validation {
            let crc32 = crc::Crc::<u32>::new(&crc::CRC_32_ISO_HDLC);
            let mut crc_calculator = crc32.digest();

            let mut temp_reader =
                BufReader::with_capacity(self.config.chunk_size, File::open(path).await?);
            let mut temp_buffer = vec![0u8; self.config.chunk_size];
            let mut temp_bytes_read = 0;

            while temp_bytes_read < file_size {
                let bytes_to_read =
                    (file_size - temp_bytes_read).min(self.config.chunk_size as u64) as usize;
                temp_reader
                    .read_exact(&mut temp_buffer[..bytes_to_read])
                    .await?;
                crc_calculator.update(&temp_buffer[..bytes_to_read]);
                temp_bytes_read += bytes_to_read as u64;
            }

            crc_calculator.finalize()
        } else {
            0
        };

        self.stream.write_all(&file_crc.to_be_bytes()).await?;

        // Transfer file content
        let mut reader = BufReader::with_capacity(self.config.chunk_size, &mut file);
        let mut bytes_sent = 0;

        while bytes_sent < file_size {
            let buffer = reader.fill_buf().await?;
            if buffer.is_empty() {
                break;
            }

            let chunk_size = buffer.len().min((file_size - bytes_sent) as usize);
            self.stream.write_all(&buffer[..chunk_size]).await?;
            reader.consume(chunk_size);

            bytes_sent += chunk_size as u64;
        }

        // Verify transfer completion
        if bytes_sent != file_size {
            return Err(TcpTargetError::File(format!(
                "Transfer incomplete: expected {} bytes, sent {} bytes",
                file_size, bytes_sent
            )));
        }

        self.stream.flush().await?;

        // Wait for receiver confirmation
        let mut ack = [0u8; 1];
        tokio::time::timeout(
            Duration::from_secs(self.config.timeout_secs),
            self.stream.read_exact(&mut ack),
        )
        .await
        .map_err(|_| TcpTargetError::Timeout("Ack timeout".to_string()))??;

        if ack[0] != 1 {
            return Err(TcpTargetError::Protocol(
                "Receiver verification failed".to_string(),
            ));
        }

        Ok(())
    }

    /// Read file from target machine
    pub async fn read_file(&mut self, save_path: impl AsRef<Path>) -> Result<(), TcpTargetError> {
        let path = save_path.as_ref();
        // Create CRC instance at function scope to ensure proper lifetime
        let crc_instance = crc::Crc::<u32>::new(&crc::CRC_32_ISO_HDLC);

        // Make sure parent directory exists
        if let Some(parent) = path.parent()
            && !parent.exists()
        {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Read file header (version + size + crc)
        let mut version_buf = [0u8; 8];
        self.stream.read_exact(&mut version_buf).await?;
        let version = u64::from_be_bytes(version_buf);
        if version != 1 {
            return Err(TcpTargetError::Protocol(
                "Unsupported transfer version".to_string(),
            ));
        }

        let mut size_buf = [0u8; 8];
        self.stream.read_exact(&mut size_buf).await?;
        let file_size = u64::from_be_bytes(size_buf);

        let mut expected_crc_buf = [0u8; 4];
        self.stream.read_exact(&mut expected_crc_buf).await?;
        let expected_crc = u32::from_be_bytes(expected_crc_buf);
        if file_size == 0 {
            // Create empty file and return early
            let _file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(path)
                .await?;
            return Ok(());
        }

        // Prepare output file
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)
            .await?;
        let mut writer = BufWriter::with_capacity(self.config.chunk_size, file);

        // Receive file content with CRC calculation if enabled
        let mut bytes_received = 0;
        let mut buffer = vec![0u8; self.config.chunk_size];
        let mut crc_calculator = if self.config.enable_crc_validation {
            Some(crc_instance.digest())
        } else {
            None
        };

        while bytes_received < file_size {
            let bytes_to_read =
                (file_size - bytes_received).min(self.config.chunk_size as u64) as usize;
            let chunk = &mut buffer[..bytes_to_read];

            self.stream.read_exact(chunk).await?;

            writer.write_all(chunk).await?;

            // Update CRC if validation is enabled
            if let Some(ref mut crc) = crc_calculator {
                crc.update(chunk);
            }

            bytes_received += bytes_to_read as u64;
        }

        // Verify transfer completion
        if bytes_received != file_size {
            return Err(TcpTargetError::File(format!(
                "Transfer incomplete: expected {} bytes, received {} bytes",
                file_size, bytes_received
            )));
        }

        writer.flush().await?;

        // Validate CRC if enabled
        if self.config.enable_crc_validation
            && let Some(crc_calculator) = crc_calculator
        {
            let actual_crc = crc_calculator.finalize();
            if actual_crc != expected_crc && expected_crc != 0 {
                return Err(TcpTargetError::File(format!(
                    "CRC validation failed: expected {:08x}, got {:08x}",
                    expected_crc, actual_crc
                )));
            }
        }

        // Final flush and sync
        writer.flush().await?;
        writer.into_inner().sync_all().await?;

        // Verify completion
        if bytes_received != file_size {
            let _ = tokio::fs::remove_file(path).await;
            return Err(TcpTargetError::File(format!(
                "Transfer incomplete: expected {} bytes, received {} bytes",
                file_size, bytes_received
            )));
        }

        // Send confirmation
        self.stream.write_all(&[1u8]).await?;
        self.stream.flush().await?;

        Ok(())
    }
}
