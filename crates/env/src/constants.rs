// Project
pub const PATH_TEMP: &str = "./.temp/";

// Default Port
pub const PORT: u16 = 25331;

//  Server
// Server - Vault (Main)
pub const SERVER_FILE_VAULT: &str = "./vault.toml"; // crates::env::vault::vault_config

// Server - Sheets
pub const SERVER_PATH_SHEETS: &str = "./sheets/";
pub const SERVER_FILE_SHEET: &str = "./sheets/{sheet-name}.yaml";

// Server - Members
pub const SERVER_PATH_MEMBER: &str = "./members/{member_id}/";
pub const SERVER_FILE_MEMBER_INFO: &str = "./members/{member_id}/info.toml";
pub const SERVER_FILE_MEMBER_PUB: &str = "./members/{member_id}/key.pub";
pub const SERVER_FILE_MEMBER_META: &str = "./members/{member_id}/meta.toml";

// Server - Storage
pub const SERVER_PATH_VISUAL_FILE: &str = "./storage/";
pub const SERVER_FILE_STORGAE_CONFIG: &str = "./storage.yaml";

//  Client
pub const CLIENT_PATH_WORKSPACE_ROOT: &str = "./.jvc/";

// Client - Verify
pub const CLIENT_PATH_VERIFIER_KEYS: &str = "./.jvc/verify/key/";

// Client - Workspace (Main)
pub const CLIENT_FILE_WORKSPACE: &str = "./.jvc/workspace.toml"; // crates::env::local::local_config

// Client - Member
pub const CLIENT_FILE_MEMBER: &str = "./.jvc/verify/member.toml";
pub const CLIENT_FILE_MEMBER_META: &str = "./.jvc/verify/meta.toml";

// Client - Other
pub const CLIENT_FILE_IGNOREFILES: &str = ".jgnore .gitignore"; // Support gitignore file.
