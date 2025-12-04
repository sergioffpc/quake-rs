use crate::Args;
use crate::stream::QuakeStreamHandlerBuilder;
use std::fs::File;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tracing::log::{error, info};

pub fn run_app(args: Args) -> anyhow::Result<()> {
    if args.daemon {
        let stdout = File::create("/tmp/async_daemon.out")?;
        let stderr = File::create("/tmp/async_daemon.err")?;

        let daemonize = daemonize::Daemonize::new()
            .pid_file("/tmp/async_daemon.pid")
            .chown_pid_file(true)
            .working_directory("/tmp")
            .stdout(stdout)
            .stderr(stderr);

        match daemonize.start() {
            Ok(_) => info!("Async daemon started"),
            Err(e) => {
                error!("Error starting daemon: {}", e);
                std::process::exit(1);
            }
        }
    }

    App::new(args)?.start();

    Ok(())
}

struct App {
    runtime: Runtime,
    resources_manager: Arc<quake_resources::ResourcesManager>,
    server_manager: Arc<quake_network::server::ServerManager>,
}

impl App {
    fn new(args: Args) -> anyhow::Result<Self> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;
        let resources_manager = runtime.block_on(async {
            Arc::new(
                quake_resources::ResourcesManager::new(args.base_path)
                    .await
                    .unwrap(),
            )
        });
        let server_manager = runtime.block_on(async {
            Arc::new(
                quake_network::server::ServerManager::new(
                    args.listen.parse().unwrap(),
                    args.cert_path,
                    args.key_path,
                )
                .await
                .unwrap(),
            )
        });

        Ok(Self {
            runtime,
            resources_manager,
            server_manager,
        })
    }

    fn start(&self) {
        let server_manager = self.server_manager.clone();
        let stream_handler_builder = QuakeStreamHandlerBuilder::new(self.resources_manager.clone());
        self.runtime
            .block_on(async move { server_manager.accept(stream_handler_builder).await.unwrap() });
    }
}
