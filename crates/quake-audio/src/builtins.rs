use crate::{AudioManager, Snd};
use quake_traits::ControlFlow;
use std::sync::Arc;

#[derive(Clone)]
pub struct AudioBuiltins {
    inner: Arc<AudioManager>,
    resources: Arc<quake_resources::Resources>,
}

impl AudioBuiltins {
    pub const BUILTIN_COMMANDS: &'static [&'static str] = &["play", "cd", "soundlist"];

    pub fn new(inner: Arc<AudioManager>, resources: Arc<quake_resources::Resources>) -> Self {
        Self { inner, resources }
    }

    async fn builtin_play(&mut self, args: &[&str]) -> anyhow::Result<ControlFlow> {
        let sound = self.load_static_sound_data_from_cache(args[0]).await?;
        self.inner.play_sound(sound).await?;
        Ok(ControlFlow::Poll)
    }

    async fn builtin_cd(&mut self, args: &[&str]) -> anyhow::Result<ControlFlow> {
        let mut iter = args.iter();
        match *iter.next().unwrap() {
            "play" => {
                let track_name =
                    format!("music/track{:02}.ogg", iter.next().unwrap().parse::<u32>()?);
                let sound = self.load_static_sound_data_from_cache(&track_name).await?;
                self.inner.play_sound(sound).await?;
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

                self.inner.play_music(sound).await?
            }
            "stop" => self.inner.stop_music().await?,
            "resume" => self.inner.resume_music().await?,
            _ => (),
        }
        Ok(ControlFlow::Poll)
    }

    async fn builtin_soundlist(&mut self) -> anyhow::Result<ControlFlow> {
        const SUPPORTED_EXTENSIONS: &[&str] = &[".mp3", ".ogg", ".flac", ".wav"];

        use std::io::Write;
        self.resources
            .cached_names()
            .await
            .filter(|name| SUPPORTED_EXTENSIONS.iter().any(|ext| name.ends_with(ext)))
            .for_each(|name| writeln!(std::io::stdout(), "{}", name).unwrap());
        Ok(ControlFlow::Poll)
    }

    async fn load_static_sound_data_from_cache(
        &self,
        name: &str,
    ) -> anyhow::Result<kira::sound::static_sound::StaticSoundData> {
        Ok(self
            .resources
            .by_cached_name::<Snd>(name)
            .await?
            .data
            .clone())
    }
}

#[async_trait::async_trait]
impl quake_traits::CommandHandler for AudioBuiltins {
    async fn handle_command(&mut self, command: &[&str]) -> anyhow::Result<ControlFlow> {
        match command[0] {
            "play" => self.builtin_play(&command[1..]).await,
            "cd" => self.builtin_cd(&command[1..]).await,
            "soundlist" => self.builtin_soundlist().await,
            _ => Ok(ControlFlow::Poll),
        }
    }
}
