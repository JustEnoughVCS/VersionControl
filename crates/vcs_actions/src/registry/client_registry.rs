use std::sync::Arc;

use action_system::{action::ActionContext, action_pool::ActionPool};
use cfg_file::config::ConfigFile;
use tcp_connection::error::TcpTargetError;
use vcs_data::data::{
    local::{LocalWorkspace, config::LocalConfig},
    user::UserDirectory,
};

use crate::{
    actions::{
        local_actions::{
            register_set_upstream_vault_action, register_update_to_latest_info_action,
        },
        sheet_actions::{register_drop_sheet_action, register_make_sheet_action},
        track_action::register_track_file_action,
        user_actions::register_change_virtual_file_edit_right_action,
    },
    connection::protocol::RemoteActionInvoke,
};

fn register_actions(pool: &mut ActionPool) {
    // Pool register here

    // Local Actions
    register_set_upstream_vault_action(pool);
    register_update_to_latest_info_action(pool);

    // Sheet Actions
    register_make_sheet_action(pool);
    register_drop_sheet_action(pool);

    // Track Action
    register_track_file_action(pool);

    // User Actions
    register_change_virtual_file_edit_right_action(pool);
}

pub fn client_action_pool() -> ActionPool {
    // Create pool
    let mut pool = ActionPool::new();

    // Register actions
    register_actions(&mut pool);

    // Add process events
    pool.set_on_proc_begin(|ctx, args| Box::pin(on_proc_begin(ctx, args)));

    // Return
    pool
}

async fn on_proc_begin(
    ctx: &mut ActionContext,
    _args: &(dyn std::any::Any + Send + Sync),
) -> Result<(), TcpTargetError> {
    // Is ctx remote
    let is_remote = ctx.is_remote_action();

    // Action name and arguments
    let action_name = ctx.action_name().to_string();
    let action_args_json = ctx.action_args_json().clone();

    // Insert LocalWorkspace Arc
    let Ok(local_config) = LocalConfig::read().await else {
        return Err(TcpTargetError::NotFound(
            "The current directory does not have a local workspace".to_string(),
        ));
    };
    let local_workspace = match LocalWorkspace::init_current_dir(local_config) {
        Some(workspace) => workspace,
        None => {
            return Err(TcpTargetError::NotFound(
                "Failed to initialize local workspace.".to_string(),
            ));
        }
    };
    let local_workspace_arc = Arc::new(local_workspace);
    ctx.insert_arc_data(local_workspace_arc);

    // Insert UserDirectory Arc
    let Some(user_directory) = UserDirectory::current_cfg_dir() else {
        return Err(TcpTargetError::NotFound(
            "The user directory does not exist.".to_string(),
        ));
    };

    let user_directory_arc = Arc::new(user_directory);
    ctx.insert_arc_data(user_directory_arc);

    // Get instance
    let Some(instance) = ctx.instance() else {
        return Err(TcpTargetError::Unsupported(
            "Missing ConnectionInstance in current context, this ActionPool does not support this call"
                .to_string()));
    };

    // If it's remote, invoke action at server
    if is_remote {
        // Build protocol message
        let msg = RemoteActionInvoke {
            action_name,
            action_args_json,
        };

        // Send
        let mut instance = instance.lock().await;
        instance.write_msgpack(&msg).await?;
    }

    // Return OK, wait for client to execute Action locally
    Ok(())
}
