use std::{
    io::{Read, Seek},
    path::PathBuf,
    str::FromStr,
};

use legion::system;
use rodio::{Decoder, OutputStreamHandle, Sink, Source};

use crate::{
    console::{Console, ConsoleCmd},
    ResourceFiles,
};

pub struct Audio {
    channels: Box<[Sink]>,
}

impl Audio {
    pub fn new(output_stream_handle: OutputStreamHandle) -> anyhow::Result<Self> {
        let mut channels = Vec::new();
        for _ in 0..32 {
            let channel = Sink::try_new(&output_stream_handle)?;
            channels.push(channel);
        }

        Ok(Self {
            channels: channels.into_boxed_slice(),
        })
    }

    pub fn play_channel<R>(&self, channel: usize, data: R) -> anyhow::Result<()>
    where
        R: Read + Seek + Send + Sync + 'static,
    {
        let source = Decoder::new(data)?;

        self.channels[channel].clear();
        self.channels[channel].append(source);
        self.channels[channel].play();

        Ok(())
    }

    pub fn loop_channel<R>(&self, channel: usize, data: R) -> anyhow::Result<()>
    where
        R: Read + Seek + Send + Sync + 'static,
    {
        let source = Decoder::new(data)?.repeat_infinite();

        self.channels[channel].clear();
        self.channels[channel].append(source);
        self.channels[channel].play();

        Ok(())
    }

    pub fn pause_channel(&self, channel: usize) {
        self.channels[channel].pause();
    }

    pub fn resume_channel(&self, channel: usize) {
        self.channels[channel].play();
    }

    pub fn stop_channel(&self, channel: usize) {
        self.channels[channel].stop();
    }

    fn execute_command(&mut self, command: &ConsoleCmd, resource_files: &mut ResourceFiles) {
        match &command[..] {
            // Plays the specified track one time.
            [ref cmd, ref action, track_number] if cmd == "cd" && action == "play" => {
                let audio_path = PathBuf::from_str(
                    format!(
                        "music/track{:02}.ogg",
                        track_number.parse::<i32>().unwrap_or(0)
                    )
                    .as_str(),
                )
                .unwrap();
                let data = resource_files.take(audio_path).unwrap();
                self.play_channel(0, data).unwrap();
            }
            // Plays the specified track.  It will be repeated until either it is manually stopped or another track is started.
            [ref cmd, ref action, track_number] if cmd == "cd" && action == "loop" => {
                let audio_path = PathBuf::from_str(
                    format!(
                        "music/track{:02}.ogg",
                        track_number.parse::<i32>().unwrap_or(0)
                    )
                    .as_str(),
                )
                .unwrap();
                let data = resource_files.take(audio_path).unwrap();
                self.loop_channel(0, data).unwrap();
            }
            // Stops the currently playing track.
            [ref cmd, ref action] if cmd == "cd" && action == "stop" => {
                self.pause_channel(0);
            }
            // Will resume playback of a stopped track.
            [ref cmd, ref action] if cmd == "cd" && action == "resume" => {
                self.resume_channel(0);
            }
            // Play a sound effect.
            [ref cmd, file_path] if cmd == "play" => {
                for channel in 1..self.channels.len() {
                    if self.channels[channel].empty() {
                        let data = resource_files.take(file_path).unwrap();
                        self.play_channel(channel, data).unwrap();
                        break;
                    }
                }
            }
            // Stops all sounds currently being played.
            [ref cmd] if cmd == "stopsound" => {
                for channel in 1..self.channels.len() {
                    self.stop_channel(channel);
                }
            }
            _ => (),
        }
    }
}

#[system]
pub fn audio_command_executor(
    #[resource] audio: &mut Audio,
    #[resource] console: &mut Console,
    #[resource] resource_files: &mut ResourceFiles,
) {
    console
        .commands()
        .for_each(|command| audio.execute_command(command, resource_files));
}
