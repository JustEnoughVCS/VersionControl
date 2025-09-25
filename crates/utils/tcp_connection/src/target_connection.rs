use tokio::{
    net::{TcpListener, TcpSocket},
    spawn,
};

use crate::{
    error::TcpTargetError,
    handle::{ClientHandle, ServerHandle},
    instance::ConnectionInstance,
    target::TcpServerTarget,
    target_configure::ServerTargetConfig,
};

impl<Client, Server> TcpServerTarget<Client, Server>
where
    Client: ClientHandle<Server>,
    Server: ServerHandle<Client>,
{
    /// Attempts to establish a connection to the TCP server.
    /// This function initiates a connection to the server address specified in the target configuration.
    /// This is a Block operation.
    pub async fn connect(&self) -> Result<(), TcpTargetError> {
        let addr = self.get_addr();
        let Ok(socket) = TcpSocket::new_v4() else {
            return Err(TcpTargetError::from("Create tcp socket failed!"));
        };
        let stream = match socket.connect(addr).await {
            Ok(stream) => stream,
            Err(e) => {
                let err = format!("Connect to `{}` failed: {}", addr, e);
                return Err(TcpTargetError::from(err));
            }
        };
        let instance = ConnectionInstance::from(stream);
        Client::process(instance).await;
        Ok(())
    }

    /// Attempts to establish a connection to the TCP server.
    /// This function initiates a connection to the server address specified in the target configuration.
    pub async fn listen(&self) -> Result<(), TcpTargetError> {
        let addr = self.get_addr();
        let listener = match TcpListener::bind(addr).await {
            Ok(listener) => listener,
            Err(_) => {
                let err = format!("Bind to `{}` failed", addr);
                return Err(TcpTargetError::from(err));
            }
        };

        let cfg: ServerTargetConfig = match self.get_server_cfg() {
            Some(cfg) => *cfg,
            None => ServerTargetConfig::default(),
        };

        if cfg.is_once() {
            // Process once (Blocked)
            let (stream, _) = match listener.accept().await {
                Ok(result) => result,
                Err(e) => {
                    let err = format!("Accept connection failed: {}", e);
                    return Err(TcpTargetError::from(err));
                }
            };
            let instance = ConnectionInstance::from(stream);
            Server::process(instance).await;
        } else {
            loop {
                // Process multiple times (Concurrent)
                let (stream, _) = match listener.accept().await {
                    Ok(result) => result,
                    Err(e) => {
                        let err = format!("Accept connection failed: {}", e);
                        return Err(TcpTargetError::from(err));
                    }
                };
                let instance = ConnectionInstance::from(stream);
                spawn(async move {
                    Server::process(instance).await;
                });
            }
        }
        Ok(())
    }
}
