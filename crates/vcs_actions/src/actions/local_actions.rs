use std::net::SocketAddr;

use action_system::{action::ActionContext, macros::action_gen};
use cfg_file::config::ConfigFile;
use log::info;
use serde::{Deserialize, Serialize};
use tcp_connection::error::TcpTargetError;
use vcs_data::data::{
    local::{
        cached_sheet::CachedSheet,
        config::LocalConfig,
        latest_info::{LatestInfo, SheetInfo},
    },
    sheet::{SheetData, SheetName},
    vault::config::VaultUuid,
};

use crate::actions::{
    auth_member, check_connection_instance, try_get_local_workspace, try_get_vault,
};

#[derive(Serialize, Deserialize)]
pub enum SetUpstreamVaultActionResult {
    // Success
    DirectedAndStained,
    Redirected,

    // Fail
    AlreadyStained,
    AuthorizeFailed(String),
    RedirectFailed(String),
    SameUpstream,

    Done,
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
        return Ok(SetUpstreamVaultActionResult::Done);
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
            // Local workspace is already stained, redirecting
            let Some(stained_uuid) = mut_local_config.stained_uuid() else {
                return Ok(SetUpstreamVaultActionResult::RedirectFailed(
                    "Stained uuid not found".to_string(),
                ));
            };
            let local_upstream = mut_local_config.upstream_addr();

            // Address changed, but same UUID.
            if vault_uuid == stained_uuid {
                if local_upstream != upstream {
                    // Set the upstream address
                    mut_local_config.set_vault_addr(upstream);

                    // Store the updated config
                    LocalConfig::write(&mut_local_config).await?;
                    return Ok(SetUpstreamVaultActionResult::Redirected);
                } else {
                    return Ok(SetUpstreamVaultActionResult::SameUpstream);
                }
            }
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

    // Sync Latest Info
    {
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
        }

        if ctx.is_proc_on_local() {
            let latest_info = instance
                .lock()
                .await
                .read_large_msgpack::<LatestInfo>(512 as u16)
                .await?;
            LatestInfo::write(&latest_info).await?;
        }
    }

    // Sync Remote Sheets
    {
        if ctx.is_proc_on_local() {
            let Ok(latest_info) = LatestInfo::read().await else {
                return Err(TcpTargetError::NotFound(
                    "Latest info not found.".to_string(),
                ));
            };

            // Collect all local versions
            let mut local_versions = vec![];
            for request_sheet in latest_info.my_sheets {
                let Ok(data) =
                    CachedSheet::cached_sheet_data(member_id.clone(), request_sheet.clone()).await
                else {
                    local_versions.push((request_sheet, 0));
                    continue;
                };
                local_versions.push((request_sheet, data.write_count()));
            }

            // Send the version list
            let len = local_versions.len();
            instance.lock().await.write_msgpack(local_versions).await?;

            if len < 1 {
                return Ok(UpdateToLatestInfoResult::Success);
            }
        }

        // Send data to local
        if ctx.is_proc_on_remote() {
            let vault = try_get_vault(&ctx)?;
            let mut mut_instance = instance.lock().await;

            let local_versions = mut_instance.read_msgpack::<Vec<(SheetName, i32)>>().await?;

            for (sheet_name, version) in local_versions.iter() {
                let sheet = vault.sheet(sheet_name).await?;
                if let Some(holder) = sheet.holder() {
                    if holder == &member_id && &sheet.write_count() != version {
                        mut_instance.write_msgpack(true).await?;
                        mut_instance
                            .write_large_msgpack((sheet_name, sheet.to_data()), 1024u16)
                            .await?;
                    }
                }
            }
            mut_instance.write_msgpack(false).await?;
            return Ok(UpdateToLatestInfoResult::Success);
        }

        // Receive data
        if ctx.is_proc_on_local() {
            let mut mut_instance = instance.lock().await;
            loop {
                let in_coming: bool = mut_instance.read_msgpack().await?;
                if in_coming {
                    let (sheet_name, data): (SheetName, SheetData) =
                        mut_instance.read_large_msgpack(1024u16).await?;

                    let Some(path) = CachedSheet::cached_sheet_path(member_id.clone(), sheet_name)
                    else {
                        return Err(TcpTargetError::NotFound("Workspace not found".to_string()));
                    };

                    SheetData::write_to(&data, path).await?;
                } else {
                    return Ok(UpdateToLatestInfoResult::Success);
                }
            }
        }
    }

    Err(TcpTargetError::NoResult("No result.".to_string()))
}
