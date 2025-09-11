use async_trait::async_trait;
use serde::{ Deserialize, Serialize };
use std::{
  borrow::Cow,
  env::current_dir,
  io::Error,
  path:: { PathBuf, Path },
};
use tokio::{
    fs,
    io::AsyncReadExt
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConfigFormat {
    Yaml,
    Toml,
    Ron,
    Json,
}

impl ConfigFormat {
    fn from_filename(filename: &str) -> Option<Self> {
        if filename.ends_with(".yaml") || filename.ends_with(".yml") {
            Some(Self::Yaml)
        } else if filename.ends_with(".toml") || filename.ends_with(".tom") {
            Some(Self::Toml)
        } else if filename.ends_with(".ron") {
            Some(Self::Ron)
        } else if filename.ends_with(".json") {
            Some(Self::Json)
        } else {
            None
        }
    }
}

#[async_trait]
pub trait ConfigFile: Serialize + for<'a> Deserialize<'a> + Default {
    type DataType: Serialize + for<'a> Deserialize<'a> + Default + Send + Sync;

    fn default_path() -> Result<PathBuf, Error>;

    async fn read() -> Self::DataType
    where
        Self: Sized + Send + Sync,
    {
        let Ok(path) = Self::default_path() else {
            return Self::DataType::default()
        };

        Self::read_from(path).await
    }

    async fn read_from(path: impl AsRef<Path> + Send) -> Self::DataType
    where
        Self: Sized + Send + Sync,
    {
        let path = path.as_ref();
        let file_path = match current_dir() {
            Ok(cwd) => cwd.join(&path),
            Err(e) => {
                eprintln!("Failed to get current directory: {}", e);
                return Self::DataType::default();
            }
        };

        // Check if file exists
        match fs::metadata(&file_path).await {
            Ok(_) => {
                // Open file
                let mut file = match fs::File::open(&file_path).await {
                    Ok(file) => file,
                    Err(e) => {
                        eprintln!("Failed to open file {}: {}", path.display(), e);
                        return Self::DataType::default();
                    }
                };

                let mut contents = String::new();

                // Read contents
                if let Err(e) = file.read_to_string(&mut contents).await {
                    eprintln!("Failed to read file {}: {}", path.display(), e);
                    return Self::DataType::default();
                }

                // Determine file format
                let format = file_path.file_name()
                    .and_then(|name| name.to_str())
                    .and_then(ConfigFormat::from_filename)
                    .unwrap_or(ConfigFormat::Json); // Default to JSON

                // Deserialize based on format
                match format {
                    ConfigFormat::Yaml => serde_yaml::from_str(&contents).unwrap_or_else(|e| {
                        eprintln!("Failed to parse YAML file {}: {}", path.display(), e);
                        Self::DataType::default()
                    }),
                    ConfigFormat::Toml => toml::from_str(&contents).unwrap_or_else(|e| {
                        eprintln!("Failed to parse TOML file {}: {}", path.display(), e);
                        Self::DataType::default()
                    }),
                    ConfigFormat::Ron => ron::from_str(&contents).unwrap_or_else(|e| {
                        eprintln!("Failed to parse RON file {}: {}", path.display(), e);
                        Self::DataType::default()
                    }),
                    ConfigFormat::Json => serde_json::from_str(&contents).unwrap_or_else(|e| {
                        eprintln!("Failed to parse JSON file {}: {}", path.display(), e);
                        Self::DataType::default()
                    }),
                }
            }
            Err(_) => {
                // Return default value when file doesn't exist
                Self::DataType::default()
            }
        }
    }

    async fn write(val: &Self::DataType)
    where
        Self: Sized + Send + Sync,
    {
        let Ok(path) = Self::default_path() else {
            return;
        };

        Self::write_to(val, path).await;
    }

    async fn write_to(val: &Self::DataType, path: impl AsRef<Path> + Send)
    where
        Self: Sized + Send + Sync,
    {
        let path = path.as_ref();

        if let Some(parent) = path.parent() {
            if ! parent.exists() {
                let _ = tokio::fs::create_dir_all(parent).await;
            }
        }

        let file_path = match current_dir() {
            Ok(cwd) => cwd.join(&path),
            Err(e) => {
                eprintln!("Failed to get current directory: {}", e);
                return;
            }
        };

        // Determine file format
        let format = file_path.file_name()
            .and_then(|name| name.to_str())
            .and_then(ConfigFormat::from_filename)
            .unwrap_or(ConfigFormat::Json); // Default to JSON

        let contents = match format {
            ConfigFormat::Yaml => serde_yaml::to_string(val).unwrap_or_else(|e| {
                eprintln!("Failed to serialize to YAML: {}", e);
                String::new()
            }),
            ConfigFormat::Toml => toml::to_string(val).unwrap_or_else(|e| {
                eprintln!("Failed to serialize to TOML: {}", e);
                String::new()
            }),
            ConfigFormat::Ron => {
                let mut pretty_config = ron::ser::PrettyConfig::new();
                pretty_config.new_line = Cow::from("\n");
                pretty_config.indentor = Cow::from("  ");

                ron::ser::to_string_pretty(val, pretty_config).unwrap_or_else(|e| {
                    eprintln!("Failed to serialize to RON: {}", e);
                    String::new()
                })
            }
            ConfigFormat::Json => serde_json::to_string(val).unwrap_or_else(|e| {
                eprintln!("Failed to serialize to JSON: {}", e);
                String::new()
            }),
        };

        // Don't write if serialization failed
        if contents.is_empty() {
            eprintln!("Serialization failed for file {}, not writing", path.display());
            return;
        }

        // Write to file
        if let Err(e) = fs::write(&file_path, contents).await {
            eprintln!("Failed to write file {}: {}", path.display(), e);
        }
    }
}