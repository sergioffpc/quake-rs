use std::collections::HashMap;
use tokio::io::{AsyncRead, AsyncWrite};

pub mod client;
pub mod commands;
pub mod server;

#[async_trait::async_trait]
pub trait StreamHandler: Send + Sync {
    async fn handle_stream(
        &self,
        sender: &mut (dyn AsyncWrite + Unpin + Send),
        receiver: &mut (dyn AsyncRead + Unpin + Send),
    );
}

#[async_trait::async_trait]
pub trait StreamHandlerBuilder: Send + Sync {
    async fn build(&self) -> anyhow::Result<Box<dyn StreamHandler>>;
}

#[async_trait::async_trait]
pub trait PacketHandler: Send + Sync {
    async fn handle(&self, data: &[u8]) -> anyhow::Result<Box<[u8]>>;
}

#[derive(Default)]
pub struct PacketDispatcher {
    handlers: HashMap<u8, Box<dyn PacketHandler>>,
}

impl PacketDispatcher {
    pub async fn dispatch(&self, data: &[u8]) -> anyhow::Result<Box<[u8]>> {
        match self.handlers.get(&data[0]) {
            Some(handler) => handler.handle(&data[1..]).await,
            None => Err(anyhow::anyhow!("No handler for packet type {}", data[0])),
        }
    }

    pub fn register_handler(&mut self, packet_type: u8, handler: Box<dyn PacketHandler>) {
        self.handlers.insert(packet_type, handler);
    }

    pub fn unregister_handler(&mut self, packet_type: u8) {
        self.handlers.remove(&packet_type);
    }
}
