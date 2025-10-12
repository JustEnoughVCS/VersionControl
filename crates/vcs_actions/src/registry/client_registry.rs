use action_system::{action::ActionContext, action_pool::ActionPool};
use tcp_connection::error::TcpTargetError;

use crate::actions::local_actions::SetUpstreamVaultAction;

fn register_actions(pool: &mut ActionPool) {
    // Pool register here
    SetUpstreamVaultAction::register_to_pool(pool);
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
