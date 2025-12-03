use std::io::Cursor;
use tokio::sync::Mutex;

pub mod commands;

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

    pub async fn play_sound<D>(&self, sound: D) -> anyhow::Result<()>
    where
        D: kira::sound::SoundData,
    {
        self.manager
            .lock()
            .await
            .play(sound)
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        Ok(())
    }

    pub async fn play_music<D>(&self, music: D) -> anyhow::Result<()>
    where
        D: kira::sound::SoundData,
    {
        self.stop_music().await?;
        self.manager
            .lock()
            .await
            .play(music)
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        Ok(())
    }

    pub async fn stop_music(&self) -> anyhow::Result<()> {
        if let Some(channel) = &self.channel {
            channel.lock().await.stop(kira::Tween::default());
        }
        Ok(())
    }

    pub async fn pause_music(&self) -> anyhow::Result<()> {
        if let Some(channel) = &self.channel {
            channel.lock().await.pause(kira::Tween::default());
        }
        Ok(())
    }

    pub async fn resume_music(&self) -> anyhow::Result<()> {
        if let Some(channel) = &self.channel {
            channel.lock().await.resume(kira::Tween::default());
        }
        Ok(())
    }
}

struct Snd {
    data: kira::sound::static_sound::StaticSoundData,
}

#[async_trait::async_trait]
impl quake_traits::FromBytes for Snd {
    async fn from_bytes(data: &[u8]) -> anyhow::Result<Self> {
        let data =
            kira::sound::static_sound::StaticSoundData::from_cursor(Cursor::new(data.to_vec()))?;

        Ok(Self { data })
    }
}
