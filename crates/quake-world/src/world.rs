use crate::CommittedEvents;
use crate::components::{EntityId, PlayerId};
use crate::systems::{DemPlayback, replay_dem_stream_system};
use quake_asset::pak::bsp::Component;
use quake_asset::pak::dem::DemEvent;
use quake_network::quic::ConnectionId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tracing::{error, info};

static WORLD_ID_GENERATOR: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorldId(u64);

impl WorldId {
    pub fn new() -> Self {
        Self(WORLD_ID_GENERATOR.fetch_add(1, Ordering::Relaxed))
    }
}

impl From<u64> for WorldId {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<WorldId> for u64 {
    fn from(value: WorldId) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorldExecutionState {
    Running,
    Stopped,
}

pub struct WorldServer {
    state: WorldExecutionState,
    entities: legion::World,
    resources: legion::Resources,
    systems: legion::Schedule,
    connections: HashMap<ConnectionId, WorldConnectionState>,
}

impl WorldServer {
    pub fn new(
        world_id: WorldId,
        world_mode: WorldMode,
        asset_manager: &quake_asset::AssetManager,
    ) -> anyhow::Result<Self> {
        let mut entities = legion::World::default();
        let mut resources = legion::Resources::default();
        let systems = match &world_mode {
            WorldMode::Demo(dem_path) => {
                let world_map =
                    Self::load_dem(dem_path, asset_manager, &mut entities, &mut resources)?;
                resources.insert(world_map);

                legion::Schedule::builder()
                    .add_system(replay_dem_stream_system())
                    .build()
            }
            WorldMode::Campaign(map_path) => {
                let world_map =
                    Self::load_map(map_path, asset_manager, &mut entities, &mut resources)?;
                resources.insert(world_map);

                legion::Schedule::builder().build()
            }
            WorldMode::Deathmatch(map_path) => {
                let world_map =
                    Self::load_map(map_path, asset_manager, &mut entities, &mut resources)?;
                resources.insert(world_map);

                legion::Schedule::builder().build()
            }
        };

        resources.insert(world_id);
        resources.insert(world_mode);

        Ok(Self {
            state: WorldExecutionState::Stopped,
            entities,
            resources,
            systems,
            connections: HashMap::new(),
        })
    }

    pub fn world_id(&self) -> WorldId {
        self.resources.get::<WorldId>().unwrap().clone()
    }

    pub fn world_mode(&self) -> WorldMode {
        self.resources.get::<WorldMode>().unwrap().clone()
    }

    pub fn world_map(&self) -> WorldMap {
        self.resources.get::<WorldMap>().unwrap().clone()
    }

    pub fn connections_iter(&self) -> impl Iterator<Item = (&ConnectionId, &WorldConnectionState)> {
        self.connections.iter()
    }

    pub fn step(&mut self) -> Option<WorldSnapshot> {
        if self.connections.is_empty() || self.state == WorldExecutionState::Stopped {
            return None;
        }

        let delta_time = self.resources.get_or_insert(Instant::now()).elapsed();
        if delta_time < Duration::from_millis(100) {
            return None;
        }

        self.resources.insert(delta_time);
        self.resources.insert(Instant::now());

        self.systems
            .execute(&mut self.entities, &mut self.resources);

        use legion::query::IntoQuery;

        let mut query = <(&EntityId,)>::query();
        let entities = query
            .iter(&self.entities)
            .map(|(entity_id,)| EntitySnapshot {
                entity_id: *entity_id,
            })
            .collect::<Vec<_>>();

        Some(WorldSnapshot {
            entities: entities.into_boxed_slice(),
            events: CommittedEvents::default(),
        })
    }

    pub fn on_join(&mut self, connection_id: ConnectionId) -> PlayerId {
        self.connections
            .insert(connection_id, WorldConnectionState::Pending);

        let player_id = PlayerId::new();
        let entity = self.entities.push((player_id,));
        let mut entry = self.entities.entry(entity).unwrap();
        entry.add_component(Component::Origin(glam::Vec3::ZERO));
        player_id
    }

    pub fn on_leave(&mut self, connection_id: ConnectionId, player_id: PlayerId) {
        self.connections.remove(&connection_id);

        use legion::query::IntoQuery;
        let mut query = <(legion::Entity, &PlayerId)>::query();
        let entity = query.iter(&self.entities).find_map(|(entity, id)| {
            if *id == player_id {
                Some(*entity)
            } else {
                None
            }
        });
        if let Some(entity) = entity {
            self.entities.remove(entity);
        }

        if self.connections.is_empty()
            || self
                .connections
                .values()
                .all(|state| *state == WorldConnectionState::Pending)
        {
            self.state = WorldExecutionState::Stopped;
        }
    }

    pub fn on_play(&mut self, connection_id: ConnectionId, player_id: PlayerId) {
        use legion::query::IntoQuery;
        let mut query = <&PlayerId>::query();
        if query.iter(&self.entities).any(|id| *id == player_id) {
            self.connections
                .insert(connection_id, WorldConnectionState::Accepted);
            self.state = WorldExecutionState::Running;
        } else {
            error!(?connection_id, ?player_id, "player not found");
        }
    }

    pub fn on_intent(&mut self, world_intent: WorldIntent) {}

    fn load_dem<P>(
        dem_path: P,
        asset_manager: &quake_asset::AssetManager,
        entities: &mut legion::World,
        resources: &mut legion::Resources,
    ) -> anyhow::Result<WorldMap>
    where
        P: AsRef<Path>,
    {
        let dem = asset_manager
            .by_name::<quake_asset::pak::dem::Dem>(dem_path.as_ref().to_str().unwrap())?;
        let Some(DemEvent::ServerInfo {
            map_path,
            precache_models,
            precache_sounds,
        }) = dem.iter().next()
        else {
            return Err(anyhow::anyhow!("no server info event found"));
        };

        resources.insert(DemPlayback::new(dem));

        Ok(WorldMap {
            map_path: map_path.clone(),
            precache_models: precache_models.clone(),
            precache_sounds: precache_sounds.clone(),
        })
    }

    fn load_map<P>(
        map_path: P,
        asset_manager: &quake_asset::AssetManager,
        entities: &mut legion::World,
        resources: &mut legion::Resources,
    ) -> anyhow::Result<WorldMap>
    where
        P: AsRef<Path>,
    {
        let bsp = asset_manager
            .by_name::<quake_asset::pak::bsp::Bsp>(map_path.as_ref().to_str().unwrap())?;
        for (i, e) in bsp.entities.iter().enumerate() {
            let entity = entities.push((EntityId(i as u64),));
            let mut entry = entities.entry(entity).unwrap();

            for c in e.components.iter() {
                match c {
                    // Identifies the entity type, used by the engine to decide behavior, logic, and default properties.
                    Component::Classname(_) => {}
                    // 3D position of the entity in world space. Defines where the entity spawns or is placed.
                    Component::Origin(_) => {}
                    // Brush or MDL model associated with the entity. For brush entities, usually *n referencing a BSP submodel.
                    Component::Model(_) => {}
                    // Bitmask controlling variant behavior of an entity. Each bit enables/disables specific features.
                    Component::SpawnFlags(_) => {}
                    // Euler angles (pitch yaw roll) used mainly for rotating brush entities. Quake uses this instead of angles for certain entities.
                    Component::Mangle(_) => {}
                    // Name of entities to be removed when this entity is activated. Used for scripted events.
                    Component::KillTarget(_) => {}
                    // Name of entities this entity will activate. Core mechanism for linking triggers, doors, buttons, etc.
                    Component::Target(_) => {}
                    // Identifier that allows other entities to reference this one via target. Acts like a logical entity ID.
                    Component::TargetName(_) => {}
                    // Sound set or sound variant index. Often selects which predefined sound to use (doors, plats, buttons).
                    Component::Sounds(_) => {}
                    // Text message displayed to the player when activated. Common in triggers, secrets, or level messages.
                    Component::Message(_) => {}
                    // Delay (in seconds) before the entity can be triggered again. Used for repeatable triggers, doors, or timed actions.
                    Component::Wait(_) => {}
                    // Light intensity or color information. Controls brightness of light entities.
                    Component::Light(_) => {}
                    // Light style index (for flickering, pulsing lights). Refers to animated light patterns (a–z).
                    Component::Style(_) => {}
                    // Name of another BSP map to load. Used by trigger_changelevel to transition between levels.
                    Component::Map(_) => {}
                    // Hit points of an entity. Applies to monsters, breakables, and damageable objects.
                    Component::Health(_) => {}
                    // Movement speed or animation speed. Used by doors, plats, trains, projectiles, etc.
                    Component::Speed(_) => {}
                    // Generic numeric parameter. Meaning depends on entity (ammo count, repetitions, number of uses).
                    Component::Count(_) => {}
                    // Vertical size or movement distance. Often used by lifts, plats, or special triggers.
                    Component::Height(_) => {}
                    // Amount of damage dealt to entities. Used by traps, triggers, and explosive entities.
                    Component::Damage(_) => {}
                    // List of WAD files used for textures. Mainly found in worldspawn.
                    Component::Wad(_) => {}
                    // Environment type (e.g. medieval, metal, base). Controls ambient sounds and some visual/audio defaults.
                    Component::WorldType(_) => {}
                    // Distance the door/platform remains visible when fully open. Prevents complete disappearance into walls.
                    Component::Lip(_) => {}
                    // Time in seconds before this entity triggers its target after being activated.
                    Component::Delay(_) => {}
                }
            }
        }

        Ok(WorldMap {
            map_path: map_path.as_ref().to_owned(),
            precache_models: Box::new([]),
            precache_sounds: Box::new([]),
        })
    }
}

pub struct WorldClient {
    network_sender: quake_network::quic::ClientSender<WorldMessage>,
    network_receiver: quake_network::quic::ClientReceiver<WorldMessage>,

    asset_manager: quake_asset::AssetManager,

    entities: legion::World,
    resources: legion::Resources,
}

impl WorldClient {
    pub async fn new(addr: SocketAddr) -> anyhow::Result<Self> {
        let certs_path = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../certs"));
        let (network_sender, network_receiver) =
            quake_network::quic::client_channel(addr, certs_path.join("ca.crt")).await?;

        let resources_path = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../res"));
        let asset_manager = quake_asset::AssetManager::new(resources_path)?;

        Ok(Self {
            network_sender,
            network_receiver,

            asset_manager,

            entities: legion::World::default(),
            resources: legion::Resources::default(),
        })
    }

    pub fn spawn(&mut self, world_mode: WorldMode) -> anyhow::Result<()> {
        info!(?world_mode, "spawn world");

        self.network_sender
            .send_message(WorldMessage::Command(WorldCommand::Spawn { world_mode }))
    }

    pub fn despawn(&mut self, world_id: WorldId) -> anyhow::Result<()> {
        info!(?world_id, "despawn world");

        self.network_sender
            .send_message(WorldMessage::Command(WorldCommand::Despawn { world_id }))
    }

    pub fn join(&mut self, world_id: WorldId) -> anyhow::Result<()> {
        if self.resources.get::<WorldId>().is_some() {
            return Err(anyhow::anyhow!("already joined a world"));
        };

        info!(?world_id, "join world");

        self.network_sender
            .send_message(WorldMessage::Command(WorldCommand::Join { world_id }))
    }

    pub fn leave(&mut self) -> anyhow::Result<()> {
        let Some(world_id) = self.resources.get::<WorldId>() else {
            return Ok(());
        };
        let player_id = self.resources.get::<PlayerId>().unwrap();
        info!(?world_id, ?player_id, "leave world");

        self.network_sender
            .send_message(WorldMessage::Command(WorldCommand::Leave {
                world_id: *world_id,
                player_id: *player_id,
            }))
    }

    pub fn step(&mut self) -> anyhow::Result<()> {
        while let Some(world_message) = self.network_receiver.try_recv_message() {
            match world_message {
                WorldMessage::Notification(world_notification) => match world_notification {
                    WorldNotification::Spawned { world_id } => self.on_spawned(world_id)?,
                    WorldNotification::Despawned => self.on_despawned()?,
                    WorldNotification::Joined {
                        world_id,
                        world_map,
                        player_id,
                    } => {
                        self.on_joined(world_id, world_map, player_id)?;
                        self.network_sender.send_message(WorldMessage::Command(
                            WorldCommand::Play {
                                world_id,
                                player_id,
                            },
                        ))?;
                    }
                    WorldNotification::Left => self.on_left(),
                },
                WorldMessage::Snapshot(snapshot) => {
                    self.on_snapshot(snapshot);
                }
                _ => (),
            }
        }

        Ok(())
    }

    fn on_spawned(&mut self, world_id: WorldId) -> anyhow::Result<()> {
        info!(?world_id, "world spawned");

        self.join(world_id)
    }

    fn on_despawned(&mut self) -> anyhow::Result<()> {
        info!("world despawned");

        self.leave()
    }

    fn on_joined(
        &mut self,
        world_id: WorldId,
        world_map: WorldMap,
        player_id: PlayerId,
    ) -> anyhow::Result<()> {
        info!(?world_id, map_path=?world_map.map_path, ?player_id, "player joined");

        self.resources.insert(world_id);
        self.resources.insert(player_id);
        self.load_map(world_map)
    }

    fn on_left(&mut self) {
        self.unload_map();
        let world_id = self.resources.remove::<WorldId>();
        let player_id = self.resources.remove::<PlayerId>();

        info!(?world_id, ?player_id, "player left");
    }

    fn on_snapshot(&mut self, snapshot: WorldSnapshot) {
        use legion::query::IntoQuery;

        for entity_snapshot in snapshot.entities.iter() {
            let mut query = <(legion::Entity, &EntityId)>::query();
            query.iter_mut(&mut self.entities).for_each(
                |(_, id)| {
                    if *id == entity_snapshot.entity_id {}
                },
            );
        }
    }

    fn load_map(&mut self, world_map: WorldMap) -> anyhow::Result<()> {
        let bsp = self
            .asset_manager
            .by_name::<quake_asset::pak::bsp::Bsp>(world_map.map_path.to_str().unwrap())?;
        for (i, e) in bsp.entities.iter().enumerate() {
            let entity = self.entities.push((EntityId(i as u64),));
            let mut entry = self.entities.entry(entity).unwrap();

            for c in e.components.iter() {
                match c {
                    // Identifies the entity type, used by the engine to decide behavior, logic, and default properties.
                    Component::Classname(_) => {}
                    // 3D position of the entity in world space. Defines where the entity spawns or is placed.
                    Component::Origin(_) => {}
                    // Brush or MDL model associated with the entity. For brush entities, usually *n referencing a BSP submodel.
                    Component::Model(_) => {}
                    // Bitmask controlling variant behavior of an entity. Each bit enables/disables specific features.
                    Component::SpawnFlags(_) => {}
                    // Euler angles (pitch yaw roll) used mainly for rotating brush entities. Quake uses this instead of angles for certain entities.
                    Component::Mangle(_) => {}
                    // Name of entities to be removed when this entity is activated. Used for scripted events.
                    Component::KillTarget(_) => {}
                    // Name of entities this entity will activate. Core mechanism for linking triggers, doors, buttons, etc.
                    Component::Target(_) => {}
                    // Identifier that allows other entities to reference this one via target. Acts like a logical entity ID.
                    Component::TargetName(_) => {}
                    // Sound set or sound variant index. Often selects which predefined sound to use (doors, plats, buttons).
                    Component::Sounds(_) => {}
                    // Text message displayed to the player when activated. Common in triggers, secrets, or level messages.
                    Component::Message(_) => {}
                    // Delay (in seconds) before the entity can be triggered again. Used for repeatable triggers, doors, or timed actions.
                    Component::Wait(_) => {}
                    // Light intensity or color information. Controls brightness of light entities.
                    Component::Light(_) => {}
                    // Light style index (for flickering, pulsing lights). Refers to animated light patterns (a–z).
                    Component::Style(_) => {}
                    // Name of another BSP map to load. Used by trigger_changelevel to transition between levels.
                    Component::Map(_) => {}
                    // Hit points of an entity. Applies to monsters, breakables, and damageable objects.
                    Component::Health(_) => {}
                    // Movement speed or animation speed. Used by doors, plats, trains, projectiles, etc.
                    Component::Speed(_) => {}
                    // Generic numeric parameter. Meaning depends on entity (ammo count, repetitions, number of uses).
                    Component::Count(_) => {}
                    // Vertical size or movement distance. Often used by lifts, plats, or special triggers.
                    Component::Height(_) => {}
                    // Amount of damage dealt to entities. Used by traps, triggers, and explosive entities.
                    Component::Damage(_) => {}
                    // List of WAD files used for textures. Mainly found in worldspawn.
                    Component::Wad(_) => {}
                    // Environment type (e.g. medieval, metal, base). Controls ambient sounds and some visual/audio defaults.
                    Component::WorldType(_) => {}
                    // Distance the door/platform remains visible when fully open. Prevents complete disappearance into walls.
                    Component::Lip(_) => {}
                    // Time in seconds before this entity triggers its target after being activated.
                    Component::Delay(_) => {}
                }
            }
        }

        Ok(())
    }

    fn unload_map(&mut self) {
        self.entities.clear();
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorldMap {
    pub map_path: PathBuf,
    pub precache_models: Box<[PathBuf]>,
    pub precache_sounds: Box<[PathBuf]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorldConnectionState {
    Pending,
    Accepted,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum WorldMessage {
    Command(WorldCommand),
    Notification(WorldNotification),

    Intent(WorldIntent),
    Snapshot(WorldSnapshot),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum WorldCommand {
    Spawn {
        world_mode: WorldMode,
    },
    Despawn {
        world_id: WorldId,
    },
    Join {
        world_id: WorldId,
    },
    Leave {
        world_id: WorldId,
        player_id: PlayerId,
    },
    Play {
        world_id: WorldId,
        player_id: PlayerId,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum WorldNotification {
    Spawned {
        world_id: WorldId,
    },
    Despawned,
    Joined {
        world_id: WorldId,
        world_map: WorldMap,
        player_id: PlayerId,
    },
    Left,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum WorldMode {
    Demo(PathBuf),
    Campaign(PathBuf),
    Deathmatch(PathBuf),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorldIntent {
    pub world_id: WorldId,
    pub player_id: PlayerId,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorldSnapshot {
    pub entities: Box<[EntitySnapshot]>,
    pub events: CommittedEvents,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EntitySnapshot {
    pub entity_id: EntityId,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum WorldEvent {}
