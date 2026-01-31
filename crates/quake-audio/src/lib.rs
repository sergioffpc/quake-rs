use kira::Tween;
use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use tracing::debug;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SoundId(usize);

impl From<usize> for SoundId {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl From<SoundId> for usize {
    fn from(value: SoundId) -> Self {
        value.0
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AudioEvent {
    Load {
        precache_sounds: Box<[PathBuf]>,
    },
    Unload,
    Play {
        handle_index: usize,
        volume: f32,
        attenuation: f32,
        sound_id: SoundId,
        position: glam::Vec3,
    },
    Stop {
        handle_index: usize,
    },
}

pub struct AudioManager {
    manager: kira::AudioManager<kira::DefaultBackend>,
    handles: HashMap<usize, StaticSoundHandle>,

    sender: std::sync::mpsc::Sender<AudioEvent>,
    receiver: std::sync::mpsc::Receiver<AudioEvent>,

    asset_manager: Rc<quake_asset::AssetManager>,
    precache: Vec<StaticSoundData>,
}

impl AudioManager {
    pub fn new(asset_manager: Rc<quake_asset::AssetManager>) -> anyhow::Result<Self> {
        let manager =
            kira::AudioManager::<kira::DefaultBackend>::new(kira::AudioManagerSettings::default())?;
        let (sender, receiver) = std::sync::mpsc::channel::<AudioEvent>();

        Ok(Self {
            manager,
            handles: HashMap::default(),

            sender,
            receiver,

            asset_manager,
            precache: Vec::default(),
        })
    }

    pub fn sender(&self) -> std::sync::mpsc::Sender<AudioEvent> {
        self.sender.clone()
    }

    pub fn step(&mut self) -> anyhow::Result<()> {
        while let Some(sound_event) = self.receiver.try_recv().ok() {
            debug!(?sound_event, "audio event");

            match sound_event {
                AudioEvent::Load { precache_sounds } => {
                    for sound_path in precache_sounds.iter() {
                        self.preload(sound_path)?;
                    }
                }
                AudioEvent::Unload => {
                    self.precache.clear();
                }
                AudioEvent::Play { sound_id, .. } => {
                    self.play(usize::from(sound_id))?;
                }
                AudioEvent::Stop { handle_index } => {
                    self.stop(handle_index);
                }
            }
        }

        Ok(())
    }

    fn preload<P>(&mut self, sound_path: P) -> anyhow::Result<()>
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

    fn play(&mut self, sound_index: usize) -> anyhow::Result<()> {
        let sound = self
            .precache
            .get(sound_index)
            .ok_or_else(|| anyhow::anyhow!("sound ID not found: {}", sound_index))?;
        let handle = self.manager.play(sound.clone())?;
        self.handles.insert(sound_index, handle);
        Ok(())
    }

    fn stop(&mut self, handle_index: usize) {
        if let Some(handle) = self.handles.get_mut(&handle_index) {
            handle.stop(Tween::default());
            self.handles.remove(&handle_index);
        }
    }
}
