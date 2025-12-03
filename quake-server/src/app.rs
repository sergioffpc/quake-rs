use crate::Args;
use std::fs::File;
use std::sync::Arc;
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

    let app = App::new(args)?;
    app.start()
}

struct App {
    runtime: tokio::runtime::Runtime,
    console: Arc<quake_console::Console>,

    server_manager: Arc<quake_network::server::ServerManager>,
}

impl App {
    fn new(args: Args) -> anyhow::Result<Self> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;
        let resources = runtime.block_on(async {
            Arc::new(
                quake_resources::Resources::new(args.base_path)
                    .await
                    .unwrap(),
            )
        });
        let console = Arc::new(quake_console::Console::default());

        let resources_commands =
            quake_resources::commands::ResourcesCommands::new(resources.clone());
        runtime.block_on(async {
            console
                .register_commands_handler(
                    quake_resources::commands::ResourcesCommands::BUILTIN_COMMANDS,
                    resources_commands.clone(),
                )
                .await
                .unwrap();
        });
        let console_commands =
            quake_console::commands::ConsoleCommands::new(console.clone(), resources.clone());
        runtime.block_on(async {
            console
                .register_commands_handler(
                    quake_console::commands::ConsoleCommands::BUILTIN_COMMANDS,
                    console_commands.clone(),
                )
                .await
                .unwrap();
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
            console,
            server_manager,
        })
    }

    fn start(&self) -> anyhow::Result<()> {
        let handles = vec![
            self.spawn_tick_loop(),
            self.spawn_repl_loop(),
            self.spawn_accept_loop()?,
        ];
        self.runtime.block_on(async {
            for handle in handles {
                handle.await.unwrap();
            }
        });
        Ok(())
    }

    fn spawn_tick_loop(&self) -> JoinHandle<()> {
        let console = self.console.clone();
        self.runtime.spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                console.execute().await.unwrap();
            }
        })
    }

    fn spawn_repl_loop(&self) -> JoinHandle<()> {
        let console = self.console.clone();
        self.runtime
            .spawn(async move { console.repl().await.unwrap() })
    }

    fn spawn_accept_loop(&self) -> anyhow::Result<JoinHandle<()>> {
        let server_manager = self.server_manager.clone();
        let handle = self
            .runtime
            .spawn(async move { server_manager.accept().await });
        Ok(handle)
    }
}
