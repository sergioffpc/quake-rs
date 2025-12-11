use std::sync::Arc;
use tokio::sync::Mutex;

pub mod commands;

mod v3;

pub struct DemoManager {
    resources_manager: Arc<quake_resources::ResourcesManager>,
    server_manager: Arc<quake_network::server::ServerManager>,
}

impl DemoManager {
    pub async fn new(
        console_manager: Arc<quake_console::ConsoleManager>,
        resources_manager: Arc<quake_resources::ResourcesManager>,
    ) -> anyhow::Result<Self> {
        let cert_path: Option<&std::path::Path> = None;
        let key_path: Option<&std::path::Path> = None;
        let server_manager = Arc::new(
            quake_network::server::ServerManager::new(
                "127.0.0.1:26001".parse()?,
                cert_path,
                key_path,
                console_manager,
            )
            .await?,
        );

        Ok(Self {
            resources_manager,
            server_manager,
        })
    }

    pub async fn start<P>(&self, path: P) -> anyhow::Result<()>
    where
        P: AsRef<std::path::Path>,
    {
        let server_manager = self.server_manager.clone();

        let iter = self
            .resources_manager
            .by_name::<quake_resources::dem::Dem>(path.as_ref().to_str().unwrap())
            .await?
            .into_iter();

        let mut packet_dispatcher = quake_network::PacketDispatcher::default();
        packet_dispatcher.register_handler(
            v3::protocol::DemoPacketHandler::OPCODE,
            Box::new(v3::protocol::DemoPacketHandler::new(iter)),
        );

        server_manager
            .accept(Arc::new(Mutex::new(packet_dispatcher)))
            .await
    }

    pub async fn stop(&self) -> anyhow::Result<()> {
        self.server_manager.close()
    }
}
