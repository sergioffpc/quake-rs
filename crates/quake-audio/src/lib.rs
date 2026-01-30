use kira::sound::static_sound::StaticSoundData;
use std::fmt::Display;
use std::io::Cursor;
use std::path::Path;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SoundId(u64);

impl From<u64> for SoundId {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<SoundId> for u64 {
    fn from(value: SoundId) -> Self {
        value.0
    }
}

impl Display for SoundId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub struct AudioManager {
    manager: kira::AudioManager<kira::DefaultBackend>,

    asset_manager: Arc<quake_asset::AssetManager>,
    precache: Vec<StaticSoundData>,
}

impl AudioManager {
    pub fn new(asset_manager: Arc<quake_asset::AssetManager>) -> anyhow::Result<Self> {
        let manager =
            kira::AudioManager::<kira::DefaultBackend>::new(kira::AudioManagerSettings::default())?;
        Ok(Self {
            asset_manager,
            manager,
            precache: Vec::new(),
        })
    }

    pub fn preload<P>(&mut self, sound_path: P) -> anyhow::Result<()>
    where
        P: AsRef<Path>,
    {
        let data = self
            .asset_manager
            .by_name::<Vec<u8>>(sound_path.as_ref().to_str().unwrap())?;
        let sound = StaticSoundData::from_cursor(Cursor::new(data))?;
        self.precache.push(sound);
        Ok(())
    }

    pub fn play(&mut self, sound_id: SoundId) -> anyhow::Result<()> {
        let sound = self
            .precache
            .get(u64::from(sound_id) as usize)
            .ok_or_else(|| anyhow::anyhow!("sound ID not found: {}", sound_id))?;
        self.manager.play(sound.clone())?;
        Ok(())
    }
}
