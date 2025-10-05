use tcp_connection::{error::TcpTargetError, instance::ConnectionInstance};

pub trait Action<Args, Return> {
    fn action_name() -> &'static str;

    fn is_remote_action() -> bool;

    fn process(
        context: ActionContext,
        args: Args,
    ) -> impl std::future::Future<Output = Result<Return, TcpTargetError>> + Send;
}

pub struct ActionContext {
    // Whether the action is executed locally or remotely
    local: bool,

    /// The connection instance in the current context,
    /// used to interact with the machine on the other end
    instance: ConnectionInstance,
}

impl ActionContext {
    /// Whether the action is executed locally
    pub fn is_local(&self) -> bool {
        self.local
    }

    /// Whether the action is executed remotely
    pub fn is_remote(&self) -> bool {
        !self.local
    }

    /// Get the connection instance in the current context
    pub fn instance(&self) -> &ConnectionInstance {
        &self.instance
    }
}
