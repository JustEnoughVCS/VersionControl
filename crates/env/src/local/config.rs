use cfg_file::ConfigFile;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

use crate::constants::CLIENT_FILE_WORKSPACE;
use crate::constants::PORT;

#[derive(Serialize, Deserialize, ConfigFile)]
#[cfg_file(path = CLIENT_FILE_WORKSPACE)]
pub struct LocalConfig {
    target: SocketAddr,
}

impl Default for LocalConfig {
    fn default() -> Self {
        Self {
            target: SocketAddr::V4(std::net::SocketAddrV4::new(
                std::net::Ipv4Addr::new(127, 0, 0, 1),
                PORT,
            )),
        }
    }
}

impl LocalConfig {}
