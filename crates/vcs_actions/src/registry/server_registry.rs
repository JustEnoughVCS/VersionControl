use action_system::action_pool::ActionPool;

use crate::actions::local_actions::register_hello_world_action;

pub fn server_action_pool() -> ActionPool {
    let mut pool = ActionPool::new();
    register_hello_world_action(&mut pool);
    pool
}
