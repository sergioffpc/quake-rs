use crate::Args;
use std::fs::File;
use std::net::ToSocketAddrs;
use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;
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
    app.start(args.listen)
}

struct App {
    console: Arc<quake_console::Console>,
}

impl App {
    fn new<P>(path: P) -> anyhow::Result<Self>
    where
        P: AsRef<std::path::Path>,
    {
        let resources = Arc::new(quake_resources::Resources::new(path)?);
        let console = Arc::new(quake_console::Console::default());

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

        Ok(Self { console })
    }

    fn start<A>(&self, address: A) -> anyhow::Result<()>
    where
        A: ToSocketAddrs,
    {
        let handles = vec![
            self.spawn_tick_loop(),
            self.spawn_repl_loop(),
            self.spawn_listen_loop(address)?,
        ];

        for handle in handles {
            handle.join().unwrap();
        }

        Ok(())
    }

    fn spawn_tick_loop(&self) -> JoinHandle<()> {
        let console = self.console.clone();
        thread::spawn(move || {
            loop {
                thread::sleep(std::time::Duration::from_millis(30));
                console.execute().unwrap();
            }
        })
    }

    fn spawn_repl_loop(&self) -> JoinHandle<()> {
        let console = self.console.clone();
        thread::spawn(move || console.repl().unwrap())
    }

    fn spawn_listen_loop<A>(&self, address: A) -> anyhow::Result<JoinHandle<()>>
    where
        A: ToSocketAddrs,
    {
        let server_manager = Arc::new(quake_network::server::ServerManager::new(address)?);
        let server_manager_builtins =
            quake_network::builtins::ServerBuiltins::new(server_manager.clone());
        {
            self.console.register_commands_handler(
                quake_network::builtins::ServerBuiltins::BUILTIN_COMMANDS,
                server_manager_builtins.clone(),
            )?;
        }
        let handle = thread::spawn(move || server_manager.start().unwrap());
        Ok(handle)
    }
}
