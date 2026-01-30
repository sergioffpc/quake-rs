use crate::component::EntityId;
use crate::system::{DemPlayback, replay_dem_stream_system};
use crate::{CommittedEvents, query};
use quake_asset::pak::bsp::Component;
use quake_asset::pak::dem::DemEvent;
use quake_network::{ConnectionId, MessageWrapper};
use serde::{Deserialize, Serialize};
use std::collections::vec_deque::Drain;
use std::collections::{HashMap, VecDeque};
use std::fmt::Display;
use std::path::PathBuf;
use std::sync::Arc;
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

impl Default for WorldId {
    fn default() -> Self {
        Self::new()
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

impl Display for WorldId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

static PLAYER_ID_GENERATOR: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlayerId(u64);

impl PlayerId {
    pub fn new() -> Self {
        Self(PLAYER_ID_GENERATOR.fetch_add(1, Ordering::Relaxed))
    }
}

impl Default for PlayerId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorldExecutionState {
    Running,
    Stopped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum WorldConnectionState {
    Suspended,
    Established,
}

#[derive(Debug, Default)]
struct Connections {
    connections: HashMap<ConnectionId, WorldConnectionState>,
}

impl Connections {
    fn insert(&mut self, connection_id: ConnectionId, state: WorldConnectionState) {
        self.connections.insert(connection_id, state);
    }

    fn remove(&mut self, connection_id: &ConnectionId) {
        self.connections.remove(connection_id);
    }

    fn is_empty(&self) -> bool {
        self.connections.is_empty()
    }

    fn active_connections(&self) -> impl Iterator<Item = (&ConnectionId, &WorldConnectionState)> {
        self.connections
            .iter()
            .filter(|(_, state)| **state == WorldConnectionState::Established)
    }

    fn no_active_connections(&self) -> bool {
        self.connections.is_empty()
            || self
                .connections
                .values()
                .all(|state| *state == WorldConnectionState::Suspended)
    }
}

pub struct WorldServer {
    state: WorldExecutionState,

    asset_manager: Arc<quake_asset::AssetManager>,

    entities: legion::World,
    resources: legion::Resources,
    systems: legion::Schedule,

    connections: Connections,
}

impl WorldServer {
    pub fn new(
        world_id: WorldId,
        asset_manager: Arc<quake_asset::AssetManager>,
    ) -> anyhow::Result<Self> {
        let entities = legion::World::default();
        let mut resources = legion::Resources::default();
        resources.insert(world_id);
        let systems = legion::Schedule::builder().build();

        Ok(Self {
            state: WorldExecutionState::Stopped,
            asset_manager,
            entities,
            resources,
            systems,
            connections: Connections::default(),
        })
    }

    pub fn world_id(&self) -> WorldId {
        *self.resources.get::<WorldId>().unwrap()
    }

    pub fn world_map(&self) -> WorldMap {
        self.resources.get::<WorldMap>().unwrap().clone()
    }

    pub fn load(&mut self, world_mode: WorldMode) -> anyhow::Result<()> {
        match &world_mode {
            WorldMode::Demo(dem_path) => {
                let dem = self
                    .asset_manager
                    .by_name::<quake_asset::pak::dem::Dem>(dem_path.to_str().unwrap())?;

                self.entities.clear();
                let world_map = self.load_dem(dem)?;
                self.resources.insert(world_map);

                self.systems = legion::Schedule::builder()
                    .add_system(replay_dem_stream_system())
                    .build()
            }
            WorldMode::Campaign(map_path) => {
                let bsp = self
                    .asset_manager
                    .by_name::<quake_asset::pak::bsp::Bsp>(map_path.to_str().unwrap())?;
                let world_map = self.load_bsp(bsp)?;
                self.resources.insert(world_map);

                self.systems = legion::Schedule::builder().build()
            }
            WorldMode::Deathmatch(map_path) => {
                let bsp = self
                    .asset_manager
                    .by_name::<quake_asset::pak::bsp::Bsp>(map_path.to_str().unwrap())?;
                let world_map = self.load_bsp(bsp)?;
                self.resources.insert(world_map);

                self.systems = legion::Schedule::builder().build()
            }
        };

        Ok(())
    }

    pub fn unload(&mut self) {
        self.entities.clear();
        self.resources.remove::<WorldMap>();
        self.systems = legion::Schedule::builder().build()
    }

    pub fn active_connections(&self) -> impl Iterator<Item = ConnectionId> + '_ {
        self.connections
            .active_connections()
            .map(|(connection_id, _)| *connection_id)
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
            .insert(connection_id, WorldConnectionState::Suspended);

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

        if self.connections.no_active_connections() {
            self.state = WorldExecutionState::Stopped;
        }
    }

    pub fn on_play(&mut self, connection_id: ConnectionId, player_id: PlayerId) {
        if query::player_exists(&self.entities, player_id) {
            self.connections
                .insert(connection_id, WorldConnectionState::Established);
            self.state = WorldExecutionState::Running;
        } else {
            error!(?connection_id, ?player_id, "player not found");
        }
    }

    pub fn on_pause(&mut self, connection_id: ConnectionId, player_id: PlayerId) {
        if query::player_exists(&self.entities, player_id) {
            self.connections
                .insert(connection_id, WorldConnectionState::Suspended);
            if self.connections.no_active_connections() {
                self.state = WorldExecutionState::Stopped;
            }
        } else {
            error!(?connection_id, ?player_id, "player not found");
        }
    }

    pub fn on_resume(&mut self, connection_id: ConnectionId, player_id: PlayerId) {
        if query::player_exists(&self.entities, player_id) {
            self.connections
                .insert(connection_id, WorldConnectionState::Established);
            self.state = WorldExecutionState::Running;
        } else {
            error!(?connection_id, ?player_id, "player not found");
        }
    }

    pub fn on_stop(&mut self, connection_id: ConnectionId, player_id: PlayerId) {
        if query::player_exists(&self.entities, player_id) {
            self.connections
                .insert(connection_id, WorldConnectionState::Suspended);
            if self.connections.no_active_connections() {
                self.state = WorldExecutionState::Stopped;
            }
        } else {
            error!(?connection_id, ?player_id, "player not found");
        }
    }

    pub fn on_intent(&mut self, world_intent: WorldIntent) {}

    fn load_dem(&mut self, dem: quake_asset::pak::dem::Dem) -> anyhow::Result<WorldMap> {
        let Some(DemEvent::ServerInfo {
            map_path,
            precache_models,
            precache_sounds,
        }) = dem.iter().next()
        else {
            return Err(anyhow::anyhow!("no server info event found"));
        };

        self.resources.insert(DemPlayback::new(dem));

        Ok(WorldMap {
            map_path: map_path.clone(),
            precache_models: precache_models.clone(),
            precache_sounds: precache_sounds.clone(),
        })
    }

    fn load_bsp(&mut self, bsp: quake_asset::pak::bsp::Bsp) -> anyhow::Result<()> {
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorldClientState {
    Playing,
    Paused,
    Stopped,
}

pub struct WorldClient {
    state: WorldClientState,

    network_manager: quake_network::NetworkClient<WorldMessage>,
    asset_manager: Arc<quake_asset::AssetManager>,

    entities: legion::World,
    resources: legion::Resources,
    systems: legion::Schedule,

    notifications: VecDeque<WorldNotification>,
}

impl WorldClient {
    pub async fn new(
        network_manager: quake_network::NetworkClient<WorldMessage>,
        asset_manager: Arc<quake_asset::AssetManager>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            state: WorldClientState::Stopped,

            network_manager,
            asset_manager,

            entities: legion::World::default(),
            resources: legion::Resources::default(),
            systems: legion::Schedule::builder().build(),

            notifications: VecDeque::default(),
        })
    }

    pub fn spawn(&mut self, world_mode: WorldMode) -> anyhow::Result<()> {
        info!(?world_mode, "spawn world");

        self.network_manager
            .send_message(WorldMessage::Command(WorldCommand::Spawn { world_mode }))
    }

    pub fn despawn(&mut self) -> anyhow::Result<()> {
        let Some(world_id) = self.resources.get::<WorldId>() else {
            return Ok(());
        };
        info!(?world_id, "despawn world");

        self.network_manager
            .send_message(WorldMessage::Command(WorldCommand::Despawn {
                world_id: *world_id,
            }))
    }

    pub fn join(&mut self, world_id: WorldId) -> anyhow::Result<()> {
        if self.resources.get::<WorldId>().is_some() {
            return Err(anyhow::anyhow!("already joined a world"));
        };
        info!(?world_id, "join world");

        self.network_manager
            .send_message(WorldMessage::Command(WorldCommand::Join { world_id }))
    }

    pub fn leave(&mut self) -> anyhow::Result<()> {
        let Some(world_id) = self.resources.get::<WorldId>() else {
            return Ok(());
        };
        let player_id = self.resources.get::<PlayerId>().unwrap();
        info!(?world_id, ?player_id, "leave world");

        self.network_manager
            .send_message(WorldMessage::Command(WorldCommand::Leave {
                world_id: *world_id,
                player_id: *player_id,
            }))
    }

    pub fn load(&mut self, map_path: PathBuf) -> anyhow::Result<()> {
        info!(?map_path, "load world");

        let bsp = self
            .asset_manager
            .by_name::<quake_asset::pak::bsp::Bsp>(map_path.to_str().unwrap())?;
        self.load_bsp(&bsp)
    }

    pub fn unload(&mut self) {
        info!("unload world");

        self.entities.clear();
    }

    pub fn play(&mut self) -> anyhow::Result<()> {
        let Some(world_id) = self.resources.get::<WorldId>() else {
            return Err(anyhow::anyhow!("not joined a world"));
        };

        if self.state != WorldClientState::Stopped {
            return Err(anyhow::anyhow!("world must be stopped to play"));
        }

        let player_id = self.resources.get::<PlayerId>().unwrap();
        info!(?world_id, ?player_id, "play world");

        self.state = WorldClientState::Playing;
        self.network_manager
            .send_message(WorldMessage::Command(WorldCommand::Play {
                world_id: *world_id,
                player_id: *player_id,
            }))
    }

    pub fn pause(&mut self) -> anyhow::Result<()> {
        let Some(world_id) = self.resources.get::<WorldId>() else {
            return Err(anyhow::anyhow!("not joined a world"));
        };

        if self.state != WorldClientState::Playing {
            return Err(anyhow::anyhow!("world must be playing to pause"));
        }

        let player_id = self.resources.get::<PlayerId>().unwrap();
        info!(?world_id, ?player_id, "pause world");

        self.state = WorldClientState::Paused;
        self.network_manager
            .send_message(WorldMessage::Command(WorldCommand::Pause {
                world_id: *world_id,
                player_id: *player_id,
            }))
    }

    pub fn resume(&mut self) -> anyhow::Result<()> {
        let Some(world_id) = self.resources.get::<WorldId>() else {
            return Err(anyhow::anyhow!("not joined a world"));
        };

        if self.state != WorldClientState::Paused {
            return Err(anyhow::anyhow!("world must be paused to resume"));
        }

        let player_id = self.resources.get::<PlayerId>().unwrap();
        info!(?world_id, ?player_id, "resume world");

        self.state = WorldClientState::Playing;
        self.network_manager
            .send_message(WorldMessage::Command(WorldCommand::Resume {
                world_id: *world_id,
                player_id: *player_id,
            }))
    }

    pub fn stop(&mut self) -> anyhow::Result<()> {
        let Some(world_id) = self.resources.get::<WorldId>() else {
            return Err(anyhow::anyhow!("not joined a world"));
        };

        if self.state == WorldClientState::Stopped {
            return Err(anyhow::anyhow!("world must be running to stop"));
        }

        let player_id = self.resources.get::<PlayerId>().unwrap();
        info!(?world_id, ?player_id, "stop world");

        self.state = WorldClientState::Stopped;
        self.network_manager
            .send_message(WorldMessage::Command(WorldCommand::Stop {
                world_id: *world_id,
                player_id: *player_id,
            }))
    }

    pub fn step(&mut self) -> anyhow::Result<()> {
        self.notifications.clear();
        while let Some(world_message) = self.network_manager.try_recv_message() {
            match world_message {
                WorldMessage::Notification(world_notification) => {
                    match &world_notification {
                        WorldNotification::Spawned { world_id } => self.on_spawned(*world_id),
                        WorldNotification::Despawned => self.on_despawned(),
                        WorldNotification::Joined {
                            world_id,
                            player_id,
                            ..
                        } => self.on_joined(*world_id, *player_id),
                        WorldNotification::Left => self.on_left(),
                    }
                    self.notifications.push_back(world_notification);
                }
                WorldMessage::Snapshot(snapshot) => {
                    self.on_snapshot(snapshot);
                }
                _ => (),
            }
        }

        if self.state == WorldClientState::Playing {
            self.systems
                .execute(&mut self.entities, &mut self.resources);
        }

        Ok(())
    }

    pub fn notifications(&mut self) -> Drain<'_, WorldNotification> {
        self.notifications.drain(..)
    }

    fn on_spawned(&mut self, world_id: WorldId) {
        info!(?world_id, "world spawned");
    }

    fn on_despawned(&mut self) {
        info!("world despawned");
    }

    fn on_joined(&mut self, world_id: WorldId, player_id: PlayerId) {
        info!(?world_id, ?player_id, "player joined");

        self.resources.insert(world_id);
        self.resources.insert(player_id);
    }

    fn on_left(&mut self) {
        self.entities.clear();
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

    fn load_bsp(&mut self, bsp: &quake_asset::pak::bsp::Bsp) -> anyhow::Result<()> {
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
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorldMap {
    pub map_path: PathBuf,
    pub precache_models: Box<[PathBuf]>,
    pub precache_sounds: Box<[PathBuf]>,
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
    Pause {
        world_id: WorldId,
        player_id: PlayerId,
    },
    Resume {
        world_id: WorldId,
        player_id: PlayerId,
    },
    Stop {
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
