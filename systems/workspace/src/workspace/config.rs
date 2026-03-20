use std::{io::Error, path::Path};

use asset_system::{
    RWDataTest,
    error::{DataReadError, DataWriteError},
    rw::RWData,
};
use config_system::rw::{read_config, write_config};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Eq, RWDataTest)]
pub struct WorkspaceConfig {
    /// Upstream address, pointing to the upstream Vault
    #[serde(rename = "upstream")]
    upstream: String,

    /// Vault Uuid, used to ensure consistency between upstream and local
    #[serde(rename = "uuid")]
    uuid: Option<String>,

    /// Account name used by self,
    /// upstream Vault uses this parameter for identity verification
    #[serde(rename = "account")]
    use_accout: Option<String>,
}

impl WorkspaceConfig {
    /// Returns a reference to the upstream address.
    pub fn upstream(&self) -> &String {
        &self.upstream
    }

    /// Sets the upstream address.
    pub fn set_upstream(&mut self, upstream: String) {
        self.upstream = upstream
    }

    /// Returns a new instance with the given upstream address.
    pub fn with_upstream(mut self, upstream: String) -> Self {
        self.upstream = upstream;
        self
    }

    /// Returns a reference to the vault UUID, if any.
    pub fn uuid(&self) -> Option<&String> {
        self.uuid.as_ref()
    }

    /// Sets the vault UUID.
    pub fn set_uuid(&mut self, uuid: String) {
        self.uuid = Some(uuid)
    }

    /// Returns a new instance with the given vault UUID.
    pub fn with_uuid(mut self, uuid: String) -> Self {
        self.uuid = Some(uuid);
        self
    }

    /// Clears the vault UUID.
    pub fn erase_uuid(&mut self) {
        self.uuid = None;
    }

    /// Returns a reference to the account name, if any.
    pub fn account(&self) -> Option<&String> {
        self.use_accout.as_ref()
    }

    /// Sets the account name.
    pub fn set_account(&mut self, account: String) {
        self.use_accout = Some(account)
    }

    /// Returns a new instance with the given account name.
    pub fn with_account(mut self, account: String) -> Self {
        self.use_accout = Some(account);
        self
    }

    /// Clears the account name.
    pub fn erase_account(&mut self) {
        self.use_accout = None;
    }
}

impl RWData<WorkspaceConfig> for WorkspaceConfig {
    async fn read(path: &Path) -> Result<WorkspaceConfig, DataReadError> {
        let read_config = read_config(path).await;
        match read_config {
            Ok(config) => Ok(config),
            Err(e) => Err(DataReadError::IoError(Error::other(e))),
        }
    }

    async fn write(data: WorkspaceConfig, path: &Path) -> Result<(), DataWriteError> {
        let write_config = write_config(path, &data).await;
        match write_config {
            Ok(_) => Ok(()),
            Err(e) => Err(DataWriteError::IoError(Error::other(e))),
        }
    }

    fn test_data() -> WorkspaceConfig {
        WorkspaceConfig::default()
    }

    fn verify_data(data_a: WorkspaceConfig, data_b: WorkspaceConfig) -> bool {
        data_a == data_b
    }
}
