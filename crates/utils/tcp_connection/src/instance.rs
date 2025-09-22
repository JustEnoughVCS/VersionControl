use std::{path::Path, time::Duration};

use rand::TryRngCore;
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

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use ring::rand::SystemRandom;
use ring::signature::{
    self, ECDSA_P256_SHA256_ASN1, ECDSA_P384_SHA384_ASN1, EcdsaKeyPair, RSA_PKCS1_2048_8192_SHA256,
    UnparsedPublicKey,
};

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
    stream: TcpStream,
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
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                tokio::fs::create_dir_all(parent).await?;
            }
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
        if self.config.enable_crc_validation {
            if let Some(crc_calculator) = crc_calculator {
                let actual_crc = crc_calculator.finalize();
                if actual_crc != expected_crc && expected_crc != 0 {
                    return Err(TcpTargetError::File(format!(
                        "CRC validation failed: expected {:08x}, got {:08x}",
                        expected_crc, actual_crc
                    )));
                }
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

    /// Initiates a challenge to the target machine to verify connection security
    ///
    /// This method performs a cryptographic challenge-response authentication:
    /// 1. Generates a random 32-byte challenge
    /// 2. Sends the challenge to the target machine
    /// 3. Receives a digital signature of the challenge
    /// 4. Verifies the signature using the appropriate public key
    ///
    /// # Arguments
    /// * `public_key_dir` - Directory containing public key files for verification
    ///
    /// # Returns
    /// * `Ok(true)` - Challenge verification successful
    /// * `Ok(false)` - Challenge verification failed
    /// * `Err(TcpTargetError)` - Error during challenge process
    pub async fn challenge(
        &mut self,
        public_key_dir: impl AsRef<Path>,
    ) -> Result<bool, TcpTargetError> {
        // Generate random challenge
        let mut challenge = [0u8; 32];
        rand::rngs::OsRng
            .try_fill_bytes(&mut challenge)
            .map_err(|e| {
                TcpTargetError::Crypto(format!("Failed to generate random challenge: {}", e))
            })?;

        // Send challenge to target
        self.stream.write_all(&challenge).await?;
        self.stream.flush().await?;

        // Read signature from target
        let mut signature = Vec::new();
        let mut signature_len_buf = [0u8; 4];
        self.stream.read_exact(&mut signature_len_buf).await?;

        let signature_len = u32::from_be_bytes(signature_len_buf) as usize;
        signature.resize(signature_len, 0);
        self.stream.read_exact(&mut signature).await?;

        // Read key identifier from target to identify which public key to use
        let mut key_id_len_buf = [0u8; 4];
        self.stream.read_exact(&mut key_id_len_buf).await?;
        let key_id_len = u32::from_be_bytes(key_id_len_buf) as usize;

        let mut key_id_buf = vec![0u8; key_id_len];
        self.stream.read_exact(&mut key_id_buf).await?;
        let key_id = String::from_utf8(key_id_buf)
            .map_err(|e| TcpTargetError::Crypto(format!("Invalid key identifier: {}", e)))?;

        // Load appropriate public key
        let public_key_path = public_key_dir.as_ref().join(format!("{}.pem", key_id));
        if !public_key_path.exists() {
            return Ok(false);
        }

        let public_key_pem = tokio::fs::read_to_string(&public_key_path).await?;

        // Try to verify with different key types
        let verified = if let Ok(rsa_key) = RsaPublicKey::from_pkcs1_pem(&public_key_pem) {
            let padding = rsa::pkcs1v15::Pkcs1v15Sign::new::<sha2::Sha256>();
            rsa_key.verify(padding, &challenge, &signature).is_ok()
        } else if let Ok(ed25519_key) =
            VerifyingKey::from_bytes(&parse_ed25519_public_key(&public_key_pem))
        {
            if signature.len() == 64 {
                let sig_bytes: [u8; 64] = signature.as_slice().try_into().map_err(|_| {
                    TcpTargetError::Crypto("Invalid signature length for Ed25519".to_string())
                })?;
                let sig = Signature::from_bytes(&sig_bytes);
                ed25519_key.verify(&challenge, &sig).is_ok()
            } else {
                false
            }
        } else if let Ok(dsa_key_info) = parse_dsa_public_key(&public_key_pem) {
            verify_dsa_signature(&dsa_key_info, &challenge, &signature)
        } else {
            false
        };

        Ok(verified)
    }

    /// Accepts a challenge from the target machine to verify connection security
    ///
    /// This method performs a cryptographic challenge-response authentication:
    /// 1. Receives a random 32-byte challenge from the target machine
    /// 2. Signs the challenge using the appropriate private key
    /// 3. Sends the digital signature back to the target machine
    /// 4. Sends the key identifier for public key verification
    ///
    /// # Arguments
    /// * `private_key_file` - Path to the private key file for signing
    /// * `verify_public_key` - Key identifier for public key verification
    ///
    /// # Returns
    /// * `Ok(true)` - Challenge response sent successfully
    /// * `Ok(false)` - Private key format not supported
    /// * `Err(TcpTargetError)` - Error during challenge response process
    pub async fn accept_challenge(
        &mut self,
        private_key_file: impl AsRef<Path>,
        verify_public_key: &str,
    ) -> Result<bool, TcpTargetError> {
        // Read challenge from initiator
        let mut challenge = [0u8; 32];
        self.stream.read_exact(&mut challenge).await?;

        // Load private key
        let private_key_pem = tokio::fs::read_to_string(&private_key_file).await?;

        // Sign the challenge with supported key types
        let signature = if let Ok(rsa_key) = RsaPrivateKey::from_pkcs1_pem(&private_key_pem) {
            let padding = rsa::pkcs1v15::Pkcs1v15Sign::new::<sha2::Sha256>();
            rsa_key.sign(padding, &challenge)?
        } else if let Ok(ed25519_key) = parse_ed25519_private_key(&private_key_pem) {
            ed25519_key.sign(&challenge).to_bytes().to_vec()
        } else if let Ok(dsa_key_info) = parse_dsa_private_key(&private_key_pem) {
            sign_with_dsa(&dsa_key_info, &challenge)?
        } else {
            return Ok(false);
        };

        // Send signature length and signature
        let signature_len = signature.len() as u32;
        self.stream.write_all(&signature_len.to_be_bytes()).await?;
        self.stream.flush().await?;
        self.stream.write_all(&signature).await?;
        self.stream.flush().await?;

        // Send key identifier for public key identification
        let key_id_bytes = verify_public_key.as_bytes();
        let key_id_len = key_id_bytes.len() as u32;
        self.stream.write_all(&key_id_len.to_be_bytes()).await?;
        self.stream.flush().await?;
        self.stream.write_all(key_id_bytes).await?;
        self.stream.flush().await?;

        Ok(true)
    }
}

/// Parse Ed25519 public key from PEM format
fn parse_ed25519_public_key(pem: &str) -> [u8; 32] {
    // Robust parsing for Ed25519 public key using pem crate
    let mut key_bytes = [0u8; 32];

    if let Ok(pem_data) = pem::parse(pem) {
        if pem_data.tag() == "PUBLIC KEY" && pem_data.contents().len() >= 32 {
            let contents = pem_data.contents();
            key_bytes.copy_from_slice(&contents[contents.len() - 32..]);
        }
    }
    key_bytes
}

/// Parse Ed25519 private key from PEM format
fn parse_ed25519_private_key(pem: &str) -> Result<SigningKey, TcpTargetError> {
    if let Ok(pem_data) = pem::parse(pem) {
        if pem_data.tag() == "PRIVATE KEY" && pem_data.contents().len() >= 32 {
            let contents = pem_data.contents();
            let mut seed = [0u8; 32];
            seed.copy_from_slice(&contents[contents.len() - 32..]);
            return Ok(SigningKey::from_bytes(&seed));
        }
    }
    Err(TcpTargetError::Crypto(
        "Invalid Ed25519 private key format".to_string(),
    ))
}

/// Parse DSA public key information from PEM
fn parse_dsa_public_key(
    pem: &str,
) -> Result<(&'static dyn signature::VerificationAlgorithm, Vec<u8>), TcpTargetError> {
    if let Ok(pem_data) = pem::parse(pem) {
        let contents = pem_data.contents().to_vec();

        // Try different DSA algorithms based on PEM tag
        match pem_data.tag() {
            "EC PUBLIC KEY" | "PUBLIC KEY" if pem.contains("ECDSA") || pem.contains("ecdsa") => {
                if pem.contains("P-256") {
                    return Ok((&ECDSA_P256_SHA256_ASN1, contents));
                } else if pem.contains("P-384") {
                    return Ok((&ECDSA_P384_SHA384_ASN1, contents));
                }
            }
            "RSA PUBLIC KEY" | "PUBLIC KEY" => {
                return Ok((&RSA_PKCS1_2048_8192_SHA256, contents));
            }
            _ => {}
        }

        // Default to RSA for unknown types
        return Ok((&RSA_PKCS1_2048_8192_SHA256, contents));
    }
    Err(TcpTargetError::Crypto(
        "Invalid DSA public key format".to_string(),
    ))
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

/// Sign with DSA
fn sign_with_dsa(
    algorithm_and_key: &(&'static dyn signature::VerificationAlgorithm, Vec<u8>),
    message: &[u8],
) -> Result<Vec<u8>, TcpTargetError> {
    let (algorithm, key_bytes) = algorithm_and_key;

    // Handle different DSA/ECDSA algorithms by comparing algorithm identifiers
    // Since we can't directly compare trait objects, we use pointer comparison
    let algorithm_ptr = algorithm as *const _ as *const ();
    let ecdsa_p256_ptr = &ECDSA_P256_SHA256_ASN1 as *const _ as *const ();
    let ecdsa_p384_ptr = &ECDSA_P384_SHA384_ASN1 as *const _ as *const ();

    if algorithm_ptr == ecdsa_p256_ptr {
        let key_pair = EcdsaKeyPair::from_pkcs8(
            ECDSA_P256_SHA256_ASN1_SIGNING,
            &key_bytes,
            &SystemRandom::new(),
        )
        .map_err(|e| {
            TcpTargetError::Crypto(format!("Failed to create ECDSA P-256 key pair: {}", e))
        })?;

        let signature = key_pair
            .sign(&SystemRandom::new(), message)
            .map_err(|e| TcpTargetError::Crypto(format!("ECDSA P-256 signing failed: {}", e)))?;

        Ok(signature.as_ref().to_vec())
    } else if algorithm_ptr == ecdsa_p384_ptr {
        let key_pair = EcdsaKeyPair::from_pkcs8(
            ECDSA_P384_SHA384_ASN1_SIGNING,
            &key_bytes,
            &SystemRandom::new(),
        )
        .map_err(|e| {
            TcpTargetError::Crypto(format!("Failed to create ECDSA P-384 key pair: {}", e))
        })?;

        let signature = key_pair
            .sign(&SystemRandom::new(), message)
            .map_err(|e| TcpTargetError::Crypto(format!("ECDSA P-384 signing failed: {}", e)))?;

        Ok(signature.as_ref().to_vec())
    } else {
        // RSA or unsupported algorithm
        Err(TcpTargetError::Unsupported(
            "DSA/ECDSA signing not supported for this algorithm type".to_string(),
        ))
    }
}
