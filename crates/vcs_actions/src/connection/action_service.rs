use std::{net::SocketAddr, path::PathBuf, sync::Arc};

use action_system::action_pool::ActionPool;
use cfg_file::config::ConfigFile;
use tcp_connection::{error::TcpTargetError, instance::ConnectionInstance};
use tokio::{
    net::{TcpListener, TcpStream},
    select, signal, spawn,
    sync::mpsc,
};
use vcs_data::data::vault::{Vault, config::VaultConfig};

use crate::registry::server_registry::server_action_pool;

// Start the server with a Vault using the specified directory
pub async fn server_entry(path: impl Into<PathBuf>) -> Result<(), TcpTargetError> {
    // Read the vault cfg
    let vault_cfg = VaultConfig::read().await?;

    // Create TCPListener
    let listener = create_tcp_listener(&vault_cfg).await?;

    // Initialize the vault
    let vault: Arc<Vault> = init_vault(vault_cfg, path.into()).await?;

    // Create ActionPool
    let action_pool: Arc<ActionPool> = Arc::new(server_action_pool());

    // Start the server
    let (_shutdown_rx, future) = build_server_future(vault.clone(), action_pool.clone(), listener);
    let _ = future.await?; // Start and block until shutdown

    Ok(())
}

async fn create_tcp_listener(cfg: &VaultConfig) -> Result<TcpListener, TcpTargetError> {
    let local_bind_addr = cfg.server_config().local_bind();
    let bind_port = cfg.server_config().port();
    let sock_addr = SocketAddr::new(local_bind_addr.clone(), bind_port);
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

    // Spawn task to handle Ctrl+C
    let shutdown_tx_clone = shutdown_tx.clone();
    spawn(async move {
        if let Ok(()) = signal::ctrl_c().await {
            let _ = shutdown_tx_clone.send(()).await;
        }
    });

    let future = async move {
        loop {
            select! {
                // Accept new connections
                accept_result = listener.accept(), if !shutdown_requested => {
                    match accept_result {
                        Ok((stream, _addr)) => {
                            active_connections += 1;
                            let _ = tx.send(1).await;

                            let vault_clone = vault.clone();
                            let action_pool_clone = action_pool.clone();
                            let tx_clone = tx.clone();
                            spawn(async move {
                                process_connection(stream, vault_clone, action_pool_clone).await;
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
                        break;
                    }
                }
            }
        }

        Ok(())
    };

    (shutdown_tx, future)
}

async fn process_connection(stream: TcpStream, vault: Arc<Vault>, action_pool: Arc<ActionPool>) {
    let instance = ConnectionInstance::from(stream);
}
