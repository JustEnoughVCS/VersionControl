use serde::{Serialize, de::DeserializeOwned};
use tcp_connection::{error::TcpTargetError, instance::ConnectionInstance};
use tokio::net::TcpStream;

pub trait Action<Args, Return>
where
    Args: Serialize + DeserializeOwned + Send,
    Return: Serialize + DeserializeOwned + Send,
{
    fn action_name() -> &'static str;

    fn is_remote_action() -> bool;

    fn process(
        context: ActionContext,
        args: Args,
    ) -> impl std::future::Future<Output = Result<Return, TcpTargetError>> + Send;
}

#[derive(Default)]
pub struct ActionContext {
    /// Whether the action is executed locally or remotely
    local: bool,

    /// The name of the action being executed
    action_name: String,

    /// The JSON-serialized arguments for the action
    action_args_json: String,

    /// The connection instance in the current context,
    /// used to interact with the machine on the other end
    instance: Option<ConnectionInstance>,
}

impl ActionContext {
    /// Generate local context
    pub fn local() -> Self {
        let mut ctx = ActionContext::default();
        ctx.local = true;
        ctx
    }

    /// Generate remote context
    pub fn remote() -> Self {
        let mut ctx = ActionContext::default();
        ctx.local = false;
        ctx
    }

    /// Build connection instance from TcpStream
    pub fn build_instance(mut self, stream: TcpStream) -> Self {
        self.instance = Some(ConnectionInstance::from(stream));
        self
    }

    /// Insert connection instance into context
    pub fn insert_instance(mut self, instance: ConnectionInstance) -> Self {
        self.instance = Some(instance);
        self
    }

    /// Pop connection instance from context
    pub fn pop_instance(&mut self) -> Option<ConnectionInstance> {
        self.instance.take()
    }
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
    pub fn instance(&self) -> &Option<ConnectionInstance> {
        &self.instance
    }

    /// Get a mutable reference to the connection instance in the current context
    pub fn instance_mut(&mut self) -> &mut Option<ConnectionInstance> {
        &mut self.instance
    }

    /// Get the action name from the context
    pub fn action_name(&self) -> &str {
        &self.action_name
    }

    /// Get the action arguments from the context
    pub fn action_args_json(&self) -> &String {
        &self.action_args_json
    }

    /// Set the action name in the context
    pub fn set_action_name(mut self, action_name: String) -> Self {
        self.action_name = action_name;
        self
    }

    /// Set the action arguments in the context
    pub fn set_action_args_json(mut self, action_args: String) -> Self {
        self.action_args_json = action_args;
        self
    }
}
