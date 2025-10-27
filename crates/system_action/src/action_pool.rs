use std::pin::Pin;

use serde::{Serialize, de::DeserializeOwned};
use serde_json;
use tcp_connection::error::TcpTargetError;

use crate::action::{Action, ActionContext};

type ProcBeginCallback = for<'a> fn(
    &'a mut ActionContext,
    args: &'a (dyn std::any::Any + Send + Sync),
) -> ProcBeginFuture<'a>;
type ProcEndCallback = fn() -> ProcEndFuture;

type ProcBeginFuture<'a> = Pin<Box<dyn Future<Output = Result<(), TcpTargetError>> + Send + 'a>>;
type ProcEndFuture = Pin<Box<dyn Future<Output = Result<(), TcpTargetError>> + Send>>;

/// A pool of registered actions that can be processed by name
pub struct ActionPool {
    /// HashMap storing action name to action implementation mapping
    actions: std::collections::HashMap<&'static str, Box<dyn ActionErased>>,

    /// Callback to execute when process begins
    on_proc_begin: Option<ProcBeginCallback>,

    /// Callback to execute when process ends
    on_proc_end: Option<ProcEndCallback>,
}

impl Default for ActionPool {
    fn default() -> Self {
        Self::new()
    }
}

impl ActionPool {
    /// Creates a new empty ActionPool
    pub fn new() -> Self {
        Self {
            actions: std::collections::HashMap::new(),
            on_proc_begin: None,
            on_proc_end: None,
        }
    }

    /// Sets a callback to be executed when process begins
    pub fn set_on_proc_begin(&mut self, callback: ProcBeginCallback) {
        self.on_proc_begin = Some(callback);
    }

    /// Sets a callback to be executed when process ends
    pub fn set_on_proc_end(&mut self, callback: ProcEndCallback) {
        self.on_proc_end = Some(callback);
    }

    /// Registers an action type with the pool
    ///
    /// Usage:
    /// ```ignore
    /// action_pool.register::<MyAction, MyArgs, MyReturn>();
    /// ```
    pub fn register<A, Args, Return>(&mut self)
    where
        A: Action<Args, Return> + Send + Sync + 'static,
        Args: serde::Serialize + serde::de::DeserializeOwned + Send + Sync + 'static,
        Return: serde::Serialize + serde::de::DeserializeOwned + Send + Sync + 'static,
    {
        let action_name = A::action_name();
        self.actions.insert(
            action_name,
            Box::new(ActionWrapper::<A, Args, Return>(std::marker::PhantomData)),
        );
    }

    /// Processes an action by name with given context and arguments
    ///
    /// Usage:
    /// ```ignore
    /// let result = action_pool.process::<MyArgs, MyReturn>("my_action", context, args).await?;
    /// ```
    /// Processes an action by name with JSON-serialized arguments
    ///
    /// Usage:
    /// ```ignore
    /// let result_json = action_pool.process_json("my_action", context, args_json).await?;
    /// let result: MyReturn = serde_json::from_str(&result_json)?;
    /// ```
    pub async fn process_json<'a>(
        &'a self,
        action_name: &'a str,
        context: ActionContext,
        args_json: String,
    ) -> Result<String, TcpTargetError> {
        if let Some(action) = self.actions.get(action_name) {
            // Set action name and args in context for callbacks
            let context = context.set_action_name(action_name.to_string());
            let mut context = context.set_action_args(args_json.clone());

            self.exec_on_proc_begin(&mut context, &args_json).await?;
            let result = action.process_json_erased(context, args_json).await?;
            self.exec_on_proc_end().await?;
            Ok(result)
        } else {
            Err(TcpTargetError::Unsupported("InvalidAction".to_string()))
        }
    }

    /// Processes an action by name with given context and arguments
    ///
    /// Usage:
    /// ```ignore
    /// let result = action_pool.process::<MyArgs, MyReturn>("my_action", context, args).await?;
    /// ```
    pub async fn process<'a, Args, Return>(
        &'a self,
        action_name: &'a str,
        mut context: ActionContext,
        args: Args,
    ) -> Result<Return, TcpTargetError>
    where
        Args: serde::de::DeserializeOwned + Send + Sync + 'static,
        Return: serde::Serialize + Send + 'static,
    {
        if let Some(action) = self.actions.get(action_name) {
            self.exec_on_proc_begin(&mut context, &args).await?;
            let result = action.process_erased(context, Box::new(args)).await?;
            let result = *result
                .downcast::<Return>()
                .map_err(|_| TcpTargetError::Unsupported("InvalidArguments".to_string()))?;
            self.exec_on_proc_end().await?;
            Ok(result)
        } else {
            Err(TcpTargetError::Unsupported("InvalidAction".to_string()))
        }
    }

    /// Executes the process begin callback if set
    async fn exec_on_proc_begin(
        &self,
        context: &mut ActionContext,
        args: &(dyn std::any::Any + Send + Sync),
    ) -> Result<(), TcpTargetError> {
        if let Some(callback) = &self.on_proc_begin {
            callback(context, args).await
        } else {
            Ok(())
        }
    }

    /// Executes the process end callback if set
    async fn exec_on_proc_end(&self) -> Result<(), TcpTargetError> {
        if let Some(callback) = &self.on_proc_end {
            callback().await
        } else {
            Ok(())
        }
    }
}

/// Trait for type-erased actions that can be stored in ActionPool
type ProcessErasedFuture = std::pin::Pin<
    Box<
        dyn std::future::Future<Output = Result<Box<dyn std::any::Any + Send>, TcpTargetError>>
            + Send,
    >,
>;
type ProcessJsonErasedFuture =
    std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, TcpTargetError>> + Send>>;

trait ActionErased: Send + Sync {
    /// Processes the action with type-erased arguments and returns type-erased result
    fn process_erased(
        &self,
        context: ActionContext,
        args: Box<dyn std::any::Any + Send>,
    ) -> ProcessErasedFuture;

    /// Processes the action with JSON-serialized arguments and returns JSON-serialized result
    fn process_json_erased(
        &self,
        context: ActionContext,
        args_json: String,
    ) -> ProcessJsonErasedFuture;
}

/// Wrapper struct that implements ActionErased for concrete Action types
struct ActionWrapper<A, Args, Return>(std::marker::PhantomData<(A, Args, Return)>);

impl<A, Args, Return> ActionErased for ActionWrapper<A, Args, Return>
where
    A: Action<Args, Return> + Send + Sync,
    Args: Serialize + DeserializeOwned + Send + Sync + 'static,
    Return: Serialize + DeserializeOwned + Send + Sync + 'static,
{
    fn process_erased(
        &self,
        context: ActionContext,
        args: Box<dyn std::any::Any + Send>,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<Box<dyn std::any::Any + Send>, TcpTargetError>>
                + Send,
        >,
    > {
        Box::pin(async move {
            let args = *args
                .downcast::<Args>()
                .map_err(|_| TcpTargetError::Unsupported("InvalidArguments".to_string()))?;
            let result = A::process(context, args).await?;
            Ok(Box::new(result) as Box<dyn std::any::Any + Send>)
        })
    }

    fn process_json_erased(
        &self,
        context: ActionContext,
        args_json: String,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, TcpTargetError>> + Send>>
    {
        Box::pin(async move {
            let args: Args = serde_json::from_str(&args_json)
                .map_err(|e| TcpTargetError::Serialization(format!("Deserialize failed: {}", e)))?;
            let result = A::process(context, args).await?;
            let result_json = serde_json::to_string(&result)
                .map_err(|e| TcpTargetError::Serialization(format!("Serialize failed: {}", e)))?;
            Ok(result_json)
        })
    }
}
