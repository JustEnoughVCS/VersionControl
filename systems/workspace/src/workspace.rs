use asset_system::rw::RWData;
use constants::workspace::{dirs::workspace_dir_workspace, files::workspace_file_config};
use framework::{SpaceRootTest, space::SpaceRoot};
use tokio::fs::create_dir_all;

use crate::workspace::config::WorkspaceConfig;

pub mod config;
pub mod error;
pub mod manager;

#[derive(Default, SpaceRootTest)]
pub struct Workspace;

impl SpaceRoot for Workspace {
    fn get_pattern() -> framework::space::SpaceRootFindPattern {
        framework::space::SpaceRootFindPattern::IncludeDotDir(workspace_dir_workspace().into())
    }

    async fn create_space(
        path: &std::path::Path,
    ) -> Result<(), framework::space::error::SpaceError> {
        let workspace_root = path.join(workspace_dir_workspace());
        let workspace_toml = path.join(workspace_file_config());

        // Create workspace directory
        create_dir_all(workspace_root)
            .await
            .map_err(framework::space::error::SpaceError::from)?;

        // Create configuration file
        WorkspaceConfig::write(WorkspaceConfig::default(), &workspace_toml)
            .await
            .map_err(|e| framework::space::error::SpaceError::Other(e.to_string()))?;

        Ok(())
    }
}
