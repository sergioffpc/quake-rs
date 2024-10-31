use std::{collections::VecDeque, io::SeekFrom};

use anyhow::bail;
use byteorder::{LittleEndian, ReadBytesExt};
use legion::system;
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::FromPrimitive;

use crate::{console::Console, ReadSeek, ResourceFiles};

#[derive(Clone, Debug)]
pub enum Message {
    Server(ServerMessage),
    Client(ClientMessage),
    Block {
        angles: [f32; 3],
        messages: Box<[ServerMessage]>,
    },
}

#[derive(Clone, Copy, Debug, FromPrimitive, ToPrimitive)]
pub enum ServerMessageId {
    Bad = 0x00,
    Nop = 0x01,
    Disconnect = 0x02,
    UpdateStat = 0x03,
    Version = 0x04,
    SetView = 0x05,
    Sound = 0x06,
    Time = 0x07,
    Print = 0x08,
    StuffText = 0x09,
    SetAngle = 0x0a,
    ServerInfo = 0x0b,
    LightStyle = 0x0c,
    UpdateName = 0x0d,
    UpdateFrags = 0x0e,
    PlayerData = 0x0f,
    StopSound = 0x10,
    UpdateColors = 0x11,
    Particle = 0x12,
    Damage = 0x13,
    SpawnStatic = 0x14,
    SpawnBaseline = 0x16,
    TempEntity = 0x17,
    SetPause = 0x18,
    SignOnStage = 0x19,
    CenterPrint = 0x1a,
    KilledMonster = 0x1b,
    FoundSecret = 0x1c,
    SpawnStaticSound = 0x1d,
    Intermission = 0x1e,
    Finale = 0x1f,
    CdTrack = 0x20,
    SellScreen = 0x21,
}

#[derive(Clone, Debug)]
pub enum ServerMessage {
    // Something is bad. This message should never appear.
    Bad,
    // No operation.
    Nop,
    // Disconnect from the server. Stops the game.
    Disconnect,
    // Updates directly any values in the player state.
    UpdateStat {
        index: u8,
        value: i32,
        state: [i32; 32],
    },
    // The version of the server.
    Version {
        version: i32,
    },
    // Sets the camera position to the origin of this entity.
    SetView {
        entity: i16,
    },
    // This message starts the play of a sound at a specific point.
    Sound {
        mask: i32,
        volume: u8,
        attenuation: f32,
        channel: i8,
        entity_id: u16,
        sound_id: u8,
        origin: [f32; 3],
    },
    // This is the time stamp of a block of messages. A time message should appear in every block.
    Time {
        time: f32,
    },
    // The client prints the text in the top left corner of the screen. The text appears on the console as well.
    Print {
        text: &'static str,
    },
    // The client transfers the text to the console and runs it.
    StuffText {
        text: &'static str,
    },
    // This message set the camera orientation.
    SetAngle {
        angles: [f32; 3],
    },
    // This message is usually one of the first messages after a level start. It loads model and sound files.
    ServerInfo {
        serverversion: u32,
        maxclients: u32,
        multi: u32,
        mapname: &'static str,
        precache_models: Box<[&'static str]>,
        nummodels: u32,
        precache_sounds: Box<[&'static str]>,
        numsounds: u32,
    },
    // This message defines a light style.
    LightStyle {
        style: u32,
        lightmap: &'static str,
    },
    // This message sets the player name.
    UpdateName {
        player: u32,
        netname: &'static str,
    },
    // This message updates the frag count of a specific player.
    UpdateFrags {
        player: u8,
        frags: u16,
    },
    // This message updates the status line and the camera coordinates.
    PlayerData {
        mask: u32,
        view_ofs_z: f32,
        ang_ofs_1: f32,
        angles: [f32; 3],
        vel: [f32; 3],
        items: u32,
        weaponframe: u32,
        armorvalue: u32,
        weaponmodel: u32,
        health: u32,
        currentammo: u32,
        ammo_shells: u32,
        ammo_nails: u32,
        ammo_rockets: u32,
        ammo_cells: u32,
        weapon: u32,
    },
    // Stops a sound.
    StopSound {
        uk_short: u32,
    },
    // Updates the colors of the specified player.
    UpdateColors {
        player: u32,
        colors: u32,
        shirtcolor: u32,
        pantscolor: u32,
    },
    // This starts particles flying around. This happens, if a barrel explodes or blood particles fly after being hit by an axe, shells or nails.
    Particle {
        origin: [f32; 3],
        vel: [f32; 3],
        color: u32,
        count: u32,
    },
    // Tells how severe was a hit and from which points it came.
    Damage {
        save: u32,
        take: u32,
        origin: [f32; 3],
    },
    // This message creates a static entity and sets the internal default values.
    SpawnStatic {
        static_entitycount: u32,
        default_modelindex: u32,
        default_frame: u32,
        default_colormap: u32,
        default_skin: u32,
        default_origin: [f32; 3],
        default_angles: [f32; 3],
    },
    // This message creates a dynamic entity and sets the internal default values.
    SpawnBaseline {
        entity: u32,
        default_modelindex: u32,
        default_frame: u32,
        default_colormap: u32,
        default_skin: u32,
        default_origin: [f32; 3],
        default_angles: [f32; 3],
    },
    // This message creates a temporary entity.
    TempEntity {
        entitytype: u32,
        entity: u32,
        origin: [f32; 3],
        trace_endpos: [f32; 3],
    },
    // Set the pause state. The time stands still but all entities get their update messages.
    SetPause {
        pausestate: u32,
    },
    // This message selects the client state.
    SignOnStage {
        signon: u32,
    },
    // Prints the specified text at the center of the screen.
    CenterPrint {
        text: &'static str,
    },
    // This message indicates the death of a monster.
    KilledMonster {
        killed_monsters: u32,
    },
    // This message receives a client, if the player enters a secret area. It comes usually with a print message.
    FoundSecret {
        found_secrets: u32,
    },
    // This message starts a static (ambient) sound not connected to an entity but to a position.
    SpawnStaticSound {
        origin: [f32; 3],
        soundnum: u32,
        vol: f32,
        attenuation: f32,
    },
    // Displays the level end screen. Depending on the multi command in the serverinfo message this is either the single player summary screen or the multi player ranking screen.
    Intermission,
    // Displays the episode end screen and some text.
    Finale {
        text: &'static str,
    },
    // This message selects the audio CD track numbers.
    CdTrack {
        fromtrack: u32,
        totrack: u32,
    },
    // Displays the help and sell screen.
    SellScreen,
    // This is the general entity update message. For every entity (potentially) in sight the server sends such a message. The message contains only the values, which changed since
    // the creation (or spawning) of the entity (with spawnstatic, spawnbaseline).
    Updateentity {
        mask: u32,
        entity: u32,
        modelindex: u32,
        frame: u32,
        colormap: u32,
        skin: u32,
        attack_state: u32,
        origin: [f32; 3],
        angles: [f32; 3],
        new: u32,
    },
}

impl ServerMessage {
    fn deserialize<R>(reader: &mut R) -> anyhow::Result<ServerMessage>
    where
        R: ReadSeek,
    {
        let code = reader.read_u8()?;
        let msg_id = match ServerMessageId::from_u8(code) {
            Some(msg_id) => msg_id,
            None => bail!("invalid message id: {}", code),
        };

        let msg = match msg_id {
            ServerMessageId::Bad => ServerMessage::Bad,
            ServerMessageId::Nop => ServerMessage::Nop,
            ServerMessageId::Disconnect => ServerMessage::Disconnect,
            ServerMessageId::UpdateStat => todo!(),
            ServerMessageId::Version => {
                let version = reader.read_i32::<LittleEndian>()?;
                ServerMessage::Version { version }
            }
            ServerMessageId::SetView => todo!(),
            ServerMessageId::Sound => todo!(),
            ServerMessageId::Time => {
                let time = reader.read_f32::<LittleEndian>()?;
                ServerMessage::Time { time }
            }
            ServerMessageId::Print => todo!(),
            ServerMessageId::StuffText => todo!(),
            ServerMessageId::SetAngle => todo!(),
            ServerMessageId::ServerInfo => todo!(),
            ServerMessageId::LightStyle => todo!(),
            ServerMessageId::UpdateName => todo!(),
            ServerMessageId::UpdateFrags => todo!(),
            ServerMessageId::PlayerData => todo!(),
            ServerMessageId::StopSound => todo!(),
            ServerMessageId::UpdateColors => todo!(),
            ServerMessageId::Particle => todo!(),
            ServerMessageId::Damage => todo!(),
            ServerMessageId::SpawnStatic => todo!(),
            ServerMessageId::SpawnBaseline => todo!(),
            ServerMessageId::TempEntity => todo!(),
            ServerMessageId::SetPause => todo!(),
            ServerMessageId::SignOnStage => todo!(),
            ServerMessageId::CenterPrint => todo!(),
            ServerMessageId::KilledMonster => todo!(),
            ServerMessageId::FoundSecret => todo!(),
            ServerMessageId::SpawnStaticSound => todo!(),
            ServerMessageId::Intermission => todo!(),
            ServerMessageId::Finale => todo!(),
            ServerMessageId::CdTrack => todo!(),
            ServerMessageId::SellScreen => todo!(),
        };

        Ok(msg)
    }
}

#[derive(Clone, Copy, Debug, FromPrimitive, ToPrimitive)]
enum ClientMessageId {
    Bad = 0x00,
    Nop = 0x01,
    Disconnect = 0x02,
    Move = 0x03,
    StringCmd = 0x04,
}

#[derive(Clone, Debug)]
pub enum ClientMessage {
    // Something is bad. This message should never appear.
    Bad,
    // No operation.
    Nop,
    // Disconnect from the server. Stops the game.
    Disconnect,
    Move {
        send_time: f32,
        angles: [f32; 3],
        fwd_move: i16,
        side_move: i16,
        up_move: i16,
        button_flags: u32,
        impulse: u8,
    },
    StringCmd {
        cmd: &'static str,
    },
}

pub trait MessageStream: Send + Sync {
    fn next(&mut self) -> anyhow::Result<Message>;
}

struct FileMessageStream<R> {
    reader: R,
}

impl<R> FileMessageStream<R>
where
    R: ReadSeek,
{
    fn new(reader: R) -> Self {
        Self { reader }
    }

    fn reset(&mut self) -> anyhow::Result<()> {
        self.reader.seek(SeekFrom::Start(0))?;

        Ok(())
    }
}

impl<R> MessageStream for FileMessageStream<R>
where
    R: ReadSeek,
{
    fn next(&mut self) -> anyhow::Result<Message> {
        let block_length = self.reader.read_i32::<LittleEndian>()?;
        let angles = [
            self.reader.read_f32::<LittleEndian>()?,
            self.reader.read_f32::<LittleEndian>()?,
            self.reader.read_f32::<LittleEndian>()?,
        ];
        let messages = Box::new(
            [0..block_length].map(|_| ServerMessage::deserialize(&mut self.reader).unwrap()),
        );

        Ok(Message::Block { angles, messages })
    }
}

struct QueueMessageStream<R> {
    readers: VecDeque<FileMessageStream<R>>,
    reader_index: usize,
}

impl<R> QueueMessageStream<R>
where
    R: ReadSeek,
{
    fn new(readers: VecDeque<FileMessageStream<R>>) -> Self {
        Self {
            readers,
            reader_index: 0usize,
        }
    }
}

impl<R> MessageStream for QueueMessageStream<R>
where
    R: ReadSeek,
{
    fn next(&mut self) -> anyhow::Result<Message> {
        let i = self.reader_index % self.readers.len();
        let reader = &mut self.readers[i];
        match reader.next() {
            Ok(message) => Ok(message),
            Err(_) => {
                reader.reset()?;

                self.reader_index += 1;
                let i = self.reader_index % self.readers.len();
                self.readers[i].next()
            }
        }
    }
}

pub enum MessageSource {
    Local(Box<dyn MessageStream>),
    Network(Box<dyn MessageStream>),
}

#[system]
pub fn message_handler(#[resource] message_stream: &mut Option<MessageSource>) {
    if let Some(source) = message_stream {
        let message = match source {
            MessageSource::Local(message_stream) => message_stream.next().unwrap(),
            MessageSource::Network(message_stream) => todo!(),
        };
    }
}

#[system]
pub fn message_command_executor(
    #[resource] message_stream: &mut Option<MessageSource>,
    #[resource] console: &mut Console,
    #[resource] resource_files: &mut ResourceFiles,
) {
    console.commands().for_each(|command| match &command[..] {
        // Play a demo.
        [ref cmd, file_path] if cmd == "playdemo" => {
            let reader = resource_files.take(file_path).unwrap();
            let file_stream = FileMessageStream::new(reader);
            *message_stream = Some(MessageSource::Local(Box::new(file_stream)));
        }
        // Stops the current playback of demos.
        [ref cmd] if cmd == "stopdemo" => {
            if let Some(MessageSource::Local(_)) = message_stream {
                *message_stream = None;
            }
        }
        // Setup a queue of demos to loop.
        [ref cmd, file_paths @ ..] if cmd == "startdemos" => {
            let queue = file_paths
                .iter()
                .map(|file_path| {
                    let reader = resource_files
                        .take(format!("{}.dem", file_path).as_str())
                        .unwrap();

                    FileMessageStream::new(reader)
                })
                .collect();
            let queue_stream = QueueMessageStream::new(queue);
            *message_stream = Some(MessageSource::Local(Box::new(queue_stream)));
        }
        _ => (),
    });
}
