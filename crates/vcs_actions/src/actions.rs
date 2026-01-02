use std::sync::Arc;

use action_system::action::ActionContext;
use cfg_file::config::ConfigFile;
use tcp_connection::{error::TcpTargetError, instance::ConnectionInstance};
use tokio::sync::{Mutex, mpsc::Sender};
use vcs_data::{
    constants::{SERVER_PATH_MEMBER_PUB, VAULT_HOST_NAME},
    data::{
        local::{LocalWorkspace, config::LocalConfig, latest_info::LatestInfo},
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

/// Try to get the LocalWorkspace instance from the context.
pub fn try_get_local_output(ctx: &ActionContext) -> Result<Arc<Sender<String>>, TcpTargetError> {
    let Some(output) = ctx.get_arc::<Sender<String>>() else {
        return Err(TcpTargetError::NotFound(
            "Client sender not found".to_string(),
        ));
    };
    Ok(output)
}

/// Authenticate member based on context and return MemberId
pub async fn auth_member(
    ctx: &ActionContext,
    instance: &Arc<Mutex<ConnectionInstance>>,
) -> Result<(MemberId, bool), TcpTargetError> {
    // Window开服Linux连接 -> 此函数内产生 early eof
    // ~ WS # jv update
    // 身份认证失败：I/O error: early eof！

    // 分析相应流程：
    // 1. 服务端发起挑战，客户端接受
    // 2. 服务端发送结果，客户端接受
    // 3. 推测此时发生 early eof ---> 无 ack，导致客户端尝试拿到结果时，服务端已经结束
    // 这很有可能是 Windows 和 Linux 对于连接处理的方案差异导致的问题，需要进一步排查

    // Start Challenge (Remote)
    if ctx.is_proc_on_remote() {
        let mut mut_instance = instance.lock().await;
        let vault = try_get_vault(ctx)?;

        let using_host_mode = mut_instance.read_msgpack::<bool>().await?;

        let result = mut_instance
            .challenge(vault.vault_path().join(SERVER_PATH_MEMBER_PUB))
            .await;

        return match result {
            Ok((pass, member_id)) => {
                if !pass {
                    // Send false to inform the client that authentication failed
                    mut_instance.write(false).await?;
                    Err(TcpTargetError::Authentication(
                        "Authenticate failed.".to_string(),
                    ))
                } else {
                    if using_host_mode {
                        if vault.config().vault_host_list().contains(&member_id) {
                            // Using Host mode authentication, and is indeed an administrator
                            mut_instance.write(true).await?;
                            Ok((member_id, true))
                        } else {
                            // Using Host mode authentication, but not an administrator
                            mut_instance.write(false).await?;
                            Err(TcpTargetError::Authentication(
                                "Authenticate failed.".to_string(),
                            ))
                        }
                    } else {
                        // Not using Host mode authentication
                        mut_instance.write(true).await?;
                        Ok((member_id, false))
                    }
                }
            }
            Err(e) => Err(e),
        };
    }

    // Accept Challenge (Local)
    if ctx.is_proc_on_local() {
        let mut mut_instance = instance.lock().await;
        let local_workspace = try_get_local_workspace(ctx)?;
        let (is_host_mode, member_name) = {
            let cfg = local_workspace.config().lock_owned().await;
            (cfg.is_host_mode(), cfg.current_account())
        };
        let user_directory = try_get_user_directory(ctx)?;

        // Inform remote whether to authenticate in Host mode
        mut_instance.write_msgpack(is_host_mode).await?;

        // Member name & Private key
        let private_key = user_directory.account_private_key_path(&member_name);
        let _ = mut_instance
            .accept_challenge(private_key, &member_name)
            .await?;

        // Read result
        let challenge_result = mut_instance.read::<bool>().await?;
        if challenge_result {
            return Ok((member_name.clone(), is_host_mode));
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
/// to verify and retrieve the current sheet name and whether it's a reference sheet.
///
/// On local:
/// - Reads the current sheet from local configuration
/// - Sends the sheet name to remote for verification
/// - Returns the sheet name and whether it's a reference sheet if remote confirms it exists
///
/// On remote:
/// - Receives sheet name from local
/// - Verifies the sheet exists in the vault
/// - Checks if the sheet is a reference sheet
/// - If allow_ref is true, reference sheets are allowed to pass verification
/// - Sends confirmation and reference status back to local
///
/// Returns a tuple of (SheetName, bool) where the bool indicates if it's a reference sheet,
/// or an error if the sheet doesn't exist or doesn't meet the verification criteria.
pub async fn get_current_sheet_name(
    ctx: &ActionContext,
    instance: &Arc<Mutex<ConnectionInstance>>,
    member_id: &MemberId,
    allow_ref: bool,
) -> Result<(SheetName, bool), TcpTargetError> {
    let mut mut_instance = instance.lock().await;
    if ctx.is_proc_on_local() {
        let workspace = try_get_local_workspace(ctx)?;
        let config = LocalConfig::read().await?;
        let latest = LatestInfo::read_from(LatestInfo::latest_info_path(
            workspace.local_path(),
            member_id,
        ))
        .await?;
        if let Some(sheet_name) = config.sheet_in_use() {
            // Send sheet name
            mut_instance.write_msgpack(sheet_name).await?;

            // Read result
            if mut_instance.read_msgpack::<bool>().await? {
                // Check if sheet is a reference sheet
                let is_ref_sheet = latest.reference_sheets.contains(sheet_name);
                if allow_ref {
                    // Allow reference sheets, directly return the determination result
                    return Ok((sheet_name.clone(), is_ref_sheet));
                } else if is_ref_sheet {
                    // Not allowed but it's a reference sheet, return an error
                    return Err(TcpTargetError::ReferenceSheetNotAllowed(
                        "Reference sheet not allowed".to_string(),
                    ));
                } else {
                    // Not allowed but not a reference sheet, return normally
                    return Ok((sheet_name.clone(), false));
                }
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
        {
            let is_ref_sheet = holder == VAULT_HOST_NAME;
            if allow_ref {
                // Allow reference sheets, directly return the determination result
                if holder == member_id || holder == VAULT_HOST_NAME {
                    mut_instance.write_msgpack(true).await?;
                    return Ok((sheet.name().clone(), is_ref_sheet));
                }
            } else if is_ref_sheet {
                // Not allowed but it's a reference sheet, return an error
                mut_instance.write_msgpack(true).await?;
                return Err(TcpTargetError::ReferenceSheetNotAllowed(
                    "Reference sheet not allowed".to_string(),
                ));
            } else {
                // Not allowed but not a reference sheet, return normally
                if holder == member_id {
                    mut_instance.write_msgpack(true).await?;
                    return Ok((sheet_name.clone(), false));
                }
            }
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

/// The macro to send formatted string to output channel.
/// Usage: local_println!(output, "format string", arg1, arg2, ...)
#[macro_export]
macro_rules! local_println {
    ($output:expr, $($arg:tt)*) => {{
        let formatted = format!($($arg)*);
        let _ = $output.send(formatted).await;
    }};
}
