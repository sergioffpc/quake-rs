use crate::Args;
use std::fs::File;
use std::net::SocketAddr;
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

    let app = App::new(args.base_path)?;
    app.start(args.listen.parse()?)
}

struct App {
    runtime: tokio::runtime::Runtime,
    console: Arc<quake_console::Console>,
}

impl App {
    fn new<P>(path: P) -> anyhow::Result<Self>
    where
        P: AsRef<std::path::Path>,
    {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;
        let resources = Arc::new(quake_resources::Resources::new(path)?);
        let console = Arc::new(quake_console::Console::default());

        let resources_builtins =
            quake_resources::builtins::ResourcesBuiltins::new(resources.clone());
        runtime.block_on(async {
            console
                .register_commands_handler(
                    quake_resources::builtins::ResourcesBuiltins::BUILTIN_COMMANDS,
                    resources_builtins.clone(),
                )
                .await
                .unwrap();
        });
        let console_builtins =
            quake_console::builtins::ConsoleBuiltins::new(console.clone(), resources.clone());
        runtime.block_on(async {
            console
                .register_commands_handler(
                    quake_console::builtins::ConsoleBuiltins::BUILTIN_COMMANDS,
                    console_builtins.clone(),
                )
                .await
                .unwrap();
        });

        Ok(Self { runtime, console })
    }

    fn start(&self, address: SocketAddr) -> anyhow::Result<()> {
        let handles = vec![
            self.spawn_tick_loop(),
            self.spawn_repl_loop(),
            self.spawn_listen_loop(address)?,
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

    fn spawn_listen_loop(&self, address: SocketAddr) -> anyhow::Result<JoinHandle<()>> {
        let server_manager = Arc::new(quake_network::server::ServerManager::new(address)?);
        let handle = self
            .runtime
            .spawn(async move { server_manager.accept().await });
        Ok(handle)
    }
}
