use constants::workspace::dirs::workspace_dir_id_mapping;
use sheet_system::index_source::{
    IndexSource,
    alias::{IndexSourceAliasesManager, convert_to_remote},
    error::IDAliasError,
};

use crate::workspace::{error::WorkspaceOperationError, manager::WorkspaceManager};

impl WorkspaceManager {
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

    /// Attempt to convert a local ID to a remote ID.
    pub async fn try_convert_to_remote(
        &self,
        local_id: u32,
    ) -> Result<Option<u32>, WorkspaceOperationError> {
        let aliases_dir = self.get_space().local_path(workspace_dir_id_mapping())?;
        match convert_to_remote(aliases_dir, local_id).await {
            Ok(remote_id) => Ok(Some(remote_id)),
            Err(IDAliasError::AliasNotFound(_)) => Ok(None),
            Err(IDAliasError::Io(e)) => Err(WorkspaceOperationError::Io(e)),
            Err(e) => Err(WorkspaceOperationError::IDAliasError(e)),
        }
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
