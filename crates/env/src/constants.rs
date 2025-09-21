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
pub const SERVER_PATH_MEMBER: &str = "./members/";
pub const SERVER_FILE_MEMBER_INFO: &str = "./members/{member_uuid}.toml"; // crates::env::member::manager
pub const SERVER_FILE_MEMBER_PUB: &str = "./key/{member_uuid}.pub";

// Server - Storage
pub const SERVER_PATH_VISUAL_FILE: &str = "./storage/";
pub const SERVER_FILE_STORGAE_CONFIG: &str = "./storage.yaml";

// -------------------------------------------------------------------------------------

// Client
pub const CLIENT_PATH_WORKSPACE_ROOT: &str = "./.jvc/";

// Client - Workspace (Main)
pub const CLIENT_FILE_WORKSPACE: &str = "./.jvc/workspace.toml"; // crates::env::local::local_config

// Client - Other
pub const CLIENT_FILE_IGNOREFILES: &str = ".jgnore .gitignore"; // Support gitignore file.

// User - Verify (System path)
pub const USER_FILE_KEY: &str = "./.jvc_user/key";
pub const USER_FILE_KEY_PUB: &str = "./.jvc_user/key.pub";
pub const USER_FILE_MEMBER: &str = "./.jvc_user/self.toml";
