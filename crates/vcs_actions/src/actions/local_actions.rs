use std::{collections::HashMap, io::ErrorKind, net::SocketAddr, path::PathBuf};

use action_system::{action::ActionContext, macros::action_gen};
use cfg_file::config::ConfigFile;
use log::info;
use serde::{Deserialize, Serialize};
use tcp_connection::error::TcpTargetError;
use tokio::time::Instant;
use vcs_data::data::{
    local::{
        cached_sheet::CachedSheet,
        config::LocalConfig,
        latest_info::{LatestInfo, SheetInfo},
        local_sheet::LocalSheetData,
        member_held::MemberHeld,
    },
    member::MemberId,
    sheet::{SheetData, SheetName},
    vault::{config::VaultUuid, virtual_file::VirtualFileId},
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
    SyncCachedSheetFail(SyncCachedSheetFailReason),
}

#[derive(Serialize, Deserialize)]
pub enum SyncCachedSheetFailReason {
    PathAlreadyExist(PathBuf),
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

    info!("Sending latest info to {}", member_id);

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
            let workspace = try_get_local_workspace(&ctx)?;
            let mut latest_info = instance
                .lock()
                .await
                .read_large_msgpack::<LatestInfo>(512 as u16)
                .await?;
            latest_info.update_instant = Some(Instant::now());
            LatestInfo::write_to(
                &latest_info,
                LatestInfo::latest_info_path(workspace.local_path(), &member_id),
            )
            .await?;
        }
    }

    info!("Update sheets to {}", member_id);

    // Sync Remote Sheets
    {
        if ctx.is_proc_on_local() {
            let workspace = try_get_local_workspace(&ctx)?;
            let Ok(latest_info) = LatestInfo::read_from(LatestInfo::latest_info_path(
                workspace.local_path(),
                &member_id,
            ))
            .await
            else {
                return Err(TcpTargetError::NotFound(
                    "Latest info not found.".to_string(),
                ));
            };

            // Collect all local versions
            let mut local_versions = vec![];
            for request_sheet in latest_info.my_sheets {
                let Ok(data) = CachedSheet::cached_sheet_data(&request_sheet).await else {
                    // For newly created sheets, the version is 0.
                    // Send -1 to distinguish from 0, ensuring the upstream will definitely send the sheet information
                    local_versions.push((request_sheet, -1));
                    continue;
                };
                local_versions.push((request_sheet, data.write_count()));
            }

            // Send the version list
            let len = local_versions.len();
            instance.lock().await.write_msgpack(local_versions).await?;

            if len < 1 {
                // Don't return here, continue to next section
            } else {
                // Send data to local
                if ctx.is_proc_on_remote() {
                    let vault = try_get_vault(&ctx)?;
                    let mut mut_instance = instance.lock().await;

                    let local_versions =
                        mut_instance.read_msgpack::<Vec<(SheetName, i32)>>().await?;

                    for (sheet_name, local_write_count) in local_versions.iter() {
                        let sheet = vault.sheet(sheet_name).await?;
                        if let Some(holder) = sheet.holder() {
                            if holder == &member_id && &sheet.write_count() != local_write_count {
                                mut_instance.write_msgpack(true).await?;
                                mut_instance
                                    .write_large_msgpack((sheet_name, sheet.to_data()), 1024u16)
                                    .await?;
                            }
                        }
                    }
                    mut_instance.write_msgpack(false).await?;
                }

                // Receive data
                if ctx.is_proc_on_local() {
                    let mut mut_instance = instance.lock().await;
                    loop {
                        let in_coming: bool = mut_instance.read_msgpack().await?;
                        if in_coming {
                            let (sheet_name, data): (SheetName, SheetData) =
                                mut_instance.read_large_msgpack(1024u16).await?;

                            let Some(path) = CachedSheet::cached_sheet_path(sheet_name) else {
                                return Err(TcpTargetError::NotFound(
                                    "Workspace not found".to_string(),
                                ));
                            };

                            SheetData::write_to(&data, path).await?;
                        } else {
                            break;
                        }
                    }
                }
            }
        } else if ctx.is_proc_on_remote() {
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
        }
    }

    info!("Fetch held status to {}", member_id);

    // Sync Held Info
    {
        if ctx.is_proc_on_local() {
            let workspace = try_get_local_workspace(&ctx)?;

            let Ok(latest_info) = LatestInfo::read_from(LatestInfo::latest_info_path(
                workspace.local_path(),
                &member_id,
            ))
            .await
            else {
                return Err(TcpTargetError::NotFound(
                    "Latest info not found.".to_string(),
                ));
            };

            // Collect files that need to know the holder
            let mut holder_wants_know = Vec::new();
            for sheet_name in &latest_info.my_sheets {
                if let Ok(sheet_data) = CachedSheet::cached_sheet_data(sheet_name).await {
                    holder_wants_know
                        .extend(sheet_data.mapping().values().map(|value| value.id.clone()));
                }
            }

            // Send request
            let mut mut_instance = instance.lock().await;
            mut_instance
                .write_large_msgpack(&holder_wants_know, 1024u16)
                .await?;

            // Receive information and write to local
            let result: HashMap<VirtualFileId, Option<MemberId>> =
                mut_instance.read_large_msgpack(1024u16).await?;

            // Read configuration file
            let path = MemberHeld::held_file_path(&member_id)?;
            let mut member_held = match MemberHeld::read_from(&path).await {
                Ok(r) => r,
                Err(_) => MemberHeld::default(),
            };

            // Write the received information
            member_held.update_held_status(result);

            // Write
            MemberHeld::write_to(&member_held, &path).await?;
        }

        if ctx.is_proc_on_remote() {
            let vault = try_get_vault(&ctx)?;
            let mut mut_instance = instance.lock().await;

            // Read the request
            let holder_wants_know: Vec<VirtualFileId> =
                mut_instance.read_large_msgpack(1024u16).await?;

            // Organize the information
            let mut result: HashMap<VirtualFileId, Option<MemberId>> = HashMap::new();
            for id in holder_wants_know {
                let Ok(meta) = vault.virtual_file_meta(&id).await else {
                    continue;
                };
                result.insert(
                    id,
                    if meta.hold_member().is_empty() {
                        None
                    } else {
                        Some(meta.hold_member().to_string())
                    },
                );
            }

            // Send information
            mut_instance.write_large_msgpack(&result, 1024u16).await?;
        }
    }

    // Sync cached sheet to local sheet
    if ctx.is_proc_on_local() {
        let workspace = try_get_local_workspace(&ctx)?;
        let local_sheet_paths =
            extract_sheet_names_from_paths(workspace.local_sheet_paths().await?)?;
        let cached_sheet_paths =
            extract_sheet_names_from_paths(CachedSheet::cached_sheet_paths().await?)?;

        // Match cached sheets and local heets, and sync content
        for (cached_sheet_name, _cached_sheet_path) in cached_sheet_paths {
            // Get local sheet path by cached_sheet_name
            let Some(local_sheet_path) = local_sheet_paths.get(&cached_sheet_name) else {
                continue;
            };

            // Read cached sheet and local sheet
            let cached_sheet = CachedSheet::cached_sheet_data(&cached_sheet_name).await?;
            let Ok(local_sheet_data) = LocalSheetData::read_from(local_sheet_path).await else {
                continue;
            };
            let mut local_sheet =
                local_sheet_data.wrap_to_local_sheet(&workspace, "".to_string(), "".to_string());

            // Read cached id mapping
            let Some(cached_sheet_id_mapping) = cached_sheet.id_mapping() else {
                continue;
            };

            for (cached_item_id, cached_item_path) in cached_sheet_id_mapping.iter() {
                let path_by_id = { local_sheet.path_by_id(cached_item_id).cloned() };

                // Get local path
                let Some(local_path) = path_by_id else {
                    continue;
                };

                if &local_path == cached_item_path {
                    continue;
                }

                // If path not match, try to move
                let move_result = local_sheet.move_mapping(&local_path, cached_item_path);
                match move_result {
                    Err(e) => match e.kind() {
                        ErrorKind::AlreadyExists => {
                            return Ok(UpdateToLatestInfoResult::SyncCachedSheetFail(
                                SyncCachedSheetFailReason::PathAlreadyExist(
                                    cached_item_path.clone(),
                                ),
                            ));
                        }
                        _ => return Err(e.into()),
                    },
                    _ => {}
                }
                local_sheet.write_to_path(&local_sheet_path).await?
            }
        }
    }

    Ok(UpdateToLatestInfoResult::Success)
}

/// Extract sheet names from file paths
fn extract_sheet_names_from_paths(
    paths: Vec<PathBuf>,
) -> Result<HashMap<SheetName, PathBuf>, std::io::Error> {
    let mut result = HashMap::new();
    for p in paths {
        let sheet_name = p
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
            .ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid file name")
            })?;
        result.insert(sheet_name, p);
    }
    Ok(result)
}
