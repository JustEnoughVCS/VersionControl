use std::collections::HashMap;

use action_system::{action::ActionContext, macros::action_gen};
use serde::{Deserialize, Serialize};
use tcp_connection::error::TcpTargetError;
use vcs_data::data::local::{
    modified_status::sign_vault_modified,
    workspace_analyzer::{FromRelativePathBuf, ToRelativePathBuf},
};

use crate::{
    remote_actions::{
        auth_member, check_connection_instance, get_current_sheet_name, try_get_vault,
    },
    write_and_return,
};

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
