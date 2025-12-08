use crate::Args;
use crate::stream::ServerStreamHandlerBuilder;
use std::fs::File;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;
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
    console_manager: Arc<quake_console::ConsoleManager>,
    resources_manager: Arc<quake_resources::ResourcesManager>,
    server_manager: Arc<quake_network::server::ServerManager>,
}

impl App {
    fn new(args: Args) -> anyhow::Result<Self> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;
        let console_manager = Arc::new(quake_console::ConsoleManager::default());
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
                    console_manager.clone(),
                )
                .await
                .unwrap(),
            )
        });

        Self::register_console_commands(
            &runtime,
            console_manager.clone(),
            resources_manager.clone(),
            args.stuffcmds,
        )?;
        Self::register_resources_commands(
            &runtime,
            console_manager.clone(),
            resources_manager.clone(),
        )?;
        Self::register_network_commands(&runtime, console_manager.clone(), server_manager.clone())?;

        Ok(Self {
            runtime,
            console_manager,
            resources_manager,
            server_manager,
        })
    }

    fn start(&self) {
        self.runtime.block_on(async {
            let handles = [self.tick_loop(), self.repl_loop(), self.accept_loop()];
            for handle in handles {
                handle
                    .unwrap()
                    .await
                    .expect("Error running task in background");
            }
        });
    }

    fn tick_loop(&self) -> anyhow::Result<JoinHandle<()>> {
        let console_manager = self.console_manager.clone();

        Ok(self.runtime.spawn(async move {
            loop {
                let sys_tick_rate = console_manager
                    .get::<f32>("sys.tick.rate")
                    .await
                    .unwrap_or(0.05);
                tokio::time::sleep(std::time::Duration::from_secs_f32(sys_tick_rate)).await;
            }
        }))
    }

    fn repl_loop(&self) -> anyhow::Result<JoinHandle<()>> {
        let console_manager = self.console_manager.clone();

        Ok(self.runtime.spawn(async move {
            if let Err(e) = console_manager.repl().await {
                error!("Error running REPL: {}", e);
            }
        }))
    }

    fn accept_loop(&self) -> anyhow::Result<JoinHandle<()>> {
        let server_manager = self.server_manager.clone();
        let stream_handler_builder =
            ServerStreamHandlerBuilder::new(self.resources_manager.clone());

        Ok(self.runtime.spawn(async move {
            if let Err(e) = server_manager.accept(stream_handler_builder).await {
                error!("Error accepting connections: {}", e);
            }
        }))
    }

    fn register_console_commands(
        runtime: &Runtime,
        console_manager: Arc<quake_console::ConsoleManager>,
        resources_manager: Arc<quake_resources::ResourcesManager>,
        stuffcmds: Vec<String>,
    ) -> anyhow::Result<()> {
        let mut console_manager_commands = quake_console::commands::ConsoleCommands::new(
            console_manager.clone(),
            resources_manager,
        );
        console_manager_commands.extend_stuffcmds(stuffcmds);

        runtime.block_on(console_manager.register_commands_handler(
            quake_console::commands::ConsoleCommands::BUILTIN_COMMANDS,
            console_manager_commands,
        ))
    }

    fn register_resources_commands(
        runtime: &Runtime,
        console_manager: Arc<quake_console::ConsoleManager>,
        resources_manager: Arc<quake_resources::ResourcesManager>,
    ) -> anyhow::Result<()> {
        let resources_manager_commands =
            quake_resources::commands::ResourcesCommands::new(resources_manager);
        runtime.block_on(console_manager.register_commands_handler(
            quake_resources::commands::ResourcesCommands::BUILTIN_COMMANDS,
            resources_manager_commands,
        ))
    }

    fn register_network_commands(
        runtime: &Runtime,
        console_manager: Arc<quake_console::ConsoleManager>,
        server_manager: Arc<quake_network::server::ServerManager>,
    ) -> anyhow::Result<()> {
        let server_manager_commands = quake_network::commands::ServerCommands::new(server_manager);
        runtime.block_on(console_manager.register_commands_handler(
            quake_network::commands::ServerCommands::BUILTIN_COMMANDS,
            server_manager_commands,
        ))
    }
}
