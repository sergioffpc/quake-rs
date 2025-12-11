use quake_resources::dem::Block;
use tracing::log::info;

pub struct DemoPacketHandler {
    iterator: Box<dyn Iterator<Item = Block> + Send + Sync>,
}

impl DemoPacketHandler {
    pub const OPCODE: u8 = 0x01;

    pub fn new<I>(iterator: I) -> Self
    where
        I: Iterator<Item = Block> + Send + Sync + 'static,
    {
        Self {
            iterator: Box::new(iterator),
        }
    }
}

#[async_trait::async_trait]
impl quake_network::PacketHandler for DemoPacketHandler {
    async fn handle(&mut self, _data: &[u8]) -> anyhow::Result<Box<[u8]>> {
        info!("Received demo packet");

        match self.iterator.next() {
            Some(block) => Ok(block.messages.into_boxed_slice()),
            None => Ok(vec![].into_boxed_slice()),
        }
    }
}
