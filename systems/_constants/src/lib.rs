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
        c! { CONFIG = "./config.toml" }
        c! { JOIN_REQUEST_KEY = "./.temp/join_requests/{member_name}.pem" }
        c! { KEY = "./key/{member_name}.pem" }
        c! { MEMBER_METADATA = "./meta/{member_name}.toml" }
    }

    /// Directory path constants
    #[constants_macros::constants("server_dir")]
    pub mod dirs {
        c! { KEYS = "./key/" }
        c! { JOIN_REQUEST_KEYS = "./.temp/join_requests/" }
        c! { MEMBER_METADATAS = "./meta/" }
        c! { VAULTS = "./v/" }
    }
}

/// File and directory path constants for the vault root
#[allow(unused)]
pub mod vault {
    /// File path constants
    #[constants_macros::constants("vault_file")]
    pub mod files {
        // ### Configs ###
        c! { CONFIG = "./vault.toml" }

        // ### Sheets ###

        // Reference sheet
        c! { REFSHEET = "./ref/{ref_sheet_name}.sheet" }

        // Member sheet backup, only used to temporarily store a member's local workspace file structure, fully controlled by the corresponding member
        c! { MEMBER_SHEET_BACKUP = "./_member/{member_name}/backups/{sheet_name}.sheet" }

        // Share sent to a reference sheet, selectively merged into that reference sheet by a Host
        c! { SHARE_TO_REF = "./req/{ref_sheet_name}/{share_id}.sheet" }

        // Share sent to a specific member, fully controlled by that member, who can optionally merge it into any of their own sheets
        c! { SHARE_TO_MEMBER = "./_member/{member_name}/shares/{share_id}.sheet" }

        // ### Storages ###
        c! { CHANGE_FILE = "./changes/CURRENT" }
        c! { CHANGE_FILE_BACKUP = "./changes/backup.{index}" }

        // Whether an index is locked; if the file exists, the member name written inside is the current holder
        c! { INDEX_LOCK = "./index/{index_slice_1}/{index_slice_2}/{index_name}/LOCK" }

        // Index version sequence
        c! { INDEX_VER = "./index/{index_slice_1}/{index_slice_2}/{index_name}/ver" }

        // Index
        c! { INDEX = "./index/{index_slice_1}/{index_slice_2}/{index_name}/{version}.index" }

        // Object, file chunk
        c! { OBJ = "./obj/{obj_hash_slice_1}/{obj_hash_slice_2}/{obj_hash}" }
    }

    /// Directory path constants
    #[constants_macros::constants("vault_dir")]
    pub mod dirs {
        c! { REFSHEETS = "./ref/" }
        c! { SHARES_TO_REF = "./req/{ref_sheet_name}/" }
        c! { MEMBER = "./_member/{member_name}/" }
        c! { MEMBER_SHEET_BACKUPS = "./_member/{member_name}/backups/" }
        c! { MEMBER_SHARES = "./_member/{member_name}/shares/" }
        c! { CHANGES = "./changes/" }
    }
}

/// File and directory path constants for the workspace root
#[allow(unused)]
pub mod workspace {
    /// File path constants
    #[constants_macros::constants("workspace_file")]
    pub mod files {
        // ### Config ###
        c! { CONFIG = "./.jv/workspace.toml" }

        // ### Sheets ###
        // Records the latest state of local physical files, used to calculate deviations
        c! { LOCAL_STATUS = "./.jv/sheets/{account}/{sheet}.local" }

        // Personal sheet, represents the desired file structure, held only by the member, can be backed up to the vault
        c! { SHEET = "./.jv/sheets/{account}/{sheet}.sheet" }

        // Draft file, when switching to another sheet, fully records modified but untracked files
        c! { DRAFTED_FILE = "./.jv/drafts/{account}_{sheet}/{mapping}" }

        // Working file
        c! { WORKING_FILE = "./{mapping}" }
    }

    /// Directory path constants
    #[constants_macros::constants("workspace_dir")]
    pub mod dirs {
        c! { WORKSPACE = "./.jv" }
        c! { VAULT_MIRROR = "./.jv/UPSTREAM/" }
        c! { LOCAL_SHEETS = "./.jv/sheets/{account}/" }
        c! { DRAFT_AREA = "./.jv/drafts/{account}_{sheet}/" }
        c! { WORKING_AREA = "./" }
    }
}

/// File and directory path constants for the user root
#[allow(unused)]
pub mod user {
    /// File path constants
    #[constants_macros::constants("user_file")]
    pub mod files {
        // Upstream public key, stored after initial login, used to verify the trustworthiness of that upstream
        c! { UPSTREAM_PUB = "./upstreams/{upstream_addr}.pem" }

        // Account private key, stored only locally, used for login authentication
        c! { PRIVATE_KEY = "./private/{account}.pem" }

        // Account public key, automatically generated from the private key and stored,
        // will be placed into the server's "join request list" upon initial login (if server.toml permits this action)
        // The server administrator can optionally approve this request
        c! { PUBLIC_KEY = "./public/{account}.pem" }
    }

    /// Directory path constants
    #[constants_macros::constants("user_dir")]
    pub mod dirs {
        c! { UPSTREAM_PUBS = "./upstreams/" }
        c! { PRIVATE_KEYS = "./private/" }
        c! { PUBLIC_KEYS = "./public/" }
    }
}
