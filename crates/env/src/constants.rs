// -------------------------------------------------------------------------------------
//

// Project
pub const PATH_TEMP: &str = "./.temp/";

// Default Port
pub const PORT: u16 = 25331;

// Server
// Server - Vault (Main)
pub const SERVER_FILE_VAULT: &str = "./vault.toml"; // crates::env::vault::vault_config

// Server - Sheets
pub const SERVER_PATH_SHEETS: &str = "./sheets/";
pub const SERVER_FILE_SHEET: &str = "./sheets/{sheet-name}.yaml";

// Server - Members
pub const SERVER_PATH_MEMBERS: &str = "./members/";
pub const SERVER_PATH_MEMBER_PUB: &str = "./key/";
pub const SERVER_FILE_MEMBER_INFO: &str = "./members/{member_id}.toml"; // crates::env::member::manager
pub const SERVER_FILE_MEMBER_PUB: &str = "./key/{member_id}.pem"; // crates::utils::tcp_connection::instance

// Server - Virtual File Storage
pub const SERVER_PATH_VIRTUAL_FILE_TEMP: &str = "./.temp/{temp_name}";
pub const SERVER_PATH_VIRTUAL_FILE_ROOT: &str = "./storage/";
pub const SERVER_PATH_VIRTUAL_FILE_STORAGE: &str = "./storage/{vf_id}/";
pub const SERVER_FILE_VIRTUAL_FILE_VERSION_INSTANCE: &str = "./storage/{vf_id}/{vf_version}.rf";
pub const SERVER_FILE_VIRTUAL_FILE_META: &str = "./storage/{vf_id}/meta.toml";

pub const SERVER_FILE_README: &str = "./README.md";

// -------------------------------------------------------------------------------------

// Client
pub const CLIENT_PATH_WORKSPACE_ROOT: &str = "./.jv/";

// Client - Workspace (Main)
pub const CLIENT_FILE_WORKSPACE: &str = "./.jv/workspace.toml"; // crates::env::local::local_config

// Client - Other
pub const CLIENT_FILE_IGNOREFILES: &str = ".jgnore .gitignore"; // Support gitignore file.

// -------------------------------------------------------------------------------------

// User - Verify (System path)
pub const USER_FILE_KEY: &str = "./.jv_user/key";
pub const USER_FILE_KEY_PUB: &str = "./.jv_user/key.pub";
pub const USER_FILE_MEMBER: &str = "./.jv_user/self.toml";
