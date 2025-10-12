use std::net::SocketAddr;

use action_system::{action::ActionContext, action_gen};
use tcp_connection::error::TcpTargetError;

#[action_gen(local)]
pub async fn set_upstream_vault_action(
    ctx: ActionContext,
    upstream: SocketAddr,
) -> Result<(), TcpTargetError> {
    if ctx.is_remote() {
        return Err(TcpTargetError::NotLocal(
            "Action was not invoked on the local machine".to_string(),
        ));
    }
    Ok(())
}
