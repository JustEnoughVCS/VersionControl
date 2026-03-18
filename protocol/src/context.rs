use framework::space::{Space, SpaceRoot};

pub struct ProtocolContext<User, Server>
where
    User: SpaceRoot,
    Server: SpaceRoot,
{
    /// Current context's Vault
    pub remote: Option<Space<Server>>,

    /// Current context's Workspace
    pub user: Option<Space<User>>,
}

impl<User, Server> ProtocolContext<User, Server>
where
    User: SpaceRoot,
    Server: SpaceRoot,
{
    /// Create a new local context with a user
    pub fn new_local(user: Space<User>) -> Self {
        Self {
            remote: None,
            user: Some(user),
        }
    }

    /// Create a new remote context with a server
    pub fn new_remote(server: Space<Server>) -> Self {
        Self {
            remote: Some(server),
            user: None,
        }
    }
}

impl<User, Server> ProtocolContext<User, Server>
where
    User: SpaceRoot,
    Server: SpaceRoot,
{
    /// Check if the context is remote (has a server)
    pub fn is_remote(&self) -> bool {
        self.remote.is_some()
    }

    /// Check if the context is local (has a user)
    pub fn is_local(&self) -> bool {
        self.user.is_some()
    }

    /// Get the remote vault if it exists
    pub fn remote(&self) -> Option<&Space<Server>> {
        self.remote.as_ref()
    }

    /// Get the local workspace if it exists
    pub fn local(&self) -> Option<&Space<User>> {
        self.user.as_ref()
    }

    /// Unwrap the local workspace, panics if not local
    pub fn unwrap_local(&self) -> &Space<User> {
        self.user.as_ref().expect("Not a local context")
    }

    /// Unwrap the remote vault, panics if not remote
    pub fn unwrap_remote(&self) -> &Space<Server> {
        self.remote.as_ref().expect("Not a remote context")
    }
}
