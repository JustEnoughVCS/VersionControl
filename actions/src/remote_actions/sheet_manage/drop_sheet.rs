use std::io::ErrorKind;

use action_system::{action::ActionContext, macros::action_gen};
use serde::{Deserialize, Serialize};
use tcp_connection::error::TcpTargetError;
use vcs_data::data::{local::vault_modified::sign_vault_modified, sheet::SheetName};

use crate::{
    remote_actions::{
        auth_member, check_connection_instance, try_get_local_workspace, try_get_vault,
    },
    write_and_return,
};

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
