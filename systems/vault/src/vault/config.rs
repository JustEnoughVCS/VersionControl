use asset_system::{RWDataTest, rw::RWData};
use config_system::rw::{read_config, write_config};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Eq, RWDataTest)]
pub struct VaultConfig {}

impl RWData<VaultConfig> for VaultConfig {
    async fn read(
        path: &std::path::PathBuf,
    ) -> Result<VaultConfig, asset_system::error::DataReadError> {
        let read_config = read_config(path).await;
        match read_config {
            Ok(config) => Ok(config),
            Err(e) => Err(asset_system::error::DataReadError::IoError(
                std::io::Error::new(std::io::ErrorKind::Other, e),
            )),
        }
    }

    async fn write(
        data: VaultConfig,
        path: &std::path::PathBuf,
    ) -> Result<(), asset_system::error::DataWriteError> {
        let write_config = write_config(path, &data).await;
        match write_config {
            Ok(_) => Ok(()),
            Err(e) => {
                return Err(asset_system::error::DataWriteError::IoError(
                    std::io::Error::new(std::io::ErrorKind::Other, e),
                ));
            }
        }
    }

    fn test_data() -> VaultConfig {
        VaultConfig::default()
    }

    fn verify_data(data_a: VaultConfig, data_b: VaultConfig) -> bool {
        &data_a == &data_b
    }
}
