use std::cell::RefCell;
use std::rc::Rc;

mod builtins;

pub struct AudioManager {
    manager: Rc<RefCell<kira::AudioManager>>,
    channel: Rc<RefCell<Option<kira::sound::static_sound::StaticSoundHandle>>>,
}

impl AudioManager {
    pub fn new(
        console: &mut quake_console::Console,
        resources: Rc<quake_resources::Resources>,
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

        Ok(Self { manager, channel })
    }
}
