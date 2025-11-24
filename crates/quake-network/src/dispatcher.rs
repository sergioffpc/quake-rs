use std::collections::HashMap;
use tracing::log::{error, warn};

#[derive(Default)]
pub struct RequestDispatcher {
    handlers: HashMap<u8, Box<dyn RequestHandler>>,
}

impl RequestDispatcher {
    pub fn dispatch(&self, from: std::net::SocketAddr, data: &[u8]) {
        if data.is_empty() {
            return;
        }

        if let Some(handler) = self.handlers.get(&data[0]) {
            if let Err(e) = handler.handle(from, &data[1..]) {
                error!("Error handling request from {}: {}", from, e);
            }
        } else {
            warn!("Received unknown packet from {}", from);
        }
    }

    pub fn register_handler(&mut self, packet_type: u8, handler: Box<dyn RequestHandler>) {
        self.handlers.insert(packet_type, handler);
    }

    pub fn unregister_handler(&mut self, packet_type: u8) {
        self.handlers.remove(&packet_type);
    }
}

pub trait RequestHandler: Send + Sync {
    fn handle(&self, from: std::net::SocketAddr, data: &[u8]) -> anyhow::Result<()>;
}
