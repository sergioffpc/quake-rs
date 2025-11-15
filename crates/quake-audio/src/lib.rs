use parking_lot::Mutex;
use std::io::Cursor;

pub mod builtins;

pub struct AudioManager {
    manager: Mutex<kira::AudioManager>,
    channel: Option<Mutex<kira::sound::static_sound::StaticSoundHandle>>,
}

impl AudioManager {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            manager: Mutex::new(kira::AudioManager::<kira::DefaultBackend>::new(
                kira::AudioManagerSettings::default(),
            )?),
            channel: None,
        })
    }

    pub fn play_sound<D>(&self, sound: D) -> anyhow::Result<()>
    where
        D: kira::sound::SoundData,
    {
        self.manager
            .lock()
            .play(sound)
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        Ok(())
    }

    pub fn play_music<D>(&self, music: D) -> anyhow::Result<()>
    where
        D: kira::sound::SoundData,
    {
        self.stop_music()?;

        self.manager
            .lock()
            .play(music)
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        Ok(())
    }

    pub fn stop_music(&self) -> anyhow::Result<()> {
        if let Some(channel) = &self.channel {
            channel.lock().stop(kira::Tween::default());
        }
        Ok(())
    }

    pub fn pause_music(&self) -> anyhow::Result<()> {
        if let Some(channel) = &self.channel {
            channel.lock().pause(kira::Tween::default());
        }
        Ok(())
    }

    pub fn resume_music(&self) -> anyhow::Result<()> {
        if let Some(channel) = &self.channel {
            channel.lock().resume(kira::Tween::default());
        }
        Ok(())
    }
}

struct Snd {
    data: kira::sound::static_sound::StaticSoundData,
}

impl quake_resources::FromBytes for Snd {
    fn from_bytes(data: &[u8]) -> anyhow::Result<Self> {
        let data =
            kira::sound::static_sound::StaticSoundData::from_cursor(Cursor::new(data.to_vec()))?;

        Ok(Self { data })
    }
}
