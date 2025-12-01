use crate::client::ClientManager;
use quake_traits::ControlFlow;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct ClientCommands {
    client_manager: Arc<Mutex<ClientManager>>,
}

impl ClientCommands {
    pub const BUILTIN_COMMANDS: &'static [&'static str] = &["connect", "disconnect", "reconnect"];

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
}

#[async_trait::async_trait]
impl quake_traits::CommandHandler for ClientCommands {
    async fn handle_command(&mut self, command: &[&str]) -> anyhow::Result<ControlFlow> {
        match command[0] {
            "connect" => self.builtin_connect(&command[1..]).await,
            "disconnect" => self.builtin_disconnect().await,
            "reconnect" => self.builtin_reconnect().await,
            _ => Ok(ControlFlow::Poll),
        }
    }
}
