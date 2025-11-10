use kira::sound::static_sound::StaticSoundData;
use quake_console::ControlFlow;
use std::cell::RefCell;
use std::io::Cursor;
use std::rc::Rc;

struct Sound {
    data: StaticSoundData,
}

impl quake_resources::FromBytes for Sound {
    fn from_bytes(data: &[u8]) -> anyhow::Result<Self> {
        let data = StaticSoundData::from_cursor(Cursor::new(data.to_vec()))?;

        Ok(Self { data })
    }
}

pub fn play(
    manager: Rc<RefCell<kira::AudioManager>>,
    resources: Rc<RefCell<quake_resources::Resources>>,
) -> quake_console::command::Command {
    Box::new(move |_, args| {
        let sound = resources
            .borrow_mut()
            .by_cached_name::<Sound>(args[0])
            .unwrap();
        manager.borrow_mut().play(sound.data.clone()).unwrap();

        ControlFlow::Poll
    })
}

pub fn cd(
    manager: Rc<RefCell<kira::AudioManager>>,
    channel: Rc<RefCell<Option<kira::sound::static_sound::StaticSoundHandle>>>,
    resources: Rc<RefCell<quake_resources::Resources>>,
) -> quake_console::command::Command {
    Box::new(move |_, args| {
        match args[0] {
            "play" => {
                let track_name = format!("music/track{:02}.ogg", args[1].parse::<u32>().unwrap());
                let sound = resources
                    .borrow_mut()
                    .by_cached_name::<Sound>(&track_name)
                    .unwrap();

                if let Some(channel) = channel.borrow_mut().as_mut() {
                    channel.stop(kira::Tween::default())
                }
                *channel.borrow_mut() =
                    Some(manager.borrow_mut().play(sound.data.clone()).unwrap());
            }
            "loop" => {
                let track_name = format!("music/track{:02}.ogg", args[1].parse::<u32>().unwrap());
                let sound = resources
                    .borrow_mut()
                    .by_cached_name::<Sound>(&track_name)
                    .unwrap();

                if let Some(channel) = channel.borrow_mut().as_mut() {
                    channel.stop(kira::Tween::default())
                }
                *channel.borrow_mut() = Some(
                    manager
                        .borrow_mut()
                        .play(
                            sound.data.with_settings(
                                kira::sound::static_sound::StaticSoundSettings::default()
                                    .loop_region(..),
                            ),
                        )
                        .unwrap(),
                );
            }
            "stop" => {
                if let Some(channel) = channel.borrow_mut().as_mut() {
                    channel.pause(kira::Tween::default())
                }
            }
            "resume" => {
                if let Some(channel) = channel.borrow_mut().as_mut() {
                    channel.resume(kira::Tween::default())
                }
            }
            _ => (),
        }

        ControlFlow::Poll
    })
}

pub fn soundlist(
    resources: Rc<RefCell<quake_resources::Resources>>,
) -> quake_console::command::Command {
    Box::new(move |ctx, _| {
        resources
            .borrow()
            .cached_names()
            .for_each(|name| writeln!(ctx.writer, "{}", name).unwrap());
        ControlFlow::Poll
    })
}
