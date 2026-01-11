use action_system::action_pool::ActionPool;

use crate::remote_actions::{
    content_manage::track::register_track_file_action,
    edit_right_manage::change_virtual_file_edit_right::register_change_virtual_file_edit_right_action,
    mapping_manage::{
        edit_mapping::register_edit_mapping_action,
        merge_share_mapping::register_merge_share_mapping_action,
        share_mapping::register_share_mapping_action,
    },
    sheet_manage::{
        drop_sheet::register_drop_sheet_action, make_sheet::register_make_sheet_action,
    },
    workspace_manage::{
        set_upstream_vault::register_set_upstream_vault_action,
        update_to_latest_info::register_update_to_latest_info_action,
    },
};

pub fn server_action_pool() -> ActionPool {
    let mut pool = ActionPool::new();

    // Local Actions
    register_set_upstream_vault_action(&mut pool);
    register_update_to_latest_info_action(&mut pool);

    // Sheet Actions
    register_make_sheet_action(&mut pool);
    register_drop_sheet_action(&mut pool);
    register_edit_mapping_action(&mut pool);

    // Share / Merge Share Actions
    register_share_mapping_action(&mut pool);
    register_merge_share_mapping_action(&mut pool);

    // Track Action
    register_track_file_action(&mut pool);

    // User Actions
    register_change_virtual_file_edit_right_action(&mut pool);

    pool
}
