// -------------------------------------------------------------------------------------
//

// Project
pub const PATH_TEMP: &str = "./.temp/";

// Default Port
pub const PORT: u16 = 25331;

// Vault Host Name
pub const VAULT_HOST_NAME: &str = "host";

// Server
// Server - Vault (Main)
pub const SERVER_FILE_VAULT: &str = "./vault.toml";

// Server - Sheets
pub const REF_SHEET_NAME: &str = "ref";
pub const SERVER_PATH_SHEETS: &str = "./sheets/";
pub const SERVER_FILE_SHEET: &str = "./sheets/{sheet_name}.yaml";

// Server - Members
pub const SERVER_PATH_MEMBERS: &str = "./members/";
pub const SERVER_PATH_MEMBER_PUB: &str = "./key/";
pub const SERVER_FILE_MEMBER_INFO: &str = "./members/{member_id}.toml";
pub const SERVER_FILE_MEMBER_PUB: &str = "./key/{member_id}.pem";

// Server - Virtual File Storage
pub const SERVER_PATH_VF_TEMP: &str = "./.temp/{temp_name}";
pub const SERVER_PATH_VF_ROOT: &str = "./storage/";
pub const SERVER_PATH_VF_STORAGE: &str = "./storage/{vf_index}/{vf_id}/";
pub const SERVER_FILE_VF_VERSION_INSTANCE: &str = "./storage/{vf_index}/{vf_id}/{vf_version}.rf";
pub const SERVER_FILE_VF_META: &str = "./storage/{vf_index}/{vf_id}/meta.yaml";

// Server - Service
pub const SERVER_FILE_LOCKFILE: &str = "./.lock";

// Server - Documents
pub const SERVER_FILE_README: &str = "./README.md";

// -------------------------------------------------------------------------------------

// Client
pub const CLIENT_PATH_WORKSPACE_ROOT: &str = "./.jv/";
pub const CLIENT_FOLDER_WORKSPACE_ROOT_NAME: &str = ".jv";

// Client - Workspace (Main)
pub const CLIENT_FILE_WORKSPACE: &str = "./.jv/workspace.toml";

// Client - Latest Information
pub const CLIENT_FILE_LATEST_INFO: &str = "./.jv/latest.json";

// Client - Sheets
pub const CLIENT_FILE_SHEET_COPY: &str = "./.jv/sheets/{sheet_name}.copy.json";

// Client - Local Draft
pub const CLIENT_PATH_LOCAL_DRAFT: &str = "./.jv/drafts/{account}_{sheet_name}/";

// Client - Other
pub const CLIENT_FILE_IGNOREFILES: &str = "IGNORE_RULES.toml";
pub const CLIENT_FILE_README: &str = "./README.md";

// -------------------------------------------------------------------------------------

// User - Verify (Documents path)
pub const USER_FILE_ACCOUNTS: &str = "./accounts/";
pub const USER_FILE_KEY: &str = "./accounts/{self_id}_private.pem";
pub const USER_FILE_MEMBER: &str = "./accounts/{self_id}.toml";
