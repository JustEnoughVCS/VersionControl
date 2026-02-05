use action_system::{action::ActionContext, macros::action_gen};
use serde::{Deserialize, Serialize};
use tcp_connection::error::TcpTargetError;
use vcs_data::{
    constants::VAULT_HOST_NAME,
    data::{local::workspace_analyzer::FromRelativePathBuf, sheet::SheetName},
};

use crate::{
    remote_actions::{
        auth_member, check_connection_instance, get_current_sheet_name, try_get_vault,
    },
    write_and_return,
};

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
