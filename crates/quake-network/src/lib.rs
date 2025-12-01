use std::collections::HashMap;

pub mod client;
pub mod commands;
pub mod server;

pub trait RequestHandler: Send + Sync {
    fn handle(&self, data: &[u8]) -> anyhow::Result<Box<[u8]>>;
}

#[derive(Default)]
pub struct RequestDispatcher {
    handlers: HashMap<u8, Box<dyn RequestHandler>>,
}

impl RequestDispatcher {
    pub fn dispatch(&self, data: &[u8]) -> anyhow::Result<Box<[u8]>> {
        match self.handlers.get(&data[0]) {
            Some(handler) => handler.handle(&data[1..]),
            None => Err(anyhow::anyhow!("No handler for packet type {}", data[0])),
        }
    }

    pub fn register_handler(&mut self, packet_type: u8, handler: Box<dyn RequestHandler>) {
        self.handlers.insert(packet_type, handler);
    }

    pub fn unregister_handler(&mut self, packet_type: u8) {
        self.handlers.remove(&packet_type);
    }
}
