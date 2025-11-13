use std::io::Cursor;
use std::sync::{Arc, RwLock};

pub mod builtins;

pub struct AudioManager {
    manager: kira::AudioManager,
    channel: Option<kira::sound::static_sound::StaticSoundHandle>,

    resources: Arc<RwLock<quake_resources::Resources>>,
}

impl AudioManager {
    pub fn new(resources: Arc<RwLock<quake_resources::Resources>>) -> anyhow::Result<Self> {
        Ok(Self {
            manager: kira::AudioManager::<kira::DefaultBackend>::new(
                kira::AudioManagerSettings::default(),
            )?,
            channel: None,

            resources,
        })
    }

    pub fn play_sound(&mut self, name: &str) -> anyhow::Result<()> {
        let sound_data = {
            let mut resources = self
                .resources
                .write()
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            self.load_resource_audio(&mut *resources, name)?
                .data
                .clone()
        };

        self.manager.play(sound_data)?;

        Ok(())
    }

    pub fn play_music(&mut self, name: &str, looped: bool) -> anyhow::Result<()> {
        let track_name = format!("music/track{:02}.ogg", name.parse::<u32>()?);

        let sound_data = {
            let mut resources = self
                .resources
                .write()
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            self.load_resource_audio(&mut *resources, track_name.as_str())?
                .data
                .clone()
        };

        self.stop_music();

        let sound_data = if looped {
            sound_data.with_settings(
                kira::sound::static_sound::StaticSoundSettings::default().loop_region(..),
            )
        } else {
            sound_data
        };
        self.channel = Some(self.manager.play(sound_data)?);

        Ok(())
    }

    pub fn stop_music(&mut self) {
        if let Some(channel) = &mut self.channel {
            channel.stop(kira::Tween::default())
        }
    }

    pub fn pause_music(&mut self) {
        if let Some(channel) = &mut self.channel {
            channel.pause(kira::Tween::default())
        }
    }

    pub fn resume_music(&mut self) {
        if let Some(channel) = &mut self.channel {
            channel.resume(kira::Tween::default())
        }
    }

    fn load_resource_audio(
        &self,
        resources: &mut quake_resources::Resources,
        name: &str,
    ) -> anyhow::Result<Arc<Snd>> {
        resources.by_cached_name::<Snd>(name)
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
