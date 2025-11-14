use crate::Args;
use std::fs::File;
use std::sync::{Arc, Mutex, RwLock};
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
        App::new(args.base_path, args.listen)
            .await?
            .server_manager
            .lock()
            .unwrap()
            .listen()
            .await
    })
}

struct App {
    resources: Arc<RwLock<quake_resources::Resources>>,
    console: Arc<Mutex<quake_console::Console>>,
    server_manager: Arc<Mutex<quake_network::server::ServerManager>>,
}

impl App {
    async fn new<P, A>(path: P, address: A) -> anyhow::Result<Self>
    where
        P: AsRef<std::path::Path>,
        A: tokio::net::ToSocketAddrs,
    {
        let resources = Arc::new(RwLock::new(quake_resources::Resources::new(path)?));
        let console = Arc::new(Mutex::new(quake_console::Console::new(resources.clone())));
        let server_manager = Arc::new(Mutex::new(
            quake_network::server::ServerManager::new(address).await?,
        ));

        let resources_builtins =
            quake_resources::builtins::ResourcesBuiltins::new(resources.clone());
        {
            let mut console = console
                .lock()
                .map_err(|_| anyhow::anyhow!("Failed to lock console"))?;
            console.register_commands_handler(
                quake_resources::builtins::ResourcesBuiltins::BUILTIN_COMMANDS,
                resources_builtins.clone(),
            );
        }
        let console_builtins = quake_console::builtins::ConsoleBuiltins::new(console.clone());
        {
            let mut console = console
                .lock()
                .map_err(|_| anyhow::anyhow!("Failed to lock console"))?;
            console.register_commands_handler(
                quake_console::builtins::ConsoleBuiltins::BUILTIN_COMMANDS,
                console_builtins.clone(),
            );
        }
        let server_manager_builtins =
            quake_network::builtins::ServerBuiltins::new(server_manager.clone());
        {
            let mut console = console
                .lock()
                .map_err(|_| anyhow::anyhow!("Failed to lock console"))?;
            console.register_commands_handler(
                quake_network::builtins::ServerBuiltins::BUILTIN_COMMANDS,
                server_manager_builtins.clone(),
            );
        }

        Ok(Self {
            resources,
            console,
            server_manager,
        })
    }
}
