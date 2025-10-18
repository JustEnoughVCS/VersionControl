use async_trait::async_trait;
use ron;
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    env::current_dir,
    io::Error,
    path::{Path, PathBuf},
};
use tokio::{fs, io::AsyncReadExt};

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

    async fn read() -> Result<Self::DataType, std::io::Error>
    where
        Self: Sized + Send + Sync,
    {
        let path = Self::default_path()?;
        Self::read_from(path).await
    }

    async fn read_from(path: impl AsRef<Path> + Send) -> Result<Self::DataType, std::io::Error>
    where
        Self: Sized + Send + Sync,
    {
        let path = path.as_ref();
        let cwd = current_dir()?;
        let file_path = cwd.join(path);

        // Check if file exists
        if fs::metadata(&file_path).await.is_err() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Config file not found",
            ));
        }

        // Open file
        let mut file = fs::File::open(&file_path).await?;

        let mut contents = String::new();
        file.read_to_string(&mut contents).await?;

        // Determine file format
        let format = file_path
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(ConfigFormat::from_filename)
            .unwrap_or(ConfigFormat::Json); // Default to JSON

        // Deserialize based on format
        let result = match format {
            ConfigFormat::Yaml => serde_yaml::from_str(&contents)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?,
            ConfigFormat::Toml => toml::from_str(&contents)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?,
            ConfigFormat::Ron => ron::from_str(&contents)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?,
            ConfigFormat::Json => serde_json::from_str(&contents)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?,
        };

        Ok(result)
    }

    async fn write(val: &Self::DataType) -> Result<(), std::io::Error>
    where
        Self: Sized + Send + Sync,
    {
        let path = Self::default_path()?;
        Self::write_to(val, path).await
    }

    async fn write_to(
        val: &Self::DataType,
        path: impl AsRef<Path> + Send,
    ) -> Result<(), std::io::Error>
    where
        Self: Sized + Send + Sync,
    {
        let path = path.as_ref();

        if let Some(parent) = path.parent()
            && !parent.exists()
        {
            tokio::fs::create_dir_all(parent).await?;
        }

        let cwd = current_dir()?;
        let file_path = cwd.join(path);

        // Determine file format
        let format = file_path
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(ConfigFormat::from_filename)
            .unwrap_or(ConfigFormat::Json); // Default to JSON

        let contents = match format {
            ConfigFormat::Yaml => serde_yaml::to_string(val)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?,
            ConfigFormat::Toml => toml::to_string(val)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?,
            ConfigFormat::Ron => {
                let mut pretty_config = ron::ser::PrettyConfig::new();
                pretty_config.new_line = Cow::from("\n");
                pretty_config.indentor = Cow::from("  ");

                ron::ser::to_string_pretty(val, pretty_config)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?
            }
            ConfigFormat::Json => serde_json::to_string(val)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?,
        };

        // Write to file
        fs::write(&file_path, contents).await?;
        Ok(())
    }

    /// Check if the file returned by `default_path` exists
    fn exist() -> bool
    where
        Self: Sized + Send + Sync,
    {
        let Ok(path) = Self::default_path() else {
            return false;
        };
        path.exists()
    }
}
