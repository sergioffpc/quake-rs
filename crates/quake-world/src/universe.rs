use crate::components::PlayerId;
use crate::world::{
    WorldCommand, WorldConnectionState, WorldId, WorldIntent, WorldMap, WorldMessage, WorldMode,
    WorldNotification, WorldServer,
};
use quake_network::quic::{ConnectionId, MessageWrapper};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::num::NonZero;
use std::path::PathBuf;
use tracing::{debug, error, info};

pub struct UniverseServer {
    network_sender: quake_network::quic::ServerSender<WorldMessage>,
    network_receiver: quake_network::quic::ServerReceiver<WorldMessage>,

    universe: ShardedUniverse,
}

impl UniverseServer {
    pub fn new(addr: SocketAddr) -> anyhow::Result<Self> {
        let certs_path = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../certs"));
        let (network_sender, network_receiver) = quake_network::quic::server_channel(
            addr,
            certs_path.join("server.crt"),
            certs_path.join("server.key"),
        )?;

        let num_shards = std::thread::available_parallelism()
            .unwrap_or(NonZero::new(1).unwrap())
            .get();
        let universe = ShardedUniverse::new(num_shards)?;

        Ok(Self {
            network_sender,
            network_receiver,
            universe,
        })
    }

    pub fn step(&mut self) -> anyhow::Result<()> {
        while let Some(MessageWrapper {
            connection_id,
            message: world_message,
        }) = self.network_receiver.try_recv_message()
        {
            match world_message {
                WorldMessage::Command(WorldCommand::Spawn { world_mode }) => {
                    self.on_spawn(connection_id, world_mode)?;
                }
                WorldMessage::Command(WorldCommand::Despawn { world_id }) => {
                    self.on_despawn(connection_id, world_id)?;
                }
                WorldMessage::Command(WorldCommand::Join { world_id }) => {
                    self.on_join(connection_id, world_id)?;
                }
                WorldMessage::Command(WorldCommand::Leave {
                    world_id,
                    player_id,
                }) => {
                    self.on_leave(connection_id, world_id, player_id)?;
                }
                WorldMessage::Command(WorldCommand::Play {
                    world_id,
                    player_id,
                }) => {
                    self.on_play(connection_id, world_id, player_id)?;
                }
                WorldMessage::Intent(world_intent) => {
                    self.on_intent(world_intent)?;
                }
                _ => (),
            }
        }

        for universe_handle in self.universe.iter_mut() {
            while let Ok(world_message) = universe_handle.world_message_receiver.try_recv() {
                self.network_sender.send_message(world_message)?;
            }
        }

        Ok(())
    }

    fn on_spawn(
        &mut self,
        connection_id: ConnectionId,
        world_mode: WorldMode,
    ) -> anyhow::Result<()> {
        let world_id = WorldId::new();
        info!(?connection_id, ?world_id, ?world_mode, "spawn world");

        self.universe.shard_routing(
            world_id,
            UniverseMessage::Command {
                connection_id,
                command: UniverseCommand::Spawn {
                    world_id,
                    world_mode,
                },
            },
        )
    }

    fn on_despawn(&mut self, connection_id: ConnectionId, world_id: WorldId) -> anyhow::Result<()> {
        info!(?connection_id, ?world_id, "despawn world");

        self.universe.shard_routing(
            world_id,
            UniverseMessage::Command {
                connection_id,
                command: UniverseCommand::Despawn { world_id },
            },
        )
    }

    fn on_join(&mut self, connection_id: ConnectionId, world_id: WorldId) -> anyhow::Result<()> {
        info!(?connection_id, ?world_id, "join world");

        self.universe.shard_routing(
            world_id,
            UniverseMessage::Command {
                connection_id,
                command: UniverseCommand::Join { world_id },
            },
        )
    }

    fn on_leave(
        &mut self,
        connection_id: ConnectionId,
        world_id: WorldId,
        player_id: PlayerId,
    ) -> anyhow::Result<()> {
        info!(?connection_id, ?world_id, ?player_id, "leave world");

        self.universe.shard_routing(
            world_id,
            UniverseMessage::Command {
                connection_id,
                command: UniverseCommand::Leave {
                    world_id,
                    player_id,
                },
            },
        )
    }

    fn on_play(
        &mut self,
        connection_id: ConnectionId,
        world_id: WorldId,
        player_id: PlayerId,
    ) -> anyhow::Result<()> {
        info!(?connection_id, ?world_id, ?player_id, "play world");

        self.universe.shard_routing(
            world_id,
            UniverseMessage::Command {
                connection_id,
                command: UniverseCommand::Play {
                    world_id,
                    player_id,
                },
            },
        )
    }

    fn on_intent(&mut self, world_intent: WorldIntent) -> anyhow::Result<()> {
        self.universe
            .shard_routing(world_intent.world_id, UniverseMessage::Intent(world_intent))
    }
}

struct ShardedUniverse {
    num_shards: usize,
    shards: Box<[UniverseHandle]>,
}

impl ShardedUniverse {
    fn new(num_shards: usize) -> anyhow::Result<Self> {
        debug!(?num_shards, "sharded universe");

        let mut shards = Vec::with_capacity(num_shards);
        for i in 0..num_shards {
            let (universe_message_sender, universe_message_receiver) =
                std::sync::mpsc::channel::<UniverseMessage>();
            let (world_message_sender, world_message_receiver) =
                std::sync::mpsc::channel::<MessageWrapper<WorldMessage>>();

            let join_handle = std::thread::Builder::new()
                .name(format!("universe-thread-{}", i))
                .spawn(|| {
                    let result = Universe::new(universe_message_receiver, world_message_sender);
                    match result {
                        Ok(mut universe) => loop {
                            universe.step().unwrap();
                        },
                        Err(err) => {
                            error!(?err, "universe thread failed")
                        }
                    }
                })?;
            shards.push(UniverseHandle {
                universe_message_sender,
                world_message_receiver,
                join_handle,
            });
        }

        Ok(Self {
            num_shards,
            shards: shards.into_boxed_slice(),
        })
    }

    fn shard_routing(&mut self, world_id: WorldId, message: UniverseMessage) -> anyhow::Result<()> {
        let shard_index = u64::from(world_id) as usize % self.num_shards;
        let universe_handle = &self.shards[shard_index];

        debug!(?message, ?shard_index, "shard selected");

        universe_handle
            .universe_message_sender
            .send(message)
            .map_err(Into::into)
    }

    fn iter(&self) -> impl Iterator<Item = &UniverseHandle> {
        self.shards.iter()
    }

    fn iter_mut(&mut self) -> impl Iterator<Item = &mut UniverseHandle> {
        self.shards.iter_mut()
    }
}

struct UniverseHandle {
    universe_message_sender: std::sync::mpsc::Sender<UniverseMessage>,
    world_message_receiver: std::sync::mpsc::Receiver<MessageWrapper<WorldMessage>>,

    join_handle: std::thread::JoinHandle<()>,
}

#[derive(Debug)]
enum UniverseMessage {
    Command {
        connection_id: ConnectionId,
        command: UniverseCommand,
    },
    Intent(WorldIntent),
}

#[derive(Debug)]
enum UniverseCommand {
    Spawn {
        world_id: WorldId,
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

struct Universe {
    universe_message_receiver: std::sync::mpsc::Receiver<UniverseMessage>,
    world_message_sender: std::sync::mpsc::Sender<MessageWrapper<WorldMessage>>,
    asset_manager: quake_asset::AssetManager,
    world_servers: HashMap<WorldId, WorldServer>,
}

impl Universe {
    fn new(
        universe_message_receiver: std::sync::mpsc::Receiver<UniverseMessage>,
        world_message_sender: std::sync::mpsc::Sender<MessageWrapper<WorldMessage>>,
    ) -> anyhow::Result<Self> {
        let resources_path = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../res"));
        let asset_manager = quake_asset::AssetManager::new(resources_path)?;

        Ok(Self {
            universe_message_receiver,
            world_message_sender,

            asset_manager,

            world_servers: HashMap::default(),
        })
    }

    fn insert(&mut self, world_id: WorldId, world_server: WorldServer) {
        self.world_servers.insert(world_id, world_server);
    }

    fn remove(&mut self, world_id: WorldId) {
        self.world_servers.remove(&world_id);
    }

    fn get(&self, world_id: &WorldId) -> Option<&WorldServer> {
        self.world_servers.get(world_id)
    }

    fn get_mut(&mut self, world_id: &WorldId) -> Option<&mut WorldServer> {
        self.world_servers.get_mut(world_id)
    }

    fn step(&mut self) -> anyhow::Result<()> {
        while let Ok(universe_message) = self.universe_message_receiver.try_recv() {
            match universe_message {
                UniverseMessage::Command {
                    connection_id,
                    command:
                        UniverseCommand::Spawn {
                            world_id,
                            world_mode,
                        },
                } => {
                    self.on_spawn(world_id, world_mode)?;
                    self.world_message_sender.send(MessageWrapper {
                        connection_id,
                        message: WorldMessage::Notification(WorldNotification::Spawned {
                            world_id,
                        }),
                    })?;
                }
                UniverseMessage::Command {
                    connection_id,
                    command: UniverseCommand::Despawn { world_id },
                } => {
                    self.on_despawn(world_id);
                    self.world_message_sender.send(MessageWrapper {
                        connection_id,
                        message: WorldMessage::Notification(WorldNotification::Despawned),
                    })?;
                }
                UniverseMessage::Command {
                    connection_id,
                    command: UniverseCommand::Join { world_id },
                } => {
                    if let Some(world_server) = self.world_servers.get_mut(&world_id) {
                        let world_map = world_server.world_map().clone();
                        let player_id = world_server.on_join(connection_id);
                        self.world_message_sender.send(MessageWrapper {
                            connection_id,
                            message: WorldMessage::Notification(WorldNotification::Joined {
                                world_id,
                                world_map,
                                player_id,
                            }),
                        })?;
                    }
                }
                UniverseMessage::Command {
                    connection_id,
                    command:
                        UniverseCommand::Leave {
                            world_id,
                            player_id,
                        },
                } => {
                    if let Some(world_server) = self.world_servers.get_mut(&world_id) {
                        world_server.on_leave(connection_id, player_id);
                        self.world_message_sender.send(MessageWrapper {
                            connection_id,
                            message: WorldMessage::Notification(WorldNotification::Left),
                        })?;
                    }
                }
                UniverseMessage::Command {
                    connection_id,
                    command:
                        UniverseCommand::Play {
                            world_id,
                            player_id,
                        },
                } => {
                    if let Some(world_server) = self.world_servers.get_mut(&world_id) {
                        world_server.on_play(connection_id, player_id);
                    }
                }
                UniverseMessage::Intent(world_intent @ WorldIntent { world_id, .. }) => {
                    if let Some(world_server) = self.world_servers.get_mut(&world_id) {
                        world_server.on_intent(world_intent);
                    }
                }
            }
        }

        for (_, world_server) in self.world_servers.iter_mut() {
            if let Some(snapshot) = world_server.step() {
                world_server
                    .connections_iter()
                    .for_each(|(connection_id, connection_state)| {
                        if let WorldConnectionState::Accepted = connection_state {
                            let result = self.world_message_sender.send(MessageWrapper {
                                connection_id: *connection_id,
                                message: WorldMessage::Snapshot(snapshot.clone()),
                            });
                            if let Err(err) = result {
                                error!(?err, ?connection_id, "failed to send snapshot");
                            }
                        }
                    })
            }
        }

        Ok(())
    }

    fn on_spawn(&mut self, world_id: WorldId, world_mode: WorldMode) -> anyhow::Result<()> {
        let world_server = WorldServer::new(world_id, world_mode, &self.asset_manager)?;

        self.insert(world_id, world_server);

        Ok(())
    }

    fn on_despawn(&mut self, world_id: WorldId) {
        self.remove(world_id);
    }
}
