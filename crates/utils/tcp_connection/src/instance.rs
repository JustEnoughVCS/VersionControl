use std::{path::Path, time::Duration};

use base64::{engine::general_purpose::STANDARD, prelude::*};
use rand::Rng;
use rsa::{
    RsaPrivateKey, RsaPublicKey,
    pkcs1::{DecodeRsaPrivateKey, DecodeRsaPublicKey},
    sha2,
};
use serde::Serialize;
use tokio::{
    fs::{File, OpenOptions},
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, BufWriter},
    net::TcpStream,
};
use uuid::Uuid;

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use ring::signature::{
    self, ECDSA_P256_SHA256_ASN1, ECDSA_P384_SHA384_ASN1, RSA_PKCS1_2048_8192_SHA256,
    UnparsedPublicKey,
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

// Helper trait for reading u64 from TcpStream
trait ReadU64Ext {
    async fn read_u64(&mut self) -> Result<u64, std::io::Error>;
}

impl ReadU64Ext for TcpStream {
    async fn read_u64(&mut self) -> Result<u64, std::io::Error> {
        let mut buf = [0u8; 8];
        self.read_exact(&mut buf).await?;
        Ok(u64::from_be_bytes(buf))
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
        let mut version_buf = [0u8; 8];
        self.stream
            .read_exact(&mut version_buf)
            .await
            .map_err(|e| TcpTargetError::from(e.to_string()))?;
        let version = u64::from_be_bytes(version_buf);
        if version != 1 {
            return Err(TcpTargetError::from("Unsupported transfer version"));
        }

        let mut size_buf = [0u8; 8];
        self.stream
            .read_exact(&mut size_buf)
            .await
            .map_err(|e| TcpTargetError::from(e.to_string()))?;
        let file_size = u64::from_be_bytes(size_buf);
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

    pub async fn challenge(
        &mut self,
        public_key_dir: impl AsRef<Path>,
    ) -> Result<bool, TcpTargetError> {
        // Generate random challenge
        let mut rng = rand::rng();
        let challenge: [u8; 32] = rng.random();

        // Send challenge to target
        self.stream
            .write_all(&challenge)
            .await
            .map_err(|e| TcpTargetError::from(e.to_string()))?;

        // Read signature from target
        let mut signature = Vec::new();
        let mut signature_len_buf = [0u8; 4];
        self.stream
            .read_exact(&mut signature_len_buf)
            .await
            .map_err(|e| TcpTargetError::from(e.to_string()))?;

        let signature_len = u32::from_be_bytes(signature_len_buf) as usize;
        signature.resize(signature_len, 0);
        self.stream
            .read_exact(&mut signature)
            .await
            .map_err(|e| TcpTargetError::from(e.to_string()))?;

        // Read UUID from target to identify which public key to use
        let mut uuid_buf = [0u8; 16];
        self.stream
            .read_exact(&mut uuid_buf)
            .await
            .map_err(|e| TcpTargetError::from(e.to_string()))?;
        let user_uuid = Uuid::from_bytes(uuid_buf);

        // Load appropriate public key
        let public_key_path = public_key_dir.as_ref().join(format!("{}.pub", user_uuid));
        if !public_key_path.exists() {
            return Ok(false);
        }

        let public_key_pem = tokio::fs::read_to_string(&public_key_path)
            .await
            .map_err(|e| TcpTargetError::from(e.to_string()))?;

        // Try to verify with different key types
        let verified = if let Ok(rsa_key) = RsaPublicKey::from_pkcs1_pem(&public_key_pem) {
            let padding = rsa::pkcs1v15::Pkcs1v15Sign::new::<sha2::Sha256>();
            rsa_key.verify(padding, &challenge, &signature).is_ok()
        } else if let Ok(ed25519_key) =
            VerifyingKey::from_bytes(&parse_ed25519_public_key(&public_key_pem))
        {
            let sig_bytes: [u8; 64] = signature.as_slice().try_into().unwrap_or([0u8; 64]);
            let sig = Signature::from_bytes(&sig_bytes);
            ed25519_key.verify(&challenge, &sig).is_ok()
        } else if let Ok(dsa_key_info) = parse_dsa_public_key(&public_key_pem) {
            verify_dsa_signature(&dsa_key_info, &challenge, &signature)
        } else {
            false
        };

        Ok(verified)
    }

    pub async fn accept_challenge(
        &mut self,
        private_key_file: impl AsRef<Path>,
        verify_user_uuid: Uuid,
    ) -> Result<bool, TcpTargetError> {
        // Read challenge from initiator
        let mut challenge = [0u8; 32];
        self.stream
            .read_exact(&mut challenge)
            .await
            .map_err(|e| TcpTargetError::from(e.to_string()))?;

        // Load private key
        let private_key_pem = tokio::fs::read_to_string(&private_key_file)
            .await
            .map_err(|e| TcpTargetError::from(e.to_string()))?;

        // Sign the challenge with supported key types
        let signature = if let Ok(rsa_key) = RsaPrivateKey::from_pkcs1_pem(&private_key_pem) {
            let padding = rsa::pkcs1v15::Pkcs1v15Sign::new::<sha2::Sha256>();
            rsa_key
                .sign(padding, &challenge)
                .map_err(|e| TcpTargetError::from(e.to_string()))?
        } else if let Ok(ed25519_key) = parse_ed25519_private_key(&private_key_pem) {
            ed25519_key.sign(&challenge).to_bytes().to_vec()
        } else if let Ok(dsa_key_info) = parse_dsa_private_key(&private_key_pem) {
            sign_with_dsa(&dsa_key_info, &challenge)
        } else {
            return Ok(false);
        };

        // Send signature length and signature
        let signature_len = signature.len() as u32;
        self.stream
            .write_all(&signature_len.to_be_bytes())
            .await
            .map_err(|e| TcpTargetError::from(e.to_string()))?;
        self.stream
            .write_all(&signature)
            .await
            .map_err(|e| TcpTargetError::from(e.to_string()))?;

        // Send UUID for public key identification
        self.stream
            .write_all(verify_user_uuid.as_bytes())
            .await
            .map_err(|e| TcpTargetError::from(e.to_string()))?;

        Ok(true)
    }
}

// Helper functions for Ed25519 support

/// Parse Ed25519 public key from PEM format
fn parse_ed25519_public_key(pem: &str) -> [u8; 32] {
    // Simple parsing for Ed25519 public key (assuming raw 32-byte key)
    let lines: Vec<&str> = pem.lines().collect();
    let mut key_bytes = [0u8; 32];

    if lines.len() >= 2 && lines[0].contains("PUBLIC KEY") {
        if let Ok(decoded) = STANDARD.decode(lines[1].trim()) {
            if decoded.len() >= 32 {
                key_bytes.copy_from_slice(&decoded[decoded.len() - 32..]);
            }
        }
    }
    key_bytes
}

/// Parse Ed25519 private key from PEM format
fn parse_ed25519_private_key(pem: &str) -> Result<SigningKey, TcpTargetError> {
    let lines: Vec<&str> = pem.lines().collect();

    if lines.len() >= 2 && lines[0].contains("PRIVATE KEY") {
        if let Ok(decoded) = STANDARD.decode(lines[1].trim()) {
            if decoded.len() >= 32 {
                let mut seed = [0u8; 32];
                seed.copy_from_slice(&decoded[decoded.len() - 32..]);
                return Ok(SigningKey::from_bytes(&seed));
            }
        }
    }
    Err(TcpTargetError::from("Invalid Ed25519 private key format"))
}

// Helper functions for DSA support

/// Parse DSA public key information from PEM
fn parse_dsa_public_key(
    pem: &str,
) -> Result<(&'static dyn signature::VerificationAlgorithm, Vec<u8>), TcpTargetError> {
    let lines: Vec<&str> = pem.lines().collect();

    if lines.len() >= 2 {
        if let Ok(decoded) = STANDARD.decode(lines[1].trim()) {
            // Try different DSA algorithms
            if pem.contains("ECDSA") || pem.contains("ecdsa") {
                if pem.contains("P-256") {
                    return Ok((&ECDSA_P256_SHA256_ASN1, decoded));
                } else if pem.contains("P-384") {
                    return Ok((&ECDSA_P384_SHA384_ASN1, decoded));
                }
            }
            // Default to RSA if no specific algorithm detected
            return Ok((&RSA_PKCS1_2048_8192_SHA256, decoded));
        }
    }
    Err(TcpTargetError::from("Invalid DSA public key format"))
}

/// Parse DSA private key information from PEM
fn parse_dsa_private_key(
    pem: &str,
) -> Result<(&'static dyn signature::VerificationAlgorithm, Vec<u8>), TcpTargetError> {
    // For DSA, private key verification uses the same algorithm as public key
    parse_dsa_public_key(pem)
}

/// Verify DSA signature
fn verify_dsa_signature(
    algorithm_and_key: &(&'static dyn signature::VerificationAlgorithm, Vec<u8>),
    message: &[u8],
    signature: &[u8],
) -> bool {
    let (algorithm, key_bytes) = algorithm_and_key;
    let public_key = UnparsedPublicKey::new(*algorithm, key_bytes);
    public_key.verify(message, signature).is_ok()
}

/// Sign with DSA (simplified - in practice this would use proper private key operations)
fn sign_with_dsa(
    _algorithm_and_key: &(&'static dyn signature::VerificationAlgorithm, Vec<u8>),
    message: &[u8],
) -> Vec<u8> {
    // Note: This is a simplified implementation. In a real scenario,
    // you would use proper private key signing operations with ring or other crypto library.
    // For now, we'll return a dummy signature for demonstration.
    let mut signature = vec![0u8; 64];
    signature[..32].copy_from_slice(&message[..32.min(message.len())]);
    signature
}
