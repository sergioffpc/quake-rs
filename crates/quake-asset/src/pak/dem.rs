use crate::FromBytes;
use crate::pak::read_vec3;
use byteorder::{LittleEndian, ReadBytesExt};
use glam::Vec3;
use num_traits::FromPrimitive;
use std::{
    io::{Cursor, Read},
    path::PathBuf,
};

pub struct Dem {
    blocks: Box<[Block]>,
}

impl Dem {
    pub fn from_slice(data: &[u8]) -> anyhow::Result<Self> {
        let mut reader = Cursor::new(data);

        let _track = read_cstring(&mut reader, b'\n')?;

        let mut blocks = Vec::new();
        loop {
            match reader.read_u32::<LittleEndian>() {
                Ok(block_size) if block_size > 0 => {
                    let angles = read_vec3(&mut reader)?;
                    let mut bytes = vec![0u8; block_size as usize];
                    reader.read_exact(&mut bytes)?;

                    blocks.push(Block {
                        angles,
                        bytes: bytes.into_boxed_slice(),
                    });
                }
                _ => break,
            }
        }

        Ok(Self {
            blocks: blocks.into_boxed_slice(),
        })
    }

    pub fn iter(&self) -> impl Iterator<Item = DemEvent> + '_ {
        self.blocks
            .iter()
            .flat_map(|block| {
                let set_angle_event = std::iter::once(DemEvent::SetAngle {
                    angles: block.angles,
                });
                set_angle_event.chain(block.iter())
            })
            .skip(1)
    }
}

impl FromBytes for Dem {
    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        Dem::from_slice(bytes)
    }
}

#[derive(Clone, Debug)]
struct Block {
    pub angles: Vec3,
    pub bytes: Box<[u8]>,
}

impl Block {
    pub fn iter(&self) -> impl Iterator<Item = DemEvent> {
        EventIterator {
            cursor: Cursor::new(&self.bytes[..]),
        }
    }
}

#[derive(Clone, Debug)]
struct EventIterator<'a> {
    cursor: Cursor<&'a [u8]>,
}

impl<'a> Iterator for EventIterator<'a> {
    type Item = DemEvent;

    fn next(&mut self) -> Option<Self::Item> {
        let opcode = match self.cursor.read_u8() {
            Ok(opcode) => opcode,
            Err(_) => return None,
        };

        if opcode & 0x80 != 0 {
            let opcode_u16 = u16::from(opcode & 0x7F);
            let base_bits = opcode_u16;

            let flags_bits = if (base_bits & UpdateFlags::U_MOREBITS.bits()) != 0 {
                let high_order_bits = u16::from(self.cursor.read_u8().unwrap());
                base_bits | (high_order_bits << 8)
            } else {
                base_bits
            };

            let update_flags = UpdateFlags::from_bits(flags_bits).unwrap();

            let entity_id = update_flags
                .contains(UpdateFlags::U_LONGENTITY)
                .then(|| self.cursor.read_u16::<LittleEndian>().unwrap())
                .unwrap_or_else(|| self.cursor.read_u8().unwrap() as u16)
                as usize;

            let model_id = update_flags
                .contains(UpdateFlags::U_MODEL)
                .then(|| self.cursor.read_u8().unwrap() as usize);

            let frame_id = update_flags
                .contains(UpdateFlags::U_FRAME)
                .then(|| self.cursor.read_u8().unwrap() as usize);

            let colormap = update_flags
                .contains(UpdateFlags::U_COLORMAP)
                .then(|| self.cursor.read_u8().unwrap());

            let skin_id = update_flags
                .contains(UpdateFlags::U_SKIN)
                .then(|| self.cursor.read_u8().unwrap() as usize);

            let effects = update_flags
                .contains(UpdateFlags::U_EFFECTS)
                .then(|| EffectsFlags::from_bits(self.cursor.read_u8().unwrap()).unwrap());

            let origin1 = update_flags
                .contains(UpdateFlags::U_ORIGIN1)
                .then(|| read_coord(&mut self.cursor).unwrap());
            let angle1 = update_flags
                .contains(UpdateFlags::U_ANGLE1)
                .then(|| read_angle(&mut self.cursor).unwrap());

            let origin2 = update_flags
                .contains(UpdateFlags::U_ORIGIN2)
                .then(|| read_coord(&mut self.cursor).unwrap());
            let angle2 = update_flags
                .contains(UpdateFlags::U_ANGLE2)
                .then(|| read_angle(&mut self.cursor).unwrap());

            let origin3 = update_flags
                .contains(UpdateFlags::U_ORIGIN3)
                .then(|| read_coord(&mut self.cursor).unwrap());
            let angle3 = update_flags
                .contains(UpdateFlags::U_ANGLE3)
                .then(|| read_angle(&mut self.cursor).unwrap());

            let no_lerp = update_flags.contains(UpdateFlags::U_NOLERP);

            return Some(DemEvent::UpdateEntity {
                entity_id,
                model_id,
                frame_id,
                colormap,
                skin_id,
                effects,
                origin1,
                origin2,
                origin3,
                angle1,
                angle2,
                angle3,
                no_lerp,
            });
        }

        match opcode {
            0x01 => Some(DemEvent::Nop),
            0x02 => Some(DemEvent::Disconnect),
            0x03 => Some(DemEvent::UpdateStat {
                stat: ClientStat::from_u8(self.cursor.read_u8().unwrap()).unwrap(),
                value: self.cursor.read_i32::<LittleEndian>().unwrap(),
            }),
            0x04 => {
                let version = self.cursor.read_i32::<LittleEndian>().unwrap();
                if version != 15 {
                    panic!("Unsupported DEM version: {}", version);
                }

                Some(DemEvent::Nop)
            }
            0x05 => Some(DemEvent::SetView {
                entity_id: self.cursor.read_u16::<LittleEndian>().unwrap() as usize,
            }),
            0x06 => {
                let field_mask = SoundFlags::from_bits(self.cursor.read_u8().unwrap()).unwrap();

                let volume = field_mask
                    .contains(SoundFlags::SND_VOLUME)
                    .then(|| self.cursor.read_u8().unwrap())
                    .unwrap_or(255) as f32
                    / 255.0;

                let attenuation = field_mask
                    .contains(SoundFlags::SND_ATTENUATION)
                    .then(|| self.cursor.read_u8().unwrap() as f32 / 64.0)
                    .unwrap_or(1.0);

                let value = self.cursor.read_u16::<LittleEndian>().unwrap();
                let entity_id = (value >> 3) as usize;
                let channel = value & 0b111;

                let sound_id = self.cursor.read_u8().unwrap() as usize;
                let origin = read_xyz_coords(&mut self.cursor).unwrap();

                Some(DemEvent::PlaySound {
                    volume,
                    attenuation,
                    entity_id,
                    channel,
                    sound_id,
                    origin,
                })
            }
            0x07 => Some(DemEvent::Time {
                time: self.cursor.read_f32::<LittleEndian>().unwrap(),
            }),
            0x08 => {
                let text = read_cstring(&mut self.cursor, 0).unwrap();
                Some(DemEvent::Print { text })
            }
            0x09 => {
                let text = read_cstring(&mut self.cursor, 0).unwrap();
                Some(DemEvent::StuffText { text })
            }
            0x0A => {
                let angles = read_xyz_angles(&mut self.cursor).unwrap();
                Some(DemEvent::SetAngle { angles })
            }
            0x0B => {
                let version = self.cursor.read_u32::<LittleEndian>().unwrap();
                if version != 15 {
                    panic!("Unsupported DEM version: {}", version);
                }

                let _max_clients = self.cursor.read_u8().unwrap();
                let _game_type = self.cursor.read_u8().unwrap();
                let _map_name = read_cstring(&mut self.cursor, 0).unwrap();
                let map_path = read_cstring(&mut self.cursor, 0).unwrap().into();

                let mut model_precache = Vec::new();
                loop {
                    let model_path = read_cstring(&mut self.cursor, 0).unwrap();
                    if model_path.is_empty() {
                        break;
                    }
                    model_precache.push(model_path.into());
                }

                let mut sound_precache = Vec::new();
                loop {
                    let sound_path = read_cstring(&mut self.cursor, 0).unwrap();
                    if sound_path.is_empty() {
                        break;
                    }
                    sound_precache.push(format!("sound/{}", sound_path).into());
                }

                Some(DemEvent::ServerInfo {
                    map_path,
                    precache_models: model_precache.into_boxed_slice(),
                    precache_sounds: sound_precache.into_boxed_slice(),
                })
            }
            0x0C => {
                let light_style = self.cursor.read_u8().unwrap();
                let light_map = read_cstring(&mut self.cursor, 0).unwrap().bytes().collect();

                Some(DemEvent::LightStyle {
                    light_style,
                    light_map,
                })
            }
            0x0D => {
                let player_id = self.cursor.read_u8().unwrap() as usize;
                let name = read_cstring(&mut self.cursor, 0).unwrap();

                Some(DemEvent::UpdateName { player_id, name })
            }
            0x0E => Some(DemEvent::UpdateFrags {
                player_id: self.cursor.read_u8().unwrap() as usize,
                frags: self.cursor.read_i16::<LittleEndian>().unwrap(),
            }),
            0x0F => {
                let client_flags =
                    ClientFlags::from_bits(self.cursor.read_u16::<LittleEndian>().unwrap())
                        .unwrap();

                let view_height = client_flags
                    .contains(ClientFlags::SU_VIEWHEIGHT)
                    .then(|| self.cursor.read_u8().unwrap())
                    .unwrap_or(22);

                let ideal_pitch = client_flags
                    .contains(ClientFlags::SU_IDEALPITCH)
                    .then(|| self.cursor.read_i8().unwrap() as f32)
                    .unwrap_or(0.0);

                let punch1 = client_flags
                    .contains(ClientFlags::SU_PUNCH1)
                    .then(|| self.cursor.read_i8().unwrap() as f32)
                    .unwrap_or(0.0);

                let velocity1 = client_flags
                    .contains(ClientFlags::SU_VELOCITY1)
                    .then(|| read_velocity(&mut self.cursor).unwrap())
                    .unwrap_or(0.0);

                let punch2 = client_flags
                    .contains(ClientFlags::SU_PUNCH2)
                    .then(|| self.cursor.read_i8().unwrap() as f32)
                    .unwrap_or(0.0);

                let velocity2 = client_flags
                    .contains(ClientFlags::SU_VELOCITY2)
                    .then(|| read_velocity(&mut self.cursor).unwrap())
                    .unwrap_or(0.0);

                let punch3 = client_flags
                    .contains(ClientFlags::SU_PUNCH3)
                    .then(|| self.cursor.read_i8().unwrap() as f32)
                    .unwrap_or(0.0);

                let velocity3 = client_flags
                    .contains(ClientFlags::SU_VELOCITY3)
                    .then(|| read_velocity(&mut self.cursor).unwrap())
                    .unwrap_or(0.0);

                // Swaps Y↔Z axes to convert from Quake's coordinate system to standard 3D
                let punch_angles = Vec3::new(punch1, punch3, -punch2);
                let velocity = Vec3::new(velocity1, velocity3, -velocity2);

                let item_flags =
                    ItemFlags::from_bits(self.cursor.read_u32::<LittleEndian>().unwrap()).unwrap();

                let weapon_frame = client_flags
                    .contains(ClientFlags::SU_WEAPONFRAME)
                    .then(|| self.cursor.read_u8().unwrap())
                    .unwrap_or(0);

                let armor = client_flags
                    .contains(ClientFlags::SU_ARMOR)
                    .then(|| self.cursor.read_u8().unwrap())
                    .unwrap_or(0);

                let weapon = client_flags
                    .contains(ClientFlags::SU_WEAPON)
                    .then(|| self.cursor.read_u8().unwrap())
                    .unwrap_or(0);

                let health = self.cursor.read_u16::<LittleEndian>().unwrap();
                let ammo = self.cursor.read_u8().unwrap();
                let shells = self.cursor.read_u8().unwrap();
                let nails = self.cursor.read_u8().unwrap();
                let rockets = self.cursor.read_u8().unwrap();
                let cells = self.cursor.read_u8().unwrap();
                let active_weapon = self.cursor.read_u8().unwrap();

                Some(DemEvent::ClientData {
                    client_flags,
                    view_height,
                    ideal_pitch,
                    punch_angles,
                    velocity,
                    item_flags,
                    weapon_frame,
                    armor,
                    weapon,
                    health,
                    ammo,
                    shells,
                    nails,
                    rockets,
                    cells,
                    active_weapon,
                })
            }
            0x10 => {
                let value = self.cursor.read_u16::<LittleEndian>().unwrap();
                let entity_id = (value >> 3) as usize;
                let channel = value & 0b111;

                Some(DemEvent::StopSound { entity_id, channel })
            }
            0x11 => {
                let player_id = self.cursor.read_u8().unwrap() as usize;

                let value = self.cursor.read_u8().unwrap();
                let shirt_color = value >> 4;
                let pants_color = value & 0b1111;

                Some(DemEvent::UpdateColors {
                    player_id,
                    shirt_color,
                    pants_color,
                })
            }
            0x12 => {
                let origin = read_xyz_coords(&mut self.cursor).unwrap();
                let velocity = read_xyz_velocity(&mut self.cursor).unwrap();
                let count = self.cursor.read_u8().unwrap();
                let color = self.cursor.read_u8().unwrap();

                Some(DemEvent::Particle {
                    origin,
                    velocity,
                    count,
                    color,
                })
            }
            0x13 => {
                let armor = self.cursor.read_u8().unwrap();
                let blood = self.cursor.read_u8().unwrap();
                let origin = read_xyz_coords(&mut self.cursor).unwrap();

                Some(DemEvent::Damage {
                    armor,
                    blood,
                    origin,
                })
            }
            0x14 => {
                let model_id = self.cursor.read_u8().unwrap() as usize;
                let frame_id = self.cursor.read_u8().unwrap() as usize;
                let colormap = self.cursor.read_u8().unwrap();
                let skin_id = self.cursor.read_u8().unwrap() as usize;

                let mut origin = Vec3::ZERO;
                let mut angles = Vec3::ZERO;
                for i in 0..3 {
                    origin[i] = read_coord(&mut self.cursor).unwrap();
                    angles[i] = read_angle(&mut self.cursor).unwrap();
                }

                Some(DemEvent::SpawnStatic {
                    model_id,
                    frame_id,
                    colormap,
                    skin_id,
                    origin,
                    angles,
                })
            }
            0x16 => {
                let entity_id = self.cursor.read_u16::<LittleEndian>().unwrap() as usize;
                let model_id = self.cursor.read_u8().unwrap() as usize;
                let frame_id = self.cursor.read_u8().unwrap() as usize;
                let colormap = self.cursor.read_u8().unwrap();
                let skin_id = self.cursor.read_u8().unwrap() as usize;

                let mut origin = Vec3::ZERO;
                let mut angles = Vec3::ZERO;
                for i in 0..3 {
                    origin[i] = read_coord(&mut self.cursor).unwrap();
                    angles[i] = read_angle(&mut self.cursor).unwrap();
                }

                Some(DemEvent::SpawnBaseline {
                    entity_id,
                    model_id,
                    frame_id,
                    colormap,
                    skin_id,
                    origin,
                    angles,
                })
            }
            0x17 => {
                let entity = match self.cursor.read_u8().unwrap() {
                    0 => TemporaryEntity::Spike {
                        origin: read_xyz_coords(&mut self.cursor).unwrap(),
                    },
                    1 => TemporaryEntity::SuperSpike {
                        origin: read_xyz_coords(&mut self.cursor).unwrap(),
                    },
                    2 => TemporaryEntity::Gunshot {
                        origin: read_xyz_coords(&mut self.cursor).unwrap(),
                    },
                    3 => TemporaryEntity::Explosion {
                        origin: read_xyz_coords(&mut self.cursor).unwrap(),
                    },
                    4 => TemporaryEntity::TarExplosion {
                        origin: read_xyz_coords(&mut self.cursor).unwrap(),
                    },
                    5 => TemporaryEntity::Lightning1 {
                        entity_id: self.cursor.read_u16::<LittleEndian>().unwrap(),
                        start: read_xyz_coords(&mut self.cursor).unwrap(),
                        end: read_xyz_coords(&mut self.cursor).unwrap(),
                    },
                    6 => TemporaryEntity::Lightning2 {
                        entity_id: self.cursor.read_u16::<LittleEndian>().unwrap(),
                        start: read_xyz_coords(&mut self.cursor).unwrap(),
                        end: read_xyz_coords(&mut self.cursor).unwrap(),
                    },
                    7 => TemporaryEntity::WizSpike {
                        origin: read_xyz_coords(&mut self.cursor).unwrap(),
                    },
                    8 => TemporaryEntity::KnightSpike {
                        origin: read_xyz_coords(&mut self.cursor).unwrap(),
                    },
                    9 => TemporaryEntity::Lightning3 {
                        entity_id: self.cursor.read_u16::<LittleEndian>().unwrap(),
                        start: read_xyz_coords(&mut self.cursor).unwrap(),
                        end: read_xyz_coords(&mut self.cursor).unwrap(),
                    },
                    10 => TemporaryEntity::LavaSplash {
                        origin: read_xyz_coords(&mut self.cursor).unwrap(),
                    },
                    11 => TemporaryEntity::Teleport {
                        origin: read_xyz_coords(&mut self.cursor).unwrap(),
                    },
                    12 => TemporaryEntity::Explosion2 {
                        origin: read_xyz_coords(&mut self.cursor).unwrap(),
                        count_start: self.cursor.read_u8().unwrap(),
                        count_length: self.cursor.read_u8().unwrap(),
                    },
                    13 => TemporaryEntity::Beam {
                        entity_id: self.cursor.read_u16::<LittleEndian>().unwrap(),
                        start: read_xyz_coords(&mut self.cursor).unwrap(),
                        end: read_xyz_coords(&mut self.cursor).unwrap(),
                    },
                    x => panic!("Unknown temporary entity type: {:#02x}", x),
                };

                Some(DemEvent::SpawnTemporary { entity })
            }
            0x18 => Some(DemEvent::SetPause {
                pause: self.cursor.read_u8().unwrap() != 0,
            }),
            0x19 => Some(DemEvent::SignOn {
                state: SignOnState::from_u8(self.cursor.read_u8().unwrap()).unwrap(),
            }),
            0x1A => {
                let text = read_cstring(&mut self.cursor, 0).unwrap();
                Some(DemEvent::CenterPrint { text })
            }
            0x1B => Some(DemEvent::KilledMonster),
            0x1C => Some(DemEvent::FoundSecret),
            0x1D => {
                let origin = read_xyz_coords(&mut self.cursor).unwrap();
                let sound_id = self.cursor.read_u8().unwrap() as usize;
                let volume = self.cursor.read_u8().unwrap();
                let attenuation = self.cursor.read_u8().unwrap();

                Some(DemEvent::SpawnStaticSound {
                    origin,
                    sound_id,
                    volume,
                    attenuation,
                })
            }
            0x1E => Some(DemEvent::Intermission),
            0x1F => {
                let text = read_cstring(&mut self.cursor, 0).unwrap();
                Some(DemEvent::Finale { text })
            }
            0x20 => Some(DemEvent::CdTrack {
                cd_track: self.cursor.read_u8().unwrap(),
                loop_track: self.cursor.read_u8().unwrap() != 0,
            }),
            0x21 => Some(DemEvent::SellScreen),
            0x22 => {
                let text = read_cstring(&mut self.cursor, 0).unwrap();
                Some(DemEvent::CutScene { text })
            }
            _ => panic!("Unknown opcode: {:#02x}", opcode),
        }
    }
}

#[derive(Clone, Debug)]
pub enum DemEvent {
    Nop,
    Disconnect,
    SetView {
        entity_id: usize,
    },
    PlaySound {
        volume: f32,
        attenuation: f32,
        entity_id: usize,
        channel: u16,
        sound_id: usize,
        origin: Vec3,
    },
    StopSound {
        entity_id: usize,
        channel: u16,
    },
    Time {
        time: f32,
    },
    Print {
        text: String,
    },
    StuffText {
        text: String,
    },
    SetAngle {
        angles: Vec3,
    },
    ServerInfo {
        map_path: PathBuf,
        precache_models: Box<[PathBuf]>,
        precache_sounds: Box<[PathBuf]>,
    },
    LightStyle {
        light_style: u8,
        light_map: Box<[u8]>,
    },
    UpdateStat {
        stat: ClientStat,
        value: i32,
    },
    UpdateName {
        player_id: usize,
        name: String,
    },
    UpdateFrags {
        player_id: usize,
        frags: i16,
    },
    UpdateColors {
        player_id: usize,
        shirt_color: u8,
        pants_color: u8,
    },
    UpdateEntity {
        entity_id: usize,
        model_id: Option<usize>,
        frame_id: Option<usize>,
        colormap: Option<u8>,
        skin_id: Option<usize>,
        effects: Option<EffectsFlags>,
        origin1: Option<f32>,
        origin2: Option<f32>,
        origin3: Option<f32>,
        angle1: Option<f32>,
        angle2: Option<f32>,
        angle3: Option<f32>,
        no_lerp: bool,
    },
    ClientData {
        client_flags: ClientFlags,
        view_height: u8,
        ideal_pitch: f32,
        punch_angles: Vec3,
        velocity: Vec3,
        item_flags: ItemFlags,
        weapon_frame: u8,
        armor: u8,
        weapon: u8,
        health: u16,
        ammo: u8,
        shells: u8,
        nails: u8,
        rockets: u8,
        cells: u8,
        active_weapon: u8,
    },
    Particle {
        origin: Vec3,
        velocity: Vec3,
        count: u8,
        color: u8,
    },
    Damage {
        armor: u8,
        blood: u8,
        origin: Vec3,
    },
    SpawnStatic {
        model_id: usize,
        frame_id: usize,
        colormap: u8,
        skin_id: usize,
        origin: Vec3,
        angles: Vec3,
    },
    SpawnBaseline {
        entity_id: usize,
        model_id: usize,
        frame_id: usize,
        colormap: u8,
        skin_id: usize,
        origin: Vec3,
        angles: Vec3,
    },
    SpawnTemporary {
        entity: TemporaryEntity,
    },
    SpawnStaticSound {
        origin: Vec3,
        sound_id: usize,
        volume: u8,
        attenuation: u8,
    },
    SetPause {
        pause: bool,
    },
    SignOn {
        state: SignOnState,
    },
    CenterPrint {
        text: String,
    },
    KilledMonster,
    FoundSecret,
    Intermission,
    Finale {
        text: String,
    },
    CdTrack {
        cd_track: u8,
        loop_track: bool,
    },
    SellScreen,
    CutScene {
        text: String,
    },
}

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    struct UpdateFlags: u16 {
        const U_MOREBITS     = 1 << 0;
        const U_ORIGIN1      = 1 << 1;
        const U_ORIGIN2      = 1 << 2;
        const U_ORIGIN3      = 1 << 3;
        const U_ANGLE2       = 1 << 4;
        const U_NOLERP       = 1 << 5;
        const U_FRAME        = 1 << 6;
        const U_SIGNAL       = 1 << 7;
        const U_ANGLE1       = 1 << 8;
        const U_ANGLE3       = 1 << 9;
        const U_MODEL        = 1 << 10;
        const U_COLORMAP     = 1 << 11;
        const U_SKIN         = 1 << 12;
        const U_EFFECTS      = 1 << 13;
        const U_LONGENTITY   = 1 << 14;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, num_derive::FromPrimitive)]
pub enum ClientStat {
    Health = 0,
    Frags = 1,
    Weapon = 2,
    Ammo = 3,
    Armor = 4,
    WeaponFrame = 5,
    Shells = 6,
    Nails = 7,
    Rockets = 8,
    Cells = 9,
    ActiveWeapon = 10,
    TotalSecrets = 11,
    TotalMonsters = 12,
    Secrets = 13,
    Monsters = 14,
}

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    struct SoundFlags: u8
    {
        const SND_VOLUME        = 1 << 0;
        const SND_ATTENUATION   = 1 << 1;
        const SND_LOOPING       = 1 << 2;
    }
}

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct ClientFlags: u16 {
        const SU_VIEWHEIGHT   = 1 << 0;
        const SU_IDEALPITCH   = 1 << 1;
        const SU_PUNCH1       = 1 << 2;
        const SU_PUNCH2       = 1 << 3;
        const SU_PUNCH3       = 1 << 4;
        const SU_VELOCITY1    = 1 << 5;
        const SU_VELOCITY2    = 1 << 6;
        const SU_VELOCITY3    = 1 << 7;
        const SU_ITEMS        = 1 << 9;
        const SU_ONGROUND     = 1 << 10;
        const SU_INWATER      = 1 << 11;
        const SU_WEAPONFRAME  = 1 << 12;
        const SU_ARMOR        = 1 << 13;
        const SU_WEAPON       = 1 << 14;
    }
}

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct ItemFlags: u32 {
        // Weapons
        const IT_SHOTGUN           = 1 << 0;
        const IT_SUPER_SHOTGUN     = 1 << 1;
        const IT_NAILGUN           = 1 << 2;
        const IT_SUPER_NAILGUN     = 1 << 3;
        const IT_GRENADE_LAUNCHER  = 1 << 4;
        const IT_ROCKET_LAUNCHER   = 1 << 5;
        const IT_LIGHTNING         = 1 << 6;
        const IT_SUPER_LIGHTNING   = 1 << 7;

        // Ammo
        const IT_SHELLS            = 1 << 8;
        const IT_NAILS             = 1 << 9;
        const IT_ROCKETS           = 1 << 10;
        const IT_CELLS             = 1 << 11;

        // Misc
        const IT_AXE               = 1 << 12;

        // Armor
        const IT_ARMOR1            = 1 << 13;
        const IT_ARMOR2            = 1 << 14;
        const IT_ARMOR3            = 1 << 15;

        // Health / keys
        const IT_SUPERHEALTH       = 1 << 16;
        const IT_KEY1              = 1 << 17;
        const IT_KEY2              = 1 << 18;

        // Powerups
        const IT_INVISIBILITY      = 1 << 19;
        const IT_INVULNERABILITY   = 1 << 20;
        const IT_SUIT              = 1 << 21;
        const IT_QUAD              = 1 << 22;

        // Sigils
        const IT_SIGIL1            = 1 << 28;
        const IT_SIGIL2            = 1 << 29;
        const IT_SIGIL3            = 1 << 30;
        const IT_SIGIL4            = 1 << 31;
    }
}

#[derive(Clone, Copy, Debug)]
pub enum TemporaryEntity {
    Spike {
        origin: Vec3,
    },
    SuperSpike {
        origin: Vec3,
    },
    Gunshot {
        origin: Vec3,
    },
    Explosion {
        origin: Vec3,
    },
    TarExplosion {
        origin: Vec3,
    },
    Lightning1 {
        entity_id: u16,
        start: Vec3,
        end: Vec3,
    },
    Lightning2 {
        entity_id: u16,
        start: Vec3,
        end: Vec3,
    },
    WizSpike {
        origin: Vec3,
    },
    KnightSpike {
        origin: Vec3,
    },
    Lightning3 {
        entity_id: u16,
        start: Vec3,
        end: Vec3,
    },
    LavaSplash {
        origin: Vec3,
    },
    Teleport {
        origin: Vec3,
    },
    Explosion2 {
        origin: Vec3,
        count_start: u8,
        count_length: u8,
    },
    Beam {
        entity_id: u16,
        start: Vec3,
        end: Vec3,
    },
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, num_derive::FromPrimitive)]
pub enum SignOnState {
    NotConnected = 0,
    ServerInfo = 1,
    Baselines = 2,
    WorldLoaded = 3,
    Connected = 4,
}

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct EffectsFlags: u8 {
        const EF_BRIGHT_FIELD = 1 << 0;
        const EF_MUZZLE_FLASH = 1 << 1;
        const EF_BRIGHT_LIGHT = 1 << 2;
        const EF_DIM_LIGHT    = 1 << 3;
    }
}

fn read_angle(reader: &mut Cursor<&[u8]>) -> anyhow::Result<f32> {
    Ok(reader.read_i8()? as f32 * (360.0 / 256.0))
}

fn read_xyz_angles(reader: &mut Cursor<&[u8]>) -> anyhow::Result<Vec3> {
    let vector = [
        read_angle(reader)?,
        read_angle(reader)?,
        read_angle(reader)?,
    ];

    // Swaps Y↔Z axes to convert from Quake's coordinate system to standard 3D
    Ok([vector[0], vector[2], -vector[1]].into())
}

fn read_coord(reader: &mut Cursor<&[u8]>) -> anyhow::Result<f32> {
    Ok(reader.read_i16::<LittleEndian>()? as f32 / 8.0)
}

fn read_xyz_coords(reader: &mut Cursor<&[u8]>) -> anyhow::Result<Vec3> {
    let vector = [
        read_coord(reader)?,
        read_coord(reader)?,
        read_coord(reader)?,
    ];

    // Swaps Y↔Z axes to convert from Quake's coordinate system to standard 3D
    Ok([vector[0], vector[2], -vector[1]].into())
}

fn read_velocity(reader: &mut Cursor<&[u8]>) -> anyhow::Result<f32> {
    Ok(reader.read_i8()? as f32 * (1.0 / 16.0))
}

fn read_xyz_velocity(reader: &mut Cursor<&[u8]>) -> anyhow::Result<Vec3> {
    let vector = [
        read_velocity(reader)?,
        read_velocity(reader)?,
        read_velocity(reader)?,
    ];

    // Swaps Y↔Z axes to convert from Quake's coordinate system to standard 3D
    Ok([vector[0], vector[2], -vector[1]].into())
}

fn read_cstring<R>(reader: &mut R, end_with: u8) -> anyhow::Result<String>
where
    R: ReadBytesExt,
{
    let mut buffer = Vec::new();
    loop {
        match reader.read_u8() {
            Ok(byte) if byte == end_with => break,
            Ok(byte) => buffer.push(byte),
            Err(e) => return Err(e.into()),
        }
    }
    Ok(String::from_utf8(buffer)?)
}
