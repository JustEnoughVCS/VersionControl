use std::sync::Arc;

use action_system::action::ActionContext;
use cfg_file::config::ConfigFile;
use tcp_connection::{error::TcpTargetError, instance::ConnectionInstance};
use tokio::sync::Mutex;
use vcs_data::{
    constants::SERVER_PATH_MEMBER_PUB,
    data::{
        local::{LocalWorkspace, config::LocalConfig},
        member::MemberId,
        sheet::SheetName,
        user::UserDirectory,
        vault::Vault,
    },
};

pub mod local_actions;
pub mod sheet_actions;
pub mod track_action;
pub mod user_actions;
pub mod vault_actions;

/// Check if the connection instance is valid in the given context.
/// This function is used to verify the connection instance in actions that require remote calls.
pub fn check_connection_instance(
    ctx: &ActionContext,
) -> Result<&Arc<Mutex<ConnectionInstance>>, TcpTargetError> {
    let Some(instance) = ctx.instance() else {
        return Err(TcpTargetError::NotFound(
            "Connection instance lost.".to_string(),
        ));
    };
    Ok(instance)
}

/// Try to get the Vault instance from the context.
pub fn try_get_vault(ctx: &ActionContext) -> Result<Arc<Vault>, TcpTargetError> {
    let Some(vault) = ctx.get_arc::<Vault>() else {
        return Err(TcpTargetError::NotFound(
            "Vault instance not found".to_string(),
        ));
    };
    Ok(vault)
}

/// Try to get the LocalWorkspace instance from the context.
pub fn try_get_local_workspace(ctx: &ActionContext) -> Result<Arc<LocalWorkspace>, TcpTargetError> {
    let Some(local_workspace) = ctx.get_arc::<LocalWorkspace>() else {
        return Err(TcpTargetError::NotFound(
            "LocalWorkspace instance not found".to_string(),
        ));
    };
    Ok(local_workspace)
}

/// Try to get the UserDirectory instance from the context.
pub fn try_get_user_directory(ctx: &ActionContext) -> Result<Arc<UserDirectory>, TcpTargetError> {
    let Some(user_directory) = ctx.get_arc::<UserDirectory>() else {
        return Err(TcpTargetError::NotFound(
            "UserDirectory instance not found".to_string(),
        ));
    };
    Ok(user_directory)
}

/// Authenticate member based on context and return MemberId
pub async fn auth_member(
    ctx: &ActionContext,
    instance: &Arc<Mutex<ConnectionInstance>>,
) -> Result<MemberId, TcpTargetError> {
    // Start Challenge (Remote)
    if ctx.is_proc_on_remote() {
        let vault = try_get_vault(ctx)?;
        let result = instance
            .lock()
            .await
            .challenge(vault.vault_path().join(SERVER_PATH_MEMBER_PUB))
            .await;

        return match result {
            Ok((pass, member_id)) => {
                if !pass {
                    // Send false to inform the client that authentication failed
                    instance.lock().await.write(false).await?;
                    Err(TcpTargetError::Authentication(
                        "Authenticate failed.".to_string(),
                    ))
                } else {
                    // Send true to inform the client that authentication was successful
                    instance.lock().await.write(true).await?;
                    Ok(member_id)
                }
            }
            Err(e) => Err(e),
        };
    }

    // Accept Challenge (Local)
    if ctx.is_proc_on_local() {
        let local_workspace = try_get_local_workspace(ctx)?;
        let user_directory = try_get_user_directory(ctx)?;

        // Member name & Private key
        let member_name = local_workspace.config().lock().await.current_account();
        let private_key = user_directory.account_private_key_path(&member_name);
        let _ = instance
            .lock()
            .await
            .accept_challenge(private_key, &member_name)
            .await?;

        // Read result
        let challenge_result = instance.lock().await.read::<bool>().await?;
        if challenge_result {
            return Ok(member_name.clone());
        } else {
            return Err(TcpTargetError::Authentication(
                "Authenticate failed.".to_string(),
            ));
        }
    }

    Err(TcpTargetError::NoResult("Auth failed.".to_string()))
}

/// Get the current sheet name based on the context (local or remote).
/// This function handles the communication between local and remote instances
/// to verify and retrieve the current sheet name.
///
/// On local:
/// - Reads the current sheet from local configuration
/// - Sends the sheet name to remote for verification
/// - Returns the sheet name if remote confirms it exists
///
/// On remote:
/// - Receives sheet name from local
/// - Verifies the sheet exists in the vault
/// - Sends confirmation back to local
///
/// Returns the verified sheet name or an error if the sheet doesn't exist
pub async fn get_current_sheet_name(
    ctx: &ActionContext,
    instance: &Arc<Mutex<ConnectionInstance>>,
    member_id: &MemberId,
) -> Result<SheetName, TcpTargetError> {
    let mut mut_instance = instance.lock().await;
    if ctx.is_proc_on_local() {
        let config = LocalConfig::read().await?;
        if let Some(sheet_name) = config.sheet_in_use() {
            // Send sheet name
            mut_instance.write_msgpack(sheet_name).await?;

            // Read result
            if mut_instance.read_msgpack::<bool>().await? {
                return Ok(sheet_name.clone());
            } else {
                return Err(TcpTargetError::NotFound("Sheet not found".to_string()));
            }
        }
        // Send empty sheet_name
        mut_instance.write_msgpack("".to_string()).await?;

        // Read result, since we know it's impossible to pass here, we just consume this result
        let _ = mut_instance.read_msgpack::<bool>().await?;

        return Err(TcpTargetError::NotFound("Sheet not found".to_string()));
    }
    if ctx.is_proc_on_remote() {
        let vault = try_get_vault(ctx)?;

        // Read sheet name
        let sheet_name: SheetName = mut_instance.read_msgpack().await?;

        // Check if sheet exists
        if let Ok(sheet) = vault.sheet(&sheet_name).await
            && let Some(holder) = sheet.holder()
            && holder == member_id
        {
            // Tell local the check is passed
            mut_instance.write_msgpack(true).await?;
            return Ok(sheet_name.clone());
        }
        // Tell local the check is not passed
        mut_instance.write_msgpack(false).await?;
        return Err(TcpTargetError::NotFound("Sheet not found".to_string()));
    }
    Err(TcpTargetError::NoResult("NoResult".to_string()))
}

/// The macro to write and return a result.
#[macro_export]
macro_rules! write_and_return {
    ($instance:expr, $result:expr) => {{
        $instance.lock().await.write($result).await?;
        return Ok($result);
    }};
}
