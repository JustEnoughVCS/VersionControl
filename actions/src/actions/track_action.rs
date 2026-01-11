use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::Arc,
    time::SystemTime,
};

use action_system::{action::ActionContext, macros::action_gen};
use cfg_file::config::ConfigFile;
use serde::{Deserialize, Serialize};
use sha1_hash::calc_sha1;
use tcp_connection::{error::TcpTargetError, instance::ConnectionInstance};
use tokio::{fs, sync::Mutex};
use vcs_data::{
    constants::CLIENT_FILE_TEMP_FILE,
    data::{
        local::{
            cached_sheet::CachedSheet, latest_file_data::LatestFileData,
            local_sheet::LocalMappingMetadata, vault_modified::sign_vault_modified,
            workspace_analyzer::AnalyzeResult,
        },
        member::MemberId,
        sheet::SheetName,
        vault::{
            config::VaultUuid,
            virtual_file::{VirtualFileId, VirtualFileVersion, VirtualFileVersionDescription},
        },
    },
};

use crate::{
    actions::{
        auth_member, check_connection_instance, get_current_sheet_name, try_get_local_output,
        try_get_local_workspace, try_get_vault,
    },
    local_println,
};

pub type NextVersion = String;
pub type UpdateDescription = String;

const TEMP_NAME: &str = "{temp_name}";

#[derive(Serialize, Deserialize)]
pub struct TrackFileActionArguments {
    // Path need to track
    pub relative_pathes: HashSet<PathBuf>,

    // File update info
    pub file_update_info: HashMap<PathBuf, (NextVersion, UpdateDescription)>,

    // Print infos
    pub print_infos: bool,

    // overwrite modified files
    pub allow_overwrite_modified: bool,
}

#[derive(Serialize, Deserialize)]
pub enum TrackFileActionResult {
    Done {
        created: Vec<PathBuf>,
        updated: Vec<PathBuf>,
        synced: Vec<PathBuf>,
        skipped: Vec<PathBuf>,
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

    VerifyFailed {
        path: PathBuf,
        reason: VerifyFailReason,
    },
}

#[derive(Serialize, Deserialize, Clone)]
pub enum VerifyFailReason {
    SheetNotFound(SheetName),
    MappingNotFound,
    VirtualFileNotFound(VirtualFileId),
    VirtualFileReadFailed(VirtualFileId),
    NotHeld,
    VersionDismatch(VirtualFileVersion, VirtualFileVersion), // (CurrentVersion, RemoteVersion)
    UpdateButNoDescription, // File needs update, but no description exists
    VersionAlreadyExist(VirtualFileVersion), // (RemoteVersion)
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
    let (member_id, is_host_mode) = match auth_member(&ctx, instance).await {
        Ok(id) => id,
        Err(e) => return Ok(TrackFileActionResult::AuthorizeFailed(e.to_string())),
    };

    // Check sheet
    let (sheet_name, is_ref_sheet) =
        get_current_sheet_name(&ctx, instance, &member_id, true).await?;

    // Can modify Sheet when not in reference sheet or in Host mode
    let can_modify_sheet = !is_ref_sheet || is_host_mode;

    if ctx.is_proc_on_local() {
        let workspace = try_get_local_workspace(&ctx)?;
        let analyzed = AnalyzeResult::analyze_local_status(&workspace).await?;
        let latest_file_data =
            LatestFileData::read_from(LatestFileData::data_path(&member_id)?).await?;

        if !analyzed.lost.is_empty() || !analyzed.moved.is_empty() {
            return Ok(TrackFileActionResult::StructureChangesNotSolved);
        }

        let Some(sheet_in_use) = workspace.config().lock().await.sheet_in_use().clone() else {
            return Err(TcpTargetError::NotFound("Sheet not found!".to_string()));
        };

        // Read local sheet and member held
        let local_sheet = workspace.local_sheet(&member_id, &sheet_in_use).await?;
        let cached_sheet = CachedSheet::cached_sheet_data(&sheet_in_use).await?;
        let member_held = LatestFileData::read_from(LatestFileData::data_path(&member_id)?).await?;

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
        let mut update_task: Vec<PathBuf> = {
            let result = modified.iter().filter_map(|p| {
                if let Ok(local_data) = local_sheet.mapping_data(p) {
                    let id = local_data.mapping_vfid();
                    let local_ver = local_data.version_when_updated();
                    let Some(latest_ver) = latest_file_data.file_version(id) else {
                        return None;
                    };
                    if let Some(held_member) = member_held.file_holder(id) {
                        // Check if holder and version match
                        if held_member == &member_id && local_ver == latest_ver {
                            return Some(p.clone());
                        }
                    }
                };
                None
            });
            result.collect()
        };

        let mut skipped_task: Vec<PathBuf> = Vec::new();

        // Filter out files that do not exist locally or have version inconsistencies and need to be synchronized
        let mut sync_task: Vec<PathBuf> = {
            let other: Vec<PathBuf> = relative_pathes
                .iter()
                .filter(|p| !created_task.contains(p) && !update_task.contains(p))
                .cloned()
                .collect();

            let result = other.iter().filter_map(|p| {
                // Not exists and not lost, first download
                if !workspace.local_path().join(p).exists() && !analyzed.lost.contains(p) {
                    return Some(p.clone());
                }

                // In cached sheet
                if !cached_sheet.mapping().contains_key(p) {
                    return None;
                }

                // In local sheet
                let local_sheet_mapping = local_sheet.mapping_data(p).ok()?;
                let vfid = local_sheet_mapping.mapping_vfid();

                if let Some(latest_version) = &latest_file_data.file_version(vfid) {
                    // Version does not match
                    if &local_sheet_mapping.version_when_updated() != latest_version {
                        let modified = modified.contains(p);
                        if modified && arguments.allow_overwrite_modified {
                            return Some(p.clone());
                        } else if modified && !arguments.allow_overwrite_modified {
                            // If not allowed to overwrite, join skipped tasks
                            skipped_task.push(p.clone());
                            return None;
                        }
                        return Some(p.clone());
                    }
                }

                // File not held and modified
                let holder = latest_file_data.file_holder(vfid);
                if (holder.is_none() || &member_id != holder.unwrap()) && modified.contains(p) {
                    // If allow overwrite modified is true, overwrite the file
                    if arguments.allow_overwrite_modified {
                        return Some(p.clone());
                    } else {
                        // If not allowed to overwrite, join skipped tasks
                        skipped_task.push(p.clone());
                        return None;
                    }
                }

                None
            });
            result.collect()
        };

        // If the sheet cannot be modified,
        // the update_task here should be considered invalid and changed to sync rollback
        if !can_modify_sheet {
            if arguments.allow_overwrite_modified {
                sync_task.append(&mut update_task);
                update_task.clear();
            } else {
                skipped_task.append(&mut update_task);
                update_task.clear();
            }
        }

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
        let mut success_create = Vec::<PathBuf>::new();
        if can_modify_sheet {
            success_create = match proc_create_tasks_local(
                &ctx,
                instance.clone(),
                &member_id,
                &sheet_name,
                tasks.0,
                arguments.print_infos,
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
        }

        // Process update tasks
        let mut success_update = Vec::<PathBuf>::new();
        if can_modify_sheet {
            success_update = match proc_update_tasks_local(
                &ctx,
                instance.clone(),
                &member_id,
                &sheet_name,
                tasks.1,
                arguments.print_infos,
                arguments.file_update_info,
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
        }

        // Process sync tasks
        let success_sync = match proc_sync_tasks_local(
            &ctx,
            instance.clone(),
            &member_id,
            &sheet_name,
            tasks.2,
            arguments.print_infos,
        )
        .await
        {
            Ok(r) => match r {
                SyncTaskResult::Success(relative_pathes) => relative_pathes,
            },
            Err(e) => return Err(e),
        };

        if success_create.len() + success_update.len() > 0 {
            sign_vault_modified(true).await;
        }

        return Ok(TrackFileActionResult::Done {
            created: success_create,
            updated: success_update,
            synced: success_sync,
            skipped: skipped_task,
        });
    }

    if ctx.is_proc_on_remote() {
        // Read tasks
        let (created_task, update_task, sync_task): (Vec<PathBuf>, Vec<PathBuf>, Vec<PathBuf>) = {
            let mut mut_instance = instance.lock().await;
            mut_instance.read_large_msgpack(1024u16).await?
        };

        // Process create tasks
        let mut success_create = Vec::<PathBuf>::new();
        if can_modify_sheet {
            success_create = match proc_create_tasks_remote(
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
        }

        // Process update tasks
        let mut success_update = Vec::<PathBuf>::new();
        if can_modify_sheet {
            success_update = match proc_update_tasks_remote(
                &ctx,
                instance.clone(),
                &member_id,
                &sheet_name,
                update_task,
                arguments.file_update_info,
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
        }

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
            },
            Err(e) => return Err(e),
        };

        return Ok(TrackFileActionResult::Done {
            created: success_create,
            updated: success_update,
            synced: success_sync,
            skipped: Vec::new(), // The server doesn't know which files were skipped
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
    print_infos: bool,
) -> Result<CreateTaskResult, TcpTargetError> {
    let workspace = try_get_local_workspace(ctx)?;
    let local_output = try_get_local_output(ctx)?;
    let mut mut_instance = instance.lock().await;
    let mut local_sheet = workspace.local_sheet(member_id, sheet_name).await?;

    if print_infos && relative_paths.len() > 0 {
        local_println!(local_output, "Creating {} files...", relative_paths.len());
    }

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
        if mut_instance.write_file(&full_path).await.is_err() {
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
            &path.clone(),
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

        // Print success info
        if print_infos {
            local_println!(local_output, "+ {}", path.display());
        }

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
    let vault = try_get_vault(ctx)?;
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
    print_infos: bool,
    file_update_info: HashMap<PathBuf, (NextVersion, UpdateDescription)>,
) -> Result<UpdateTaskResult, TcpTargetError> {
    let workspace = try_get_local_workspace(ctx)?;
    let local_output = try_get_local_output(ctx)?;
    let mut mut_instance = instance.lock().await;
    let mut local_sheet = workspace.local_sheet(member_id, sheet_name).await?;

    let mut success = Vec::new();

    if print_infos && relative_paths.len() > 0 {
        local_println!(local_output, "Updating {} files...", relative_paths.len());
    }

    for path in relative_paths.iter() {
        let Ok(mapping) = local_sheet.mapping_data(path) else {
            // Is mapping not found, write empty
            mut_instance.write_msgpack("".to_string()).await?;
            continue;
        };
        // Read and send file version
        let Ok(_) = mut_instance
            .write_msgpack(mapping.version_when_updated())
            .await
        else {
            continue;
        };

        // Read verify result
        let verify_result: bool = mut_instance.read_msgpack().await?;
        if !verify_result {
            let reason = mut_instance.read_msgpack::<VerifyFailReason>().await?;
            return Ok(UpdateTaskResult::VerifyFailed {
                path: path.clone(),
                reason: reason.clone(),
            });
        }

        // Calc hash
        let hash_result = match sha1_hash::calc_sha1(workspace.local_path().join(path), 2048).await
        {
            Ok(r) => r,
            Err(_) => {
                mut_instance.write_msgpack(false).await?; // Not Ready
                continue;
            }
        };

        // Get next version
        let Some((next_version, description)) = file_update_info.get(path) else {
            mut_instance.write_msgpack(false).await?; // Not Ready
            continue;
        };

        // Write
        mut_instance.write_msgpack(true).await?; // Ready
        mut_instance.write_file(path).await?;

        // Read upload result
        let upload_result: bool = mut_instance.read_msgpack().await?;
        if upload_result {
            // Success
            let mapping_data_mut = local_sheet.mapping_data_mut(path).unwrap();
            let version = mapping_data_mut.version_when_updated().clone();
            mapping_data_mut.set_hash_when_updated(hash_result.hash);
            mapping_data_mut.set_version_when_updated(next_version.clone());
            mapping_data_mut.set_version_desc_when_updated(VirtualFileVersionDescription {
                creator: member_id.clone(),
                description: description.clone(),
            });
            mapping_data_mut.set_last_modifiy_check_result(false); // Mark file not modified

            // Write
            local_sheet.write().await?;

            // Push path into success vec
            success.push(path.clone());

            // Print success info
            if print_infos {
                local_println!(
                    local_output,
                    "↑ {} ({} -> {})",
                    path.display(),
                    version,
                    next_version
                );
            }
        }
    }

    Ok(UpdateTaskResult::Success(success))
}

async fn proc_update_tasks_remote(
    ctx: &ActionContext,
    instance: Arc<Mutex<ConnectionInstance>>,
    member_id: &MemberId,
    sheet_name: &SheetName,
    relative_paths: Vec<PathBuf>,
    file_update_info: HashMap<PathBuf, (NextVersion, UpdateDescription)>,
) -> Result<UpdateTaskResult, TcpTargetError> {
    let vault = try_get_vault(ctx)?;
    let mut mut_instance = instance.lock().await;

    let mut success = Vec::new();

    for path in relative_paths.iter() {
        // Read version
        let Ok(version) = mut_instance.read_msgpack::<VirtualFileVersion>().await else {
            continue;
        };
        if version.is_empty() {
            continue;
        }

        // Verify
        let Some((next_version, description)) = file_update_info.get(path) else {
            mut_instance.write_msgpack(false).await?;
            let reason = VerifyFailReason::UpdateButNoDescription;
            mut_instance.write_msgpack(reason.clone()).await?;
            return Ok(UpdateTaskResult::VerifyFailed {
                path: path.clone(),
                reason,
            }); // Sheet not found
        };
        let Ok(mut sheet) = vault.sheet(sheet_name).await else {
            mut_instance.write_msgpack(false).await?;
            let reason = VerifyFailReason::SheetNotFound(sheet_name.clone());
            mut_instance.write_msgpack(reason.clone()).await?;
            return Ok(UpdateTaskResult::VerifyFailed {
                path: path.clone(),
                reason,
            }); // Sheet not found
        };
        let Some(mapping_data) = sheet.mapping_mut().get_mut(path) else {
            mut_instance.write_msgpack(false).await?;
            let reason = VerifyFailReason::MappingNotFound;
            mut_instance.write_msgpack(reason.clone()).await?;
            return Ok(UpdateTaskResult::VerifyFailed {
                path: path.clone(),
                reason,
            }); // Mapping not found
        };
        let Ok(vf) = vault.virtual_file(&mapping_data.id) else {
            mut_instance.write_msgpack(false).await?;
            let reason = VerifyFailReason::VirtualFileNotFound(mapping_data.id.clone());
            mut_instance.write_msgpack(reason.clone()).await?;
            return Ok(UpdateTaskResult::VerifyFailed {
                path: path.clone(),
                reason,
            }); // Virtual file not found
        };
        let Ok(vf_metadata) = vf.read_meta().await else {
            mut_instance.write_msgpack(false).await?;
            let reason = VerifyFailReason::VirtualFileReadFailed(mapping_data.id.clone());
            mut_instance.write_msgpack(reason.clone()).await?;
            return Ok(UpdateTaskResult::VerifyFailed {
                path: path.clone(),
                reason,
            }); // Read virtual file metadata failed
        };
        if vf_metadata.versions().contains(next_version) {
            mut_instance.write_msgpack(false).await?;
            let reason = VerifyFailReason::VersionAlreadyExist(version);
            mut_instance.write_msgpack(reason.clone()).await?;
            return Ok(UpdateTaskResult::VerifyFailed {
                path: path.clone(),
                reason,
            }); // VersionAlreadyExist
        }
        if vf_metadata.hold_member() != member_id {
            mut_instance.write_msgpack(false).await?;
            let reason = VerifyFailReason::NotHeld;
            mut_instance.write_msgpack(reason.clone()).await?;
            return Ok(UpdateTaskResult::VerifyFailed {
                path: path.clone(),
                reason,
            }); // Member not held it
        };
        if vf_metadata.version_latest() != version {
            mut_instance.write_msgpack(false).await?;
            let reason =
                VerifyFailReason::VersionDismatch(version.clone(), vf_metadata.version_latest());
            mut_instance.write_msgpack(reason.clone()).await?;
            return Ok(UpdateTaskResult::VerifyFailed {
                path: path.clone(),
                reason,
            }); // Version does not match
        };
        mut_instance.write_msgpack(true).await?; // Verified

        // Read if local ready
        let ready: bool = mut_instance.read_msgpack().await?;
        if !ready {
            continue;
        }

        // Read and update virtual file
        match vault
            .update_virtual_file_from_connection(
                &mut mut_instance,
                member_id,
                &mapping_data.id,
                next_version,
                VirtualFileVersionDescription {
                    creator: member_id.clone(),
                    description: description.clone(),
                },
            )
            .await
        {
            Ok(_) => {
                // Update version to sheet
                mapping_data.version = next_version.clone();

                // Persist
                sheet.persist().await?;

                success.push(path.clone());
                mut_instance.write_msgpack(true).await?; // Success
            }
            Err(e) => {
                mut_instance.write_msgpack(false).await?; // Fail
                return Err(e.into());
            }
        }
    }

    Ok(UpdateTaskResult::Success(success))
}

type SyncVersionInfo = Option<(
    VirtualFileVersion,
    VirtualFileVersionDescription,
    VirtualFileId,
)>;

async fn proc_sync_tasks_local(
    ctx: &ActionContext,
    instance: Arc<Mutex<ConnectionInstance>>,
    member_id: &MemberId,
    sheet_name: &SheetName,
    relative_paths: Vec<PathBuf>,
    print_infos: bool,
) -> Result<SyncTaskResult, TcpTargetError> {
    let workspace = try_get_local_workspace(ctx)?;
    let local_output = try_get_local_output(ctx)?;
    let mut mut_instance = instance.lock().await;
    let mut success: Vec<PathBuf> = Vec::new();

    if print_infos && relative_paths.len() > 0 {
        local_println!(local_output, "Syncing {} files...", relative_paths.len());
    }

    for path in relative_paths {
        let Some((version, description, vfid)) =
            mut_instance.read_msgpack::<SyncVersionInfo>().await?
        else {
            continue;
        };

        // Generate a temp path
        let temp_path = workspace
            .local_path()
            .join(CLIENT_FILE_TEMP_FILE.replace(TEMP_NAME, &VaultUuid::new_v4().to_string()));

        let copy_to = workspace.local_path().join(&path);

        // Read file
        match mut_instance.read_file(&temp_path).await {
            Ok(_) => {
                if !temp_path.exists() {
                    continue;
                }
            }
            Err(_) => {
                continue;
            }
        }

        // Calc hash
        let new_hash = match calc_sha1(&temp_path, 2048).await {
            Ok(hash) => hash,
            Err(_) => {
                continue;
            }
        };

        // Calc size
        let new_size = match fs::metadata(&temp_path).await.map(|meta| meta.len()) {
            Ok(size) => size,
            Err(_) => {
                continue;
            }
        };

        // Write file
        if copy_to.exists() {
            if let Err(_) = fs::remove_file(&copy_to).await {
                continue;
            }
        } else {
            // Not exist, create directory
            if let Some(path) = copy_to.clone().parent() {
                fs::create_dir_all(path).await?;
            }
        }
        if let Err(_) = fs::rename(&temp_path, &copy_to).await {
            continue;
        }

        // Modify local sheet
        let mut local_sheet = match workspace.local_sheet(member_id, sheet_name).await {
            Ok(sheet) => sheet,
            Err(_) => {
                continue;
            }
        };

        // Get or create mapping
        let mapping = match local_sheet.mapping_data_mut(&path) {
            Ok(m) => m,
            Err(_) => {
                // First download
                let mut data = LocalMappingMetadata::default();
                data.set_mapping_vfid(vfid);
                if let Err(_) = local_sheet.add_mapping(&path, data) {
                    continue;
                }
                match local_sheet.mapping_data_mut(&path) {
                    Ok(m) => m,
                    Err(_) => {
                        continue;
                    }
                }
            }
        };

        let time = SystemTime::now();
        mapping.set_hash_when_updated(new_hash.hash);
        mapping.set_last_modifiy_check_result(false); // Mark not modified
        mapping.set_version_when_updated(version);
        mapping.set_version_desc_when_updated(description);
        mapping.set_size_when_updated(new_size);
        mapping.set_time_when_updated(time);
        mapping.set_last_modifiy_check_time(time);
        if let Err(_) = local_sheet.write().await {
            continue;
        }

        success.push(path.clone());

        // Print success info
        if print_infos {
            local_println!(local_output, "↓ {}", path.display());
        }
    }
    Ok(SyncTaskResult::Success(success))
}

async fn proc_sync_tasks_remote(
    ctx: &ActionContext,
    instance: Arc<Mutex<ConnectionInstance>>,
    _member_id: &MemberId,
    sheet_name: &SheetName,
    relative_paths: Vec<PathBuf>,
) -> Result<SyncTaskResult, TcpTargetError> {
    let vault = try_get_vault(ctx)?;
    let sheet = vault.sheet(sheet_name).await?;
    let mut mut_instance = instance.lock().await;
    let mut success: Vec<PathBuf> = Vec::new();

    for path in relative_paths {
        // Get mapping
        let Some(mapping) = sheet.mapping().get(&path) else {
            mut_instance.write_msgpack::<SyncVersionInfo>(None).await?; // (ready)
            continue;
        };
        // Get virtual file
        let Ok(vf) = vault.virtual_file(&mapping.id) else {
            mut_instance.write_msgpack::<SyncVersionInfo>(None).await?; // (ready)
            continue;
        };
        // Read metadata and get real path
        let vf_meta = &vf.read_meta().await?;
        let real_path = vault.virtual_file_real_path(&mapping.id, &vf_meta.version_latest());
        let version = vf_meta.version_latest();
        mut_instance
            .write_msgpack::<SyncVersionInfo>(Some((
                version.clone(),
                vf_meta.version_description(version).cloned().unwrap_or(
                    VirtualFileVersionDescription {
                        creator: MemberId::default(),
                        description: "".to_string(),
                    },
                ),
                vf.id(),
            )))
            .await?; // (ready)
        if mut_instance.write_file(real_path).await.is_err() {
            continue;
        } else {
            success.push(path);
        }
    }
    Ok(SyncTaskResult::Success(success))
}
