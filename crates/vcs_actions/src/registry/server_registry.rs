use action_system::action_pool::ActionPool;

use crate::actions::{
    local_actions::{register_set_upstream_vault_action, register_update_to_latest_info_action},
    sheet_actions::{register_drop_sheet_action, register_make_sheet_action},
};

pub fn server_action_pool() -> ActionPool {
    let mut pool = ActionPool::new();

    // Local Actions
    register_set_upstream_vault_action(&mut pool);
    register_update_to_latest_info_action(&mut pool);

    // Sheet Actions
    register_make_sheet_action(&mut pool);
    register_drop_sheet_action(&mut pool);

    pool
}
