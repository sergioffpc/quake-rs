use clap::Parser;
use std::path::PathBuf;
use std::rc::Rc;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter};

mod builtins;

#[derive(Parser)]
#[command(name = env!("CARGO_PKG_NAME"))]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Quake client")]
struct Args {
    #[arg(long, default_value = "resources/")]
    base_path: PathBuf,
}

fn main() {
    let args = Args::parse();

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let resources = Rc::new(quake_resources::Resources::new(&args.base_path).unwrap());
    let mut console = quake_console::Console::new(resources.clone());
    console.register_command("connect", builtins::connect());
    console.register_command("reconnect", builtins::reconnect());
    console.register_command("disconnect", builtins::disconnect());
    console.register_command("playdemo", builtins::playdemo(resources.clone()));
    console.repl().unwrap();
}
