use action_system::action_pool::ActionPool;

use crate::actions::local_actions::SetUpstreamVaultAction;

pub fn server_action_pool() -> ActionPool {
    let mut pool = ActionPool::new();
    SetUpstreamVaultAction::register_to_pool(&mut pool);
    pool
}
