use std::path::PathBuf;

use crate::{
    context::ProtocolContext,
    member::Member,
    protocol::{
        error::{FetchLatestInfoFailed, ProtocolAuthorizeFailed, VaultOperationFailed},
        fetched_info::FetchedInfo,
        index_transfer::IndexesTransfer,
        operations::{VaultHostOperations, VaultOperations},
    },
};
use framework::space::{Space, SpaceRoot};
use vault_system::vault::Vault;
use workspace_system::workspace::Workspace;

pub mod error;
pub mod fetched_info;
pub mod index_transfer;
pub mod operations;

pub trait BasicProtocol {
    type User: SpaceRoot;
    type Server: SpaceRoot;

    /// Protocol name
    /// For example: return "jvcs" to accept protocols starting with "jvcs://"
    fn protocol_name() -> &'static str;

    /// Authentication
    ///
    /// authorizing is executed on the client side, and for some protocols also on the server side,
    /// ultimately outputting the authentication result
    /// - Ok(Member): indicates a successfully authenticated member
    /// - Err(ProtocolAuthorizeFailed): indicates the reason for failure
    fn authorizing(
        &self,
        ctx: &ProtocolContext<Self::User, Self::Server>,
    ) -> impl Future<Output = Result<Member, ProtocolAuthorizeFailed>> + Send;

    /// Host authentication
    ///
    /// Authenticate as a host (administrator) on the server side.
    /// Returns the authenticated host member if successful.
    fn authorizing_host(
        &self,
        ctx: &ProtocolContext<Self::User, Self::Server>,
    ) -> impl Future<Output = Result<Member, ProtocolAuthorizeFailed>> + Send;

    /// Build user space
    ///
    /// Build and return the user space based on the given current directory path.
    fn build_user_space(&self, current_dir: PathBuf) -> Space<Self::User>;

    /// Build server space
    ///
    /// Build and return the server space based on the given current directory path.
    fn build_server_space(&self, current_dir: PathBuf) -> Space<Self::Server>;

    /// Fetch the latest information from upstream
    ///
    /// - On the local side, workspace will be Some
    /// - On the remote side, vault will be Some
    ///
    /// # Result
    /// - The remote side returns Ok(Some(FetchedInfo)) to send data to the local side
    /// - The local side returns Ok(Some(FetchedInfo)) to get the latest information
    fn fetch_latest_info(
        &self,
        workspace: Option<Space<Workspace>>,
        vault: Option<Space<Vault>>,
    ) -> impl Future<Output = Result<Option<FetchedInfo>, FetchLatestInfoFailed>> + Send;

    /// Transfer indexes
    ///
    /// - `index_transfer` and `storage` represent the index file and
    /// the corresponding block storage path, respectively.
    /// - If `vault` is Some, send block information from the Vault.
    /// - If `workspace` is Some, send block information from the Workspace.
    ///
    /// Each block transfer is atomic, but the overall transfer is not atomic.
    /// Blocks that have been successfully transferred are permanently retained on the upstream side.
    ///
    /// After the block information is correctly stored on the other side,
    /// transfer the `index_file` and register it to the corresponding IndexSource.
    ///
    /// If the IndexSource to be registered is a `local_id`, the server will issue a `remote_id`
    /// and write it into the local ID mapping.
    fn transfer_indexes(
        &self,
        index_transfer: IndexesTransfer,
        storage: PathBuf,
        workspace: Option<Space<Workspace>>,
        vault: Option<Space<Vault>>,
    ) -> impl Future<Output = Result<(), FetchLatestInfoFailed>> + Send;

    /// Handle operations
    ///
    /// Requests are sent from the client to the server,
    /// and the following requests are uniformly processed on the server side.
    fn handle_operation(
        &self,
        operation: VaultOperations,
        authorized_member: Member,
        vault: Space<Vault>,
    ) -> impl Future<Output = Result<(), VaultOperationFailed>> + Send;

    /// Handle host operations
    ///
    /// Requests are sent from the host (administrator) to the server,
    /// and the following requests are uniformly processed on the server side.
    fn handle_host_operation(
        &self,
        operation: VaultHostOperations,
        authorized_host: Member,
        vault: Space<Vault>,
    ) -> impl Future<Output = Result<(), VaultOperationFailed>> + Send;
}
