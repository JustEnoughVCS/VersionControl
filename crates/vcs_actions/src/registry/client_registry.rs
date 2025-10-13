use action_system::{action::ActionContext, action_pool::ActionPool};
use tcp_connection::error::TcpTargetError;

use crate::actions::local_actions::register_set_upstream_vault_action;

fn register_actions(pool: &mut ActionPool) {
    // Pool register here
    register_set_upstream_vault_action(pool);
}

pub fn client_action_pool() -> ActionPool {
    // Create pool
    let mut pool = ActionPool::new();

    // Register actions
    register_actions(&mut pool);

    // Add process events
    pool.set_on_proc_begin(|ctx| Box::pin(on_proc_begin(ctx)));

    // Return
    pool
}

async fn on_proc_begin(ctx: &ActionContext) -> Result<(), TcpTargetError> {
    // Get instance
    let Some(_instance) = ctx.instance() else {
        return Err(TcpTargetError::Unsupported(
            "Missing ConnectionInstance in current context, this ActionPool does not support this call"
                .to_string()));
    };

    // If it's remote, invoke action at server
    if ctx.is_remote() {
        // instance.write_text(text)
    }

    Ok(())
}
