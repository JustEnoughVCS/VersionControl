use crate::workspace::{Workspace, config::WorkspaceConfig, error::WorkspaceOperationError};
use asset_system::asset::ReadOnlyAsset;
use constants::workspace::files::workspace_file_config;
use framework::space::Space;

pub mod id_aliases;

pub struct WorkspaceManager {
    space: Space<Workspace>,
}

impl WorkspaceManager {
    pub fn new() -> Self {
        WorkspaceManager {
            space: Space::new(Workspace),
        }
    }

    /// Get an immutable reference to the internal Space
    pub fn get_space(&self) -> &Space<Workspace> {
        &self.space
    }

    /// Get a mutable reference to the internal Space
    pub fn get_space_mut(&mut self) -> &mut Space<Workspace> {
        &mut self.space
    }

    /// Get a read-only instance of the workspace configuration file
    pub fn workspace_config(
        &self,
    ) -> Result<ReadOnlyAsset<WorkspaceConfig>, WorkspaceOperationError> {
        let config_path = self.space.local_path(workspace_file_config())?;
        if !config_path.exists() {
            return Err(WorkspaceOperationError::ConfigNotFound);
        }
        let asset = ReadOnlyAsset::from(config_path);
        Ok(asset)
    }
}
