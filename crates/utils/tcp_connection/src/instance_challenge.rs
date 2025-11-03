use std::path::Path;

use rand::TryRngCore;
use rsa::{
    RsaPrivateKey, RsaPublicKey,
    pkcs1::{DecodeRsaPrivateKey, DecodeRsaPublicKey},
    sha2,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use ring::rand::SystemRandom;
use ring::signature::{
    self, ECDSA_P256_SHA256_ASN1, ECDSA_P384_SHA384_ASN1, EcdsaKeyPair, RSA_PKCS1_2048_8192_SHA256,
    UnparsedPublicKey,
};

use crate::{error::TcpTargetError, instance::ConnectionInstance};

const ECDSA_P256_SHA256_ASN1_SIGNING: &signature::EcdsaSigningAlgorithm =
    &signature::ECDSA_P256_SHA256_ASN1_SIGNING;
const ECDSA_P384_SHA384_ASN1_SIGNING: &signature::EcdsaSigningAlgorithm =
    &signature::ECDSA_P384_SHA384_ASN1_SIGNING;

impl ConnectionInstance {
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
    /// * `Ok((true, "KeyId"))` - Challenge verification successful
    /// * `Ok((false, "KeyId"))` - Challenge verification failed
    /// * `Err(TcpTargetError)` - Error during challenge process
    pub async fn challenge(
        &mut self,
        public_key_dir: impl AsRef<Path>,
    ) -> Result<(bool, String), TcpTargetError> {
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
            return Ok((false, key_id));
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

        Ok((verified, key_id))
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
        let private_key_pem = tokio::fs::read_to_string(&private_key_file)
            .await
            .map_err(|e| {
                TcpTargetError::NotFound(format!(
                    "Read private key \"{}\" failed: \"{}\"",
                    private_key_file
                        .as_ref()
                        .display()
                        .to_string()
                        .split("/")
                        .last()
                        .unwrap_or("UNKNOWN"),
                    e
                ))
            })?;

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

    if let Ok(pem_data) = pem::parse(pem)
        && pem_data.tag() == "PUBLIC KEY"
        && pem_data.contents().len() >= 32
    {
        let contents = pem_data.contents();
        key_bytes.copy_from_slice(&contents[contents.len() - 32..]);
    }
    key_bytes
}

/// Parse Ed25519 private key from PEM format
fn parse_ed25519_private_key(pem: &str) -> Result<SigningKey, TcpTargetError> {
    if let Ok(pem_data) = pem::parse(pem)
        && pem_data.tag() == "PRIVATE KEY"
        && pem_data.contents().len() >= 32
    {
        let contents = pem_data.contents();
        let mut seed = [0u8; 32];
        seed.copy_from_slice(&contents[contents.len() - 32..]);
        return Ok(SigningKey::from_bytes(&seed));
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
            key_bytes,
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
            key_bytes,
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
