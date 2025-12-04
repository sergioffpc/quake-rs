use crate::packets::{QuakeConnectionRequestHandler, QuakeConsoleRequestHandler};
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tracing::log::error;

#[derive(Clone)]
pub struct QuakeStreamHandlerBuilder {
    resources_manager: Arc<quake_resources::ResourcesManager>,
}

impl QuakeStreamHandlerBuilder {
    pub fn new(resources_manager: Arc<quake_resources::ResourcesManager>) -> Self {
        Self { resources_manager }
    }
}

#[async_trait::async_trait]
impl quake_network::StreamHandlerBuilder for QuakeStreamHandlerBuilder {
    async fn build(&self) -> anyhow::Result<Box<dyn quake_network::StreamHandler>> {
        Ok(Box::new(
            QuakeStream::new(self.resources_manager.clone()).await?,
        ))
    }
}

pub struct QuakeStream {
    packet_dispatcher: Arc<quake_network::PacketDispatcher>,
}

impl QuakeStream {
    pub async fn new(
        resources_manager: Arc<quake_resources::ResourcesManager>,
    ) -> anyhow::Result<Self> {
        let console_manager = Arc::new(quake_console::ConsoleManager::default());

        Self::register_console_commands(console_manager.clone(), resources_manager.clone()).await?;
        Self::register_resources_commands(console_manager.clone(), resources_manager.clone())
            .await?;

        let mut packet_dispatcher = quake_network::PacketDispatcher::default();
        packet_dispatcher.register_handler(
            QuakeConnectionRequestHandler::OPCODE,
            Box::new(QuakeConnectionRequestHandler),
        );
        packet_dispatcher.register_handler(
            QuakeConsoleRequestHandler::OPCODE,
            Box::new(QuakeConsoleRequestHandler::new(console_manager.clone())),
        );

        Ok(Self {
            packet_dispatcher: Arc::new(packet_dispatcher),
        })
    }

    async fn register_console_commands(
        console_manager: Arc<quake_console::ConsoleManager>,
        resources_manager: Arc<quake_resources::ResourcesManager>,
    ) -> anyhow::Result<()> {
        let console_commands = quake_console::commands::ConsoleCommands::new(
            console_manager.clone(),
            resources_manager.clone(),
        );
        console_manager
            .register_commands_handler(
                quake_console::commands::ConsoleCommands::BUILTIN_COMMANDS,
                console_commands,
            )
            .await
    }

    async fn register_resources_commands(
        console_manager: Arc<quake_console::ConsoleManager>,
        resources_manager: Arc<quake_resources::ResourcesManager>,
    ) -> anyhow::Result<()> {
        let resources_commands =
            quake_resources::commands::ResourcesCommands::new(resources_manager.clone());
        console_manager
            .register_commands_handler(
                quake_resources::commands::ResourcesCommands::BUILTIN_COMMANDS,
                resources_commands,
            )
            .await
    }
}

#[async_trait::async_trait]
impl quake_network::StreamHandler for QuakeStream {
    async fn handle_stream(
        &self,
        sender: &mut (dyn AsyncWrite + Unpin + Send),
        receiver: &mut (dyn AsyncRead + Unpin + Send),
    ) {
        let mut data = Vec::new();
        match receiver.read_to_end(&mut data).await {
            Ok(n) => {
                if n == 0 {
                    return;
                }
                match self.packet_dispatcher.dispatch(&data).await {
                    Ok(response) => {
                        if !response.is_empty() {
                            if let Err(e) = sender.write_all(&response).await {
                                error!("Error writing response: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Error handling request: {}", e);
                    }
                }
            }
            Err(e) => {
                error!("Error reading incoming stream: {}", e);
            }
        }
    }
}
