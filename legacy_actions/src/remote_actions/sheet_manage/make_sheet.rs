use action_system::{action::ActionContext, macros::action_gen};
use serde::{Deserialize, Serialize};
use tcp_connection::error::TcpTargetError;
use vcs_data::{
    constants::VAULT_HOST_NAME,
    data::{local::modified_status::sign_vault_modified, sheet::SheetName},
};

use crate::{
    remote_actions::{auth_member, check_connection_instance, try_get_vault},
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
