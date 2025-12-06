use crate::client::ClientManager;
use crate::server::ServerManager;
use std::sync::Arc;
use tabled::Table;
use tabled::settings::{Padding, Style};
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
    ) -> anyhow::Result<(String, quake_traits::ControlFlow)> {
        let address = Self::parse_address(args)?;
        self.client_manager.lock().await.connect(address).await?;
        Ok((String::default(), quake_traits::ControlFlow::Poll))
    }

    fn parse_address(args: &[&str]) -> anyhow::Result<std::net::SocketAddr> {
        const DEFAULT_HOST: &str = "127.0.0.1";
        const DEFAULT_PORT: u16 = 26000;

        let address_str = match args.len() {
            0 => format!("{}:{}", DEFAULT_HOST, DEFAULT_PORT),
            1 => format!("{}:{}", args[0], DEFAULT_PORT),
            2 => format!("{}:{}", args[0], args[1]),
            _ => return Err(anyhow::anyhow!("Too many arguments")),
        };

        Ok(address_str.parse()?)
    }

    async fn disconnect(&mut self) -> anyhow::Result<(String, quake_traits::ControlFlow)> {
        self.client_manager.lock().await.disconnect().await?;
        Ok((String::default(), quake_traits::ControlFlow::Poll))
    }

    async fn reconnect(&mut self) -> anyhow::Result<(String, quake_traits::ControlFlow)> {
        self.client_manager.lock().await.reconnect().await?;
        Ok((String::default(), quake_traits::ControlFlow::Poll))
    }

    async fn rexec(
        &mut self,
        args: &[&str],
    ) -> anyhow::Result<(String, quake_traits::ControlFlow)> {
        let message = args.join(" ");
        let (mut tx, mut rx) = self.client_manager.lock().await.open_stream().await?;
        tx.write(format!("\x04{message}").as_bytes()).await?;
        tx.finish()?;

        let output = rx.read_to_end(usize::MAX).await?;

        Ok((String::from_utf8(output)?, quake_traits::ControlFlow::Poll))
    }
}

#[async_trait::async_trait]
impl quake_traits::CommandHandler for ClientCommands {
    async fn handle_command(
        &mut self,
        command: &[&str],
    ) -> anyhow::Result<(String, quake_traits::ControlFlow)> {
        match command[0] {
            "connect" => self.connect(&command[1..]).await,
            "disconnect" => self.disconnect().await,
            "reconnect" => self.reconnect().await,
            "rexec" => self.rexec(&command[1..]).await,
            _ => Ok((String::default(), quake_traits::ControlFlow::Poll)),
        }
    }
}

#[derive(Clone)]
pub struct ServerCommands {
    server_manager: Arc<ServerManager>,
}

impl ServerCommands {
    pub const BUILTIN_COMMANDS: &'static [&'static str] = &["net_stats"];

    pub fn new(server_manager: Arc<ServerManager>) -> Self {
        Self { server_manager }
    }

    async fn net_stats(&mut self) -> anyhow::Result<(String, quake_traits::ControlFlow)> {
        let net_stats_data = self.server_manager.stats();
        let buffer = Table::new(vec![net_stats_data])
            .with(Style::re_structured_text())
            .with(Padding::new(1, 1, 0, 0))
            .to_string();
        Ok((buffer, quake_traits::ControlFlow::Poll))
    }
}

#[async_trait::async_trait]
impl quake_traits::CommandHandler for ServerCommands {
    async fn handle_command(
        &mut self,
        command: &[&str],
    ) -> anyhow::Result<(String, quake_traits::ControlFlow)> {
        match command[0] {
            "net_stats" => self.net_stats().await,
            _ => Ok((String::default(), quake_traits::ControlFlow::Poll)),
        }
    }
}
