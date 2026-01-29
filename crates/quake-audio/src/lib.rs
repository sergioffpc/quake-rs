use kira::sound::static_sound::StaticSoundData;
use std::fmt::Display;
use std::io::Cursor;

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
    precache: Vec<StaticSoundData>,
}

impl AudioManager {
    pub fn new() -> anyhow::Result<Self> {
        let manager =
            kira::AudioManager::<kira::DefaultBackend>::new(kira::AudioManagerSettings::default())?;
        Ok(Self {
            manager,
            precache: Vec::new(),
        })
    }

    pub fn preload<T>(&mut self, cursor: Cursor<T>) -> anyhow::Result<()>
    where
        T: AsRef<[u8]> + Send + Sync + 'static,
    {
        let sound = StaticSoundData::from_cursor(cursor)?;
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
