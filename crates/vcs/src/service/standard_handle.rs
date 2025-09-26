use tcp_connection::handle::{ClientHandle, ServerHandle};

pub struct StandardClientHandle;
pub struct StandardServerHandle;

impl ClientHandle<StandardServerHandle> for StandardClientHandle {
    async fn process(instance: tcp_connection::instance::ConnectionInstance) {
        todo!()
    }
}

impl ServerHandle<StandardClientHandle> for StandardServerHandle {
    async fn process(instance: tcp_connection::instance::ConnectionInstance) {
        todo!()
    }
}
