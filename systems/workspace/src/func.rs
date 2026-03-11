use std::{env::current_dir, path::PathBuf};

use asset_system::asset::Handle;
use framework::space::Space;

use crate::workspace::{
    Workspace, config::WorkspaceConfig, error::WorkspaceOperationError, manager::WorkspaceManager,
};

/// Create a workspace at the current location
pub async fn create_workspace_here() -> Result<(), WorkspaceOperationError> {
    create_workspace(current_dir()?).await
}

/// Create a workspace at the specified location
pub async fn create_workspace(path: impl Into<PathBuf>) -> Result<(), WorkspaceOperationError> {
    let path = path.into();
    let mut space = Space::new(Workspace);
    space.set_current_dir(path)?;
    space.init_here().await?;

    Ok(())
}

#[allow(unused)]
/// Get a handle to the workspace configuration file and edit its content
pub async fn operate_workspace_config<F>(
    workspace_path: impl Into<PathBuf>,
    operate: F,
) -> Result<(), WorkspaceOperationError>
where
    F: AsyncFn(Handle<WorkspaceConfig>),
{
    // Get the workspace manager and set the context to the given workspace path
    let mut mgr = WorkspaceManager::new();
    mgr.get_space_mut().set_current_dir(workspace_path.into())?;

    // Obtain the read-only asset of the workspace configuration file,
    // lock it to get a writable handle, and pass it to the function for execution
    let config = mgr.workspace_config()?;
    let handle = config.get_handle().await?;
    operate(handle).await;

    Ok(())
}
