use std::sync::Arc;
use tracing::log::info;

pub struct ConnectPacketHandler;

impl ConnectPacketHandler {
    pub const OPCODE: u8 = 0x01;
}

#[async_trait::async_trait]
impl quake_network::PacketHandler for ConnectPacketHandler {
    async fn handle(&mut self, data: &[u8]) -> anyhow::Result<Box<[u8]>> {
        info!("Received connect packet");

        if data != b"QUAKE\x03" {
            return Err(anyhow::anyhow!("Invalid protocol version"));
        }

        Ok(b"OK".to_vec().into_boxed_slice())
    }
}

pub struct ConsolePacketHandler {
    console_manager: Arc<quake_console::ConsoleManager>,
}

impl ConsolePacketHandler {
    pub const OPCODE: u8 = 0x04;

    pub fn new(console_manager: Arc<quake_console::ConsoleManager>) -> Self {
        Self { console_manager }
    }
}

#[async_trait::async_trait]
impl quake_network::PacketHandler for ConsolePacketHandler {
    async fn handle(&mut self, data: &[u8]) -> anyhow::Result<Box<[u8]>> {
        let text = String::from_utf8_lossy(data).to_string();
        info!("Received console packet: {}", text);

        self.console_manager.append_text(text.as_str()).await;

        Ok(b"OK".to_vec().into_boxed_slice())
    }
}
