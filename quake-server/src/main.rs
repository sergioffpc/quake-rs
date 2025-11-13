use clap::Parser;
use std::fs::File;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter};

mod app;
mod builtins;

#[derive(Parser)]
#[command(name = env!("CARGO_PKG_NAME"))]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Quake Server")]
struct Args {
    #[arg(long, default_value = "resources/", help = "Base path for resources")]
    base_path: PathBuf,

    #[arg(long, default_value_t = false, help = "Run as a daemon")]
    daemon: bool,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let args = Args::parse();
    let resources = Arc::new(RwLock::new(quake_resources::Resources::new(
        &args.base_path,
    )?));
    let console = Arc::new(Mutex::new(quake_console::Console::new(resources.clone())));

    let resources_builtins = quake_resources::builtins::ResourcesBuiltins::new(resources.clone());
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
            Ok(_) => println!("Async daemon started"),
            Err(e) => {
                eprintln!("Error starting daemon: {}", e);
                std::process::exit(1);
            }
        }
    }

    let runtime = tokio::runtime::Runtime::new()?;

    if args.daemon {
        runtime.block_on(async { app::run_app().await })?;
    } else {
        runtime.spawn(app::run_app());

        let mut console = console
            .lock()
            .map_err(|_| anyhow::anyhow!("Failed to lock console"))?;
        console.repl()?;
    }

    Ok(())
}
