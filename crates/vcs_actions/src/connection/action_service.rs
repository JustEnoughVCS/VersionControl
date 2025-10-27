use std::{
    net::SocketAddr,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

use action_system::{action::ActionContext, action_pool::ActionPool};
use cfg_file::config::ConfigFile;
use log::{error, info, warn};
use tcp_connection::{error::TcpTargetError, instance::ConnectionInstance};
use tokio::{
    net::{TcpListener, TcpStream},
    select, signal, spawn,
    sync::mpsc,
};
use vcs_data::data::vault::{Vault, config::VaultConfig};

use crate::{
    connection::protocol::RemoteActionInvoke, registry::server_registry::server_action_pool,
};

// Start the server with a Vault using the specified directory
pub async fn server_entry(vault_path: impl Into<PathBuf>) -> Result<(), TcpTargetError> {
    // Read the vault cfg
    let vault_cfg = VaultConfig::read().await?;

    // Create TCPListener
    let listener = create_tcp_listener(&vault_cfg).await?;

    // Initialize the vault
    let vault: Arc<Vault> = init_vault(vault_cfg, vault_path.into()).await?;

    // Lock the vault
    vault.lock().map_err(|e| {
        error!("{}", e);
        TcpTargetError::Locked(e.to_string())
    })?;

    // Create ActionPool
    let action_pool: Arc<ActionPool> = Arc::new(server_action_pool());

    // Start the server
    let (_shutdown_rx, future) = build_server_future(vault.clone(), action_pool.clone(), listener);
    future.await?; // Start and block until shutdown

    // Unlock the vault
    vault.unlock()?;

    Ok(())
}

async fn create_tcp_listener(cfg: &VaultConfig) -> Result<TcpListener, TcpTargetError> {
    let local_bind_addr = cfg.server_config().local_bind();
    let bind_port = cfg.server_config().port();
    let sock_addr = SocketAddr::new(*local_bind_addr, bind_port);
    let listener = TcpListener::bind(sock_addr).await?;

    Ok(listener)
}

async fn init_vault(cfg: VaultConfig, path: PathBuf) -> Result<Arc<Vault>, TcpTargetError> {
    // Init and create the vault
    let Some(vault) = Vault::init(cfg, path) else {
        return Err(TcpTargetError::NotFound("Vault not found".to_string()));
    };
    let vault: Arc<Vault> = Arc::new(vault);

    Ok(vault)
}

fn build_server_future(
    vault: Arc<Vault>,
    action_pool: Arc<ActionPool>,
    listener: TcpListener,
) -> (
    mpsc::Sender<()>,
    impl std::future::Future<Output = Result<(), TcpTargetError>>,
) {
    let (tx, mut rx) = mpsc::channel::<i32>(100);
    let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
    let mut active_connections = 0;
    let mut shutdown_requested = false;

    // Spawn task to handle Ctrl+C with rapid exit detection
    let shutdown_tx_clone = shutdown_tx.clone();
    spawn(async move {
        let mut ctrl_c_count = 0;
        let mut last_ctrl_c_time = Instant::now();

        while let Ok(()) = signal::ctrl_c().await {
            let now = Instant::now();

            // Reset counter if more than 5 seconds have passed
            if now.duration_since(last_ctrl_c_time) > Duration::from_secs(5) {
                ctrl_c_count = 0;
            }

            ctrl_c_count += 1;
            last_ctrl_c_time = now;

            let _ = shutdown_tx_clone.send(()).await;

            // If 3 Ctrl+C within 5 seconds, exit immediately
            if ctrl_c_count >= 3 {
                info!("Shutdown. (3/3)");
                std::process::exit(0);
            } else {
                info!("Ctrl + C to force shutdown. ({} / 3)", ctrl_c_count);
            }
        }
    });

    let future = async move {
        loop {
            select! {
                // Accept new connections
                accept_result = listener.accept(), if !shutdown_requested => {
                    match accept_result {
                        Ok((stream, _addr)) => {
                            info!("New connection. (now {})", active_connections);
                            let _ = tx.send(1).await;

                            let vault_clone = vault.clone();
                            let action_pool_clone = action_pool.clone();
                            let tx_clone = tx.clone();

                            spawn(async move {
                                process_connection(stream, vault_clone, action_pool_clone).await;
                                info!("A connection closed. (now {})", active_connections);
                                let _ = tx_clone.send(-1).await;
                            });
                        }
                        Err(_) => {
                            continue;
                        }
                    }
                }

                // Handle connection count updates
                Some(count_change) = rx.recv() => {
                    active_connections = (active_connections as i32 + count_change) as usize;

                    // Check if we should shutdown after all connections are done
                    if shutdown_requested && active_connections == 0 {
                        break;
                    }
                }

                // Handle shutdown signal
                _ = shutdown_rx.recv() => {
                    shutdown_requested = true;
                    // If no active connections, break immediately
                    if active_connections == 0 {
                        info!("No active connections. Shutting down.");
                        break;
                    } else {
                        warn!("Cannot shutdown while active connections exist! ({} active)", active_connections);
                    }
                }
            }
        }

        Ok(())
    };

    (shutdown_tx, future)
}

async fn process_connection(stream: TcpStream, vault: Arc<Vault>, action_pool: Arc<ActionPool>) {
    // Setup connection instance
    let mut instance = ConnectionInstance::from(stream);

    // Read action name and action arguments
    let msg = match instance.read_msgpack::<RemoteActionInvoke>().await {
        Ok(msg) => msg,
        Err(e) => {
            error!("Failed to read action message: {}", e);
            return;
        }
    };

    // Build context
    let ctx: ActionContext = ActionContext::remote().insert_instance(instance);

    // Insert vault into context
    let ctx = ctx.insert_arc(vault);

    info!(
        "Process action `{}` with argument `{}`",
        msg.action_name, msg.action_args_json
    );

    // Process action
    let result = action_pool
        .process_json(&msg.action_name, ctx, msg.action_args_json)
        .await;

    match result {
        Ok(_result_json) => {}
        Err(e) => {
            warn!("Failed to process action `{}`: {}", msg.action_name, e);
        }
    }
}
