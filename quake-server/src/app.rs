use crate::Args;
use std::fs::File;
use std::sync::Arc;
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

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async {
        let app = App::new(args.base_path, args.listen).await?;
        app.server_manager.listen().await
    })
}

struct App {
    resources: Arc<quake_resources::Resources>,
    console: Arc<quake_console::Console>,
    server_manager: Arc<quake_network::server::ServerManager>,
}

impl App {
    async fn new<P, A>(path: P, address: A) -> anyhow::Result<Self>
    where
        P: AsRef<std::path::Path>,
        A: tokio::net::ToSocketAddrs,
    {
        let resources = Arc::new(quake_resources::Resources::new(path)?);
        let console = Arc::new(quake_console::Console::default());
        let server_manager = Arc::new(quake_network::server::ServerManager::new(address).await?);

        let resources_builtins =
            quake_resources::builtins::ResourcesBuiltins::new(resources.clone());
        {
            console.register_commands_handler(
                quake_resources::builtins::ResourcesBuiltins::BUILTIN_COMMANDS,
                resources_builtins.clone(),
            )?;
        }
        let console_builtins =
            quake_console::builtins::ConsoleBuiltins::new(console.clone(), resources.clone());
        {
            console.register_commands_handler(
                quake_console::builtins::ConsoleBuiltins::BUILTIN_COMMANDS,
                console_builtins.clone(),
            )?;
        }
        let server_manager_builtins =
            quake_network::builtins::ServerBuiltins::new(server_manager.clone());
        {
            console.register_commands_handler(
                quake_network::builtins::ServerBuiltins::BUILTIN_COMMANDS,
                server_manager_builtins.clone(),
            )?;
        }

        Ok(Self {
            resources,
            console,
            server_manager,
        })
    }
}
