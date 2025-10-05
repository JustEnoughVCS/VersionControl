use tcp_connection::error::TcpTargetError;

use crate::action::{Action, ActionContext};

/// A pool of registered actions that can be processed by name
pub struct ActionPool {
    /// HashMap storing action name to action implementation mapping
    actions: std::collections::HashMap<&'static str, Box<dyn ActionErased>>,
}

impl ActionPool {
    /// Creates a new empty ActionPool
    pub fn new() -> Self {
        Self {
            actions: std::collections::HashMap::new(),
        }
    }

    /// Registers an action type with the pool
    ///
    /// Usage:
    /// ```
    /// action_pool.register::<MyAction, MyArgs, MyReturn>();
    /// ```
    pub fn register<A, Args, Return>(&mut self)
    where
        A: Action<Args, Return> + Send + Sync + 'static,
        Args: serde::de::DeserializeOwned + Send + Sync + 'static,
        Return: serde::Serialize + Send + Sync + 'static,
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
    /// ```
    /// let result = action_pool.process::<MyArgs, MyReturn>("my_action", context, args).await?;
    /// ```
    pub async fn process<'a, Args, Return>(
        &'a self,
        action_name: &'a str,
        context: ActionContext,
        args: Args,
    ) -> Result<Return, TcpTargetError>
    where
        Args: serde::de::DeserializeOwned + Send + 'static,
        Return: serde::Serialize + Send + 'static,
    {
        if let Some(action) = self.actions.get(action_name) {
            let result = action.process_erased(context, Box::new(args)).await?;
            let result = *result
                .downcast::<Return>()
                .map_err(|_| TcpTargetError::Unsupported("InvalidArguments".to_string()))?;
            Ok(result)
        } else {
            Err(TcpTargetError::Unsupported("InvalidAction".to_string()))
        }
    }
}

/// Trait for type-erased actions that can be stored in ActionPool
trait ActionErased: Send + Sync {
    /// Processes the action with type-erased arguments and returns type-erased result
    fn process_erased(
        &self,
        context: ActionContext,
        args: Box<dyn std::any::Any + Send>,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<Box<dyn std::any::Any + Send>, TcpTargetError>>
                + Send,
        >,
    >;
}

/// Wrapper struct that implements ActionErased for concrete Action types
struct ActionWrapper<A, Args, Return>(std::marker::PhantomData<(A, Args, Return)>);

impl<A, Args, Return> ActionErased for ActionWrapper<A, Args, Return>
where
    A: Action<Args, Return> + Send + Sync,
    Args: serde::de::DeserializeOwned + Send + Sync + 'static,
    Return: serde::Serialize + Send + Sync + 'static,
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
}
