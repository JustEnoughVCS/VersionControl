#[macro_export]
macro_rules! c {
    ($name:ident = $value:expr) => {
        const $name: &str = $value;
    };
}

pub const TEMP_FILE_PREFIX: &str = ".tmp_";
pub const LOCK_FILE_PREFIX: &str = "~";

/// File and directory path constants for the server root
#[allow(unused)]
pub mod server {
    /// File constants
    #[constants_macros::constants("server_file")]
    pub mod files {
        c! { CONFIG = "server.toml" }

        // Storage location for keys and passwords
        c! { KEY = "auth/key/{member_name}.pem" }
        c! { PASSWORD = "auth/pswd/{member_name}.pswd" }

        // Only Key holders are allowed to initiate Join Request
        c! { JOIN_REQUEST_KEY = ".temp/join_requests/{member_name}.pem" }

        // User metadata is stored here
        // Typically includes:
        // - Nickname
        // - Email
        // - Other information
        //
        // Facilitates queries by other members
        c! { MEMBER_METADATA = "meta/{member_name}.toml" }
    }

    /// Directory path constants
    #[constants_macros::constants("server_dir")]
    pub mod dirs {
        c! { KEYS = "auth/key/" }
        c! { PASSWORDS = "auth/pswd/" }
        c! { JOIN_REQUEST_KEYS = ".temp/join_requests/" }
        c! { MEMBER_METADATAS = "meta/" }
        c! { VAULTS = "v/" }
    }
}

/// File and directory path constants for the vault root
#[allow(unused)]
pub mod vault {
    /// Others
    #[constants_macros::constants("vault_value")]
    pub mod values {
        c! { INDEX_FILE_SUFFIX = "bidx" }
        c! { INDEX_FILE_SUFFIX_DOT = ".bidx" }

        c! { SHEET_FILE_SUFFIX = "sheet" }
        c! { SHEET_FILE_SUFFIX_DOT = ".sheet" }
    }

    /// File path constants
    #[constants_macros::constants("vault_file")]
    pub mod files {
        // ### Configs ###
        c! { CONFIG = "vault.toml" }

        // ### Sheets ###

        // Reference sheet
        c! { REFSHEET = "ref/{ref_sheet_name}.sheet" }

        // Member sheet backup, only used to temporarily store a member's local workspace file structure, fully controlled by the corresponding member
        c! { MEMBER_SHEET_BACKUP = "_member/{member_name}/backups/{sheet_name}.sheet" }

        // ### Rules ###
        c! { IGNORE_RULE_SCRIPT = "rules/ignore/{script_name}.lua" }

        // ### Storages ###
        c! { CHANGE_FILE = "changes/CURRENT" }
        c! { CHANGE_FILE_BACKUP = "changes/backup.{index}" }

        // Whether an index is locked; if the file exists, the member name written inside is the current holder
        c! { INDEX_LOCK = "index/{index_slice_1}/{index_slice_2}/{index_name}/LOCK" }

        // Index version sequence
        c! { INDEX_VER = "index/{index_slice_1}/{index_slice_2}/{index_name}/ver" }

        // Index
        c! { INDEX = "index/{index_slice_1}/{index_slice_2}/{index_name}/{version}.index" }

        // Object, file chunk
        c! { OBJ = "obj/{obj_hash_slice_1}/{obj_hash_slice_2}/{obj_hash}" }
    }

    /// Directory path constants
    #[constants_macros::constants("vault_dir")]
    pub mod dirs {
        c! { REFSHEETS = "ref/" }
        c! { MEMBER_ROOT = "_member/" }
        c! { MEMBER = "_member/{member_name}/" }
        c! { MEMBER_SHEET_BACKUPS = "_member/{member_name}/backups/" }
        c! { IGNORE_RULES = "rules/ignore/" }
        c! { CHANGES = "changes/" }
    }
}

/// File and directory path constants for the workspace root
#[allow(unused)]
pub mod workspace {
    /// File path constants
    #[constants_macros::constants("workspace_file")]
    pub mod files {
        // ### Config ###
        c! { CONFIG = ".jv/workspace.toml" }

        // ### Sheets ###
        // Records the latest state of local physical files, used to calculate deviations
        c! { LOCAL_STATUS = ".jv/sheets/{sheet}.local" }

        // Personal sheet, represents the desired file structure, held only by the member, can be backed up to the vault
        c! { SHEET = ".jv/sheets/{sheet}.sheet" }

        // Current sheet name
        c! { CURRENT_SHEET = ".jv/sheets/CURRENT" }

        // Draft file, when switching to another sheet, fully records modified but untracked files
        c! { DRAFTED_FILE = ".jv/drafts/{account}_{sheet}/{mapping}" }

        // Working file
        c! { WORKING_FILE = "{mapping}" }
    }

    /// Directory path constants
    #[constants_macros::constants("workspace_dir")]
    pub mod dirs {
        c! { WORKSPACE = ".jv" }
        c! { VAULT_MIRROR = ".jv/UPSTREAM/" }
        c! { LOCAL_SHEETS = ".jv/sheets/" }
        c! { DRAFT_AREA = ".jv/drafts/{account}_{sheet}/" }
        c! { ID_MAPPING = ".jv/idmp/" }
        c! { WORKING_AREA = "" }
    }
}

/// File and directory path constants for the user root
#[allow(unused)]
pub mod user {
    /// Others
    #[constants_macros::constants("user_value")]
    pub mod values {
        c! { USER_CONFIG_NAME = "usr.toml" }
    }

    /// File path constants
    #[constants_macros::constants("user_file")]
    pub mod files {
        // Upstream public key, stored after initial login, used to verify the trustworthiness of that upstream
        c! { JVCS_UPSTREAM_PUB = ".jvcs/upstreams/{jvcs_upstream_addr}.pem" }

        // Account private key, stored only locally, used for login authentication
        c! { PRIVATE_KEY = ".jvcs/private/{account}.pem" }

        // Account public key, automatically generated from the private key and stored,
        // will be placed into the server's "join request list" upon initial login (if server.toml permits this action)
        // The server administrator can optionally approve this request
        c! { PUBLIC_KEY = ".jvcs/public/{account}.pem" }

        // Account configuration file
        c! { USER_CONFIG = ".jvcs/usr.toml" }
    }

    /// Directory path constants
    #[constants_macros::constants("user_dir")]
    pub mod dirs {
        c! { ROOT = ".jvcs/" }
        c! { UPSTREAM_PUBS = ".jvcs/upstreams/" }
        c! { PRIVATE_KEYS = ".jvcs/private/" }
        c! { PUBLIC_KEYS = ".jvcs/public/" }
    }
}
