use crate::error::ConfigError;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub async fn read_config<C>(path: impl AsRef<Path>) -> Result<C, ConfigError>
where
    C: for<'a> Deserialize<'a>,
{
    let path_ref = path.as_ref();
    let ext = path_ref
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("json");
    let format = ext_fmt(&ext.to_lowercase());

    let content = std::fs::read_to_string(path_ref).map_err(ConfigError::ConfigReadFailed)?;

    match format {
        Format::Yaml => {
            serde_yaml::from_str(&content).map_err(|e| ConfigError::Other(e.to_string()))
        }
        Format::Toml => toml::from_str(&content).map_err(|e| ConfigError::Other(e.to_string())),
        Format::Json => {
            serde_json::from_str(&content).map_err(|e| ConfigError::Other(e.to_string()))
        }
    }
}

pub async fn write_config<C>(path: impl AsRef<Path>, config: &C) -> Result<(), ConfigError>
where
    C: Serialize,
{
    let path_ref = path.as_ref();
    let ext = path_ref
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("json");
    let format = ext_fmt(&ext.to_lowercase());

    let content = match format {
        Format::Yaml => {
            serde_yaml::to_string(config).map_err(|e| ConfigError::Other(e.to_string()))?
        }
        Format::Toml => toml::to_string(config).map_err(|e| ConfigError::Other(e.to_string()))?,
        Format::Json => {
            serde_json::to_string_pretty(config).map_err(|e| ConfigError::Other(e.to_string()))?
        }
    };

    std::fs::write(path_ref, content).map_err(ConfigError::ConfigWriteFailed)
}

enum Format {
    Yaml,
    Toml,
    Json,
}

fn ext_fmt(ext: &str) -> Format {
    match ext {
        "yaml" | "yml" => Format::Yaml,
        "toml" | "tml" => Format::Toml,
        "json" => Format::Json,
        _ => Format::Json,
    }
}
