use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct RemoteActionInvoke {
    pub action_name: String,
    pub action_args_json: String,
}
