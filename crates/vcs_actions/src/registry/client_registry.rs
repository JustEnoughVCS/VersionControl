use action_system::{action::ActionContext, action_pool::ActionPool};
use tcp_connection::error::TcpTargetError;

use crate::{
    actions::local_actions::register_hello_world_action, connection::protocol::RemoteActionInvoke,
};

fn register_actions(pool: &mut ActionPool) {
    // Pool register here
    register_hello_world_action(pool);
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
    ctx: &ActionContext,
    _args: &(dyn std::any::Any + Send + Sync),
) -> Result<(), TcpTargetError> {
    // Is ctx remote
    let is_remote = ctx.is_remote();

    // Action name and arguments
    let action_name = ctx.action_name().to_string();
    let action_args_json = ctx.action_args_json().clone();

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
            action_name: action_name,
            action_args_json: action_args_json,
        };

        // Send
        let mut instance = instance.lock().await;
        instance.write_msgpack(&msg).await?;
    }

    // Return OK, wait for client to execute Action locally
    Ok(())
}
