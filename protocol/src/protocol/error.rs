use crate::member::Member;

#[derive(Debug, thiserror::Error)]
pub enum ProtocolAuthorizeFailed {
    #[error("Member not found")]
    MemberNotFound,

    #[error("No permission")]
    NoPermission,
}

#[derive(Debug, thiserror::Error)]
pub enum FetchLatestInfoFailed {
    #[error("Connection failed")]
    Connection,

    #[error("Permission denied")]
    PermissionDenied,
}

#[derive(Debug, thiserror::Error)]
pub enum VaultOperationFailed {
    /// Index is already held
    /// Cannot advance version, claim, or relinquish ownership
    #[error("Index already held by `{0}`")]
    IndexAlreadyHeldBy(Member),

    /// Sheet depends on local namespace
    /// Cannot backup or write Ref
    #[error("Sheet depends on local namespace")]
    SheetDependsOnLocalNamespace,

    /// Operation not supported by the current protocol
    #[error("Operation not supported")]
    OperationNotSupported,

    #[error("IO error: `{0}`")]
    IOError(#[from] std::io::Error),
}
