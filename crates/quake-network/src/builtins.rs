use crate::client::ClientManager;
use crate::server::ServerManager;
use quake_traits::ControlFlow;
use std::sync::Arc;

#[derive(Clone)]
pub struct ClientBuiltins {
    inner: Arc<ClientManager>,
}

impl ClientBuiltins {
    pub const BUILTIN_COMMANDS: &'static [&'static str] = &["connect", "reconnect", "disconnect"];

    pub fn new(inner: Arc<ClientManager>) -> Self {
        Self { inner }
    }

    pub fn builtin_connect(&mut self, args: &[&str]) -> anyhow::Result<ControlFlow> {
        Ok(ControlFlow::Poll)
    }

    pub fn builtin_reconnect(&mut self) -> anyhow::Result<ControlFlow> {
        Ok(ControlFlow::Poll)
    }

    pub fn builtin_disconnect(&mut self) -> anyhow::Result<ControlFlow> {
        Ok(ControlFlow::Poll)
    }
}

#[async_trait::async_trait]
impl quake_traits::CommandHandler for ClientBuiltins {
    async fn handle_command(&mut self, command: &[&str]) -> anyhow::Result<ControlFlow> {
        match command[0] {
            "connect" => self.builtin_connect(&command[1..]),
            "reconnect" => self.builtin_reconnect(),
            "disconnect" => self.builtin_disconnect(),
            _ => Ok(ControlFlow::Poll),
        }
    }
}

#[derive(Clone)]
pub struct ServerBuiltins {
    inner: Arc<ServerManager>,
}

impl ServerBuiltins {
    pub const BUILTIN_COMMANDS: &'static [&'static str] = &[];

    pub fn new(inner: Arc<ServerManager>) -> Self {
        Self { inner }
    }
}

#[async_trait::async_trait]
impl quake_traits::CommandHandler for ServerBuiltins {
    async fn handle_command(&mut self, command: &[&str]) -> anyhow::Result<ControlFlow> {
        match command[0] {
            _ => Ok(ControlFlow::Poll),
        }
    }
}
