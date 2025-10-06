use action_system::{action::ActionContext, action_gen, action_pool::ActionPool};
use tcp_connection::error::TcpTargetError;

#[tokio::main]
async fn main() {
    let mut pool = ActionPool::new();
    PrintNameAction::register_to_pool(&mut pool);

    PrintNameAction::process_at_pool(&pool, ActionContext::local(), "World".to_string())
        .await
        .unwrap();
}

#[action_gen]
async fn print_name_action(_ctx: ActionContext, name: String) -> Result<(), TcpTargetError> {
    println!("Hello, {}!", name);
    Ok(())
}
