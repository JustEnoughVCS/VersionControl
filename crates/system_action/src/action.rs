use serde::{Serialize, de::DeserializeOwned};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;
use tcp_connection::{error::TcpTargetError, instance::ConnectionInstance};
use tokio::{net::TcpStream, sync::Mutex};

/// # Trait - Action<Args, Return>
///
/// A trait used to describe the interaction pattern between client and server
///
/// ## Generics
///
/// Args: Represents the parameter type required for this action
///
/// Return: Represents the return type of this action
///
/// The above generics must implement serde's Serialize and DeserializeOwned traits,
/// and must be sendable between threads
///
/// ## Implementation
///
/// ```ignore
/// pub trait Action<Args, Return>
/// where
///     Args: Serialize + DeserializeOwned + Send,
///     Return: Serialize + DeserializeOwned + Send,
/// {
///     /// Name, used to inform the server which action to execute
///     fn action_name() -> &'static str;
///
///     /// Whether it's a local Action, used to inform the system if it only runs locally
///     fn is_remote_action() -> bool;
///
///     /// Action processing logic
///     fn process(
///         context: ActionContext,
///         args: Args,
///     ) -> impl std::future::Future<Output = Result<Return, TcpTargetError>> + Send;
/// }
/// ```
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

/// # Struct - ActionContext
///
/// Used to inform the Action about the current execution environment
///
/// ## Creation
///
/// Create ActionContext using the following methods:
///
/// ```ignore
///
/// // The instance here is the connection instance passed from external sources for communicating with the server
/// // For specific usage, please refer to the `/crates/utils/tcp_connection` section
///
/// fn init_local_action_ctx(instance: ConnectionInstance) {
///     // Create context and specify execution on local
///     let mut ctx = ActionContext::local();
/// }
///
/// fn init_remote_action_ctx(instance: ConnectionInstance) {
///     // Create context and specify execution on remote
///     let mut ctx = ActionContext::remote();
/// }
#[derive(Default)]
pub struct ActionContext {
    /// Whether the action is executed locally or remotely
    proc_on_local: bool,

    /// Whether the action being executed in the current context is a remote action
    is_remote_action: bool,

    /// The name of the action being executed
    action_name: String,

    /// The JSON-serialized arguments for the action
    action_args_json: String,

    /// The connection instance in the current context,
    instance: Option<Arc<Mutex<ConnectionInstance>>>,

    /// Generic data storage for arbitrary types
    data: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
}

impl ActionContext {
    /// Generate local context
    pub fn local() -> Self {
        ActionContext {
            proc_on_local: true,
            ..Default::default()
        }
    }

    /// Generate remote context
    pub fn remote() -> Self {
        ActionContext {
            proc_on_local: false,
            ..Default::default()
        }
    }

    /// Build connection instance from TcpStream
    pub fn build_instance(mut self, stream: TcpStream) -> Self {
        self.instance = Some(Arc::new(Mutex::new(ConnectionInstance::from(stream))));
        self
    }

    /// Insert connection instance into context
    pub fn insert_instance(mut self, instance: ConnectionInstance) -> Self {
        self.instance = Some(Arc::new(Mutex::new(instance)));
        self
    }

    /// Pop connection instance from context
    pub fn pop_instance(&mut self) -> Option<Arc<Mutex<ConnectionInstance>>> {
        self.instance.take()
    }
}

impl ActionContext {
    /// Whether the action is executed locally
    pub fn is_proc_on_local(&self) -> bool {
        self.proc_on_local
    }

    /// Whether the action is executed remotely
    pub fn is_proc_on_remote(&self) -> bool {
        !self.proc_on_local
    }

    /// Whether the action being executed in the current context is a remote action
    pub fn is_remote_action(&self) -> bool {
        self.is_remote_action
    }

    /// Set whether the action being executed in the current context is a remote action
    pub fn set_is_remote_action(&mut self, is_remote_action: bool) {
        self.is_remote_action = is_remote_action;
    }

    /// Get the connection instance in the current context
    pub fn instance(&self) -> &Option<Arc<Mutex<ConnectionInstance>>> {
        &self.instance
    }

    /// Get a mutable reference to the connection instance in the current context
    pub fn instance_mut(&mut self) -> &mut Option<Arc<Mutex<ConnectionInstance>>> {
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
    pub fn set_action_args(mut self, action_args: String) -> Self {
        self.action_args_json = action_args;
        self
    }

    /// Insert arbitrary data in the context
    pub fn with_data<T: Any + Send + Sync>(mut self, value: T) -> Self {
        self.data.insert(TypeId::of::<T>(), Arc::new(value));
        self
    }

    /// Insert arbitrary data as Arc in the context
    pub fn with_arc_data<T: Any + Send + Sync>(mut self, value: Arc<T>) -> Self {
        self.data.insert(TypeId::of::<T>(), value);
        self
    }

    /// Insert arbitrary data in the context
    pub fn insert_data<T: Any + Send + Sync>(&mut self, value: T) {
        self.data.insert(TypeId::of::<T>(), Arc::new(value));
    }

    /// Insert arbitrary data as Arc in the context
    pub fn insert_arc_data<T: Any + Send + Sync>(&mut self, value: Arc<T>) {
        self.data.insert(TypeId::of::<T>(), value);
    }

    /// Get arbitrary data from the context
    pub fn get<T: Any + Send + Sync>(&self) -> Option<&T> {
        self.data
            .get(&TypeId::of::<T>())
            .and_then(|arc| arc.downcast_ref::<T>())
    }

    /// Get arbitrary data as Arc from the context
    pub fn get_arc<T: Any + Send + Sync>(&self) -> Option<Arc<T>> {
        self.data
            .get(&TypeId::of::<T>())
            .and_then(|arc| Arc::clone(arc).downcast::<T>().ok())
    }

    /// Remove and return arbitrary data from the context
    pub fn remove<T: Any + Send + Sync>(&mut self) -> Option<Arc<T>> {
        self.data
            .remove(&TypeId::of::<T>())
            .and_then(|arc| arc.downcast::<T>().ok())
    }

    /// Check if the context contains data of a specific type
    pub fn contains<T: Any + Send + Sync>(&self) -> bool {
        self.data.contains_key(&TypeId::of::<T>())
    }

    /// Take ownership of the context and extract data of a specific type
    pub fn take<T: Any + Send + Sync>(mut self) -> (Self, Option<Arc<T>>) {
        let value = self
            .data
            .remove(&TypeId::of::<T>())
            .and_then(|arc| arc.downcast::<T>().ok());
        (self, value)
    }
}
