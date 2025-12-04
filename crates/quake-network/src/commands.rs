use crate::client::ClientManager;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct ClientCommands {
    client_manager: Arc<Mutex<ClientManager>>,
}

impl ClientCommands {
    pub const BUILTIN_COMMANDS: &'static [&'static str] =
        &["connect", "disconnect", "reconnect", "rexec"];

    pub fn new(client_manager: Arc<Mutex<ClientManager>>) -> Self {
        Self { client_manager }
    }

    async fn connect(
        &mut self,
        args: &[&str],
    ) -> anyhow::Result<(&[u8], quake_traits::ControlFlow)> {
        self.client_manager
            .lock()
            .await
            .connect(args[0].parse()?)
            .await?;
        Ok((&[], quake_traits::ControlFlow::Poll))
    }

    async fn disconnect(&mut self) -> anyhow::Result<(&[u8], quake_traits::ControlFlow)> {
        self.client_manager.lock().await.disconnect().await?;
        Ok((&[], quake_traits::ControlFlow::Poll))
    }

    async fn reconnect(&mut self) -> anyhow::Result<(&[u8], quake_traits::ControlFlow)> {
        self.client_manager.lock().await.reconnect().await?;
        Ok((&[], quake_traits::ControlFlow::Poll))
    }

    async fn rexec(&mut self, args: &[&str]) -> anyhow::Result<(&[u8], quake_traits::ControlFlow)> {
        let message = args.join(" ");
        let (mut tx, mut rx) = self.client_manager.lock().await.open_stream().await?;
        tx.write(format!("\x04{message}").as_bytes()).await?;
        tx.finish()?;

        let output = rx.read_to_end(usize::MAX).await?;

        Ok((output.leak(), quake_traits::ControlFlow::Poll))
    }
}

#[async_trait::async_trait]
impl quake_traits::CommandHandler for ClientCommands {
    async fn handle_command(
        &mut self,
        command: &[&str],
    ) -> anyhow::Result<(&[u8], quake_traits::ControlFlow)> {
        match command[0] {
            "connect" => self.connect(&command[1..]).await,
            "disconnect" => self.disconnect().await,
            "reconnect" => self.reconnect().await,
            "rexec" => self.rexec(&command[1..]).await,
            _ => Ok((&[], quake_traits::ControlFlow::Poll)),
        }
    }
}
