use std::net::SocketAddr;

use action_system::{action::ActionContext, macros::action_gen};
use log::info;
use tcp_connection::error::TcpTargetError;

#[action_gen]
pub async fn hello_world_action(ctx: ActionContext, _n: ()) -> Result<(), TcpTargetError> {
    // Ensure the instance is available
    let Some(instance) = ctx.instance() else {
        return Err(TcpTargetError::NotFound(
            "Connection Instance Lost.".to_string(),
        ));
    };

    if ctx.is_local() {
        // Invoke on local
        // Send the message to the server
        let _ = instance.lock().await.write_text("Hello World!").await;
    } else if ctx.is_remote() {
        // Read the message from the client
        let read = instance.lock().await.read_text().await?;
        info!("{}", read)
    }

    Ok(())
}

#[action_gen]
pub async fn set_upstream_vault_action(
    ctx: ActionContext,
    _upstream: SocketAddr,
) -> Result<(), TcpTargetError> {
    // Ensure the instance is available
    let Some(instance) = ctx.instance() else {
        return Err(TcpTargetError::NotFound(
            "Connection Instance Lost.".to_string(),
        ));
    };

    if ctx.is_local() {
        // Invoke on local
        // Send the message to the server
        let _ = instance.lock().await.write_text("Hello World!").await;
    } else if ctx.is_remote() {
        // Read the message from the client
        let read = instance.lock().await.read_text().await?;
        info!("{}", read)
    }

    Ok(())
}
