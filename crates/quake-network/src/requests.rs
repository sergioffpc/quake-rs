use std::collections::HashMap;
use std::fmt::Debug;
use tracing::log::info;

pub trait RequestHandler: Debug + Send + Sync {
    fn handle(&self, data: &[u8]) -> anyhow::Result<Box<[u8]>>;
}

#[derive(Debug, Default)]
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

#[derive(Debug)]
pub struct ConnectionControlRequestHandler;

impl RequestHandler for ConnectionControlRequestHandler {
    fn handle(&self, data: &[u8]) -> anyhow::Result<Box<[u8]>> {
        if data != b"QUAKE\x03" {
            return Err(anyhow::anyhow!("Invalid connection control request"));
        }

        info!("Received connection control request");
        Ok(vec![0x81].into_boxed_slice())
    }
}

#[derive(Debug)]
pub struct ServerInfoControlRequestHandler;

impl RequestHandler for ServerInfoControlRequestHandler {
    fn handle(&self, data: &[u8]) -> anyhow::Result<Box<[u8]>> {
        if data != b"QUAKE\x03" {
            return Err(anyhow::anyhow!("Invalid server info control request"));
        }

        info!("Received server info control request");
        Ok(vec![].into_boxed_slice())
    }
}
