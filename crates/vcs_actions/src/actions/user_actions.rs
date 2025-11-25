// Hold
// Throw
// Import
// Export

use std::path::PathBuf;

use action_system::{action::ActionContext, macros::action_gen};
use serde::{Deserialize, Serialize};
use tcp_connection::error::TcpTargetError;
use vcs_data::data::local::vault_modified::sign_vault_modified;

use crate::actions::{
    auth_member, check_connection_instance, get_current_sheet_name, try_get_vault,
};

#[derive(Serialize, Deserialize)]
pub enum ChangeVirtualFileEditRightResult {
    // Success
    Success {
        success_hold: Vec<PathBuf>,
        success_throw: Vec<PathBuf>,
    },

    // Fail
    AuthorizeFailed(String),
    DoNothing,
}

#[derive(Serialize, Deserialize, PartialEq)]
pub enum EditRightChangeBehaviour {
    Hold,
    Throw,
}

/// The server part only checks:
/// 1. Whether the file exists
/// 2. Whether the file has no holder
/// If both conditions are met, send success information to the local client
///
/// All version checks are handled locally
#[action_gen]
pub async fn change_virtual_file_edit_right_action(
    ctx: ActionContext,
    relative_paths: Vec<(PathBuf, EditRightChangeBehaviour)>,
) -> Result<ChangeVirtualFileEditRightResult, TcpTargetError> {
    let instance = check_connection_instance(&ctx)?;

    // Auth Member
    let member_id = match auth_member(&ctx, instance).await {
        Ok(id) => id,
        Err(e) => {
            return Ok(ChangeVirtualFileEditRightResult::AuthorizeFailed(
                e.to_string(),
            ));
        }
    };

    // Check sheet
    let sheet_name = get_current_sheet_name(&ctx, instance, &member_id).await?;

    if ctx.is_proc_on_remote() {
        let mut mut_instance = instance.lock().await;
        let mut success_hold: Vec<PathBuf> = Vec::new();
        let mut success_throw: Vec<PathBuf> = Vec::new();
        let vault = try_get_vault(&ctx)?;
        for (path, behaviour) in relative_paths {
            let Ok(sheet) = vault.sheet(&sheet_name).await else {
                continue;
            };
            let Some(mapping) = sheet.mapping().get(&path) else {
                continue;
            };
            let Ok(has_edit_right) = vault
                .has_virtual_file_edit_right(&member_id, &mapping.id)
                .await
            else {
                continue;
            };

            // Throw file
            if has_edit_right && behaviour == EditRightChangeBehaviour::Throw {
                match vault
                    .grant_virtual_file_edit_right(&member_id, &mapping.id)
                    .await
                {
                    Ok(_) => {
                        success_throw.push(path.clone());
                    }
                    Err(_) => continue,
                }
            } else
            // Hold file
            if !has_edit_right && behaviour == EditRightChangeBehaviour::Hold {
                match vault.revoke_virtual_file_edit_right(&mapping.id).await {
                    Ok(_) => {
                        success_hold.push(path.clone());
                    }
                    Err(_) => continue,
                }
            }
        }

        // Write success list
        mut_instance
            .write_large_msgpack::<(Vec<PathBuf>, Vec<PathBuf>)>(
                (success_hold.clone(), success_throw.clone()),
                4096u16,
            )
            .await?;
        return Ok(ChangeVirtualFileEditRightResult::Success {
            success_hold,
            success_throw,
        });
    }

    if ctx.is_proc_on_local() {
        let mut mut_instance = instance.lock().await;
        let (success_hold, success_throw) = mut_instance
            .read_large_msgpack::<(Vec<PathBuf>, Vec<PathBuf>)>(4096u16)
            .await?;

        // If there are any successful items, mark as modified
        if success_hold.len() + success_throw.len() > 0 {
            sign_vault_modified(true).await;
        }
        return Ok(ChangeVirtualFileEditRightResult::Success {
            success_hold,
            success_throw,
        });
    }

    Ok(ChangeVirtualFileEditRightResult::DoNothing)
}
