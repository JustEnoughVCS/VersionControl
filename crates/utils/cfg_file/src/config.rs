use async_trait::async_trait;
use bincode2;
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
    Bincode,
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
        } else if filename.ends_with(".bcfg") {
            Some(Self::Bincode)
        } else {
            None
        }
    }
}

/// # Trait - ConfigFile
///
/// Used to implement more convenient persistent storage functionality for structs
///
/// This trait requires the struct to implement Default and serde's Serialize and Deserialize traits
///
/// ## Implementation
///
/// ```ignore
/// // Your struct
/// #[derive(Default, Serialize, Deserialize)]
/// struct YourData;
///
/// impl ConfigFile for YourData {
///     type DataType = YourData;
///
///     // Specify default path
///     fn default_path() -> Result<PathBuf, Error> {
///         Ok(current_dir()?.join("data.json"))
///     }
/// }
/// ```
///
/// > **Using derive macro**
/// >
/// > We provide the derive macro `#[derive(ConfigFile)]`
/// >
/// > You can implement this trait more quickly, please check the module cfg_file::cfg_file_derive
///
#[async_trait]
pub trait ConfigFile: Serialize + for<'a> Deserialize<'a> + Default {
    type DataType: Serialize + for<'a> Deserialize<'a> + Default + Send + Sync;

    fn default_path() -> Result<PathBuf, Error>;

    /// # Read from default path
    ///
    /// Read data from the path specified by default_path()
    ///
    /// ```ignore
    /// fn main() -> Result<(), std::io::Error> {
    ///     let data = YourData::read().await?;
    /// }
    /// ```
    async fn read() -> Result<Self::DataType, std::io::Error>
    where
        Self: Sized + Send + Sync,
    {
        let path = Self::default_path()?;
        Self::read_from(path).await
    }

    /// # Read from the given path
    ///
    /// Read data from the path specified by the path parameter
    ///
    /// ```ignore
    /// fn main() -> Result<(), std::io::Error> {
    ///     let data_path = current_dir()?.join("data.json");
    ///     let data = YourData::read_from(data_path).await?;
    /// }
    /// ```
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

        // Determine file format first
        let format = file_path
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(ConfigFormat::from_filename)
            .unwrap_or(ConfigFormat::Json); // Default to JSON

        // Deserialize based on format
        let result = match format {
            ConfigFormat::Yaml => {
                let mut file = fs::File::open(&file_path).await?;
                let mut contents = String::new();
                file.read_to_string(&mut contents).await?;
                serde_yaml::from_str(&contents)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?
            }
            ConfigFormat::Toml => {
                let mut file = fs::File::open(&file_path).await?;
                let mut contents = String::new();
                file.read_to_string(&mut contents).await?;
                toml::from_str(&contents)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?
            }
            ConfigFormat::Ron => {
                let mut file = fs::File::open(&file_path).await?;
                let mut contents = String::new();
                file.read_to_string(&mut contents).await?;
                ron::from_str(&contents)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?
            }
            ConfigFormat::Json => {
                let mut file = fs::File::open(&file_path).await?;
                let mut contents = String::new();
                file.read_to_string(&mut contents).await?;
                serde_json::from_str(&contents)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?
            }
            ConfigFormat::Bincode => {
                // For Bincode, we need to read the file as bytes directly
                let bytes = fs::read(&file_path).await?;
                bincode2::deserialize(&bytes)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?
            }
        };

        Ok(result)
    }

    /// # Write to default path
    ///
    /// Write data to the path specified by default_path()
    ///
    /// ```ignore
    /// fn main() -> Result<(), std::io::Error> {
    ///     let data = YourData::default();
    ///     YourData::write(&data).await?;
    /// }
    /// ```
    async fn write(val: &Self::DataType) -> Result<(), std::io::Error>
    where
        Self: Sized + Send + Sync,
    {
        let path = Self::default_path()?;
        Self::write_to(val, path).await
    }
    /// # Write to the given path
    ///
    /// Write data to the path specified by the path parameter
    ///
    /// ```ignore
    /// fn main() -> Result<(), std::io::Error> {
    ///     let data = YourData::default();
    ///     let data_path = current_dir()?.join("data.json");
    ///     YourData::write_to(&data, data_path).await?;
    /// }
    /// ```
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

        match format {
            ConfigFormat::Yaml => {
                let contents = serde_yaml::to_string(val)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                fs::write(&file_path, contents).await?
            }
            ConfigFormat::Toml => {
                let contents = toml::to_string(val)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                fs::write(&file_path, contents).await?
            }
            ConfigFormat::Ron => {
                let mut pretty_config = ron::ser::PrettyConfig::new();
                pretty_config.new_line = Cow::from("\n");
                pretty_config.indentor = Cow::from("  ");

                let contents = ron::ser::to_string_pretty(val, pretty_config)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                fs::write(&file_path, contents).await?
            }
            ConfigFormat::Json => {
                let contents = serde_json::to_string(val)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                fs::write(&file_path, contents).await?
            }
            ConfigFormat::Bincode => {
                let bytes = bincode2::serialize(val)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                fs::write(&file_path, bytes).await?
            }
        }
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
