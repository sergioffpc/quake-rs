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

    pub async fn builtin_connect(&self, args: &[&str]) -> anyhow::Result<ControlFlow> {
        self.inner.connect(args[0]).await?;
        Ok(ControlFlow::Poll)
    }

    pub fn builtin_reconnect(&self) -> anyhow::Result<ControlFlow> {
        Ok(ControlFlow::Poll)
    }

    pub fn builtin_disconnect(&self) -> anyhow::Result<ControlFlow> {
        Ok(ControlFlow::Poll)
    }
}

#[async_trait::async_trait]
impl quake_traits::CommandHandler for ClientBuiltins {
    async fn handle_command(&mut self, command: &[&str]) -> anyhow::Result<ControlFlow> {
        match command[0] {
            "connect" => self.builtin_connect(&command[1..]).await,
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
