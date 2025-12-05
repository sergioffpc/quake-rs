use crate::{AudioManager, Snd};
use std::fmt::Write;
use std::sync::Arc;

#[derive(Clone)]
pub struct AudioCommands {
    audio_manager: Arc<AudioManager>,
    resources_manager: Arc<quake_resources::ResourcesManager>,
}

impl AudioCommands {
    pub const BUILTIN_COMMANDS: &'static [&'static str] = &["play", "cd", "soundlist"];

    pub fn new(
        audio_manager: Arc<AudioManager>,
        resources_manager: Arc<quake_resources::ResourcesManager>,
    ) -> Self {
        Self {
            audio_manager,
            resources_manager,
        }
    }

    async fn play(&mut self, args: &[&str]) -> anyhow::Result<(String, quake_traits::ControlFlow)> {
        let sound = self.load_static_sound_data_from_cache(args[0]).await?;
        self.audio_manager.play_sound(sound).await?;
        Ok((String::default(), quake_traits::ControlFlow::Poll))
    }

    async fn cd(&mut self, args: &[&str]) -> anyhow::Result<(String, quake_traits::ControlFlow)> {
        let mut iter = args.iter();
        match *iter.next().unwrap() {
            "play" => {
                let track_name =
                    format!("music/track{:02}.ogg", iter.next().unwrap().parse::<u32>()?);
                let sound = self.load_static_sound_data_from_cache(&track_name).await?;
                self.audio_manager.play_sound(sound).await?;
            }
            "loop" => {
                let track_name =
                    format!("music/track{:02}.ogg", iter.next().unwrap().parse::<u32>()?);
                let sound = self
                    .load_static_sound_data_from_cache(&track_name)
                    .await?
                    .with_settings(
                        kira::sound::static_sound::StaticSoundSettings::default().loop_region(..),
                    );

                self.audio_manager.play_music(sound).await?
            }
            "stop" => self.audio_manager.stop_music().await?,
            "resume" => self.audio_manager.resume_music().await?,
            _ => (),
        }
        Ok((String::default(), quake_traits::ControlFlow::Poll))
    }

    async fn soundlist(&mut self) -> anyhow::Result<(String, quake_traits::ControlFlow)> {
        const SUPPORTED_EXTENSIONS: &[&str] = &[".mp3", ".ogg", ".flac", ".wav"];

        let mut buffer = String::new();
        self.resources_manager
            .cached_names()
            .await
            .filter(|name| SUPPORTED_EXTENSIONS.iter().any(|ext| name.ends_with(ext)))
            .for_each(|name| writeln!(&mut buffer, "{}", name).unwrap());
        Ok((buffer, quake_traits::ControlFlow::Poll))
    }

    async fn load_static_sound_data_from_cache(
        &self,
        name: &str,
    ) -> anyhow::Result<kira::sound::static_sound::StaticSoundData> {
        Ok(self
            .resources_manager
            .by_cached_name::<Snd>(name)
            .await?
            .data
            .clone())
    }
}

#[async_trait::async_trait]
impl quake_traits::CommandHandler for AudioCommands {
    async fn handle_command(
        &mut self,
        command: &[&str],
    ) -> anyhow::Result<(String, quake_traits::ControlFlow)> {
        match command[0] {
            "play" => self.play(&command[1..]).await,
            "cd" => self.cd(&command[1..]).await,
            "soundlist" => self.soundlist().await,
            _ => Ok((String::default(), quake_traits::ControlFlow::Poll)),
        }
    }
}
