use kira::sound::static_sound::StaticSoundData;
use quake_console::ControlFlow;
use std::cell::RefCell;
use std::io::Cursor;
use std::rc::Rc;

pub fn play(
    manager: Rc<RefCell<kira::AudioManager>>,
    resources: Rc<quake_resources::Resources>,
) -> quake_console::command::Command {
    Box::new(move |_, args| {
        let data = resources.by_name::<Vec<u8>>(args[0]).unwrap();
        manager
            .borrow_mut()
            .play(StaticSoundData::from_cursor(Cursor::new(data)).unwrap())
            .unwrap();

        ControlFlow::Poll
    })
}

pub fn cd(
    manager: Rc<RefCell<kira::AudioManager>>,
    channel: Rc<RefCell<Option<kira::sound::static_sound::StaticSoundHandle>>>,
    resources: Rc<quake_resources::Resources>,
) -> quake_console::command::Command {
    Box::new(move |_, args| {
        match args[0] {
            "play" => {
                let track_name = format!("music/track{:02}.ogg", args[1].parse::<u32>().unwrap());
                let data = resources.by_name::<Vec<u8>>(&track_name).unwrap();
                let sound_data = StaticSoundData::from_cursor(Cursor::new(data)).unwrap();

                if let Some(channel) = channel.borrow_mut().as_mut() {
                    channel.stop(kira::Tween::default())
                }
                *channel.borrow_mut() = Some(manager.borrow_mut().play(sound_data).unwrap());
            }
            "loop" => {
                let track_name = format!("music/track{:02}.ogg", args[1].parse::<u32>().unwrap());
                let data = resources.by_name::<Vec<u8>>(&track_name).unwrap();
                let sound_data = StaticSoundData::from_cursor(Cursor::new(data))
                    .unwrap()
                    .with_settings(
                        kira::sound::static_sound::StaticSoundSettings::default().loop_region(..),
                    );

                if let Some(channel) = channel.borrow_mut().as_mut() {
                    channel.stop(kira::Tween::default())
                }
                *channel.borrow_mut() = Some(manager.borrow_mut().play(sound_data).unwrap());
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
