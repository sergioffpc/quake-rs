use crate::client::ClientManager;
use crate::server::ServerManager;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct ClientBuiltins {
    inner: Arc<Mutex<ClientManager>>,
}

impl ClientBuiltins {
    pub const BUILTIN_COMMANDS: &'static [&'static str] = &["connect", "reconnect", "disconnect"];

    pub fn new(manager: Arc<Mutex<ClientManager>>) -> Self {
        Self { inner: manager }
    }

    pub fn builtin_connect(&mut self, args: &[&str]) -> anyhow::Result<()> {
        Ok(())
    }

    pub fn builtin_reconnect(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    pub fn builtin_disconnect(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl quake_traits::CommandHandler for ClientBuiltins {
    async fn handle_command(&mut self, command: &[&str]) -> anyhow::Result<()> {
        match command[0] {
            "connect" => self.builtin_connect(&command[1..]),
            "reconnect" => self.builtin_reconnect(),
            "disconnect" => self.builtin_disconnect(),
            _ => Ok(()),
        }
    }
}

#[derive(Clone)]
pub struct ServerBuiltins {
    inner: Arc<Mutex<ServerManager>>,
}

impl ServerBuiltins {
    pub const BUILTIN_COMMANDS: &'static [&'static str] = &[];

    pub fn new(manager: Arc<Mutex<ServerManager>>) -> Self {
        Self { inner: manager }
    }
}

#[async_trait::async_trait]
impl quake_traits::CommandHandler for ServerBuiltins {
    async fn handle_command(&mut self, command: &[&str]) -> anyhow::Result<()> {
        match command[0] {
            _ => Ok(()),
        }
    }
}
