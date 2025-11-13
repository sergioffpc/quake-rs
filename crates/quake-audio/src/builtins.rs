use crate::AudioManager;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct AudioBuiltins {
    inner: Arc<Mutex<AudioManager>>,
}

impl AudioBuiltins {
    pub const BUILTIN_COMMANDS: &'static [&'static str] = &["play", "cd", "soundlist"];

    pub fn new(manager: Arc<Mutex<AudioManager>>) -> Self {
        Self { inner: manager }
    }

    pub fn builtin_play(&mut self, args: &[&str]) -> anyhow::Result<()> {
        self.lock_manager()?.play_sound(args[0])
    }

    pub fn builtin_cd(&mut self, args: &[&str]) -> anyhow::Result<()> {
        let mut manager = self.lock_manager()?;

        let mut iter = args.iter();
        match iter.next().unwrap() {
            &"play" => manager.play_music(iter.next().unwrap(), false)?,
            &"loop" => manager.play_music(iter.next().unwrap(), true)?,
            &"stop" => manager.stop_music(),
            &"resume" => manager.resume_music(),
            _ => (),
        }

        Ok(())
    }

    pub fn builtin_soundlist(&mut self) -> anyhow::Result<()> {
        const SUPPORTED_EXTENSIONS: &[&str] = &[".mp3", ".ogg", ".flac", ".wav"];

        use std::io::Write;
        let manager = self.lock_manager()?;
        let resources = manager
            .resources
            .read()
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        resources
            .cached_names()
            .filter(|name| SUPPORTED_EXTENSIONS.iter().any(|ext| name.ends_with(ext)))
            .for_each(|name| writeln!(std::io::stdout(), "{}", name).unwrap());

        Ok(())
    }

    fn lock_manager(&self) -> anyhow::Result<std::sync::MutexGuard<AudioManager>> {
        self.inner.lock().map_err(|e| anyhow::anyhow!("{}", e))
    }
}

#[async_trait::async_trait]
impl quake_traits::CommandHandler for AudioBuiltins {
    async fn handle_command(&mut self, command: &[&str]) -> anyhow::Result<()> {
        match command[0] {
            "play" => self.builtin_play(&command[1..]),
            "cd" => self.builtin_cd(&command[1..]),
            "soundlist" => self.builtin_soundlist(),
            _ => Ok(()),
        }
    }
}
