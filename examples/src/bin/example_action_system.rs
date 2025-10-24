use action_system::{action::ActionContext, action_pool::ActionPool, macros::action_gen};
use tcp_connection::error::TcpTargetError;

#[tokio::main]
async fn main() {
    let mut pool = ActionPool::new();
    register_print_name_action(&mut pool);

    proc_print_name_action(&pool, ActionContext::local(), "World".to_string())
        .await
        .unwrap();
}

#[action_gen]
async fn print_name_action(_ctx: ActionContext, name: String) -> Result<(), TcpTargetError> {
    println!("Hello, {}!", name);
    Ok(())
}
