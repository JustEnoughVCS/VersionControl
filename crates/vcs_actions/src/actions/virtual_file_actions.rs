use std::{collections::HashSet, path::PathBuf, sync::Arc};

use action_system::{action::ActionContext, macros::action_gen};
use cfg_file::config::ConfigFile;
use serde::{Deserialize, Serialize};
use tcp_connection::{error::TcpTargetError, instance::ConnectionInstance};
use tokio::sync::Mutex;
use vcs_data::data::{
    local::{
        cached_sheet::CachedSheet, file_status::AnalyzeResult, local_sheet::LocalMappingMetadata,
        member_held::MemberHeld,
    },
    member::MemberId,
    sheet::SheetName,
    vault::virtual_file::{VirtualFileId, VirtualFileVersion, VirtualFileVersionDescription},
};

use crate::actions::{
    auth_member, check_connection_instance, get_current_sheet_name, try_get_local_workspace,
    try_get_vault,
};

#[derive(Serialize, Deserialize)]
pub struct TrackFileActionArguments {
    // Path need to track
    pub relative_pathes: HashSet<PathBuf>,

    // Display progress bar
    pub display_progressbar: bool,
}

#[derive(Serialize, Deserialize)]
pub enum TrackFileActionResult {
    Done {
        created: Vec<PathBuf>,
        updated: Vec<PathBuf>,
        synced: Vec<PathBuf>,
    },

    // Fail
    AuthorizeFailed(String),

    /// There are local move or missing items that have not been resolved,
    /// this situation does not allow track
    StructureChangesNotSolved,

    CreateTaskFailed(CreateTaskResult),
    UpdateTaskFailed(UpdateTaskResult),
    SyncTaskFailed(SyncTaskResult),
}

#[derive(Serialize, Deserialize)]
pub enum CreateTaskResult {
    Success(Vec<PathBuf>), // Success(success_relative_pathes)

    /// Create file on existing path in the sheet
    CreateFileOnExistPath(PathBuf),

    /// Sheet not found
    SheetNotFound(SheetName),
}

#[derive(Serialize, Deserialize)]
pub enum UpdateTaskResult {
    Success(Vec<PathBuf>), // Success(success_relative_pathes)
}

#[derive(Serialize, Deserialize)]
pub enum SyncTaskResult {
    Success(Vec<PathBuf>), // Success(success_relative_pathes)
}
#[action_gen]
pub async fn track_file_action(
    ctx: ActionContext,
    arguments: TrackFileActionArguments,
) -> Result<TrackFileActionResult, TcpTargetError> {
    let relative_pathes = arguments.relative_pathes;
    let instance = check_connection_instance(&ctx)?;

    // Auth Member
    let member_id = match auth_member(&ctx, instance).await {
        Ok(id) => id,
        Err(e) => return Ok(TrackFileActionResult::AuthorizeFailed(e.to_string())),
    };

    // Check sheet
    let sheet_name = get_current_sheet_name(&ctx, instance, &member_id).await?;

    if ctx.is_proc_on_local() {
        let workspace = try_get_local_workspace(&ctx)?;
        let analyzed = AnalyzeResult::analyze_local_status(&workspace).await?;

        if !analyzed.lost.is_empty() || !analyzed.moved.is_empty() {
            return Ok(TrackFileActionResult::StructureChangesNotSolved);
        }

        let Some(sheet_in_use) = workspace.config().lock().await.sheet_in_use().clone() else {
            return Err(TcpTargetError::NotFound("Sheet not found!".to_string()));
        };

        // Read local sheet and member held
        let local_sheet = workspace.local_sheet(&member_id, &sheet_in_use).await?;
        let cached_sheet = CachedSheet::cached_sheet_data(&sheet_in_use).await?;
        let member_held = MemberHeld::read_from(MemberHeld::held_file_path(&member_id)?).await?;

        let modified = analyzed
            .modified
            .intersection(&relative_pathes)
            .cloned()
            .collect::<Vec<_>>();

        // Filter out created files
        let created_task = analyzed
            .created
            .intersection(&relative_pathes)
            .cloned()
            .collect::<Vec<_>>();

        // Filter out modified files that need to be updated
        let update_task: Vec<PathBuf> = {
            let result = modified.iter().filter_map(|p| {
                if let (Ok(local_data), Some(cached_data)) =
                    (local_sheet.mapping_data(p), cached_sheet.mapping().get(p))
                {
                    let id = local_data.mapping_vfid();
                    let local_ver = local_data.version_when_updated();
                    if let Some(held_member) = member_held.file_holder(id) {
                        // Check if holder and version match
                        if held_member == &member_id && local_ver == &cached_data.version {
                            return Some(p.clone());
                        }
                    }
                };
                return None;
            });
            result.collect()
        };

        // Filter out files that do not exist locally or have version inconsistencies and need to be synchronized
        let sync_task: Vec<PathBuf> = {
            let other: Vec<PathBuf> = relative_pathes
                .iter()
                .filter(|p| !created_task.contains(p) && !update_task.contains(p))
                .cloned()
                .collect();

            let result = other.iter().filter_map(|p| {
                // In cached sheet
                let Some(cached_sheet_mapping) = cached_sheet.mapping().get(p) else {
                    return None;
                };

                // Check if path mapping at local sheet
                if let Ok(data) = local_sheet.mapping_data(p) {
                    // Version does not match
                    if data.version_when_updated() != &cached_sheet_mapping.version {
                        return Some(p.clone());
                    }

                    // File modified
                    if modified.contains(p) {
                        return Some(p.clone());
                    }
                }

                return None;
            });
            result.collect()
        };

        // Package tasks
        let tasks: (Vec<PathBuf>, Vec<PathBuf>, Vec<PathBuf>) =
            (created_task, update_task, sync_task);

        // Send to remote
        {
            let mut mut_instance = instance.lock().await;
            mut_instance
                .write_large_msgpack(tasks.clone(), 1024u16)
                .await?;
            // Drop mutex here
        }

        // Process create tasks
        let success_create =
            match proc_create_tasks_local(&ctx, instance.clone(), &member_id, &sheet_name, tasks.0)
                .await
            {
                Ok(r) => match r {
                    CreateTaskResult::Success(relative_pathes) => relative_pathes,
                    _ => {
                        return Ok(TrackFileActionResult::CreateTaskFailed(r));
                    }
                },
                Err(e) => return Err(e),
            };

        // Process update tasks
        let success_update =
            match proc_update_tasks_local(&ctx, instance.clone(), &member_id, &sheet_name, tasks.1)
                .await
            {
                Ok(r) => match r {
                    UpdateTaskResult::Success(relative_pathes) => relative_pathes,
                    _ => {
                        return Ok(TrackFileActionResult::UpdateTaskFailed(r));
                    }
                },
                Err(e) => return Err(e),
            };

        // Process sync tasks
        let success_sync =
            match proc_sync_tasks_local(&ctx, instance.clone(), &member_id, &sheet_name, tasks.2)
                .await
            {
                Ok(r) => match r {
                    SyncTaskResult::Success(relative_pathes) => relative_pathes,
                    _ => {
                        return Ok(TrackFileActionResult::SyncTaskFailed(r));
                    }
                },
                Err(e) => return Err(e),
            };

        return Ok(TrackFileActionResult::Done {
            created: success_create,
            updated: success_update,
            synced: success_sync,
        });
    }

    if ctx.is_proc_on_remote() {
        // Read tasks
        let (created_task, update_task, sync_task): (Vec<PathBuf>, Vec<PathBuf>, Vec<PathBuf>) = {
            let mut mut_instance = instance.lock().await;
            mut_instance.read_large_msgpack(1024u16).await?
        };

        // Process create tasks
        let success_create = match proc_create_tasks_remote(
            &ctx,
            instance.clone(),
            &member_id,
            &sheet_name,
            created_task,
        )
        .await
        {
            Ok(r) => match r {
                CreateTaskResult::Success(relative_pathes) => relative_pathes,
                _ => {
                    return Ok(TrackFileActionResult::CreateTaskFailed(r));
                }
            },
            Err(e) => return Err(e),
        };

        // Process update tasks
        let success_update = match proc_update_tasks_remote(
            &ctx,
            instance.clone(),
            &member_id,
            &sheet_name,
            update_task,
        )
        .await
        {
            Ok(r) => match r {
                UpdateTaskResult::Success(relative_pathes) => relative_pathes,
                _ => {
                    return Ok(TrackFileActionResult::UpdateTaskFailed(r));
                }
            },
            Err(e) => return Err(e),
        };

        // Process sync tasks
        let success_sync = match proc_sync_tasks_remote(
            &ctx,
            instance.clone(),
            &member_id,
            &sheet_name,
            sync_task,
        )
        .await
        {
            Ok(r) => match r {
                SyncTaskResult::Success(relative_pathes) => relative_pathes,
                _ => {
                    return Ok(TrackFileActionResult::SyncTaskFailed(r));
                }
            },
            Err(e) => return Err(e),
        };

        return Ok(TrackFileActionResult::Done {
            created: success_create,
            updated: success_update,
            synced: success_sync,
        });
    }

    Err(TcpTargetError::NoResult("No result.".to_string()))
}

async fn proc_create_tasks_local(
    ctx: &ActionContext,
    instance: Arc<Mutex<ConnectionInstance>>,
    member_id: &MemberId,
    sheet_name: &SheetName,
    relative_paths: Vec<PathBuf>,
) -> Result<CreateTaskResult, TcpTargetError> {
    let workspace = try_get_local_workspace(&ctx)?;
    let mut mut_instance = instance.lock().await;
    let mut local_sheet = workspace.local_sheet(member_id, sheet_name).await?;

    // Wait for remote detection of whether the sheet exists
    let has_sheet = mut_instance.read_msgpack::<bool>().await?;
    if !has_sheet {
        return Ok(CreateTaskResult::SheetNotFound(sheet_name.clone()));
    }

    // Wait for remote detection of whether the file exists
    let (hasnt_duplicate, duplicate_path) = mut_instance.read_msgpack::<(bool, PathBuf)>().await?;
    if !hasnt_duplicate {
        return Ok(CreateTaskResult::CreateFileOnExistPath(duplicate_path));
    }

    let mut success_relative_pathes = Vec::new();

    // Start sending files
    for path in relative_paths {
        let full_path = workspace.local_path().join(&path);

        // Send file
        if let Err(_) = mut_instance.write_file(&full_path).await {
            continue;
        }

        // Read virtual file id and version
        let (vfid, version, version_desc) = mut_instance
            .read_msgpack::<(
                VirtualFileId,
                VirtualFileVersion,
                VirtualFileVersionDescription,
            )>()
            .await?;

        // Add mapping to local sheet
        let hash = sha1_hash::calc_sha1(&full_path, 2048).await.unwrap().hash;
        let time = std::fs::metadata(&full_path)?.modified()?;
        local_sheet.add_mapping(
            path.clone(),
            LocalMappingMetadata::new(
                hash,                                 // hash_when_updated
                time,                                 // time_when_updated
                std::fs::metadata(&full_path)?.len(), // size_when_updated
                version_desc,                         // version_desc_when_updated
                version,                              // version_when_updated
                vfid,                                 // mapping_vfid
                time,                                 // last_modifiy_check_itme
                false,                                // last_modifiy_check_result
            ),
        )?;

        success_relative_pathes.push(path);
    }

    // Write local sheet
    local_sheet.write().await?;

    Ok(CreateTaskResult::Success(success_relative_pathes))
}

async fn proc_create_tasks_remote(
    ctx: &ActionContext,
    instance: Arc<Mutex<ConnectionInstance>>,
    member_id: &MemberId,
    sheet_name: &SheetName,
    relative_paths: Vec<PathBuf>,
) -> Result<CreateTaskResult, TcpTargetError> {
    let vault = try_get_vault(&ctx)?;
    let mut mut_instance = instance.lock().await;

    // Sheet check
    let Ok(mut sheet) = vault.sheet(sheet_name).await else {
        // Sheet not found
        mut_instance.write_msgpack(false).await?;
        return Ok(CreateTaskResult::SheetNotFound(sheet_name.to_string()));
    };
    mut_instance.write_msgpack(true).await?;

    // Duplicate create precheck
    for path in relative_paths.iter() {
        if sheet.mapping().contains_key(path) {
            // Duplicate file
            mut_instance.write_msgpack((false, path)).await?;
            return Ok(CreateTaskResult::CreateFileOnExistPath(path.clone()));
        }
    }
    mut_instance.write_msgpack((true, PathBuf::new())).await?;

    let mut success_relative_pathes = Vec::new();

    // Start receiving files
    for path in relative_paths {
        // Read file and create virtual file
        let Ok(vfid) = vault
            .create_virtual_file_from_connection(&mut mut_instance, member_id)
            .await
        else {
            continue;
        };

        // Record virtual file to sheet
        let vf_meta = vault.virtual_file(&vfid)?.read_meta().await?;
        sheet
            .add_mapping(path.clone(), vfid.clone(), vf_meta.version_latest())
            .await?;

        // Tell client the virtual file id and version
        mut_instance
            .write_msgpack((
                vfid,
                vf_meta.version_latest(),
                vf_meta
                    .version_description(vf_meta.version_latest())
                    .unwrap(),
            ))
            .await?;

        success_relative_pathes.push(path);
    }

    sheet.persist().await?;

    Ok(CreateTaskResult::Success(success_relative_pathes))
}

async fn proc_update_tasks_local(
    ctx: &ActionContext,
    instance: Arc<Mutex<ConnectionInstance>>,
    member_id: &MemberId,
    sheet_name: &SheetName,
    relative_paths: Vec<PathBuf>,
) -> Result<UpdateTaskResult, TcpTargetError> {
    Ok(UpdateTaskResult::Success(Vec::new()))
}

async fn proc_update_tasks_remote(
    ctx: &ActionContext,
    instance: Arc<Mutex<ConnectionInstance>>,
    member_id: &MemberId,
    sheet_name: &SheetName,
    relative_paths: Vec<PathBuf>,
) -> Result<UpdateTaskResult, TcpTargetError> {
    Ok(UpdateTaskResult::Success(Vec::new()))
}

async fn proc_sync_tasks_local(
    ctx: &ActionContext,
    instance: Arc<Mutex<ConnectionInstance>>,
    member_id: &MemberId,
    sheet_name: &SheetName,
    relative_paths: Vec<PathBuf>,
) -> Result<SyncTaskResult, TcpTargetError> {
    Ok(SyncTaskResult::Success(Vec::new()))
}

async fn proc_sync_tasks_remote(
    ctx: &ActionContext,
    instance: Arc<Mutex<ConnectionInstance>>,
    member_id: &MemberId,
    sheet_name: &SheetName,
    relative_paths: Vec<PathBuf>,
) -> Result<SyncTaskResult, TcpTargetError> {
    Ok(SyncTaskResult::Success(Vec::new()))
}
