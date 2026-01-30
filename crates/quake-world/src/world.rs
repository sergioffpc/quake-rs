use crate::component::{EntityId, Transform};
use crate::system::{DemPlayback, replay_dem_stream_system};
use crate::world::WorldNotification::Spawned;
use crate::{CommittedEvents, EventReader, EventWriter, query};
use legion::IntoQuery;
use quake_asset::pak::dem::DemEvent;
use quake_audio::AudioEvent;
use quake_network::{ConnectionId, MessageWrapper};
use quake_render::RenderEvent;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Display;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tracing::{debug, error, info};

static WORLD_ID_GENERATOR: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorldId(usize);

impl WorldId {
    pub fn generate() -> Self {
        Self(WORLD_ID_GENERATOR.fetch_add(1, Ordering::Relaxed))
    }
}

impl From<usize> for WorldId {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl From<WorldId> for usize {
    fn from(value: WorldId) -> Self {
        value.0
    }
}

impl Display for WorldId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

static PLAYER_ID_GENERATOR: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlayerId(usize);

impl PlayerId {
    pub fn generate() -> Self {
        Self(PLAYER_ID_GENERATOR.fetch_add(1, Ordering::Relaxed))
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
            WorldMode::Campaign(map_path) => self.systems = legion::Schedule::builder().build(),
            WorldMode::Deathmatch(map_path) => self.systems = legion::Schedule::builder().build(),
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
        self.resources.insert(EventWriter::default());

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
            snapshots: entities.into_boxed_slice(),
            events: self.resources.get_mut::<EventWriter>().unwrap().commit(),
        })
    }

    pub fn on_join(&mut self, connection_id: ConnectionId) -> PlayerId {
        self.connections
            .insert(connection_id, WorldConnectionState::Suspended);

        let player_id = PlayerId::generate();
        let entity = self.entities.push((player_id,));
        let mut entry = self.entities.entry(entity).unwrap();
        entry.add_component(Transform::default());
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
    asset_manager: Rc<quake_asset::AssetManager>,

    entities: legion::World,
    resources: legion::Resources,
    systems: legion::Schedule,
}

impl WorldClient {
    pub async fn new(
        network_manager: quake_network::NetworkClient<WorldMessage>,
        asset_manager: Rc<quake_asset::AssetManager>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            state: WorldClientState::Stopped,

            network_manager,
            asset_manager,

            entities: legion::World::default(),
            resources: legion::Resources::default(),
            systems: legion::Schedule::builder().build(),
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
        Ok(())
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

    pub fn step(
        &mut self,
        audio_sender: std::sync::mpsc::Sender<AudioEvent>,
        render_sender: std::sync::mpsc::Sender<RenderEvent>,
    ) -> anyhow::Result<()> {
        while let Some(world_message) = self.network_manager.try_recv_message() {
            match world_message {
                WorldMessage::Notification(world_notification) => match &world_notification {
                    WorldNotification::Spawned { world_id } => self.on_spawned(*world_id),
                    WorldNotification::Despawned => self.on_despawned(),
                    WorldNotification::Joined {
                        world_id,
                        world_map,
                        player_id,
                    } => {
                        audio_sender.send(AudioEvent::Load {
                            precache_sounds: world_map.precache_sounds.clone(),
                        })?;
                        render_sender.send(RenderEvent::Load {
                            precache_models: world_map.precache_models.clone(),
                        })?;
                        self.on_joined(*world_id, *player_id);
                    }
                    WorldNotification::Left => self.on_left(),
                },
                WorldMessage::Snapshot(WorldSnapshot { snapshots, events }) => {
                    for event in EventReader::from(events) {
                        match event {
                            WorldEvent::Entity(entity_event) => {
                                self.on_entity_event(entity_event);
                            }
                            WorldEvent::Render(render_event) => render_sender.send(render_event)?,
                            WorldEvent::Audio(audio_event) => audio_sender.send(audio_event)?,
                        }
                    }
                    for snapshot in snapshots {
                        self.on_entity_snapshot(snapshot);
                    }
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

    fn on_entity_event(&mut self, event: EntityEvent) {
        debug!(?event, "entity event");
    }

    fn on_entity_snapshot(&mut self, snapshot: EntitySnapshot) {
        debug!(?snapshot, "entity snapshot");

        let mut query = <(legion::Entity, &EntityId)>::query();
        query
            .iter_mut(&mut self.entities)
            .for_each(|(_, id)| if *id == snapshot.entity_id {});
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
    pub snapshots: Box<[EntitySnapshot]>,
    pub events: CommittedEvents,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EntitySnapshot {
    pub entity_id: EntityId,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum EntityEvent {
    Spawn {
        entity_id: EntityId,
        transform: Transform,
    },
    Despawn {
        entity_id: EntityId,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum WorldEvent {
    Entity(EntityEvent),
    Render(RenderEvent),
    Audio(AudioEvent),
}
