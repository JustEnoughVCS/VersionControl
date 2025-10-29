use std::net::SocketAddr;

use action_system::{action::ActionContext, macros::action_gen};
use cfg_file::config::ConfigFile;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use tcp_connection::error::TcpTargetError;
use vcs_data::data::{local::config::LocalConfig, vault::config::VaultUuid};

use crate::actions::{
    auth_member, check_connection_instance, try_get_local_workspace, try_get_vault,
};

#[derive(Serialize, Deserialize)]
pub enum SetUpstreamVaultActionResult {
    // Success
    DirectedAndStained,

    // Fail
    AlreadyStained,
    AuthorizeFailed(String),
}

#[action_gen]
pub async fn set_upstream_vault_action(
    ctx: ActionContext,
    upstream: SocketAddr,
) -> Result<SetUpstreamVaultActionResult, TcpTargetError> {
    // Ensure the instance is available
    let instance = check_connection_instance(&ctx)?;

    // Step1: Auth Member
    if let Err(e) = auth_member(&ctx, instance).await {
        return Ok(SetUpstreamVaultActionResult::AuthorizeFailed(e.to_string()));
    }

    // Step2: Direct
    if ctx.is_proc_on_remote() {
        let vault = try_get_vault(&ctx)?;
        instance
            .lock()
            .await
            .write(vault.config().vault_uuid().clone())
            .await?;
    }

    if ctx.is_proc_on_local() {
        info!("Authorize successful. directing to upstream vault.");

        // Read the vault UUID from the instance
        let vault_uuid = instance.lock().await.read::<VaultUuid>().await?;

        let local_workspace = try_get_local_workspace(&ctx)?;
        let local_config = local_workspace.config();

        let mut mut_local_config = local_config.lock().await;
        if !mut_local_config.stained() {
            // Stain the local workspace
            mut_local_config.stain(vault_uuid);

            // Set the upstream address
            mut_local_config.set_vault_addr(upstream);

            // Store the updated config
            LocalConfig::write(&mut_local_config).await?;

            info!("Workspace stained!");
            return Ok(SetUpstreamVaultActionResult::DirectedAndStained);
        } else {
            warn!("Workspace already stained!");
            return Ok(SetUpstreamVaultActionResult::AlreadyStained);
        }
    }

    Err(TcpTargetError::NoResult("No result.".to_string()))
}
