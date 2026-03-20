use std::io::ErrorKind;

use action_system::{action::ActionContext, macros::action_gen};
use serde::{Deserialize, Serialize};
use tcp_connection::error::TcpTargetError;
use vcs_data::data::{
    local::modified_status::sign_vault_modified,
    vault::mapping_share::{ShareMergeMode, SheetShareId},
};

use crate::{
    remote_actions::{
        auth_member, check_connection_instance, get_current_sheet_name, try_get_vault,
    },
    write_and_return,
};

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
        if let MergeShareMappingActionResult::Success = result {
            sign_vault_modified(true).await;
        }
        return Ok(result);
    }

    Ok(MergeShareMappingActionResult::Success)
}
