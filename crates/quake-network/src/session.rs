use std::net::SocketAddr;
use std::sync::Arc;

#[derive(Default)]
pub struct SessionManager {
    sessions: dashmap::DashMap<SocketAddr, Session>,
}

impl SessionManager {
    pub fn get(
        &self,
        address: &SocketAddr,
    ) -> Option<dashmap::mapref::one::Ref<'_, SocketAddr, Session>> {
        self.sessions.get(address)
    }

    pub fn get_mut(
        &self,
        address: &SocketAddr,
    ) -> Option<dashmap::mapref::one::RefMut<'_, SocketAddr, Session>> {
        self.sessions.get_mut(address)
    }

    pub fn insert(&self, address: SocketAddr, session: Session) {
        self.sessions.insert(address, session);
    }

    pub fn remove(&self, address: &SocketAddr) {
        self.sessions.remove(address);
    }
}

pub struct Session {
    console: Arc<quake_console::Console>,
    player_state: Arc<quake_entity::EntityState>,
}

impl Session {
    pub async fn new() -> anyhow::Result<Self> {
        let console = Arc::new(quake_console::Console::default());
        let player_state = Arc::new(quake_entity::EntityState::default());

        Self::register_session_commands(console.clone(), player_state.clone()).await?;

        Ok(Self {
            console,
            player_state,
        })
    }

    async fn register_session_commands(
        console: Arc<quake_console::Console>,
        entity_state: Arc<quake_entity::EntityState>,
    ) -> anyhow::Result<()> {
        let entity_commands = quake_entity::commands::EntityCommands::new(entity_state.clone());
        console
            .register_commands_handler(
                quake_entity::commands::EntityCommands::BUILTIN_COMMANDS,
                entity_commands,
            )
            .await
    }
}
