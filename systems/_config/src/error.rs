use std::path::PathBuf;

#[derive(thiserror::Error, Debug)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    ConfigReadFailed(#[source] std::io::Error),

    #[error("Failed to write config file: {0}")]
    ConfigWriteFailed(#[source] std::io::Error),

    #[error("Config file not found: {0}")]
    FileNotFound(PathBuf),

    #[error("IO error: {0}")]
    Io(#[source] std::io::Error),

    #[error("Other error: {0}")]
    Other(String),
}
