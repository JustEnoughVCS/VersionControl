use std::net::SocketAddr;

use action_system::{action::ActionContext, macros::action_gen};
use cfg_file::config::ConfigFile;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use tcp_connection::error::TcpTargetError;
use vcs_data::data::{
    local::{
        config::LocalConfig,
        latest_info::{LatestInfo, SheetInfo},
    },
    vault::config::VaultUuid,
};

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
    let instance = check_connection_instance(&ctx)?;

    // Auth Member
    if let Err(e) = auth_member(&ctx, instance).await {
        return Ok(SetUpstreamVaultActionResult::AuthorizeFailed(e.to_string()));
    }

    // Direct
    if ctx.is_proc_on_remote() {
        let vault = try_get_vault(&ctx)?;
        instance
            .lock()
            .await
            .write(*vault.config().vault_uuid())
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

#[derive(Serialize, Deserialize)]
pub enum UpdateToLatestInfoResult {
    Success,

    // Fail
    AuthorizeFailed(String),
}

#[action_gen]
pub async fn update_to_latest_info_action(
    ctx: ActionContext,
    _unused: (),
) -> Result<UpdateToLatestInfoResult, TcpTargetError> {
    let instance = check_connection_instance(&ctx)?;

    let member_id = match auth_member(&ctx, instance).await {
        Ok(id) => id,
        Err(e) => return Ok(UpdateToLatestInfoResult::AuthorizeFailed(e.to_string())),
    };

    if ctx.is_proc_on_remote() {
        let vault = try_get_vault(&ctx)?;

        // Build latest info
        let mut latest_info = LatestInfo::default();

        // Sheet
        let mut member_owned = Vec::new();
        let mut member_visible = Vec::new();

        for sheet in vault.sheets().await? {
            if sheet.holder().is_some() && sheet.holder().unwrap() == &member_id {
                member_owned.push(sheet.name().clone());
            } else {
                member_visible.push(SheetInfo {
                    sheet_name: sheet.name().clone(),
                    holder_name: match sheet.holder() {
                        Some(holder) => Some(holder.clone()),
                        None => None,
                    },
                });
            }
        }

        latest_info.my_sheets = member_owned;
        latest_info.other_sheets = member_visible;

        // RefSheet
        let ref_sheet_data = vault.sheet(&"ref".to_string()).await?.to_data();
        latest_info.ref_sheet_content = ref_sheet_data;

        // Members
        let members = vault.members().await?;
        latest_info.vault_members = members;

        // Send
        instance
            .lock()
            .await
            .write_large_msgpack(latest_info, 512 as u16)
            .await?;

        return Ok(UpdateToLatestInfoResult::Success);
    }

    if ctx.is_proc_on_local() {
        let latest_info = instance
            .lock()
            .await
            .read_large_msgpack::<LatestInfo>(512 as u16)
            .await?;
        LatestInfo::write(&latest_info).await?;

        return Ok(UpdateToLatestInfoResult::Success);
    }

    Err(TcpTargetError::NoResult("No result.".to_string()))
}
