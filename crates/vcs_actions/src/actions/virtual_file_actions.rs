use std::path::PathBuf;

use action_system::{action::ActionContext, macros::action_gen};
use serde::{Deserialize, Serialize};
use tcp_connection::error::TcpTargetError;

use crate::actions::{auth_member, check_connection_instance};

#[derive(Serialize, Deserialize)]
pub enum TrackFileActionResult {
    Success,

    // Fail
    AuthorizeFailed(String),
}

#[action_gen]
pub async fn track_file_action(
    ctx: ActionContext,
    relative_pathes: Vec<PathBuf>,
) -> Result<TrackFileActionResult, TcpTargetError> {
    let instance = check_connection_instance(&ctx)?;

    // Auth Member
    if let Err(e) = auth_member(&ctx, instance).await {
        return Ok(TrackFileActionResult::AuthorizeFailed(e.to_string()));
    };

    if ctx.is_proc_on_local() {}

    Err(TcpTargetError::NoResult("No result.".to_string()))
}
