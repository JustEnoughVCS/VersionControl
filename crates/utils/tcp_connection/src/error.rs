use std::io;
use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum TcpTargetError {
    #[error("I/O error: {0}")]
    Io(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Cryptographic error: {0}")]
    Crypto(String),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Authentication failed: {0}")]
    Authentication(String),

    #[error("File operation error: {0}")]
    File(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Invalid configuration: {0}")]
    Config(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Unsupported operation: {0}")]
    Unsupported(String),
}

impl From<io::Error> for TcpTargetError {
    fn from(error: io::Error) -> Self {
        TcpTargetError::Io(error.to_string())
    }
}

impl From<serde_json::Error> for TcpTargetError {
    fn from(error: serde_json::Error) -> Self {
        TcpTargetError::Serialization(error.to_string())
    }
}

impl From<&str> for TcpTargetError {
    fn from(value: &str) -> Self {
        TcpTargetError::Protocol(value.to_string())
    }
}

impl From<String> for TcpTargetError {
    fn from(value: String) -> Self {
        TcpTargetError::Protocol(value)
    }
}

impl From<rsa::errors::Error> for TcpTargetError {
    fn from(error: rsa::errors::Error) -> Self {
        TcpTargetError::Crypto(error.to_string())
    }
}

impl From<ed25519_dalek::SignatureError> for TcpTargetError {
    fn from(error: ed25519_dalek::SignatureError) -> Self {
        TcpTargetError::Crypto(error.to_string())
    }
}

impl From<ring::error::Unspecified> for TcpTargetError {
    fn from(error: ring::error::Unspecified) -> Self {
        TcpTargetError::Crypto(error.to_string())
    }
}

impl From<base64::DecodeError> for TcpTargetError {
    fn from(error: base64::DecodeError) -> Self {
        TcpTargetError::Serialization(error.to_string())
    }
}

impl From<pem::PemError> for TcpTargetError {
    fn from(error: pem::PemError) -> Self {
        TcpTargetError::Crypto(error.to_string())
    }
}
