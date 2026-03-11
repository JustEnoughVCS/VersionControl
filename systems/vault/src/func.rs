use crate::vault::{Vault, config::VaultConfig, error::VaultOperationError, manager::VaultManager};
use asset_system::asset::Handle;
use framework::space::Space;
use std::{env::current_dir, path::PathBuf};

/// Create a vault at the current location
pub async fn create_vault_here() -> Result<(), VaultOperationError> {
    create_vault(current_dir()?).await
}

/// Create a vault at the specified location
pub async fn create_vault(path: impl Into<PathBuf>) -> Result<(), VaultOperationError> {
    let path = path.into();
    let mut space = Space::new(Vault);
    space.set_current_dir(path)?;
    space.init_here().await?;

    Ok(())
}

#[allow(unused)]
/// Get a handle to the vault configuration file and edit its content
pub async fn operate_vault_config<F>(
    vault_path: impl Into<PathBuf>,
    operate: F,
) -> Result<(), VaultOperationError>
where
    F: AsyncFn(Handle<VaultConfig>),
{
    // Get the vault manager and set the context to the given vault path
    let mut mgr = VaultManager::new();
    mgr.get_space_mut().set_current_dir(vault_path.into())?;

    // Obtain the vault configuration, and pass it to the function for execution
    let config = mgr.vault_config()?;
    let handle = config.get_handle().await?;
    operate(handle).await;

    Ok(())
}
