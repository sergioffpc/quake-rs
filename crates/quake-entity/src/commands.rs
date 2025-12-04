use crate::EntityState;
use quake_traits::ControlFlow;
use std::sync::Arc;

#[derive(Clone)]
pub struct EntityCommands {
    entity: Arc<EntityState>,
}

impl EntityCommands {
    pub const BUILTIN_COMMANDS: &'static [&'static str] = &["name"];

    pub fn new(entity: Arc<EntityState>) -> Self {
        Self { entity }
    }

    async fn builtin_name(&mut self, args: &[&str]) -> anyhow::Result<ControlFlow> {
        Ok(ControlFlow::Poll)
    }
}

#[async_trait::async_trait]
impl quake_traits::CommandHandler for EntityCommands {
    async fn handle_command(&mut self, command: &[&str]) -> anyhow::Result<ControlFlow> {
        match command[0] {
            "name" => self.builtin_name(&command[1..]).await,
            _ => Ok(ControlFlow::Poll),
        }
    }
}
