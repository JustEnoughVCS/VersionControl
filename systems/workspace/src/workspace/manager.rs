use crate::workspace::{Workspace, config::WorkspaceConfig, error::WorkspaceOperationError};
use asset_system::asset::ReadOnlyAsset;
use constants::workspace::{dirs::workspace_dir_id_mapping, files::workspace_file_config};
use framework::space::Space;
use sheet_system::index_source::{IndexSource, alias::IndexSourceAliasesManager};

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

    /// Attempt to convert an index source to a remote namespace.
    /// This method takes an `IndexSource` and tries to map it to a remote namespace
    /// using the workspace's ID alias directory. If not found, the original
    /// `IndexSource` is returned as a fallback.
    ///
    /// - `index_source` - The index source to convert
    /// - `Result<IndexSource, WorkspaceOperationError>` - The converted index source on success,
    ///   or the original index source if alias fails. Returns an error if there's
    ///   a problem accessing the workspace directory.
    pub async fn try_to_remote_index(
        &self,
        index_source: IndexSource,
    ) -> Result<IndexSource, WorkspaceOperationError> {
        let aliases_dir = self.get_space().local_path(workspace_dir_id_mapping())?;
        Ok(match index_source.to_remote_namespace(aliases_dir).await {
            Ok(index_source) => index_source,
            Err((index_source, _)) => index_source,
        })
    }

    /// Write a alias between local and remote IDs
    pub async fn write_id_alias(
        &self,
        local_id: u32,
        remote_id: u32,
    ) -> Result<(), WorkspaceOperationError> {
        let aliases_dir = self.get_space().local_path(workspace_dir_id_mapping())?;
        IndexSourceAliasesManager::write_alias(aliases_dir, local_id, remote_id)
            .await
            .map_err(|e| WorkspaceOperationError::IDAliasError(e))
    }

    /// Delete a alias between local and remote IDs
    pub async fn delete_id_alias(&self, local_id: u32) -> Result<(), WorkspaceOperationError> {
        let aliases_dir = self.get_space().local_path(workspace_dir_id_mapping())?;
        IndexSourceAliasesManager::delete_alias(aliases_dir, local_id)
            .await
            .map_err(|e| WorkspaceOperationError::IDAliasError(e))
    }

    /// Check if a alias exists between local and remote IDs
    pub async fn id_aliases_exists(&self, local_id: u32) -> Result<bool, WorkspaceOperationError> {
        let aliases_dir = self.get_space().local_path(workspace_dir_id_mapping())?;
        IndexSourceAliasesManager::alias_exists(aliases_dir, local_id)
            .await
            .map_err(|e| WorkspaceOperationError::IDAliasError(e))
    }
}
