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
pub const SERVER_SUFFIX_SHEET_FILE: &str = ".bcfg";
pub const SERVER_SUFFIX_SHEET_FILE_NO_DOT: &str = "bcfg";
pub const REF_SHEET_NAME: &str = "ref";
pub const SERVER_PATH_SHEETS: &str = "./sheets/";
pub const SERVER_PATH_SHARES: &str = "./sheets/shares/{sheet_name}/";
pub const SERVER_FILE_SHEET: &str = "./sheets/{sheet_name}.bcfg";
pub const SERVER_FILE_SHEET_SHARE: &str = "./sheets/shares/{sheet_name}/{share_id}.bcfg";

// Server - Members
pub const SERVER_PATH_MEMBERS: &str = "./members/";
pub const SERVER_PATH_MEMBER_PUB: &str = "./key/";
pub const SERVER_FILE_MEMBER_INFO: &str = "./members/{member_id}.bcfg";
pub const SERVER_FILE_MEMBER_PUB: &str = "./key/{member_id}.pem";

// Server - Virtual File Storage
pub const SERVER_PATH_VF_TEMP: &str = "./.temp/{temp_name}";
pub const SERVER_PATH_VF_ROOT: &str = "./storage/";
pub const SERVER_PATH_VF_STORAGE: &str = "./storage/{vf_index}/{vf_id}/";
pub const SERVER_FILE_VF_VERSION_INSTANCE: &str = "./storage/{vf_index}/{vf_id}/{vf_version}.rf";
pub const SERVER_FILE_VF_META: &str = "./storage/{vf_index}/{vf_id}/meta.bcfg";
pub const SERVER_NAME_VF_META: &str = "meta.bcfg";

// Server - Updates
pub const SERVER_FILE_UPDATES: &str = "./.updates.txt";

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
pub const CLIENT_FILE_LATEST_INFO: &str = "./.jv/latest/{account}.vault.bcfg";
pub const CLIENT_FILE_LATEST_DATA: &str = "./.jv/latest/{account}.file.bcfg";

// Client - Local
pub const CLIENT_SUFFIX_LOCAL_SHEET_FILE: &str = ".bcfg";
pub const CLIENT_SUFFIX_CACHED_SHEET_FILE: &str = ".bcfg";
pub const CLIENT_PATH_LOCAL_DRAFT: &str = "./.jv/drafts/{account}/{sheet_name}/";
pub const CLIENT_PATH_LOCAL_SHEET: &str = "./.jv/sheets/local/";
pub const CLIENT_FILE_LOCAL_SHEET: &str = "./.jv/sheets/local/{account}/{sheet_name}.bcfg";
pub const CLIENT_PATH_CACHED_SHEET: &str = "./.jv/sheets/cached/";
pub const CLIENT_FILE_CACHED_SHEET: &str = "./.jv/sheets/cached/{sheet_name}.bcfg";

pub const CLIENT_FILE_LOCAL_SHEET_NOSET: &str = "./.jv/.temp/wrong.json";
pub const CLIENT_FILE_MEMBER_HELD_NOSET: &str = "./.jv/.temp/wrong.json";
pub const CLIENT_FILE_LATEST_INFO_NOSET: &str = "./.jv/.temp/wrong.json";

// Client - Other
pub const CLIENT_FILE_IGNOREFILES: &str = "IGNORE_RULES.toml";
pub const CLIENT_FILE_TODOLIST: &str = "./SETUP.txt";
pub const CLIENT_FILE_GITIGNORE: &str = "./.jv/.gitignore";
pub const CLIENT_CONTENT_GITIGNORE: &str = "# Git support for JVCS Workspace

# Ignore cached datas
/sheets/cached/
/latest/

.vault_modified";
pub const CLIENT_FILE_VAULT_MODIFIED: &str = "./.jv/.vault_modified";
pub const CLIENT_FILE_TEMP_FILE: &str = "./.jv/.temp/download/{temp_name}";

// -------------------------------------------------------------------------------------

// User - Verify (Documents path)
pub const USER_FILE_ACCOUNTS: &str = "./accounts/";
pub const USER_FILE_KEY: &str = "./accounts/{self_id}_private.pem";
pub const USER_FILE_MEMBER: &str = "./accounts/{self_id}.toml";
