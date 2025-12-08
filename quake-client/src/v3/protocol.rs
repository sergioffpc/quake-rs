use quake_console::ConsoleManager;
use quake_network::PacketHandler;
use std::sync::Arc;

#[derive(Default)]
pub struct BadPacketHandler;
impl BadPacketHandler {
    pub const OPCODE: u8 = 0x00;
}

#[async_trait::async_trait]
impl PacketHandler for BadPacketHandler {
    async fn handle(&self, _data: &[u8]) -> anyhow::Result<Box<[u8]>> {
        Err(anyhow::anyhow!("Bad packet"))
    }
}

#[derive(Default)]
pub struct NopPacketHandler;
impl NopPacketHandler {
    pub const OPCODE: u8 = 0x01;
}

#[async_trait::async_trait]
impl PacketHandler for NopPacketHandler {
    async fn handle(&self, _data: &[u8]) -> anyhow::Result<Box<[u8]>> {
        Ok(vec![].into_boxed_slice())
    }
}

pub struct DisconnectPacketHandler {
    console_manager: Arc<ConsoleManager>,
}

impl DisconnectPacketHandler {
    pub const OPCODE: u8 = 0x02;

    pub fn new(console_manager: Arc<ConsoleManager>) -> Self {
        Self { console_manager }
    }
}

#[async_trait::async_trait]
impl PacketHandler for DisconnectPacketHandler {
    async fn handle(&self, _data: &[u8]) -> anyhow::Result<Box<[u8]>> {
        self.console_manager.append_text("disconnect").await;

        Ok(vec![].into_boxed_slice())
    }
}

pub struct UpdateStatPacketHandler;
impl UpdateStatPacketHandler {
    pub const OPCODE: u8 = 0x03;
}

#[async_trait::async_trait]
impl PacketHandler for UpdateStatPacketHandler {
    async fn handle(&self, data: &[u8]) -> anyhow::Result<Box<[u8]>> {
        todo!()
    }
}

pub struct VersionPacketHandler;
impl VersionPacketHandler {
    pub const OPCODE: u8 = 0x04;
}

#[async_trait::async_trait]
impl PacketHandler for VersionPacketHandler {
    async fn handle(&self, data: &[u8]) -> anyhow::Result<Box<[u8]>> {
        todo!()
    }
}

pub struct SetViewPacketHandler;
impl SetViewPacketHandler {
    pub const OPCODE: u8 = 0x05;
}

#[async_trait::async_trait]
impl PacketHandler for SetViewPacketHandler {
    async fn handle(&self, data: &[u8]) -> anyhow::Result<Box<[u8]>> {
        todo!()
    }
}

pub struct SoundPacketHandler;
impl SoundPacketHandler {
    pub const OPCODE: u8 = 0x06;
}

#[async_trait::async_trait]
impl PacketHandler for SoundPacketHandler {
    async fn handle(&self, data: &[u8]) -> anyhow::Result<Box<[u8]>> {
        todo!()
    }
}

pub struct TimePacketHandler;
impl TimePacketHandler {
    pub const OPCODE: u8 = 0x07;
}

#[async_trait::async_trait]
impl PacketHandler for TimePacketHandler {
    async fn handle(&self, data: &[u8]) -> anyhow::Result<Box<[u8]>> {
        todo!()
    }
}

pub struct PrintPacketHandler;
impl PrintPacketHandler {
    pub const OPCODE: u8 = 0x08;
}

#[async_trait::async_trait]
impl PacketHandler for PrintPacketHandler {
    async fn handle(&self, data: &[u8]) -> anyhow::Result<Box<[u8]>> {
        todo!()
    }
}

pub struct StuffTextPacketHandler;
impl StuffTextPacketHandler {
    pub const OPCODE: u8 = 0x09;
}

#[async_trait::async_trait]
impl PacketHandler for StuffTextPacketHandler {
    async fn handle(&self, data: &[u8]) -> anyhow::Result<Box<[u8]>> {
        todo!()
    }
}

pub struct SetAnglePacketHandler;
impl SetAnglePacketHandler {
    pub const OPCODE: u8 = 0x0a;
}

#[async_trait::async_trait]
impl PacketHandler for SetAnglePacketHandler {
    async fn handle(&self, data: &[u8]) -> anyhow::Result<Box<[u8]>> {
        todo!()
    }
}

pub struct ServerInfoPacketHandler;
impl ServerInfoPacketHandler {
    pub const OPCODE: u8 = 0x0b;
}

#[async_trait::async_trait]
impl PacketHandler for ServerInfoPacketHandler {
    async fn handle(&self, data: &[u8]) -> anyhow::Result<Box<[u8]>> {
        todo!()
    }
}

pub struct LightStylePacketHandler;
impl LightStylePacketHandler {
    pub const OPCODE: u8 = 0x0c;
}

#[async_trait::async_trait]
impl PacketHandler for LightStylePacketHandler {
    async fn handle(&self, data: &[u8]) -> anyhow::Result<Box<[u8]>> {
        todo!()
    }
}

pub struct UpdateNamePacketHandler;
impl UpdateNamePacketHandler {
    pub const OPCODE: u8 = 0x0d;
}

#[async_trait::async_trait]
impl PacketHandler for UpdateNamePacketHandler {
    async fn handle(&self, data: &[u8]) -> anyhow::Result<Box<[u8]>> {
        todo!()
    }
}

pub struct UpdateFragsPacketHandler;
impl UpdateFragsPacketHandler {
    pub const OPCODE: u8 = 0x0e;
}

#[async_trait::async_trait]
impl PacketHandler for UpdateFragsPacketHandler {
    async fn handle(&self, data: &[u8]) -> anyhow::Result<Box<[u8]>> {
        todo!()
    }
}
pub struct ClientDataPacketHandler;
impl ClientDataPacketHandler {
    pub const OPCODE: u8 = 0x0f;
}

#[async_trait::async_trait]
impl PacketHandler for ClientDataPacketHandler {
    async fn handle(&self, data: &[u8]) -> anyhow::Result<Box<[u8]>> {
        todo!()
    }
}

pub struct StopSoundPacketHandler;
impl StopSoundPacketHandler {
    pub const OPCODE: u8 = 0x10;
}

#[async_trait::async_trait]
impl PacketHandler for StopSoundPacketHandler {
    async fn handle(&self, data: &[u8]) -> anyhow::Result<Box<[u8]>> {
        todo!()
    }
}
pub struct UpdateColorsPacketHandler;
impl UpdateColorsPacketHandler {
    pub const OPCODE: u8 = 0x11;
}

#[async_trait::async_trait]
impl PacketHandler for UpdateColorsPacketHandler {
    async fn handle(&self, data: &[u8]) -> anyhow::Result<Box<[u8]>> {
        todo!()
    }
}

pub struct ParticlePacketHandler;
impl ParticlePacketHandler {
    pub const OPCODE: u8 = 0x12;
}

#[async_trait::async_trait]
impl PacketHandler for ParticlePacketHandler {
    async fn handle(&self, data: &[u8]) -> anyhow::Result<Box<[u8]>> {
        todo!()
    }
}

pub struct DamagePacketHandler;
impl DamagePacketHandler {
    pub const OPCODE: u8 = 0x13;
}

#[async_trait::async_trait]
impl PacketHandler for DamagePacketHandler {
    async fn handle(&self, data: &[u8]) -> anyhow::Result<Box<[u8]>> {
        todo!()
    }
}
pub struct SpawnStaticPacketHandler;
impl SpawnStaticPacketHandler {
    pub const OPCODE: u8 = 0x14;
}

#[async_trait::async_trait]
impl PacketHandler for SpawnStaticPacketHandler {
    async fn handle(&self, data: &[u8]) -> anyhow::Result<Box<[u8]>> {
        todo!()
    }
}

pub struct SpawnBinaryPacketHandler;
impl SpawnBinaryPacketHandler {
    pub const OPCODE: u8 = 0x15;
}

#[async_trait::async_trait]
impl PacketHandler for SpawnBinaryPacketHandler {
    async fn handle(&self, _data: &[u8]) -> anyhow::Result<Box<[u8]>> {
        unimplemented!("This packet is deprecated and should not be used")
    }
}

pub struct SpawnBaselinePacketHandler;
impl SpawnBaselinePacketHandler {
    pub const OPCODE: u8 = 0x16;
}

#[async_trait::async_trait]
impl PacketHandler for SpawnBaselinePacketHandler {
    async fn handle(&self, data: &[u8]) -> anyhow::Result<Box<[u8]>> {
        todo!()
    }
}

pub struct TempEntityPacketHandler;
impl TempEntityPacketHandler {
    pub const OPCODE: u8 = 0x17;
}

#[async_trait::async_trait]
impl PacketHandler for TempEntityPacketHandler {
    async fn handle(&self, data: &[u8]) -> anyhow::Result<Box<[u8]>> {
        todo!()
    }
}

pub struct SetPausePacketHandler;
impl SetPausePacketHandler {
    pub const OPCODE: u8 = 0x18;
}

#[async_trait::async_trait]
impl PacketHandler for SetPausePacketHandler {
    async fn handle(&self, data: &[u8]) -> anyhow::Result<Box<[u8]>> {
        todo!()
    }
}

pub struct SignOnNumPacketHandler;
impl SignOnNumPacketHandler {
    pub const OPCODE: u8 = 0x19;
}

#[async_trait::async_trait]
impl PacketHandler for SignOnNumPacketHandler {
    async fn handle(&self, data: &[u8]) -> anyhow::Result<Box<[u8]>> {
        todo!()
    }
}

pub struct CenterPrintPacketHandler;
impl CenterPrintPacketHandler {
    pub const OPCODE: u8 = 0x1a;
}

#[async_trait::async_trait]
impl PacketHandler for CenterPrintPacketHandler {
    async fn handle(&self, _data: &[u8]) -> anyhow::Result<Box<[u8]>> {
        todo!()
    }
}

pub struct KilledMonsterPacketHandler;
impl KilledMonsterPacketHandler {
    pub const OPCODE: u8 = 0x1b;
}

#[async_trait::async_trait]
impl PacketHandler for KilledMonsterPacketHandler {
    async fn handle(&self, _data: &[u8]) -> anyhow::Result<Box<[u8]>> {
        todo!()
    }
}

pub struct FoundSecretPacketHandler;
impl FoundSecretPacketHandler {
    pub const OPCODE: u8 = 0x1c;
}

#[async_trait::async_trait]
impl PacketHandler for FoundSecretPacketHandler {
    async fn handle(&self, _data: &[u8]) -> anyhow::Result<Box<[u8]>> {
        todo!()
    }
}

pub struct SpawnStaticSoundPacketHandler;
impl SpawnStaticSoundPacketHandler {
    pub const OPCODE: u8 = 0x1d;
}

#[async_trait::async_trait]
impl PacketHandler for SpawnStaticSoundPacketHandler {
    async fn handle(&self, _data: &[u8]) -> anyhow::Result<Box<[u8]>> {
        todo!()
    }
}

pub struct InterMissionPacketHandler;
impl InterMissionPacketHandler {
    pub const OPCODE: u8 = 0x1e;
}

#[async_trait::async_trait]
impl PacketHandler for InterMissionPacketHandler {
    async fn handle(&self, _data: &[u8]) -> anyhow::Result<Box<[u8]>> {
        todo!()
    }
}

pub struct FinalePacketHandler;
impl FinalePacketHandler {
    pub const OPCODE: u8 = 0x1f;
}

#[async_trait::async_trait]
impl PacketHandler for FinalePacketHandler {
    async fn handle(&self, _data: &[u8]) -> anyhow::Result<Box<[u8]>> {
        todo!()
    }
}

pub struct CdTrackPacketHandler;
impl CdTrackPacketHandler {
    pub const OPCODE: u8 = 0x20;
}

#[async_trait::async_trait]
impl PacketHandler for CdTrackPacketHandler {
    async fn handle(&self, _data: &[u8]) -> anyhow::Result<Box<[u8]>> {
        todo!()
    }
}

pub struct SellScreenPacketHandler;
impl SellScreenPacketHandler {
    pub const OPCODE: u8 = 0x21;
}

#[async_trait::async_trait]
impl PacketHandler for SellScreenPacketHandler {
    async fn handle(&self, _data: &[u8]) -> anyhow::Result<Box<[u8]>> {
        todo!()
    }
}

pub struct UpdateEntityPacketHandler;
impl UpdateEntityPacketHandler {
    pub const OPCODE: u8 = 0x80;
}

#[async_trait::async_trait]
impl PacketHandler for UpdateEntityPacketHandler {
    async fn handle(&self, _data: &[u8]) -> anyhow::Result<Box<[u8]>> {
        todo!()
    }
}
