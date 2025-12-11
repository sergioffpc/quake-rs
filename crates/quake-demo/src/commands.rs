use crate::DemoManager;
use std::sync::Arc;
use tracing::info;

#[derive(Clone)]
pub struct DemoCommands {
    console_manager: Arc<quake_console::ConsoleManager>,
    demo_manager: Arc<DemoManager>,
}

impl DemoCommands {
    pub const BUILTIN_COMMANDS: &'static [&'static str] =
        &["playdemo", "stopdemo", "demos", "startdemos"];

    pub fn new(
        console_manager: Arc<quake_console::ConsoleManager>,
        demo_manager: Arc<DemoManager>,
    ) -> Self {
        Self {
            console_manager,
            demo_manager,
        }
    }

    async fn playdemo(&self, args: &[&str]) -> anyhow::Result<(String, quake_traits::ControlFlow)> {
        self.demo_manager.start(args[0]).await?;
        self.console_manager
            .append_text("disconnect; connect 127.0.0.1 26001")
            .await;

        Ok((String::default(), quake_traits::ControlFlow::Poll))
    }

    async fn stopdemo(&self, args: &[&str]) -> anyhow::Result<(String, quake_traits::ControlFlow)> {
        self.demo_manager.stop().await?;
        self.console_manager.append_text("disconnect").await;

        Ok((String::default(), quake_traits::ControlFlow::Poll))
    }

    async fn demos(&self) -> anyhow::Result<(String, quake_traits::ControlFlow)> {
        Ok((String::default(), quake_traits::ControlFlow::Poll))
    }

    async fn startdemos(
        &self,
        args: &[&str],
    ) -> anyhow::Result<(String, quake_traits::ControlFlow)> {
        info!("Starting demos: {:?}", args);

        Ok((String::default(), quake_traits::ControlFlow::Poll))
    }
}

#[async_trait::async_trait]
impl quake_traits::CommandHandler for DemoCommands {
    async fn handle_command(
        &mut self,
        command: &[&str],
    ) -> anyhow::Result<(String, quake_traits::ControlFlow)> {
        match command[0] {
            "playdemo" => self.playdemo(&command[1..]).await,
            "stopdemo" => self.stopdemo(&command[1..]).await,
            "demos" => self.demos().await,
            "startdemos" => self.startdemos(&command[1..]).await,
            _ => Ok((String::default(), quake_traits::ControlFlow::Poll)),
        }
    }
}
