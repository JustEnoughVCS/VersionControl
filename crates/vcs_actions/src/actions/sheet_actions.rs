use std::{collections::HashMap, io::ErrorKind};

use action_system::{action::ActionContext, macros::action_gen};
use serde::{Deserialize, Serialize};
use tcp_connection::error::TcpTargetError;
use vcs_data::{
    constants::VAULT_HOST_NAME,
    data::{
        local::{
            vault_modified::sign_vault_modified,
            workspace_analyzer::{FromRelativePathBuf, ToRelativePathBuf},
        },
        sheet::SheetName,
        vault::sheet_share::{ShareMergeMode, SheetShareId},
    },
};

use crate::{
    actions::{
        auth_member, check_connection_instance, get_current_sheet_name, try_get_local_workspace,
        try_get_vault,
    },
    write_and_return,
};

#[derive(Default, Serialize, Deserialize)]
pub enum MakeSheetActionResult {
    Success,
    SuccessRestore,

    // Fail
    AuthorizeFailed(String),
    SheetAlreadyExists,
    SheetCreationFailed(String),

    #[default]
    Unknown,
}

/// Build a sheet with context
#[action_gen]
pub async fn make_sheet_action(
    ctx: ActionContext,
    sheet_name: SheetName,
) -> Result<MakeSheetActionResult, TcpTargetError> {
    let instance = check_connection_instance(&ctx)?;

    // Auth Member
    let (member_id, is_host_mode) = match auth_member(&ctx, instance).await {
        Ok(id) => id,
        Err(e) => return Ok(MakeSheetActionResult::AuthorizeFailed(e.to_string())),
    };

    if ctx.is_proc_on_remote() {
        let vault = try_get_vault(&ctx)?;
        let holder = if is_host_mode {
            VAULT_HOST_NAME.to_string()
        } else {
            member_id
        };

        // Check if the sheet already exists
        if let Ok(mut sheet) = vault.sheet(&sheet_name).await {
            // If the sheet has no holder, assign it to the current member (restore operation)
            if sheet.holder().is_none() {
                sheet.set_holder(holder.clone());
                match sheet.persist().await {
                    Ok(_) => {
                        write_and_return!(instance, MakeSheetActionResult::SuccessRestore);
                    }
                    Err(e) => {
                        write_and_return!(
                            instance,
                            MakeSheetActionResult::SheetCreationFailed(e.to_string())
                        );
                    }
                }
            } else {
                write_and_return!(instance, MakeSheetActionResult::SheetAlreadyExists);
            }
        } else {
            // Create the sheet
            match vault.create_sheet(&sheet_name, &holder).await {
                Ok(_) => {
                    write_and_return!(instance, MakeSheetActionResult::Success);
                }
                Err(e) => {
                    write_and_return!(
                        instance,
                        MakeSheetActionResult::SheetCreationFailed(e.to_string())
                    );
                }
            }
        }
    }

    if ctx.is_proc_on_local() {
        let result = instance
            .lock()
            .await
            .read::<MakeSheetActionResult>()
            .await?;
        if matches!(result, MakeSheetActionResult::Success) {
            sign_vault_modified(true).await;
        }
        return Ok(result);
    }

    Err(TcpTargetError::NoResult("No result.".to_string()))
}

#[derive(Default, Serialize, Deserialize)]
pub enum DropSheetActionResult {
    Success,

    // Fail
    SheetInUse,
    AuthorizeFailed(String),
    SheetNotExists,
    SheetDropFailed(String),
    NoHolder,
    NotOwner,

    #[default]
    Unknown,
}

#[action_gen]
pub async fn drop_sheet_action(
    ctx: ActionContext,
    sheet_name: SheetName,
) -> Result<DropSheetActionResult, TcpTargetError> {
    let instance = check_connection_instance(&ctx)?;

    // Auth Member
    let (member_id, is_host_mode) = match auth_member(&ctx, instance).await {
        Ok(id) => id,
        Err(e) => {
            return Ok(DropSheetActionResult::AuthorizeFailed(e.to_string()));
        }
    };

    // Check sheet in use on local
    if ctx.is_proc_on_local() {
        let local_workspace = try_get_local_workspace(&ctx)?;
        if let Some(sheet) = local_workspace.config().lock().await.sheet_in_use() {
            if sheet == &sheet_name {
                instance.lock().await.write(false).await?;
                return Ok(DropSheetActionResult::SheetInUse);
            }
            instance.lock().await.write(true).await?;
        } else {
            instance.lock().await.write(true).await?;
        }
    }

    if ctx.is_proc_on_remote() {
        // Check if client sheet is in use
        let sheet_in_use = instance.lock().await.read::<bool>().await?;
        if !sheet_in_use {
            return Ok(DropSheetActionResult::SheetInUse);
        }

        let vault = try_get_vault(&ctx)?;

        // Check if the sheet exists
        let mut sheet = match vault.sheet(&sheet_name).await {
            Ok(sheet) => sheet,
            Err(e) => {
                if e.kind() == ErrorKind::NotFound {
                    write_and_return!(instance, DropSheetActionResult::SheetNotExists);
                } else {
                    write_and_return!(
                        instance,
                        DropSheetActionResult::SheetDropFailed(e.to_string())
                    );
                }
            }
        };

        // Get the sheet's holder
        let Some(holder) = sheet.holder() else {
            write_and_return!(instance, DropSheetActionResult::NoHolder);
        };

        // Verify that the sheet holder is either the current user or the host
        // All sheets belong to the host
        if holder != &member_id && !is_host_mode {
            write_and_return!(instance, DropSheetActionResult::NotOwner);
        }

        // Drop the sheet
        sheet.forget_holder();
        match sheet.persist().await {
            Ok(_) => {
                write_and_return!(instance, DropSheetActionResult::Success);
            }
            Err(e) => {
                write_and_return!(
                    instance,
                    DropSheetActionResult::SheetDropFailed(e.to_string())
                );
            }
        }
    }

    if ctx.is_proc_on_local() {
        let result = instance
            .lock()
            .await
            .read::<DropSheetActionResult>()
            .await?;
        if matches!(result, DropSheetActionResult::Success) {
            sign_vault_modified(true).await;
        }
        return Ok(result);
    }

    Err(TcpTargetError::NoResult("No result.".to_string()))
}

pub type OperationArgument = (EditMappingOperations, Option<ToRelativePathBuf>);

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum EditMappingOperations {
    Move,
    Erase,
}

#[derive(Serialize, Deserialize, Default)]
pub enum EditMappingActionResult {
    Success,

    // Fail
    AuthorizeFailed(String),
    EditNotAllowed,
    MappingNotFound(FromRelativePathBuf),
    InvalidMove(InvalidMoveReason),

    #[default]
    Unknown,
}

#[derive(Serialize, Deserialize)]
pub enum InvalidMoveReason {
    MoveOperationButNoTarget(FromRelativePathBuf),
    ContainsDuplicateMapping(ToRelativePathBuf),
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EditMappingActionArguments {
    pub operations: HashMap<FromRelativePathBuf, OperationArgument>,
}

/// This Action only modifies Sheet Mapping and
/// does not interfere with the actual location of local files or Local Mapping
#[action_gen]
pub async fn edit_mapping_action(
    ctx: ActionContext,
    args: EditMappingActionArguments,
) -> Result<EditMappingActionResult, TcpTargetError> {
    let instance = check_connection_instance(&ctx)?;

    // Auth Member
    let (member_id, is_host_mode) = match auth_member(&ctx, instance).await {
        Ok(id) => id,
        Err(e) => {
            return Ok(EditMappingActionResult::AuthorizeFailed(e.to_string()));
        }
    };

    // Check sheet
    let (sheet_name, is_ref_sheet) =
        get_current_sheet_name(&ctx, instance, &member_id, true).await?;

    // Can modify Sheet when not in reference sheet or in Host mode
    let can_modify_sheet = !is_ref_sheet || is_host_mode;

    if !can_modify_sheet {
        return Ok(EditMappingActionResult::EditNotAllowed);
    }

    if ctx.is_proc_on_remote() {
        let vault = try_get_vault(&ctx)?;
        let mut sheet = vault.sheet(&sheet_name).await?;

        // Precheck
        for (from_path, (operation, to_path)) in args.operations.iter() {
            // Check mapping exists
            if !sheet.mapping().contains_key(from_path) {
                write_and_return!(
                    instance,
                    EditMappingActionResult::MappingNotFound(from_path.clone())
                );
            }

            // Move check
            if operation == &EditMappingOperations::Move {
                // Check if target exists
                if let Some(to_path) = to_path {
                    // Check if target is duplicate
                    if sheet.mapping().contains_key(to_path) {
                        write_and_return!(
                            instance,
                            EditMappingActionResult::InvalidMove(
                                InvalidMoveReason::ContainsDuplicateMapping(to_path.clone())
                            )
                        );
                    }
                } else {
                    write_and_return!(
                        instance,
                        EditMappingActionResult::InvalidMove(
                            InvalidMoveReason::MoveOperationButNoTarget(from_path.clone())
                        )
                    );
                }
            }
        }

        // Process
        for (from_path, (operation, to_path)) in args.operations {
            match operation {
                // During the Precheck phase, it has been ensured that:
                // 1. The mapping to be edited for the From path indeed exists
                // 2. The location of the To path is indeed empty
                // 3. In Move mode, To path can be safely unwrapped
                // Therefore, the following unwrap() calls are safe to execute
                EditMappingOperations::Move => {
                    let mapping = sheet.mapping_mut().remove(&from_path).unwrap();
                    let to_path = to_path.unwrap();
                    sheet
                        .add_mapping(to_path, mapping.id, mapping.version)
                        .await?;
                }
                EditMappingOperations::Erase => {
                    sheet.mapping_mut().remove(&from_path).unwrap();
                }
            }
        }

        // Write
        sheet.persist().await?;

        write_and_return!(instance, EditMappingActionResult::Success);
    }

    if ctx.is_proc_on_local() {
        let result = instance
            .lock()
            .await
            .read::<EditMappingActionResult>()
            .await?;
        if matches!(result, EditMappingActionResult::Success) {
            sign_vault_modified(true).await;
        }
        return Ok(result);
    }

    Ok(EditMappingActionResult::Success)
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct ShareMappingArguments {
    pub mappings: Vec<FromRelativePathBuf>,
    pub description: String,
    // None = current sheet,
    // Some(sheet_name) = other ref(public) sheet
    pub from_sheet: Option<SheetName>,
    pub to_sheet: SheetName,
}

#[derive(Serialize, Deserialize, Default)]
pub enum ShareMappingActionResult {
    Success,

    // Fail
    AuthorizeFailed(String),
    TargetSheetNotFound(SheetName),
    TargetIsSelf,
    MappingNotFound(FromRelativePathBuf),

    #[default]
    Unknown,
}

#[action_gen]
pub async fn share_mapping_action(
    ctx: ActionContext,
    args: ShareMappingArguments,
) -> Result<ShareMappingActionResult, TcpTargetError> {
    let instance = check_connection_instance(&ctx)?;

    // Auth Member
    let (member_id, _is_host_mode) = match auth_member(&ctx, instance).await {
        Ok(id) => id,
        Err(e) => {
            return Ok(ShareMappingActionResult::AuthorizeFailed(e.to_string()));
        }
    };

    // Check sheet
    let sheet_name = args.from_sheet.unwrap_or(
        get_current_sheet_name(&ctx, instance, &member_id, true)
            .await?
            .0,
    );

    if ctx.is_proc_on_remote() {
        let vault = try_get_vault(&ctx)?;
        let sheet = vault.sheet(&sheet_name).await?;

        // Tip: Because sheet_name may specify a sheet that does not belong to the user,
        // a secondary verification is required.

        // Check if the sheet holder is Some and matches the member_id or is the host
        let Some(holder) = sheet.holder() else {
            // If there's no holder, the sheet cannot be shared from
            write_and_return!(
                instance,
                ShareMappingActionResult::AuthorizeFailed("Sheet has no holder".to_string())
            );
        };

        // Verify the holder is either the current member or the host
        if holder != &member_id && holder != VAULT_HOST_NAME {
            write_and_return!(
                instance,
                ShareMappingActionResult::AuthorizeFailed(
                    "Not sheet holder or ref sheet".to_string()
                )
            );
        }

        let to_sheet_name = args.to_sheet;

        // Verify target sheet exists
        if !vault.sheet_names()?.contains(&to_sheet_name) {
            // Does not exist
            write_and_return!(
                instance,
                ShareMappingActionResult::TargetSheetNotFound(to_sheet_name.clone())
            );
        }

        // Verify sheet is not self
        if sheet_name == to_sheet_name {
            // Is self
            write_and_return!(instance, ShareMappingActionResult::TargetIsSelf);
        }

        // Verify all mappings are correct
        for mapping in args.mappings.iter() {
            if !sheet.mapping().contains_key(mapping) {
                // If any mapping is invalid, indicate failure
                write_and_return!(
                    instance,
                    ShareMappingActionResult::MappingNotFound(mapping.clone())
                );
            }
        }

        // Execute sharing logic
        sheet
            .share_mappings(&to_sheet_name, args.mappings, &member_id, args.description)
            .await?;

        // Sharing successful
        write_and_return!(instance, ShareMappingActionResult::Success);
    }

    if ctx.is_proc_on_local() {
        let result = instance
            .lock()
            .await
            .read::<ShareMappingActionResult>()
            .await?;
        return Ok(result);
    }

    Ok(ShareMappingActionResult::Success)
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct MergeShareMappingArguments {
    pub share_id: SheetShareId,
    pub share_merge_mode: ShareMergeMode,
}

#[derive(Serialize, Deserialize, Default)]
pub enum MergeShareMappingActionResult {
    Success,

    // Fail
    HasConflicts,
    AuthorizeFailed(String),
    EditNotAllowed,
    ShareIdNotFound(SheetShareId),
    MergeFails(String),

    #[default]
    Unknown,
}

#[action_gen]
pub async fn merge_share_mapping_action(
    ctx: ActionContext,
    args: MergeShareMappingArguments,
) -> Result<MergeShareMappingActionResult, TcpTargetError> {
    let instance = check_connection_instance(&ctx)?;

    // Auth Member
    let (member_id, is_host_mode) = match auth_member(&ctx, instance).await {
        Ok(id) => id,
        Err(e) => {
            return Ok(MergeShareMappingActionResult::AuthorizeFailed(
                e.to_string(),
            ));
        }
    };

    // Check sheet
    let (sheet_name, is_ref_sheet) =
        get_current_sheet_name(&ctx, instance, &member_id, true).await?;

    // Can modify Sheet when not in reference sheet or in Host mode
    let can_modify_sheet = !is_ref_sheet || is_host_mode;

    if !can_modify_sheet {
        return Ok(MergeShareMappingActionResult::EditNotAllowed);
    }

    if ctx.is_proc_on_remote() {
        let vault = try_get_vault(&ctx)?;
        let share_id = args.share_id;

        // Get the share and sheet
        let (sheet, share) = if vault.share_file_path(&sheet_name, &share_id).exists() {
            let sheet = vault.sheet(&sheet_name).await?;
            let share = sheet.get_share(&share_id).await?;
            (sheet, share)
        } else {
            // Share does not exist
            write_and_return!(
                instance,
                MergeShareMappingActionResult::ShareIdNotFound(share_id.clone())
            );
        };

        // Perform the merge
        match sheet.merge_share(share, args.share_merge_mode).await {
            Ok(_) => write_and_return!(instance, MergeShareMappingActionResult::Success),
            Err(e) => match e.kind() {
                ErrorKind::AlreadyExists => {
                    write_and_return!(instance, MergeShareMappingActionResult::HasConflicts);
                }
                _ => {
                    write_and_return!(
                        instance,
                        MergeShareMappingActionResult::MergeFails(e.to_string())
                    );
                }
            },
        }
    }

    if ctx.is_proc_on_local() {
        let result = instance
            .lock()
            .await
            .read::<MergeShareMappingActionResult>()
            .await?;
        match result {
            MergeShareMappingActionResult::Success => {
                sign_vault_modified(true).await;
            }
            _ => {}
        }
        return Ok(result);
    }

    Ok(MergeShareMappingActionResult::Success)
}
