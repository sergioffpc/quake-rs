use crate::client::ClientManager;
use crate::server::ServerManager;
use quake_traits::ControlFlow;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct ClientCommands {
    client_manager: Arc<Mutex<ClientManager>>,
}

impl ClientCommands {
    pub const BUILTIN_COMMANDS: &'static [&'static str] =
        &["connect", "disconnect", "reconnect", "say"];

    pub fn new(client_manager: Arc<Mutex<ClientManager>>) -> Self {
        Self { client_manager }
    }

    async fn builtin_connect(&mut self, args: &[&str]) -> anyhow::Result<ControlFlow> {
        self.client_manager
            .lock()
            .await
            .connect(args[0].parse()?)
            .await?;
        Ok(ControlFlow::Poll)
    }

    async fn builtin_disconnect(&mut self) -> anyhow::Result<ControlFlow> {
        self.client_manager.lock().await.disconnect().await?;
        Ok(ControlFlow::Poll)
    }

    async fn builtin_reconnect(&mut self) -> anyhow::Result<ControlFlow> {
        self.client_manager.lock().await.reconnect().await?;
        Ok(ControlFlow::Poll)
    }

    async fn builtin_say(&mut self, args: &[&str]) -> anyhow::Result<ControlFlow> {
        let message = args.join(" ");

        let (mut tx, _rx) = self.client_manager.lock().await.open_stream().await?;
        tx.write(format!("\x04say {message}").as_bytes()).await?;
        tx.finish()?;

        Ok(ControlFlow::Poll)
    }
}

#[async_trait::async_trait]
impl quake_traits::CommandHandler for ClientCommands {
    async fn handle_command(&mut self, command: &[&str]) -> anyhow::Result<ControlFlow> {
        match command[0] {
            "connect" => self.builtin_connect(&command[1..]).await,
            "disconnect" => self.builtin_disconnect().await,
            "reconnect" => self.builtin_reconnect().await,
            "say" => self.builtin_say(&command[1..]).await,
            _ => Ok(ControlFlow::Poll),
        }
    }
}

#[derive(Clone)]
pub struct ServerCommands {
    server_manager: Arc<ServerManager>,
}

impl ServerCommands {
    pub const BUILTIN_COMMANDS: &'static [&'static str] = &["say"];

    pub fn new(server_manager: Arc<ServerManager>) -> Self {
        Self { server_manager }
    }

    async fn builtin_say(&mut self, args: &[&str]) -> anyhow::Result<ControlFlow> {
        self.server_manager
            .broadcast(args.join(" ").into_bytes())
            .await?;
        Ok(ControlFlow::Poll)
    }
}

#[async_trait::async_trait]
impl quake_traits::CommandHandler for ServerCommands {
    async fn handle_command(&mut self, command: &[&str]) -> anyhow::Result<ControlFlow> {
        match command[0] {
            "say" => self.builtin_say(&command[1..]).await,
            _ => Ok(ControlFlow::Poll),
        }
    }
}
