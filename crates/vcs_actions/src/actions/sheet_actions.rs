use action_system::{action::ActionContext, macros::action_gen};
use serde::{Deserialize, Serialize};
use tcp_connection::error::TcpTargetError;
use vcs_data::data::sheet::SheetName;

use crate::actions::{auth_member, check_connection_instance, try_get_vault};

#[derive(Default, Serialize, Deserialize)]
pub enum MakeSheetActionResult {
    Success,

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
    let member_id = match auth_member(&ctx, instance).await {
        Ok(id) => id,
        Err(e) => return Ok(MakeSheetActionResult::AuthorizeFailed(e.to_string())),
    };

    if ctx.is_proc_on_remote() {
        let vault = try_get_vault(&ctx)?;

        // Check if the sheet already exists
        if vault.sheet(&sheet_name).await.is_ok() {
            instance
                .lock()
                .await
                .write(MakeSheetActionResult::SheetAlreadyExists)
                .await?;
            return Ok(MakeSheetActionResult::SheetAlreadyExists);
        } else {
            // Create the sheet
            match vault.create_sheet(&sheet_name, &member_id).await {
                Ok(_) => {
                    instance
                        .lock()
                        .await
                        .write(MakeSheetActionResult::Success)
                        .await?;
                    return Ok(MakeSheetActionResult::Success);
                }
                Err(e) => {
                    instance
                        .lock()
                        .await
                        .write(MakeSheetActionResult::SheetCreationFailed(e.to_string()))
                        .await?;
                    return Ok(MakeSheetActionResult::SheetCreationFailed(e.to_string()));
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
        return Ok(result);
    }

    Err(TcpTargetError::NoResult("No result.".to_string()))
}
