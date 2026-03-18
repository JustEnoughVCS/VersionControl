use sheet_system::sheet::Sheet;

use crate::member::Member;

pub type SheetName = String;

pub enum VaultOperations {
    /// Claim ownership of an index
    HoldIndex(u32),

    /// Release ownership of an index
    ThrowIndex(u32),

    /// Backup a Sheet to personal space
    BackupSheet(Sheet),

    /// Download a Sheet from personal space
    DownloadSheet(SheetName),

    /// Download a RefSheet from public space
    DownloadRefSheet(SheetName),
}

pub enum VaultHostOperations {
    /// Forcefully grant ownership of an index to a member
    HoldIndexForce(Member, u32),

    /// Forcefully discard ownership of some indices
    ThrowIndexForce(Vec<u32>),

    /// Write a RefSheet to upstream
    WriteRefSheet(Sheet),

    /// Erase a Ref
    EraseRefSheet(SheetName),

    /// Erase some indices
    DangerousEraseIndex(Vec<u32>),

    /// Erase some versions of an index
    DangerousEraseVersion(u32, Vec<u16>),
}
