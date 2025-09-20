use std::{path::Path, time::Duration};

use serde::Serialize;
use tokio::{
    fs::{File, OpenOptions},
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, BufWriter},
    net::TcpStream,
};

use crate::error::TcpTargetError;

const CHUNK_SIZE: usize = 8 * 1024;

pub struct ConnectionInstance {
    stream: TcpStream,
}

impl From<TcpStream> for ConnectionInstance {
    fn from(value: TcpStream) -> Self {
        Self { stream: value }
    }
}

impl ConnectionInstance {
    /// Serialize data and write to the target machine
    pub async fn write<Data>(&mut self, data: Data) -> Result<(), TcpTargetError>
    where
        Data: Default + Serialize,
    {
        let Ok(json_text) = serde_json::to_string(&data) else {
            return Err(TcpTargetError::from("Serialize failed."));
        };
        Self::write_text(self, json_text).await?;
        Ok(())
    }

    /// Read data from target machine and deserialize
    pub async fn read<Data>(&mut self, buffer_size: impl Into<u32>) -> Result<Data, TcpTargetError>
    where
        Data: Default + serde::de::DeserializeOwned,
    {
        let Ok(json_text) = Self::read_text(self, buffer_size).await else {
            return Err(TcpTargetError::from("Read failed."));
        };
        let Ok(deser_obj) = serde_json::from_str::<Data>(&json_text) else {
            return Err(TcpTargetError::from("Deserialize failed."));
        };
        Ok(deser_obj)
    }

    /// Serialize data and write to the target machine
    pub async fn write_large<Data>(&mut self, data: Data) -> Result<(), TcpTargetError>
    where
        Data: Default + Serialize,
    {
        let Ok(json_text) = serde_json::to_string(&data) else {
            return Err(TcpTargetError::from("Serialize failed."));
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
            return Err(TcpTargetError::from("Read failed."));
        };
        let Ok(deser_obj) = serde_json::from_str::<Data>(&json_text) else {
            return Err(TcpTargetError::from("Deserialize failed."));
        };
        Ok(deser_obj)
    }

    /// Write text to the target machine
    pub async fn write_text(&mut self, text: impl Into<String>) -> Result<(), TcpTargetError> {
        // Parse text
        let text = text.into();
        // Write
        match self.stream.write_all(text.as_bytes()).await {
            Ok(_) => Ok(()),
            Err(err) => Err(TcpTargetError::from(err.to_string())),
        }
    }

    /// Read text from the target machine
    pub async fn read_text(
        &mut self,
        buffer_size: impl Into<u32>,
    ) -> Result<String, TcpTargetError> {
        // Create buffer
        let mut buffer = vec![0; buffer_size.into() as usize];
        // Read
        match self.stream.read(&mut buffer).await {
            Ok(n) => {
                let text = String::from_utf8_lossy(&buffer[..n]).to_string();
                Ok(text)
            }
            Err(err) => Err(TcpTargetError::from(err.to_string())),
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
                Err(err) => return Err(TcpTargetError::from(err.to_string())),
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
                Err(err) => return Err(TcpTargetError::from(err.to_string())),
            }
        }

        Ok(String::from_utf8_lossy(&buffer).to_string())
    }

    /// Write file to target machine.
    pub async fn write_file(&mut self, file_path: impl AsRef<Path>) -> Result<(), TcpTargetError> {
        let path = file_path.as_ref();

        // Validate file
        if !path.exists() {
            return Err(TcpTargetError::from(format!(
                "File not found: {}",
                path.display()
            )));
        }
        if path.is_dir() {
            return Err(TcpTargetError::from(format!(
                "Path is directory: {}",
                path.display()
            )));
        }

        // Open file and get metadata
        let mut file = File::open(path)
            .await
            .map_err(|e| TcpTargetError::from(e.to_string()))?;
        let file_size = file
            .metadata()
            .await
            .map_err(|e| TcpTargetError::from(e.to_string()))?
            .len();
        if file_size == 0 {
            return Err(TcpTargetError::from("Cannot send empty file"));
        }

        // Send file header (version + size)
        self.stream
            .write_all(&1u64.to_be_bytes())
            .await
            .map_err(|e| TcpTargetError::from(e.to_string()))?;
        self.stream
            .write_all(&file_size.to_be_bytes())
            .await
            .map_err(|e| TcpTargetError::from(e.to_string()))?;

        // Transfer file content
        let mut reader = BufReader::with_capacity(CHUNK_SIZE, &mut file);
        let mut bytes_sent = 0;

        while bytes_sent < file_size {
            let buffer = reader
                .fill_buf()
                .await
                .map_err(|e| TcpTargetError::from(e.to_string()))?;
            if buffer.is_empty() {
                break;
            }

            let chunk_size = buffer.len().min((file_size - bytes_sent) as usize);
            self.stream
                .write_all(&buffer[..chunk_size])
                .await
                .map_err(|e| TcpTargetError::from(e.to_string()))?;
            reader.consume(chunk_size);

            bytes_sent += chunk_size as u64;
        }

        // Verify transfer completion
        if bytes_sent != file_size {
            return Err(TcpTargetError::from(format!(
                "Transfer incomplete: expected {} bytes, sent {} bytes",
                file_size, bytes_sent
            )));
        }

        self.stream
            .flush()
            .await
            .map_err(|e| TcpTargetError::from(e.to_string()))?;

        // Wait for receiver confirmation
        let mut ack = [0u8; 1];
        tokio::time::timeout(Duration::from_secs(10), self.stream.read_exact(&mut ack))
            .await
            .map_err(|e| TcpTargetError::from(e.to_string()))?
            .map_err(|e| TcpTargetError::from(e.to_string()))?;

        if ack[0] != 1 {
            return Err(TcpTargetError::from("Receiver verification failed"));
        }

        Ok(())
    }

    /// Read file from target machine
    pub async fn read_file(&mut self, save_path: impl AsRef<Path>) -> Result<(), TcpTargetError> {
        let path = save_path.as_ref();

        // Make sure parent directory exists
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(|e| TcpTargetError::from(e.to_string()))?;
            }
        }

        // Read file header (version + size)
        let version = self
            .stream
            .read_u64()
            .await
            .map_err(|e| TcpTargetError::from(e.to_string()))?;
        if version != 1 {
            return Err(TcpTargetError::from("Unsupported transfer version"));
        }

        let file_size = self
            .stream
            .read_u64()
            .await
            .map_err(|e| TcpTargetError::from(e.to_string()))?;
        if file_size == 0 {
            return Err(TcpTargetError::from("Cannot receive zero-length file"));
        }

        // Prepare output file
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)
            .await
            .map_err(|e| TcpTargetError::from(e.to_string()))?;
        let mut writer = BufWriter::with_capacity(CHUNK_SIZE, file);

        // Receive file content
        let mut buffer = vec![0u8; CHUNK_SIZE];
        let mut bytes_received = 0;

        while bytes_received < file_size {
            let read_size = buffer.len().min((file_size - bytes_received) as usize);
            self.stream
                .read_exact(&mut buffer[..read_size])
                .await
                .map_err(|e| TcpTargetError::from(e.to_string()))?;

            writer
                .write_all(&buffer[..read_size])
                .await
                .map_err(|e| TcpTargetError::from(e.to_string()))?;
            bytes_received += read_size as u64;
        }

        // Final flush and sync
        writer
            .flush()
            .await
            .map_err(|e| TcpTargetError::from(e.to_string()))?;
        writer
            .into_inner()
            .sync_all()
            .await
            .map_err(|e| TcpTargetError::from(e.to_string()))?;

        // Verify completion
        if bytes_received != file_size {
            let _ = tokio::fs::remove_file(path).await;
            return Err(TcpTargetError::from(format!(
                "Transfer incomplete: expected {} bytes, received {} bytes",
                file_size, bytes_received
            )));
        }

        // Send confirmation
        self.stream
            .write_all(&[1])
            .await
            .map_err(|e| TcpTargetError::from(e.to_string()))?;
        self.stream
            .flush()
            .await
            .map_err(|e| TcpTargetError::from(e.to_string()))?;

        Ok(())
    }
}
