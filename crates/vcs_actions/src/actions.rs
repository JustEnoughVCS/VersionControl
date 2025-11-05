use std::sync::Arc;

use action_system::action::ActionContext;
use tcp_connection::{error::TcpTargetError, instance::ConnectionInstance};
use tokio::sync::Mutex;
use vcs_data::{
    constants::SERVER_PATH_MEMBER_PUB,
    data::{local::LocalWorkspace, member::MemberId, user::UserDirectory, vault::Vault},
};

pub mod local_actions;
pub mod sheet_actions;
pub mod user_actions;
pub mod vault_actions;
pub mod virtual_file_actions;

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

/// The macro to write and return a result.
#[macro_export]
macro_rules! write_and_return {
    ($instance:expr, $result:expr) => {{
        $instance.lock().await.write($result).await?;
        return Ok($result);
    }};
}
