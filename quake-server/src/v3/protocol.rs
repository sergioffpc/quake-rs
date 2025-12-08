use std::sync::Arc;
use tracing::log::info;

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
    async fn handle(&self, data: &[u8]) -> anyhow::Result<Box<[u8]>> {
        let text = String::from_utf8_lossy(data).to_string();
        info!("Received console command: {}", text);

        let (output, _) = self.console_manager.execute_command(text.as_str()).await?;

        Ok(output.into_bytes().into_boxed_slice())
    }
}
