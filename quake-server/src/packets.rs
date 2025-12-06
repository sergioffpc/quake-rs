use std::sync::Arc;
use tracing::log::info;

pub struct QuakeConnectionRequestHandler;

impl QuakeConnectionRequestHandler {
    pub const OPCODE: u8 = 0x01;
}

#[async_trait::async_trait]
impl quake_network::PacketHandler for QuakeConnectionRequestHandler {
    async fn handle(&self, data: &[u8]) -> anyhow::Result<Box<[u8]>> {
        if data != b"QUAKE\x03" {
            return Err(anyhow::anyhow!("Invalid connection control request"));
        }

        info!("Received connection control request");
        Ok(vec![0x81].into_boxed_slice())
    }
}

pub struct QuakeConsoleRequestHandler {
    console_manager: Arc<quake_console::ConsoleManager>,
}

impl QuakeConsoleRequestHandler {
    pub const OPCODE: u8 = 0x04;

    pub fn new(console_manager: Arc<quake_console::ConsoleManager>) -> Self {
        Self { console_manager }
    }
}

#[async_trait::async_trait]
impl quake_network::PacketHandler for QuakeConsoleRequestHandler {
    async fn handle(&self, data: &[u8]) -> anyhow::Result<Box<[u8]>> {
        let text = String::from_utf8_lossy(data).to_string();
        info!("Received console command: {}", text);

        let (output, _) = self.console_manager.execute_command(text.as_str()).await?;

        Ok(output.into_bytes().into_boxed_slice())
    }
}
