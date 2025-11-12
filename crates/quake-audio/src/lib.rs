use kira::sound::static_sound::StaticSoundData;
use std::cell::RefCell;
use std::io::Cursor;
use std::rc::Rc;

mod builtins;

pub struct AudioManager {
    manager: Rc<RefCell<kira::AudioManager>>,
    channel: Rc<RefCell<Option<kira::sound::static_sound::StaticSoundHandle>>>,
}

impl AudioManager {
    pub fn new(
        console: &mut quake_console::Console,
        resources: Rc<RefCell<quake_resources::Resources>>,
    ) -> anyhow::Result<Self> {
        let manager = Rc::new(RefCell::new(
            kira::AudioManager::<kira::DefaultBackend>::new(kira::AudioManagerSettings::default())?,
        ));
        let channel = Rc::new(RefCell::new(None));

        console.register_command("play", builtins::play(manager.clone(), resources.clone()));
        console.register_command(
            "cd",
            builtins::cd(manager.clone(), channel.clone(), resources.clone()),
        );
        console.register_command("soundlist", builtins::soundlist(resources.clone()));

        Ok(Self { manager, channel })
    }
}

struct Snd {
    data: StaticSoundData,
}

impl quake_resources::FromBytes for Snd {
    fn from_bytes(data: &[u8]) -> anyhow::Result<Self> {
        let data = StaticSoundData::from_cursor(Cursor::new(data.to_vec()))?;

        Ok(Self { data })
    }
}
