use crate::protocol::BasicProtocol;
use std::marker::PhantomData;

/// Upstream, used by Workspace to describe the protocol of UpstreamVault
pub struct Upstream<Protocol>
where
    Protocol: BasicProtocol,
{
    /// Protocol of the target upstream machine
    _p: PhantomData<Protocol>,

    /// Address of the target upstream machine
    target_address: String,
}

impl<Protocol> Upstream<Protocol>
where
    Protocol: BasicProtocol,
{
    pub fn new(addr: &str) -> Self {
        Upstream {
            _p: PhantomData,
            target_address: addr.to_string(),
        }
    }
}

/// Host, used by Vault to describe its own protocol
pub struct Host<Protocol>
where
    Protocol: BasicProtocol,
{
    /// Protocol of the target upstream machine
    _p: PhantomData<Protocol>,
}

impl<Protocol> Upstream<Protocol>
where
    Protocol: BasicProtocol,
{
    pub fn address(addr: &str) -> Self {
        Upstream {
            _p: PhantomData,
            target_address: addr.to_string(),
        }
    }
}

impl<Protocol> Host<Protocol>
where
    Protocol: BasicProtocol,
{
    pub fn new() -> Self {
        Host { _p: PhantomData }
    }
}
