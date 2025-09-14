use tokio::{net::TcpListener, spawn};

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
    pub async fn connect(&self) -> Result<(), TcpTargetError> {
        Ok(())
    }

    pub async fn listen(&self) -> Result<(), TcpTargetError> {
        let addr = self.get_addr();
        let listener = match TcpListener::bind(addr.clone()).await {
            Ok(listener) => listener,
            Err(_) => {
                let err = format!("Bind to `{}` failed", addr);
                return Err(TcpTargetError::from(err));
            }
        };

        let cfg: ServerTargetConfig = match self.get_server_cfg() {
            Some(cfg) => cfg.clone(),
            None => ServerTargetConfig::default(),
        };

        if cfg.is_once() {
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
